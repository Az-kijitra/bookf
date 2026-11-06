//! bookf back end: database location, commands called from the Svelte front end.

mod db;
mod import;
mod lookup;

use base64::Engine;
use db::Book;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

const SETTINGS_FILE: &str = "settings.json";
const COVERS_DIR: &str = "covers";
const BACKUP_FILE: &str = "books_backup.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Settings {
    db_path: Option<PathBuf>,
}

struct Library {
    path: PathBuf,
    conn: Connection,
}

impl Library {
    fn dir(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }
}

struct AppState {
    config_dir: PathBuf,
    lib: Mutex<Library>,
}

#[derive(Serialize)]
struct LibraryInfo {
    db_path: String,
    count: usize,
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

/// Opens the database and writes a JSON snapshot next to it, so a sync conflict or a
/// broken file can always be recovered from the state at the last start.
fn open_library(path: &Path) -> CmdResult<Library> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("フォルダーを作成できません: {e}"))?;
    }
    let conn = db::open(path).map_err(|e| format!("データベースを開けません ({}): {e}", path.display()))?;
    let _ = import::backfill_isbns(&conn);
    let lib = Library { path: path.to_path_buf(), conn };
    if let Ok(books) = db::list(&lib.conn) {
        if !books.is_empty() {
            if let Ok(json) = serde_json::to_string_pretty(&books) {
                let _ = std::fs::write(lib.dir().join(BACKUP_FILE), json);
            }
        }
    }
    Ok(lib)
}

#[tauri::command]
fn library_info(state: State<AppState>) -> CmdResult<LibraryInfo> {
    let lib = state.lib.lock().map_err(err)?;
    let count = db::list(&lib.conn).map_err(err)?.len();
    Ok(LibraryInfo { db_path: lib.path.display().to_string(), count })
}

#[tauri::command]
fn list_books(state: State<AppState>) -> CmdResult<Vec<Book>> {
    let lib = state.lib.lock().map_err(err)?;
    db::list(&lib.conn).map_err(err)
}

#[tauri::command]
fn save_book(state: State<AppState>, book: Book) -> CmdResult<Book> {
    if book.title.trim().is_empty() {
        return Err("タイトルを入力してください。".into());
    }
    let lib = state.lib.lock().map_err(err)?;
    db::save(&lib.conn, &book).map_err(err)
}

#[tauri::command]
fn delete_book(state: State<AppState>, id: i64) -> CmdResult<()> {
    let lib = state.lib.lock().map_err(err)?;
    if let Some(rel) = db::get(&lib.conn, id).map_err(err)?.and_then(|b| b.cover_local_path) {
        let _ = std::fs::remove_file(lib.dir().join(rel));
    }
    db::delete(&lib.conn, id).map_err(err)
}

#[tauri::command]
fn import_notion_csv(state: State<AppState>, path: String) -> CmdResult<import::ImportResult> {
    let lib = state.lib.lock().map_err(err)?;
    import::import_file(&lib.conn, Path::new(&path))
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
    let (url, dir) = {
        let lib = state.lib.lock().map_err(err)?;
        let book = db::get(&lib.conn, id).map_err(err)?.ok_or("本が見つかりません")?;
        (book.cover_url.ok_or("表紙の URL がありません")?, lib.dir())
    };
    let (bytes, ext) = lookup::download(&url).await?;
    let rel = format!("{COVERS_DIR}/{id}.{ext}");
    std::fs::create_dir_all(dir.join(COVERS_DIR)).map_err(err)?;
    std::fs::write(dir.join(&rel), bytes).map_err(err)?;
    let lib = state.lib.lock().map_err(err)?;
    let mut book = db::get(&lib.conn, id).map_err(err)?.ok_or("本が見つかりません")?;
    book.cover_local_path = Some(rel);
    db::save(&lib.conn, &book).map_err(err)
}

/// The cached cover as a `data:` URL (no asset-protocol scope needed), or None if it is missing.
#[tauri::command]
fn cover_data(state: State<AppState>, rel: String) -> CmdResult<Option<String>> {
    let lib = state.lib.lock().map_err(err)?;
    let rel_path = Path::new(&rel);
    if rel_path.is_absolute() || rel_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("不正な表紙のパスです".into());
    }
    let Ok(bytes) = std::fs::read(lib.dir().join(rel_path)) else { return Ok(None) };
    let mime = match rel_path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    Ok(Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}

/// Sets the cover image from a local file path. Copies the file into `covers/{id}.{ext}`.
#[tauri::command]
fn set_cover_from_file(state: State<AppState>, id: i64, file_path: String) -> CmdResult<Book> {
    let src = PathBuf::from(&file_path);
    if !src.exists() || !src.is_file() {
        return Err("画像ファイルが見つかりません".into());
    }
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_lowercase();
    let bytes = std::fs::read(&src).map_err(|e| format!("ファイルを読み込めません: {e}"))?;

    let lib = state.lib.lock().map_err(err)?;
    let mut book = db::get(&lib.conn, id).map_err(err)?.ok_or("本が見つかりません")?;

    if let Some(ref old_rel) = book.cover_local_path {
        let _ = std::fs::remove_file(lib.dir().join(old_rel));
    }

    let rel = format!("{COVERS_DIR}/{id}.{ext}");
    std::fs::create_dir_all(lib.dir().join(COVERS_DIR)).map_err(err)?;
    std::fs::write(lib.dir().join(&rel), bytes).map_err(err)?;

    book.cover_local_path = Some(rel);
    db::save(&lib.conn, &book).map_err(err)
}

/// Sets the cover image from a custom remote URL. Downloads and caches it.
#[tauri::command]
async fn set_cover_from_url(state: State<'_, AppState>, id: i64, url: String) -> CmdResult<Book> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("URLが空です".into());
    }
    let (bytes, ext) = lookup::download(&url).await?;

    let lib = state.lib.lock().map_err(err)?;
    let mut book = db::get(&lib.conn, id).map_err(err)?.ok_or("本が見つかりません")?;

    if let Some(ref old_rel) = book.cover_local_path {
        let _ = std::fs::remove_file(lib.dir().join(old_rel));
    }

    let rel = format!("{COVERS_DIR}/{id}.{ext}");
    std::fs::create_dir_all(lib.dir().join(COVERS_DIR)).map_err(err)?;
    std::fs::write(lib.dir().join(&rel), bytes).map_err(err)?;

    book.cover_url = Some(url);
    book.cover_local_path = Some(rel);
    db::save(&lib.conn, &book).map_err(err)
}

/// Removes the cover image of a book.
#[tauri::command]
fn remove_cover(state: State<AppState>, id: i64) -> CmdResult<Book> {
    let lib = state.lib.lock().map_err(err)?;
    let mut book = db::get(&lib.conn, id).map_err(err)?.ok_or("本が見つかりません")?;

    if let Some(ref old_rel) = book.cover_local_path {
        let _ = std::fs::remove_file(lib.dir().join(old_rel));
    }

    book.cover_url = None;
    book.cover_local_path = None;
    db::save(&lib.conn, &book).map_err(err)
}

/// Switches to another database file (e.g. in the Google Drive folder). With `copy_current`,
/// the current database and its covers are copied there first when the target does not exist yet.
#[tauri::command]
fn set_db_path(state: State<AppState>, path: String, copy_current: bool) -> CmdResult<LibraryInfo> {
    let target = PathBuf::from(&path);
    let mut lib = state.lib.lock().map_err(err)?;
    if target == lib.path {
        return Err("現在と同じファイルです。".into());
    }
    if copy_current && !target.exists() {
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        std::fs::copy(&lib.path, &target).map_err(|e| format!("コピーできません: {e}"))?;
        let (from, to) = (lib.dir().join(COVERS_DIR), target.parent().unwrap_or(Path::new(".")).join(COVERS_DIR));
        if let Ok(entries) = std::fs::read_dir(&from) {
            std::fs::create_dir_all(&to).map_err(err)?;
            for e in entries.flatten() {
                let dest = to.join(e.file_name());
                if !dest.exists() {
                    let _ = std::fs::copy(e.path(), dest);
                }
            }
        }
    }
    let new_lib = open_library(&target)?;
    save_settings(&state.config_dir, &Settings { db_path: Some(target) })?;
    *lib = new_lib;
    let count = db::list(&lib.conn).map_err(err)?.len();
    Ok(LibraryInfo { db_path: lib.path.display().to_string(), count })
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
            let lib = match open_library(&path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{e}; falling back to {}", default_path.display());
                    open_library(&default_path)?
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
