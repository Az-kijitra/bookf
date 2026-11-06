# SPEC-0300-0003: 書誌情報の検索と表紙

| 項目 | 内容 |
|---|---|
| Spec ID | SPEC-0300-0003 |
| 領域 | 0300 機能 |
| 親 | SPEC-0100-0001 |
| ステータス | 有効 |
| 最終更新 | 2026-11-06 |
| 関連 Issue | ISS-0002 |
| 実装 | `src-tauri/src/lookup.rs`、`src-tauri/src/store.rs`、`lib/components/LookupDialog.svelte`、`lib/components/CoverDialog.svelte` |

## 1. 取得元

| サービス | 用途 |
|---|---|
| CiNii Books OpenSearch（`https://ci.nii.ac.jp/books/opensearch/search`） | タイトル・著者・ISBN での検索（最大 15 件） |
| openBD（`https://api.openbd.jp/v1/get`） | ISBN からの正式書名・出版社・著者・出版日・書影 |
| Google Books（`https://books.google.com/books/content?vid=isbn:…`） | openBD に書影が無いときの書影 |
| Amazon 画像（`https://images-na.ssl-images-amazon.com/images/P/{ASIN}.01.LZZZZZZZ.jpg`） | ASIN からの書影 |

API キーは使わない。すべての通信は User-Agent `bookf/<バージョン> (+https://github.com/Az-kijitra/bookf)` で行い、タイムアウトは 15 秒。

## 2. 検索（`search_books`）

検索語の種類で動きを変える。

1. **URL**（`http://`・`https://` で始まる）: §3 のリンク先からの取得。
2. **ISBN**（ハイフン・空白を除いて 10 桁または 13 桁）: ISBN-13 にして openBD で引く。見つからなければ CiNii Books の ISBN 検索。どちらでも見つからなければ候補は 0 件（タイトルの無い候補は最後に除くため）。
3. **それ以外**: CiNii Books のタイトル検索。0 件なら、空白やコロンで区切った 1 語目をタイトル・2 語目を著者として再検索し、それでも 0 件なら 1 語目だけで検索する。

最後に、ISBN のある候補を openBD でまとめて補い、各候補のリンクを Amazon の URL（ISBN-10 があれば `/dp/{ISBN-10}`、無ければ検索 URL）にし、書影が無ければ Google Books の書影 URL を入れる。タイトルの無い候補は除く。

### 2.1 候補の反映

- 新規登録: 候補の値で書籍を作る。候補を選ばずに「手動で追加」すると、検索語をタイトルにする。ダイアログで状態と種別を選べる。
- 既存の本の更新: 候補にある値だけを上書きする。**登録済みのリンクは残す**（版や Audible など、正確なページを指していることが多いため）。タイトルが空の候補ではタイトルを変えない。
- 表紙の URL が変わったら、保存済みの表紙を捨てて取り直す。

## 3. リンク先からの取得（`fetch_link_metadata`）

### 3.1 Amazon / Audible

ホストが `amazon.co.jp`・`amazon.com`・`audible.co.jp`・`audible.com`・`amzn.to`・`amzn.asia`（とそのサブドメイン）の URL は、**ページを取得しない**（ISS-0002）。

1. URL から ASIN を取り出す（`/dp/`・`/gp/product/`・`/ASIN/`・`/d/` の後、Audible は `/pd/…/` の後の 10 文字）。
2. 書影 URL を ASIN から組み立てる。
3. ASIN が ISBN-10 なら ISBN-13 にして、openBD → CiNii Books で書名・著者・出版社・出版日を補う。
4. タイトルが取れなければ、URL のタイトル部分（`amazon.co.jp/<タイトル>/dp/…`）を使う。
5. タイトルも書影も無ければエラー（「ISBN かタイトルで検索してください」）。

ISBN の無い Kindle / Audible の本では、書影と URL 中のタイトルしか取れない。

### 3.2 一般の Web ページ

ページを 1 回だけ取得し（5 MB まで）、次の順に読む。ISBN は取らない。

| 項目 | 読む場所 |
|---|---|
| タイトル | `og:title`、無ければ `<title>` の `|`・`｜` より前 |
| 表紙 | `og:image` または `twitter:image`（相対 URL はページの URL で解決） |
| 著者 | `<meta name="author">` または `article:author` |
| 説明 | `description` または `og:description` |

## 4. 表紙

- 書籍を保存したとき、表紙の URL があって保存済みの表紙が無ければ、ダウンロードして `covers/{id}.{拡張子}` に保存する。失敗したら URL の画像をそのまま表示し、次の保存で再試行する。
- ダウンロードは Content-Type が画像のもの（`application/octet-stream` は JPEG として扱う）、10 MB まで。100 バイト未満の画像（Amazon の「書影なし」の 1×1 GIF など）は書影なしとして扱う。
- 表紙の変更ダイアログ: (1) PC 内の画像ファイル（PNG / JPG / WebP / GIF、10 MB まで）、(2) 画像の URL、(3) 表紙の削除。新しい表紙を保存したら古い表紙ファイルは消す。
- 画面には `cover_data` で `data:` URL として渡す。

## 改訂履歴

- 2026-11-06: 旧 SPEC-0100 §4 から分割。Amazon / Audible のページを取得しない方針（ISS-0002）、ダウンロードの上限、表紙の変更ダイアログを追記。
