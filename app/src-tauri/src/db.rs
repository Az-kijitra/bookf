//! SQLite storage of the reading log (SPEC-0300-0002).
//!
//! The database file may live in a cloud-synced folder (Google Drive etc.), so the
//! rollback journal (`DELETE`) is used instead of WAL: no `-wal` / `-shm` side files
//! are left behind for the sync client to pick up half-written.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Book {
    pub id: Option<i64>,
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub isbn: Option<String>,
    pub status: String,
    pub media_type: String,
    pub rating: i64,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub link_url: Option<String>,
    pub cover_url: Option<String>,
    /// Relative to the database folder (e.g. `covers/12.jpg`) so the same file works on
    /// every PC regardless of where the synced folder is mounted.
    pub cover_local_path: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

const COLUMNS: &str = "id, title, subtitle, author, publisher, published_date, isbn, status, media_type, rating, \
    started_at, completed_at, link_url, cover_url, cover_local_path, notes, created_at, updated_at";

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS books (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            subtitle TEXT,
            author TEXT,
            publisher TEXT,
            published_date TEXT,
            isbn TEXT,
            status TEXT NOT NULL DEFAULT '未着手',
            media_type TEXT NOT NULL DEFAULT '書籍',
            rating INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            completed_at TEXT,
            link_url TEXT,
            cover_url TEXT,
            cover_local_path TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE INDEX IF NOT EXISTS idx_books_status ON books(status);
        CREATE INDEX IF NOT EXISTS idx_books_media_type ON books(media_type);
        CREATE INDEX IF NOT EXISTS idx_books_rating ON books(rating);
        CREATE INDEX IF NOT EXISTS idx_books_completed_at ON books(completed_at);",
    )
}

fn from_row(r: &Row) -> rusqlite::Result<Book> {
    Ok(Book {
        id: r.get(0)?,
        title: r.get(1)?,
        subtitle: r.get(2)?,
        author: r.get(3)?,
        publisher: r.get(4)?,
        published_date: r.get(5)?,
        isbn: r.get(6)?,
        status: r.get(7)?,
        media_type: r.get(8)?,
        rating: r.get(9)?,
        started_at: r.get(10)?,
        completed_at: r.get(11)?,
        link_url: r.get(12)?,
        cover_url: r.get(13)?,
        cover_local_path: r.get(14)?,
        notes: r.get(15)?,
        created_at: r.get(16)?,
        updated_at: r.get(17)?,
    })
}

/// Empty strings are stored as NULL so "not set" has one representation.
fn norm(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Book>> {
    let mut stmt =
        conn.prepare(&format!("SELECT {COLUMNS} FROM books ORDER BY COALESCE(completed_at, started_at, created_at) DESC, id DESC"))?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Book>> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM books WHERE id = ?1"), [id], from_row).optional()
}

/// Inserts when `id` is None, otherwise updates. Returns the stored row.
pub fn save(conn: &Connection, b: &Book) -> rusqlite::Result<Book> {
    let rating = b.rating.clamp(0, 5);
    let title = b.title.trim();
    let status = if b.status.trim().is_empty() { "未着手" } else { b.status.trim() };
    let media = if b.media_type.trim().is_empty() { "書籍" } else { b.media_type.trim() };
    let id = match b.id {
        None => {
            conn.execute(
                "INSERT INTO books (title, subtitle, author, publisher, published_date, isbn, status, media_type, rating,
                    started_at, completed_at, link_url, cover_url, cover_local_path, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    title,
                    norm(&b.subtitle),
                    norm(&b.author),
                    norm(&b.publisher),
                    norm(&b.published_date),
                    norm(&b.isbn),
                    status,
                    media,
                    rating,
                    norm(&b.started_at),
                    norm(&b.completed_at),
                    norm(&b.link_url),
                    norm(&b.cover_url),
                    norm(&b.cover_local_path),
                    norm(&b.notes)
                ],
            )?;
            conn.last_insert_rowid()
        }
        Some(id) => {
            conn.execute(
                "UPDATE books SET title = ?1, subtitle = ?2, author = ?3, publisher = ?4, published_date = ?5, isbn = ?6,
                    status = ?7, media_type = ?8, rating = ?9, started_at = ?10, completed_at = ?11, link_url = ?12,
                    cover_url = ?13, cover_local_path = ?14, notes = ?15, updated_at = datetime('now', 'localtime')
                 WHERE id = ?16",
                params![
                    title,
                    norm(&b.subtitle),
                    norm(&b.author),
                    norm(&b.publisher),
                    norm(&b.published_date),
                    norm(&b.isbn),
                    status,
                    media,
                    rating,
                    norm(&b.started_at),
                    norm(&b.completed_at),
                    norm(&b.link_url),
                    norm(&b.cover_url),
                    norm(&b.cover_local_path),
                    norm(&b.notes),
                    id
                ],
            )?;
            id
        }
    };
    get(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM books WHERE id = ?1", [id])?;
    Ok(())
}

pub fn exists_title_author(conn: &Connection, title: &str, author: Option<&str>) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM books WHERE title = ?1 AND COALESCE(author, '') = COALESCE(?2, '')",
        params![title.trim(), author.map(str::trim).filter(|s| !s.is_empty())],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    #[test]
    fn insert_update_delete() {
        let c = mem();
        let b = save(&c, &Book { title: " 具体と抽象 ".into(), author: Some("細谷 功".into()), rating: 9, ..Default::default() }).unwrap();
        assert_eq!(b.title, "具体と抽象");
        assert_eq!(b.rating, 5, "rating is clamped");
        assert_eq!(b.status, "未着手");
        assert_eq!(b.media_type, "書籍");
        let mut u = b.clone();
        u.notes = Some("  ".into());
        u.status = "読了".into();
        let u = save(&c, &u).unwrap();
        assert_eq!(u.notes, None, "blank becomes NULL");
        assert_eq!(u.status, "読了");
        assert_eq!(list(&c).unwrap().len(), 1);
        assert!(exists_title_author(&c, "具体と抽象", Some("細谷 功")).unwrap());
        delete(&c, u.id.unwrap()).unwrap();
        assert!(list(&c).unwrap().is_empty());
    }
}
