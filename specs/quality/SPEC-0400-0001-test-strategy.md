# SPEC-0400-0001: テスト戦略と静的チェック

| 項目 | 内容 |
|---|---|
| Spec ID | SPEC-0400-0001 |
| 領域 | 0400 品質・テスト |
| 親 | SPEC-0100-0001 |
| ステータス | 有効 |
| 最終更新 | 2026-11-06 |
| 関連 Issue | ISS-0005 |

## 1. 方針

- 画面やネットワークに依存しない純粋な関数に処理を寄せ、そこを単体テストで固める。コンポーネントとコマンドは薄く保つ。
- 外部サービスに触るテストは既定では動かさない（`#[ignore]`）。
- 利用者の実データ（`%APPDATA%\dev.bookf.app`）に触れるテストは作らない。ファイルを扱うテストは一時フォルダーで行う。

## 2. テストの種類

| 種類 | 対象 | 実行 |
|---|---|---|
| Vitest | `lib/library.ts`（絞り込み・並べ替え・集計など）、`lib/features/keymap.ts`（キー表の全体）、`lib/features/menu.ts`、リリース用スクリプト、ドキュメントの整合（SPEC-0500-0001）、構造テスト | `npm test` |
| cargo test | `db.rs`、`import.rs`（日付・評価・取り込み・重複・ISBN の補完）、`lookup.rs`（ISBN 変換・CiNii と openBD の解析・HTML の解析・Amazon URL の解釈）、`store.rs`（バックアップ・表紙の差し替えと削除・パスの検査・コピー）、`lib.rs`（設定・画像ファイルの検査） | `cargo test` |
| 通信を伴うテスト | 実際の CiNii / openBD / Web ページでの検索とリンク先の取得 | `cargo test -- --ignored live_` |
| 実データの取り込み確認 | 手元の Notion CSV を、メモリー上のデータベースに取り込んで件数を表示 | `BOOKF_CSV=<path> cargo test -- --ignored real_notion_export` |

## 3. 構造テスト（`app/src/structure.test.ts`）

- `routes/+page.svelte` は 400 行以下。
- バックエンドのソースに `Mozilla/5.0`・`sec-ch-ua`・`curl.exe` を含まない（ブラウザーを装わない。ISS-0002）。

## 4. 静的チェック

| チェック | コマンド |
|---|---|
| 型チェック | `npm run check`（svelte-check） |
| Rust の整形 | `cargo fmt --check`（`rustfmt.toml`: 行幅 140） |
| Rust の lint | `cargo clippy --all-targets -- -D warnings` |

## 5. CI

GitHub Actions（`.github/workflows/ci.yml`、windows-latest）で、push と pull request のたびに次を順に実行する: Vitest → svelte-check → rustfmt → clippy → cargo test → フロントエンドのビルド。

## 6. 未整備

起動したアプリを操作する E2E テストは無い（ISS-0005）。画面の変更は手で確認する。
