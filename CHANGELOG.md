# 変更履歴

## Unreleased

### 変更

- リンク先からの書誌情報の取得で、Amazon / Audible のページを取得しないようにしました。URL に含まれる ASIN / ISBN とタイトルだけを使い、openBD と CiNii Books で補完します。
- すべての通信で、ブラウザーを装う User-Agent とヘッダー、`curl.exe` による取得をやめ、`bookf/<バージョン>` を名乗るようにしました。
- 表紙のダウンロードを、画像の Content-Type で 10 MB までに制限しました。一般 Web ページの取得は 5 MB までです。
- 書誌情報の検索で、タイトルが分からない候補を「（タイトル不明）」と表示し、選んだときは検索語をタイトルにします。

### 開発

- LICENSE (MIT)、GitHub Actions による CI（Vitest、svelte-check、rustfmt、clippy、cargo test、ビルド）を追加しました。
- `rustfmt.toml` を追加し、Rust のコードを整形しました。
- 生成物 `app/src-tauri/gen/` を管理対象から外しました。

## 0.1.0

最初の版です（未配布）。
