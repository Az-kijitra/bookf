//! bookf back end: database location, commands called from the Svelte front end.
//! File handling lives in [`store`], network lookups in [`lookup`].

mod db;
mod import;
mod lookup;
mod store;

use base64::Engine;
use db::Book;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use store::Library;
use tauri::{Manager, State};

const SETTINGS_FILE: &str = "settings.json";
/// Extensions accepted by "表紙画像を変更 → ファイルから".
const IMAGE_EXTS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

#[derive(Debug, Default, Serialize, Deserialize)]
struct Settings {
    db_path: Option<PathBuf>,
}

struct AppState {
    config_dir: PathBuf,
    lib: Mutex<Library>,
}

impl AppState {
    fn lib(&self) -> CmdResult<MutexGuard<'_, Library>> {
        self.lib.lock().map_err(err)
    }
}

#[derive(Serialize)]
struct LibraryInfo {
    db_path: String,
    count: usize,
}

impl LibraryInfo {
    fn of(lib: &Library) -> CmdResult<LibraryInfo> {
        Ok(LibraryInfo { db_path: lib.path.display().to_string(), count: lib.count()? })
    }
}

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn load_settings(config_dir: &Path) -> Settings {
    std::fs::read_to_string(config_dir.join(SETTINGS_FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_settings(config_dir: &Path, s: &Settings) -> CmdResult<()> {
    std::fs::create_dir_all(config_dir).map_err(err)?;
    std::fs::write(config_dir.join(SETTINGS_FILE), serde_json::to_string_pretty(s).map_err(err)?).map_err(err)
}

/// Reads a local image chosen by the user; returns the bytes and the normalised extension.
fn read_image_file(path: &Path) -> CmdResult<(Vec<u8>, &'static str)> {
    if !path.is_file() {
        return Err("画像ファイルが見つかりません".into());
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let ext = match IMAGE_EXTS.iter().find(|e| **e == ext) {
        Some(&"jpeg") => "jpg",
        Some(e) => e,
        None => return Err(format!("対応していない画像形式です（{}）", IMAGE_EXTS.join(" / "))),
    };
    let len = std::fs::metadata(path).map_err(err)?.len() as usize;
    if len > lookup::MAX_IMAGE_BYTES {
        return Err("画像が大きすぎます（10 MB まで）".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("ファイルを読み込めません: {e}"))?;
    Ok((bytes, ext))
}

#[tauri::command]
fn library_info(state: State<AppState>) -> CmdResult<LibraryInfo> {
    LibraryInfo::of(&*state.lib()?)
}

#[tauri::command]
fn list_books(state: State<AppState>) -> CmdResult<Vec<Book>> {
    db::list(&state.lib()?.conn).map_err(err)
}

#[tauri::command]
fn save_book(state: State<AppState>, book: Book) -> CmdResult<Book> {
    if book.title.trim().is_empty() {
        return Err("タイトルを入力してください。".into());
    }
    db::save(&state.lib()?.conn, &book).map_err(err)
}

#[tauri::command]
fn delete_book(state: State<AppState>, id: i64) -> CmdResult<()> {
    state.lib()?.delete_book(id)
}

#[tauri::command]
fn import_notion_csv(state: State<AppState>, path: String) -> CmdResult<import::ImportResult> {
    import::import_file(&state.lib()?.conn, Path::new(&path))
}

#[tauri::command]
async fn search_books(query: String) -> CmdResult<Vec<lookup::Candidate>> {
    lookup::search(&query).await
}

#[tauri::command]
async fn fetch_link_metadata(url: String) -> CmdResult<lookup::Candidate> {
    lookup::fetch_from_link(&url).await
}

/// Downloads the book's `cover_url` into `covers/` beside the database and records the relative path.
#[tauri::command]
async fn fetch_cover(state: State<'_, AppState>, id: i64) -> CmdResult<Book> {
    let url = state.lib()?.book(id)?.cover_url.ok_or("表紙の URL がありません")?;
    let (bytes, ext) = lookup::download(&url).await?;
    state.lib()?.replace_cover(id, &bytes, ext, None)
}

/// The cached cover as a `data:` URL (no asset-protocol scope needed), or None if it is missing.
#[tauri::command]
fn cover_data(state: State<AppState>, rel: String) -> CmdResult<Option<String>> {
    Ok(state
        .lib()?
        .read_cover(&rel)?
        .map(|(bytes, mime)| format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}

/// Sets the cover image from a local file (copied into `covers/`).
#[tauri::command]
fn set_cover_from_file(state: State<AppState>, id: i64, file_path: String) -> CmdResult<Book> {
    let (bytes, ext) = read_image_file(Path::new(&file_path))?;
    state.lib()?.replace_cover(id, &bytes, ext, None)
}

/// Sets the cover image from a custom remote URL (downloaded into `covers/`).
#[tauri::command]
async fn set_cover_from_url(state: State<'_, AppState>, id: i64, url: String) -> CmdResult<Book> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("URLが空です".into());
    }
    let (bytes, ext) = lookup::download(&url).await?;
    state.lib()?.replace_cover(id, &bytes, ext, Some(Some(url)))
}

/// Removes the cover image of a book.
#[tauri::command]
fn remove_cover(state: State<AppState>, id: i64) -> CmdResult<Book> {
    state.lib()?.clear_cover(id)
}

/// Switches to another database file (e.g. in the Google Drive folder). With `copy_current`,
/// the current database and its covers are copied there first when the target does not exist yet.
#[tauri::command]
fn set_db_path(state: State<AppState>, path: String, copy_current: bool) -> CmdResult<LibraryInfo> {
    let target = PathBuf::from(&path);
    let mut lib = state.lib()?;
    if target == lib.path {
        return Err("現在と同じファイルです。".into());
    }
    if copy_current && !target.exists() {
        store::copy_library(&lib.path, &target)?;
    }
    let new_lib = Library::open(&target)?;
    save_settings(&state.config_dir, &Settings { db_path: Some(target) })?;
    *lib = new_lib;
    LibraryInfo::of(&lib)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings = load_settings(&config_dir);
            let default_path = app.path().app_data_dir()?.join("books.db");
            let path = settings.db_path.unwrap_or(default_path.clone());
            // A configured path that cannot be opened (e.g. the synced drive is not mounted) falls back to the local default.
            let lib = match Library::open(&path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{e}; falling back to {}", default_path.display());
                    Library::open(&default_path)?
                }
            };
            app.manage(AppState { config_dir, lib: Mutex::new(lib) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_info,
            list_books,
            save_book,
            delete_book,
            import_notion_csv,
            search_books,
            fetch_link_metadata,
            fetch_cover,
            set_cover_from_file,
            set_cover_from_url,
            remove_cover,
            cover_data,
            set_db_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running bookf");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_tolerate_broken_files() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_settings(tmp.path()).db_path.is_none());
        save_settings(tmp.path(), &Settings { db_path: Some(PathBuf::from(r"G:\マイドライブ\books.db")) }).unwrap();
        assert_eq!(load_settings(tmp.path()).db_path, Some(PathBuf::from(r"G:\マイドライブ\books.db")));
        std::fs::write(tmp.path().join(SETTINGS_FILE), "{broken").unwrap();
        assert!(load_settings(tmp.path()).db_path.is_none());
    }

    #[test]
    fn image_files_are_checked_before_reading() {
        let tmp = tempfile::tempdir().unwrap();
        let jpeg = tmp.path().join("cover.JPEG");
        std::fs::write(&jpeg, b"img").unwrap();
        assert_eq!(read_image_file(&jpeg).unwrap(), (b"img".to_vec(), "jpg"));

        let txt = tmp.path().join("notes.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert!(read_image_file(&txt).unwrap_err().contains("対応していない"));
        assert!(read_image_file(&tmp.path().join("missing.png")).is_err());
        assert!(read_image_file(tmp.path()).is_err(), "a folder is not an image");
    }
}
