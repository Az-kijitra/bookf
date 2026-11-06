# SPEC-0600-0001: Windows インストーラーによる配布

| 項目 | 内容 |
|---|---|
| Spec ID | SPEC-0600-0001 |
| 領域 | 0600 配布・リリース |
| 親 | SPEC-0100-0001 |
| ステータス | 有効 |
| 最終更新 | 2026-11-06 |
| 実装 | `src-tauri/tauri.conf.json`、`app/scripts/release/*.mjs`、`.github/workflows/release.yml` |

## 1. 配布物

| 項目 | 内容 |
|---|---|
| 形式 | NSIS インストーラーだけ（MSI は作らない） |
| ファイル名 | `bookf_<version>_x64-setup.exe`（x64 のみ） |
| チェックサム | `bookf_<version>_x64-setup.exe.sha256`（`sha256sum -c` 互換の 1 行: `<hex> *<ファイル名>`） |
| インストール先 | 現在のユーザーだけ（`installMode: currentUser`、管理者権限不要） |
| 表示言語 | 日本語のみ（言語選択なし） |
| アイコン | `icons/icon.ico`（インストーラーとアンインストーラー） |
| コード署名 | なし（SmartScreen の警告が出ることがある。README とマニュアルに手順を書く） |
| WebView2 | 無い PC ではインストーラーが取得して導入する（Tauri の既定） |
| 配布場所 | GitHub Releases |

## 2. バージョンとビルド

- バージョンは `MAJOR.MINOR.PATCH`。次の 5 か所が一致していなければならない: `tauri.conf.json`、`package.json`、`package-lock.json`、`Cargo.toml`、`Cargo.lock`（`npm run release:check` で検査）。
- `npm run release:build`（`app/` で実行）: バージョンの一致を確かめ、古いインストーラーを消してから `tauri build --bundles nsis` を実行する。インストーラーがちょうど 1 つ、期待どおりの名前で作られ、MSI が無いことを確かめ、チェックサムファイルを書く。インストーラーは実行しない。
- 初回のビルドは、Tauri が NSIS を GitHub から取得するのでネットワークが必要。

## 3. リリース手順

1. 5 か所のバージョンを上げる（`npm version <x.y.z> --no-git-tag-version` と、`tauri.conf.json`・`Cargo.toml` の手修正、`cargo check` で `Cargo.lock` を更新）。
2. `CHANGELOG.md` に `## <x.y.z>` の節を書き、README とマニュアルのインストーラー名と、マニュアルの題名の版を更新する（`npm test` が不一致を検出する）。
3. コミットして `v<x.y.z>` のタグを付け、push する。
4. GitHub Actions（`release.yml`）が、タグと版の一致を確かめ、テストを実行し、インストーラーとチェックサムを付けた**下書き**のリリースを作る。
5. 下書きを確認して公開する。

手元でビルドする場合は、手順 1〜2 の後に `npm run release:build` を実行し、`app/src-tauri/target/release/bundle/nsis/` の 2 つのファイルを Releases に上げる。

## 4. アンインストール

Windows の「設定」→「アプリ」から削除する。「アプリのデータを削除する」（既定はオフ）を選ぶと `%APPDATA%\dev.bookf.app` を削除する。別の場所に移したデータベースは削除しない。
