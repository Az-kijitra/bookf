# bookf

[![CI](https://github.com/Az-kijitra/bookf/actions/workflows/ci.yml/badge.svg)](https://github.com/Az-kijitra/bookf/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`bookf` は、Windows 10 / 11 向けの、キーボード操作を中心にした読書記録アプリです（**Tauri 2 + Rust + Svelte 5 + SQLite** 製）。

タイトルや ISBN を入れるだけで書誌情報と表紙を補完し、評価・読了日・感想を素早く記録できます。データベースは 1 つのファイル (`books.db`) なので、Google ドライブなどの同期フォルダーに置けば複数の PC で共有できます。Notion の読書リストを CSV で取り込むこともできます。

GUI ファイラー [`guif`](https://github.com/Az-kijitra/guif) と同じ Nord 配色・等幅フォント・細線枠の外観と、キーボード中心の操作感を受け継いでいます。

---

## インストール

### 方法 1: インストーラーを実行（推奨）

1. [Releases](https://github.com/Az-kijitra/bookf/releases) から最新のインストーラー `bookf_0.1.0_x64-setup.exe` をダウンロードして実行します。
2. 画面の案内に従って進めます。インストール先は既定で `%LOCALAPPDATA%\bookf` です（現在のユーザーのみ、管理者権限不要）。
3. スタートメニューの「bookf」から起動します。

> **SmartScreen の警告が出たとき**
> インストーラーにはコード署名がないため、「Windows によって PC が保護されました」という警告が出ることがあります。「詳細情報」を選び、表示される「実行」ボタンを押して進めてください。

### 方法 2: ソースコードからビルドする

<details>
<summary>ビルド手順を表示</summary>

#### 必要要件

- Windows 10 または Windows 11 (64 ビット版)
- Rust 1.80 以上 (`cargo`)
- Node.js 20 以上 (`npm`)
- Microsoft Edge WebView2 ランタイム（Windows 11 には標準で入っています）

```powershell
# クローン
git clone https://github.com/Az-kijitra/bookf.git
cd bookf/app

# 依存パッケージのインストール
npm install

# 開発サーバー起動 (ホットリロード付き)
npm run tauri dev

# テスト
npm test                                          # フロントエンド (Vitest)
npm run check                                     # 型チェック (svelte-check)
cargo test --manifest-path src-tauri/Cargo.toml   # バックエンド (Rust)

# インストーラーとチェックサムのビルド (app/src-tauri/target/release/bundle/nsis/ に出力)
npm run release:build
```

リリースの手順は [SPEC-0600-0001](specs/release/SPEC-0600-0001-windows-installer.md) にあります。
</details>

---

## 主な特徴

- **書誌情報の自動補完**: タイトル・著者名・ISBN・URL のどれかで検索し、正式書名・著者・出版社・出版日・ISBN・表紙を取り込みます。
- **リンク先からの取得**: 本に登録したリンクから書誌情報を補います。一般の Web ページは OGP タグを読み、Amazon / Audible の URL は URL 中の ASIN / ISBN だけを使います（[データの取得元](#データの取得元)）。
- **表紙のローカル保存**: 表紙はデータベースの隣の `covers/` に保存し、オフラインでも表示します。URL やローカルの画像ファイルから差し替えることもできます。
- **3 ペイン画面**: 左に絞り込み（ステータス・種別・評価）、中央に書籍一覧、右に詳細と感想。列見出しで並べ替えられます。
- **キーボード中心**: `j` / `k` で移動、`1`〜`5` で★評価、`e` で感想を編集、`/` で絞り込み検索。
- **同期フォルダーで共有**: 「保存先を移動」でデータベースと表紙を Google ドライブなどへ移せます。同期と相性の良い設定 (`journal_mode = DELETE`) で開きます。
- **自動バックアップ**: データベースを開くたびに、全書籍を `books_backup.json` に書き出します。
- **Notion CSV 取り込み**: Notion の読書リストから書き出した CSV を取り込みます。同じタイトルと著者の本は重複して登録しません。

---

## キー操作

| キー | 動作 |
|---|---|
| `Ctrl+N` | 本を追加 |
| `Ctrl+I` | Notion CSV を取り込む |
| `Ctrl+O` | データベースを開く |
| `/` / `Ctrl+F` | 絞り込み検索 |
| `Tab` | サイドバー ⇔ 書籍一覧 |
| `j` `k` / `↑` `↓` | 移動（`g` / `G` で先頭 / 末尾、`PageUp` / `PageDown`） |
| `1`〜`5` / `0` | ★評価を付ける / 消す |
| `Enter` / `o` | リンクをブラウザで開く |
| `e` | 感想を編集（`Ctrl+S` で保存、`Esc` で戻る） |
| `u` | 書誌情報を検索して更新 |
| `s` / `r` | 並び替えの項目を切り替え / 昇順・降順を反転 |
| `Delete` | 本を削除（確認あり） |

全一覧は [ユーザーマニュアル §6](docs/user_manual.md#6-キー割り当て) にあります。

---

## データの保存場所

| 内容 | 場所 |
|---|---|
| データベース（既定） | `%APPDATA%\dev.bookf.app\books.db` |
| 表紙画像 | データベースと同じフォルダーの `covers\` |
| 自動バックアップ | データベースと同じフォルダーの `books_backup.json` |
| 設定（データベースの場所） | `%APPDATA%\dev.bookf.app\settings.json` |

### Google ドライブなどで共有する

1. メニューの「ファイル」→「保存先を移動…」で、同期フォルダー内の場所（例: `G:\マイドライブ\読書記録\books.db`）を選びます。
2. 今のデータベースと表紙がそこへコピーされ、以後はそのファイルを使います。
3. 別の PC では「ファイル」→「データベースを開く…」(`Ctrl+O`) で同じファイルを選びます。

同期フォルダーが見つからないとき（ドライブが未接続など）は、既定の場所のデータベースを開きます。同じファイルを 2 台で同時に編集すると同期のコンフリクトになるので、使う PC を切り替える前に同期が終わっていることを確かめてください。

### Notion から移行する

Notion の読書リストを「CSV」で書き出し、「ファイル」→「Notion CSV 取り込み…」(`Ctrl+I`) で選びます。読み込む列は `タイトル`, `著者`, `ステータス`, `種別`, `評価`（⭐ の数）, `読了日`, `開始`（`2024年6月16日` 形式）, `リンク`, `ISBN`, `感想` / `メモ` です。詳しくは [ユーザーマニュアル §7.2](docs/user_manual.md#72-notion-から移行する) にあります。

---

## データの取得元

書誌情報と表紙は、次の公開サービスから取得します。API キーは不要です。

| サービス | 用途 |
|---|---|
| [CiNii Books](https://ci.nii.ac.jp/books/)（国立情報学研究所） | タイトル・著者・ISBN による書誌検索 |
| [openBD](https://openbd.jp/) | ISBN による書誌情報と書影 |
| Google Books | 書影（openBD に無いとき） |
| Amazon | ASIN から組み立てた書影 URL |

- 通信はすべて `bookf/<バージョン>` を名乗って行い、ブラウザーを装うことはありません。
- Amazon / Audible のページそのものは取得しません。リンクの URL に書かれた ASIN / ISBN とタイトルだけを使います。
- 取得した書誌情報と画像の権利は、それぞれの提供元と権利者にあります。各サービスの利用条件に従って、個人の記録の範囲でお使いください。

---

## アンインストール

Windows の「設定」→「アプリ」から「bookf」を選んでアンインストールします。

アンインストール画面の「アプリのデータを削除する」にチェックを入れると、`%APPDATA%\dev.bookf.app` の中身（既定の場所のデータベース・表紙・設定）も削除されます。記録を残したいときはチェックを外したまま実行してください。同期フォルダーなど別の場所に移したデータベースは削除されません。

---

## ドキュメント

- [ユーザーマニュアル](docs/user_manual.md): すべての操作、全キー割り当て、同期フォルダーでの運用、バックアップからの復元、困ったとき
- [仕様台帳](specs/_registry.md): 画面とキー操作・データモデル・書誌情報の取得・保存先と同期・テスト・配布の仕様一覧
- [変更履歴](CHANGELOG.md): リリース履歴

---

## ライセンス

[MIT License](LICENSE)
