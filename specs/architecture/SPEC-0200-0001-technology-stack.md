# SPEC-0200-0001: 技術スタックとモジュール構成

| 項目 | 内容 |
|---|---|
| Spec ID | SPEC-0200-0001 |
| 領域 | 0200 アーキテクチャ・技術基盤 |
| 親 | SPEC-0100-0001 |
| ステータス | 有効 |
| 最終更新 | 2026-11-06 |
| 関連 Issue | ISS-0001, ISS-0005 |

## 1. 技術スタック

| 層 | 採用技術 |
|---|---|
| アプリ基盤 | Tauri 2（WebView2） |
| バックエンド | Rust（edition 2021、1.80 以上）。`rusqlite`（SQLite 同梱）、`reqwest`（rustls）、`csv`、`regex` |
| フロントエンド | Svelte 5 + SvelteKit（`adapter-static`、SPA）、TypeScript |
| テスト | Vitest、`cargo test` |
| 配布 | NSIS インストーラー（SPEC-0600-0001） |

## 2. モジュール構成

### 2.1 バックエンド（`app/src-tauri/src/`）

| モジュール | 役割 |
|---|---|
| `lib.rs` | Tauri のコマンド定義、設定 (`settings.json`) の読み書き、起動処理。コマンドは薄く保ち、処理は下のモジュールに任せる。 |
| `store.rs` | 開いているライブラリー（データベース接続と、その隣の `covers/`・`books_backup.json`）のファイル操作。パスだけで動き、Tauri に依存しない。 |
| `db.rs` | `books` テーブルのスキーマと CRUD（SPEC-0300-0002）。 |
| `import.rs` | Notion CSV の取り込み、リンクからの ISBN の補完（SPEC-0300-0002）。 |
| `lookup.rs` | 書誌情報の検索、リンク先の解析、表紙のダウンロード（SPEC-0300-0003）。 |

### 2.2 フロントエンド（`app/src/`）

| 場所 | 役割 |
|---|---|
| `routes/+page.svelte` | 画面の組み立てと状態の保持だけを行う（400 行以下。SPEC-0400-0001）。 |
| `lib/components/` | `BookList`・`Sidebar`・`DetailPane`・`StatusBar`・`MenuBar`・`LookupDialog`・`CoverDialog`・`Stars`。 |
| `lib/features/keymap.ts` | キー入力から動作を決める純粋な関数（SPEC-0300-0001 §3）。 |
| `lib/features/menu.ts` | メニューバーの内容を状態から組み立てる。 |
| `lib/features/file-dialogs.ts` | ファイル選択ダイアログ。 |
| `lib/library.ts` | 絞り込み・検索・並べ替え・集計などの純粋な関数。 |
| `lib/api.ts`・`lib/types.ts` | バックエンドのコマンド呼び出しと型。 |

## 3. フロントエンドとバックエンドの境界

| コマンド | 内容 |
|---|---|
| `library_info` / `list_books` | データベースの場所と件数、全書籍 |
| `save_book` / `delete_book` | 書籍の保存（タイトル必須）、削除（表紙ファイルも削除） |
| `import_notion_csv` | CSV の取り込み（1 トランザクション） |
| `search_books` / `fetch_link_metadata` | 書誌情報の検索、リンク先からの取得 |
| `fetch_cover` / `set_cover_from_url` / `set_cover_from_file` / `remove_cover` / `cover_data` | 表紙の保存・変更・削除・読み出し（`data:` URL） |
| `set_db_path` | データベースの切り替え（必要ならコピーしてから） |

- エラーは日本語の文字列で返し、画面のステータスバーに表示する。
- 表紙は `data:` URL で渡すので、asset プロトコルのスコープは使わない。
- Tauri の権限 (`capabilities/default.json`) は `core:default`・`opener:default`・`dialog:default` だけ。

## 4. セキュリティ上の約束

- 外部への通信は `lookup.rs` だけが行う。User-Agent は `bookf/<バージョン> (+リポジトリ URL)` とし、ブラウザーを装わない。外部コマンド（`curl.exe` など）は起動しない（構造テストで検査。SPEC-0400-0001）。
- ダウンロードの上限: 画像 10 MB（Content-Type が画像のものだけ）、HTML 5 MB。
- データベースに記録された表紙のパスは、ライブラリーのフォルダーの中だけを指せる（`..`・絶対パスは拒否）。
- CSP は未設定（ISS-0005）。
