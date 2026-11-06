//! Import of the CSV exported from the Notion "Reading List" database (SPEC-0300-0002).
//!
//! Columns: タイトル, ステータス, リンク, 種別, 著者, 評価, 読了日, 開始.
//! 評価 is a run of ⭐️ characters, dates are written like `2024年6月16日`.

use crate::db::{self, Book};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
}

/// `2024年6月16日` -> `2024-06-16`. Already ISO-like `2024/6/16` and `2024-06-16` are accepted too.
pub fn parse_jp_date(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let nums: Vec<u32> = s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect();
    match nums.as_slice() {
        [y, m, d, ..] if *y >= 1000 && (1..=12).contains(m) && (1..=31).contains(d) => Some(format!("{y:04}-{m:02}-{d:02}")),
        _ => None,
    }
}

/// Number of ⭐ (U+2B50, the variation selector U+FE0F is ignored), or a plain digit 0-5.
pub fn parse_rating(s: &str) -> i64 {
    let stars = s.chars().filter(|&c| c == '⭐' || c == '★').count() as i64;
    if stars > 0 {
        return stars.min(5);
    }
    s.trim().parse::<i64>().map(|n| n.clamp(0, 5)).unwrap_or(0)
}

/// Extracts an ISBN-10 or ISBN-13 from an Amazon URL (e.g. `/dp/4121022726/`).
pub fn extract_isbn_from_url(url: &str) -> Option<String> {
    for marker in &["/dp/", "/gp/product/", "/ASIN/"] {
        if let Some(pos) = url.find(marker) {
            let start = pos + marker.len();
            let rest = &url[start..];
            let segment: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let segment = segment.to_ascii_uppercase();
            if segment.len() == 10
                && segment[..9].chars().all(|c| c.is_ascii_digit())
                && segment[9..].chars().all(|c| c.is_ascii_digit() || c == 'X')
            {
                return Some(segment);
            }
            if segment.len() == 13 && segment.chars().all(|c| c.is_ascii_digit()) {
                return Some(segment);
            }
        }
    }
    None
}

/// Backfills missing `isbn` from `link_url` for all existing books in the database.
pub fn backfill_isbns(conn: &Connection) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare("SELECT id, link_url FROM books WHERE isbn IS NULL AND link_url IS NOT NULL")?;
    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.filter_map(Result::ok).collect();
    let mut updated = 0;
    for (id, link) in rows {
        if let Some(isbn) = extract_isbn_from_url(&link) {
            conn.execute("UPDATE books SET isbn = ?1 WHERE id = ?2", rusqlite::params![isbn, id])?;
            updated += 1;
        }
    }
    Ok(updated)
}

pub fn import_reader<R: std::io::Read>(conn: &Connection, reader: R) -> Result<ImportResult, String> {
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(reader);
    let headers: Vec<String> =
        rdr.headers().map_err(|e| e.to_string())?.iter().map(|h| h.trim_start_matches('\u{feff}').trim().to_string()).collect();
    let idx: HashMap<&str, usize> = headers.iter().enumerate().map(|(i, h)| (h.as_str(), i)).collect();
    if !idx.contains_key("タイトル") {
        return Err("「タイトル」列が見つかりません。Notion からエクスポートした CSV を選んでください。".into());
    }
    let mut result = ImportResult::default();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| e.to_string())?;
        let col = |name: &str| idx.get(name).and_then(|&i| rec.get(i)).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let Some(title) = col("タイトル") else {
            result.skipped += 1;
            continue;
        };
        let author = col("著者");
        if db::exists_title_author(conn, &title, author.as_deref()).map_err(|e| e.to_string())? {
            result.skipped += 1;
            continue;
        }
        let link = col("リンク");
        let isbn = col("ISBN").or_else(|| link.as_deref().and_then(extract_isbn_from_url));
        let book = Book {
            title,
            author,
            status: col("ステータス").unwrap_or_else(|| "未着手".into()),
            media_type: col("種別").unwrap_or_else(|| "書籍".into()),
            rating: col("評価").map(|r| parse_rating(&r)).unwrap_or(0),
            link_url: link,
            completed_at: col("読了日").and_then(|d| parse_jp_date(&d)),
            started_at: col("開始").and_then(|d| parse_jp_date(&d)),
            isbn,
            notes: col("感想").or_else(|| col("メモ")),
            ..Default::default()
        };
        db::save(conn, &book).map_err(|e| e.to_string())?;
        result.imported += 1;
    }
    Ok(result)
}

pub fn import_file(conn: &Connection, path: &Path) -> Result<ImportResult, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("CSV を開けません: {e}"))?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let r = import_reader(&tx, f)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BOOKF_CSV=<path> cargo test -- --ignored real_notion_export`: checks a real export without touching any database file.
    #[test]
    #[ignore]
    fn real_notion_export() {
        let path = std::env::var("BOOKF_CSV").expect("set BOOKF_CSV");
        let conn = Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        let r = import_file(&conn, Path::new(&path)).unwrap();
        let books = db::list(&conn).unwrap();
        println!("{r:?}");
        let rated = books.iter().filter(|b| b.rating > 0).count();
        let dated = books.iter().filter(|b| b.completed_at.is_some()).count();
        let with_isbn = books.iter().filter(|b| b.isbn.is_some()).count();
        let total = books.len();
        println!("books={total} rated={rated} with_completed_at={dated} with_isbn={with_isbn}");
        assert!(r.imported > 0);
    }

    #[test]
    fn dates() {
        assert_eq!(parse_jp_date("2024年6月16日").as_deref(), Some("2024-06-16"));
        assert_eq!(parse_jp_date("2024/12/1").as_deref(), Some("2024-12-01"));
        assert_eq!(parse_jp_date(""), None);
        assert_eq!(parse_jp_date("いつか"), None);
    }

    #[test]
    fn ratings() {
        assert_eq!(parse_rating("⭐️⭐️⭐️"), 3);
        assert_eq!(parse_rating("⭐️⭐️⭐️⭐️⭐️"), 5);
        assert_eq!(parse_rating(""), 0);
        assert_eq!(parse_rating("4"), 4);
    }

    #[test]
    fn imports_notion_rows_and_skips_duplicates() {
        let conn = Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        let csv = "\u{feff}タイトル,ステータス,リンク,種別,著者,評価,読了日,開始\r\n\
            具体と抽象,読了,https://example.com/a,オーディブル,細谷　功,⭐️⭐️⭐️,2024年6月25日,\r\n\
            プロジェクトのトラブル解決大全,読了,,書籍,木部 智之,⭐️⭐️⭐️,2024年9月7日,2024年8月31日\r\n\
            ,読了,,書籍,,,,\r\n";
        let r = import_reader(&conn, csv.as_bytes()).unwrap();
        assert_eq!(r, ImportResult { imported: 2, skipped: 1 });
        let r2 = import_reader(&conn, csv.as_bytes()).unwrap();
        assert_eq!(r2, ImportResult { imported: 0, skipped: 3 });
        let books = db::list(&conn).unwrap();
        let b = books.iter().find(|b| b.title == "プロジェクトのトラブル解決大全").unwrap();
        assert_eq!(b.started_at.as_deref(), Some("2024-08-31"));
        assert_eq!(b.completed_at.as_deref(), Some("2024-09-07"));
        assert_eq!(b.rating, 3);
        let a = books.iter().find(|b| b.title == "具体と抽象").unwrap();
        assert_eq!(a.media_type, "オーディブル");
        assert_eq!(a.link_url.as_deref(), Some("https://example.com/a"));
    }

    #[test]
    fn isbn_is_taken_from_amazon_links_only_when_it_is_one() {
        assert_eq!(extract_isbn_from_url("https://www.amazon.co.jp/dp/4121022726/ref=x").as_deref(), Some("4121022726"));
        assert_eq!(extract_isbn_from_url("https://www.amazon.co.jp/gp/product/412102272x").as_deref(), Some("412102272X"));
        assert_eq!(extract_isbn_from_url("https://www.amazon.co.jp/dp/B09771DDFL"), None, "Kindle ASIN is not an ISBN");
        assert_eq!(extract_isbn_from_url("https://example.com/book"), None);
    }

    #[test]
    fn missing_isbns_are_backfilled_from_links() {
        let conn = Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        let save = |title: &str, link: Option<&str>, isbn: Option<&str>| {
            db::save(&conn, &Book { title: title.into(), link_url: link.map(Into::into), isbn: isbn.map(Into::into), ..Default::default() })
                .unwrap()
        };
        let a = save("a", Some("https://www.amazon.co.jp/dp/4121022726"), None);
        let b = save("b", Some("https://www.amazon.co.jp/dp/4000000000"), Some("9780000000000"));
        let c = save("c", Some("https://example.com/"), None);
        assert_eq!(backfill_isbns(&conn).unwrap(), 1);
        assert_eq!(db::get(&conn, a.id.unwrap()).unwrap().unwrap().isbn.as_deref(), Some("4121022726"));
        assert_eq!(db::get(&conn, b.id.unwrap()).unwrap().unwrap().isbn.as_deref(), Some("9780000000000"), "existing ISBN kept");
        assert_eq!(db::get(&conn, c.id.unwrap()).unwrap().unwrap().isbn, None);
    }
}
