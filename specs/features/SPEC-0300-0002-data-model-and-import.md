# SPEC-0300-0002: データモデルと Notion CSV 取り込み

| 項目 | 内容 |
|---|---|
| Spec ID | SPEC-0300-0002 |
| 領域 | 0300 機能 |
| 親 | SPEC-0100-0001 |
| ステータス | 有効 |
| 最終更新 | 2026-11-06 |
| 実装 | `src-tauri/src/db.rs`、`src-tauri/src/import.rs` |

## 1. データモデル (SQLite)

```sql
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,                     -- タイトル（必須）
    subtitle TEXT,                           -- サブタイトル
    author TEXT,                             -- 著者
    publisher TEXT,                          -- 出版社
    published_date TEXT,                     -- 出版日（取得元の書式のまま）
    isbn TEXT,                               -- ISBN-10 または ISBN-13（ハイフンなし）
    status TEXT NOT NULL DEFAULT '未着手',   -- 状態
    media_type TEXT NOT NULL DEFAULT '書籍', -- 種別
    rating INTEGER NOT NULL DEFAULT 0,       -- 評価 0（未評価）〜 5
    started_at TEXT,                         -- 開始日 YYYY-MM-DD
    completed_at TEXT,                       -- 読了日 YYYY-MM-DD
    link_url TEXT,                           -- リンク（Amazon・出版社・記事など）
    cover_url TEXT,                          -- 表紙の取得元 URL
    cover_local_path TEXT,                   -- 保存した表紙。データベースのフォルダーからの相対パス（covers/12.jpg）
    notes TEXT,                              -- 感想・メモ
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
-- 索引: status, media_type, rating, completed_at
```

- 表紙のパスを相対パスにするのは、同期フォルダーのドライブ文字やフォルダーが PC ごとに違っても同じファイルを使えるようにするため。
- 保存時、文字列の前後の空白を取り除き、空文字は NULL にする。
- スキーマは起動時に `CREATE TABLE IF NOT EXISTS` で作る。

## 2. Notion CSV の取り込み

Notion の読書リストのデータベースを「CSV」で書き出したファイルを読み込む。UTF-8（BOM 付きも可）。

| CSV の列 | 取り込み先 | 変換 |
|---|---|---|
| `タイトル`（必須） | title | この列が無いファイルはエラー。空の行は飛ばす。 |
| `著者` | author | |
| `ステータス` | status | 空なら「未着手」 |
| `種別` | media_type | 空なら「書籍」 |
| `評価` | rating | `⭐`（または `★`）の数、または数字 0〜5 |
| `読了日` / `開始` | completed_at / started_at | `2024年6月16日`・`2024/6/16`・`2024-06-16` → `2024-06-16`。読めなければ空 |
| `リンク` | link_url | |
| `ISBN` | isbn | 無ければリンクの Amazon URL（`/dp/`・`/gp/product/`・`/ASIN/` の後の ISBN-10 / 13）から |
| `感想` または `メモ` | notes | |

- 同じタイトルと著者の本がすでにあれば飛ばす（何度取り込んでも重複しない）。
- 全行を 1 つのトランザクションで取り込む。途中でエラーになったら何も登録しない。
- 結果として「追加 n 件、スキップ m 件」を表示する。

## 3. ISBN の補完

データベースを開くたびに、ISBN が空でリンクが Amazon の本について、リンクから ISBN を補う。Kindle などの `B0…` で始まる ASIN は ISBN ではないので補わない。

## 改訂履歴

- 2026-11-06: 旧 SPEC-0100 §3 から分割。ISBN・感想・メモ列、トランザクション、ISBN の補完を実装に合わせて追記。
