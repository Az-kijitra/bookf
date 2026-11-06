//! The open library: database connection plus the files kept beside it
//! (`covers/`, `books_backup.json`). Everything here works on paths, so it is tested
//! with temporary folders and without Tauri.

use crate::{db, import};
use rusqlite::Connection;
use std::path::{Component, Path, PathBuf};

pub const COVERS_DIR: &str = "covers";
pub const BACKUP_FILE: &str = "books_backup.json";

pub struct Library {
    pub path: PathBuf,
    pub conn: Connection,
}

impl Library {
    /// Folder that holds the database, its covers and its backup.
    pub fn dir(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    /// Opens the database and writes a JSON snapshot next to it, so a sync conflict or a
    /// broken file can always be recovered from the state at the last open.
    pub fn open(path: &Path) -> Result<Library, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("フォルダーを作成できません: {e}"))?;
        }
        let conn = db::open(path).map_err(|e| format!("データベースを開けません ({}): {e}", path.display()))?;
        let _ = import::backfill_isbns(&conn);
        let lib = Library { path: path.to_path_buf(), conn };
        lib.write_backup();
        Ok(lib)
    }

    /// Writes `books_backup.json`; skipped when there are no books, so an empty or broken
    /// database never overwrites a good backup.
    fn write_backup(&self) {
        let Ok(books) = db::list(&self.conn) else { return };
        if books.is_empty() {
            return;
        }
        if let Ok(json) = serde_json::to_string_pretty(&books) {
            let _ = std::fs::write(self.dir().join(BACKUP_FILE), json);
        }
    }

    pub fn count(&self) -> Result<usize, String> {
        db::list(&self.conn).map(|b| b.len()).map_err(|e| e.to_string())
    }

    pub fn book(&self, id: i64) -> Result<db::Book, String> {
        db::get(&self.conn, id).map_err(|e| e.to_string())?.ok_or_else(|| "本が見つかりません".into())
    }

    /// Stores the image as `covers/{id}.{ext}`, removes the book's previous cover file and
    /// saves the new relative path (and `cover_url`, when given).
    pub fn replace_cover(&self, id: i64, bytes: &[u8], ext: &str, cover_url: Option<Option<String>>) -> Result<db::Book, String> {
        let mut book = self.book(id)?;
        let rel = format!("{COVERS_DIR}/{id}.{ext}");
        if let Some(old) = book.cover_local_path.as_deref().filter(|old| *old != rel) {
            self.remove_cover_file(old);
        }
        std::fs::create_dir_all(self.dir().join(COVERS_DIR)).map_err(|e| e.to_string())?;
        std::fs::write(self.dir().join(&rel), bytes).map_err(|e| format!("表紙を保存できません: {e}"))?;
        book.cover_local_path = Some(rel);
        if let Some(url) = cover_url {
            book.cover_url = url;
        }
        db::save(&self.conn, &book).map_err(|e| e.to_string())
    }

    /// Removes the cover file and clears both cover fields.
    pub fn clear_cover(&self, id: i64) -> Result<db::Book, String> {
        let mut book = self.book(id)?;
        if let Some(old) = book.cover_local_path.take() {
            self.remove_cover_file(&old);
        }
        book.cover_url = None;
        db::save(&self.conn, &book).map_err(|e| e.to_string())
    }

    /// Deletes the book and its cover file.
    pub fn delete_book(&self, id: i64) -> Result<(), String> {
        if let Some(rel) = db::get(&self.conn, id).map_err(|e| e.to_string())?.and_then(|b| b.cover_local_path) {
            self.remove_cover_file(&rel);
        }
        db::delete(&self.conn, id).map_err(|e| e.to_string())
    }

    /// The cover file's bytes and MIME type; `Ok(None)` when the file is missing.
    pub fn read_cover(&self, rel: &str) -> Result<Option<(Vec<u8>, &'static str)>, String> {
        let rel_path = safe_cover_path(rel)?;
        let Ok(bytes) = std::fs::read(self.dir().join(rel_path)) else { return Ok(None) };
        Ok(Some((bytes, mime_of(rel_path))))
    }

    fn remove_cover_file(&self, rel: &str) {
        if let Ok(p) = safe_cover_path(rel) {
            let _ = std::fs::remove_file(self.dir().join(p));
        }
    }
}

/// A cover path stored in the database must stay inside the library folder.
pub fn safe_cover_path(rel: &str) -> Result<&Path, String> {
    let p = Path::new(rel);
    if rel.is_empty() || p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("不正な表紙のパスです".into());
    }
    Ok(p)
}

fn mime_of(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
}

/// Copies the database and its covers to `target` (used when moving the library into a
/// synced folder). Existing cover files at the destination are kept.
pub fn copy_library(from: &Path, target: &Path) -> Result<(), String> {
    let target_dir = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(target_dir).map_err(|e| e.to_string())?;
    std::fs::copy(from, target).map_err(|e| format!("コピーできません: {e}"))?;
    let from_covers = from.parent().unwrap_or(Path::new(".")).join(COVERS_DIR);
    let Ok(entries) = std::fs::read_dir(&from_covers) else { return Ok(()) };
    let to_covers = target_dir.join(COVERS_DIR);
    std::fs::create_dir_all(&to_covers).map_err(|e| e.to_string())?;
    for e in entries.flatten() {
        let dest = to_covers.join(e.file_name());
        if !dest.exists() {
            let _ = std::fs::copy(e.path(), dest);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Book;

    fn lib_in(dir: &Path) -> Library {
        Library::open(&dir.join("books.db")).unwrap()
    }

    fn add(lib: &Library, title: &str) -> Book {
        db::save(&lib.conn, &Book { title: title.into(), ..Default::default() }).unwrap()
    }

    #[test]
    fn backup_is_written_only_when_there_are_books() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = lib_in(tmp.path());
        assert!(!tmp.path().join(BACKUP_FILE).exists());
        add(&lib, "具体と抽象");
        drop(lib);
        let _ = lib_in(tmp.path());
        let json = std::fs::read_to_string(tmp.path().join(BACKUP_FILE)).unwrap();
        assert!(json.contains("具体と抽象"));
    }

    #[test]
    fn replace_cover_swaps_files_and_records_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = lib_in(tmp.path());
        let id = add(&lib, "本").id.unwrap();

        let b = lib.replace_cover(id, b"png-bytes", "png", None).unwrap();
        assert_eq!(b.cover_local_path.as_deref(), Some("covers/1.png"));
        assert!(tmp.path().join("covers/1.png").exists());

        let b = lib.replace_cover(id, b"jpg-bytes", "jpg", Some(Some("https://example.com/c.jpg".into()))).unwrap();
        assert_eq!(b.cover_local_path.as_deref(), Some("covers/1.jpg"));
        assert_eq!(b.cover_url.as_deref(), Some("https://example.com/c.jpg"));
        assert!(!tmp.path().join("covers/1.png").exists(), "old cover is removed");
        assert_eq!(lib.read_cover("covers/1.jpg").unwrap(), Some((b"jpg-bytes".to_vec(), "image/jpeg")));
    }

    #[test]
    fn clear_cover_and_delete_remove_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = lib_in(tmp.path());
        let id = add(&lib, "本").id.unwrap();
        lib.replace_cover(id, b"x", "gif", Some(Some("https://c".into()))).unwrap();
        let b = lib.clear_cover(id).unwrap();
        assert_eq!((b.cover_url, b.cover_local_path), (None, None));
        assert!(!tmp.path().join("covers/1.gif").exists());

        lib.replace_cover(id, b"x", "webp", None).unwrap();
        lib.delete_book(id).unwrap();
        assert!(!tmp.path().join("covers/1.webp").exists());
        assert!(lib.book(id).is_err());
    }

    #[test]
    fn cover_paths_outside_the_library_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = lib_in(tmp.path());
        for bad in ["../secret.jpg", "covers/../../x.jpg", "C:\\Windows\\win.ini", "/etc/passwd", ""] {
            assert!(lib.read_cover(bad).is_err(), "{bad}");
        }
        assert_eq!(lib.read_cover("covers/missing.jpg").unwrap(), None);
    }

    #[test]
    fn copy_library_copies_db_and_covers_without_overwriting() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("local");
        let lib = lib_in(&src);
        let id = add(&lib, "本").id.unwrap();
        lib.replace_cover(id, b"new", "jpg", None).unwrap();
        drop(lib);

        let dst = tmp.path().join("drive/読書記録");
        std::fs::create_dir_all(dst.join(COVERS_DIR)).unwrap();
        std::fs::write(dst.join("covers/1.jpg"), b"keep").unwrap();
        copy_library(&src.join("books.db"), &dst.join("books.db")).unwrap();

        let moved = lib_in(&dst);
        assert_eq!(moved.count().unwrap(), 1);
        assert_eq!(std::fs::read(dst.join("covers/1.jpg")).unwrap(), b"keep");
    }
}
