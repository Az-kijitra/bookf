//! Book metadata lookup (SPEC-0100 §4):
//! 1. Link URL: a general web page is read once for its OGP / meta tags. Amazon and Audible pages
//!    are never fetched; only the ASIN / ISBN and title slug written in the URL itself are used.
//! 2. CiNii Books API (National Institute of Informatics - Japanese library catalog)
//! 3. openBD API (Japanese publishers' database for official bibliographic data and covers)
//! 4. Amazon cover image URL built from the ASIN
//! 5. Google Books cover fallback by ISBN
//!
//! Every request identifies itself honestly as `bookf/<version>`; no browser impersonation.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

/// Largest cover image accepted by [`download`].
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
/// Largest HTML page read by [`fetch_from_link`].
const MAX_HTML_BYTES: usize = 5 * 1024 * 1024;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static regex is valid")
}

static RE_ASIN: LazyLock<Regex> = LazyLock::new(|| re(r"/(?:dp|gp/product|ASIN|d)/([A-Za-z0-9]{10})(?:[/?#]|$)"));
static RE_AUDIBLE_ASIN: LazyLock<Regex> = LazyLock::new(|| re(r"/pd/(?:[^/?#]+/)?([A-Za-z0-9]{10})(?:[/?#]|$)"));
static RE_AMAZON_SLUG: LazyLock<Regex> = LazyLock::new(|| re(r"amazon\.co\.jp/(?:Audible-)?([^/?#]+)/dp/"));
static RE_TITLE_TAG: LazyLock<Regex> = LazyLock::new(|| re(r#"(?is)<title[^>]*>(.*?)</title>"#));
static RE_OG_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?is)<meta[^>]+(?:property|name)=["']og:title["'][^>]+content=["'](.*?)["']|<meta[^>]+content=["'](.*?)["'][^>]+(?:property|name)=["']og:title["']"#,
    )
});
static RE_OG_IMAGE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?is)<meta[^>]+(?:property|name)=["'](?:og:image|twitter:image)["'][^>]+content=["'](.*?)["']|<meta[^>]+content=["'](.*?)["'][^>]+(?:property|name)=["'](?:og:image|twitter:image)["']"#,
    )
});
static RE_META_AUTHOR: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?is)<meta[^>]+(?:name|property)=["'](?:author|article:author)["'][^>]+content=["'](.*?)["']"#));
static RE_META_DESC: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?is)<meta[^>]+(?:name|property)=["'](?:description|og:description)["'][^>]+content=["'](.*?)["']"#));

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub isbn: Option<String>,
    pub cover_url: Option<String>,
    pub link_url: Option<String>,
    pub description: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("bookf/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Az-kijitra/bookf)"))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

/// True for Amazon / Audible hosts, whose pages bookf does not fetch.
pub fn is_amazon_url(url: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(url) else { return false };
    let Some(host) = u.host_str() else { return false };
    ["amazon.co.jp", "amazon.com", "audible.co.jp", "audible.com", "amzn.to", "amzn.asia"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Digits of an ISBN typed with or without hyphens; `None` if it is not an ISBN-10/13.
pub fn as_isbn(q: &str) -> Option<String> {
    let s: String = q.chars().filter(|c| !matches!(c, '-' | ' ' | '‐' | '－')).collect();
    let s = s.to_ascii_uppercase();
    let ok13 = s.len() == 13 && s.chars().all(|c| c.is_ascii_digit());
    let ok10 = s.len() == 10 && s[..9].chars().all(|c| c.is_ascii_digit()) && s[9..].chars().all(|c| c.is_ascii_digit() || c == 'X');
    (ok13 || ok10).then_some(s)
}

/// Extracts an Amazon / Audible ASIN or ISBN-10 (10 alphanumeric characters) from a URL.
pub fn extract_asin_from_url(url: &str) -> Option<String> {
    RE_ASIN.captures(url).or_else(|| RE_AUDIBLE_ASIN.captures(url)).and_then(|c| c.get(1)).map(|m| m.as_str().to_ascii_uppercase())
}

/// Title written in an Amazon slug URL (e.g. `amazon.co.jp/SLUG/dp/...`).
pub fn extract_title_from_amazon_url(url: &str) -> Option<String> {
    let slug = RE_AMAZON_SLUG.captures(url)?.get(1)?.as_str();
    let decoded = urlencoding::decode(slug).ok()?;
    let t = decoded.replace(['-', '‐', '―'], " ").trim().to_string();
    (!t.is_empty()).then_some(t)
}

/// ISBN-13 (978 prefix only) to ISBN-10, which Amazon uses as the ASIN of printed books.
pub fn isbn13_to_10(isbn13: &str) -> Option<String> {
    if isbn13.len() != 13 || !isbn13.starts_with("978") || !isbn13.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let body = &isbn13[3..12];
    let sum: u32 = body.chars().enumerate().map(|(i, c)| (10 - i as u32) * c.to_digit(10).unwrap()).sum();
    let check = (11 - sum % 11) % 11;
    let c = if check == 10 { 'X' } else { char::from_digit(check, 10).unwrap() };
    Some(format!("{body}{c}"))
}

/// Converts an ISBN-10 to an ISBN-13 (978 prefix).
pub fn isbn10_to_13(isbn10: &str) -> Option<String> {
    if isbn10.len() != 10 || !isbn10[..9].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let body = format!("978{}", &isbn10[..9]);
    let sum: u32 = body
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let weight = if i % 2 == 0 { 1 } else { 3 };
            weight * c.to_digit(10).unwrap()
        })
        .sum();
    let check = (10 - (sum % 10)) % 10;
    Some(format!("{body}{check}"))
}

pub fn amazon_url(isbn: Option<&str>, title: &str) -> String {
    if let Some(isbn) = isbn {
        let ten = if isbn.len() == 10 { Some(isbn.to_string()) } else { isbn13_to_10(isbn) };
        if let Some(ten) = ten {
            return format!("https://www.amazon.co.jp/dp/{ten}");
        }
        return format!("https://www.amazon.co.jp/s?k={isbn}");
    }
    format!("https://www.amazon.co.jp/s?k={}&i=stripbooks", urlencoding::encode(title))
}

/// Amazon high-res cover URL from ASIN.
pub fn amazon_cover_url(asin: &str) -> String {
    format!("https://images-na.ssl-images-amazon.com/images/P/{asin}.01.LZZZZZZZ.jpg")
}

/// Direct cover image URL from Google Books by ISBN.
pub fn google_cover_url(isbn: &str) -> String {
    format!("https://books.google.com/books/content?vid=isbn:{isbn}&printsec=frontcover&img=1&zoom=1")
}

fn s(v: &Value) -> Option<String> {
    v.as_str().map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}

/// Parses CiNii Books OpenSearch JSON response.
pub fn parse_cinii(v: &Value) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let Some(graphs) = v.get("@graph").and_then(Value::as_array) else { return candidates };
    for g in graphs {
        let Some(items) = g.get("items").and_then(Value::as_array) else { continue };
        for it in items {
            let Some(raw_title) = s(&it["title"]) else { continue };
            let (title, subtitle) = match raw_title.split_once(" : ") {
                Some((m, sub)) => (m.trim().to_string(), Some(sub.trim().to_string())),
                None => (raw_title, None),
            };

            let author =
                s(&it["dc:creator"]).map(|a| a.trim_end_matches('著').trim_end_matches('編').trim_end_matches('訳').trim().to_string());

            let publisher = match &it["dc:publisher"] {
                Value::Array(arr) => arr.first().and_then(s),
                Value::String(_) => s(&it["dc:publisher"]),
                _ => None,
            };

            let published_date = s(&it["prism:publicationDate"]).or_else(|| s(&it["dc:date"]));

            let mut isbn = None;
            let parts = match &it["dcterms:hasPart"] {
                Value::Array(arr) => arr.clone(),
                Value::Object(_) => vec![it["dcterms:hasPart"].clone()],
                _ => vec![],
            };
            for p in parts {
                if let Some(id_str) = s(&p["@id"]) {
                    if let Some(code) = id_str.strip_prefix("urn:isbn:") {
                        if let Some(normalized) = as_isbn(code) {
                            isbn = Some(normalized);
                            break;
                        }
                    }
                }
            }

            candidates.push(Candidate {
                link_url: Some(amazon_url(isbn.as_deref(), &title)),
                title,
                subtitle,
                author,
                publisher,
                published_date,
                isbn,
                cover_url: None,
                description: None,
            });
        }
    }
    candidates
}

/// openBD `pubdate` is `20160525`, `201605` or `2016-05-25`.
fn openbd_date(d: &str) -> String {
    let digits: String = d.chars().filter(char::is_ascii_digit).collect();
    match digits.len() {
        8 => format!("{}-{}-{}", &digits[..4], &digits[4..6], &digits[6..]),
        6 => format!("{}-{}", &digits[..4], &digits[4..]),
        _ => d.to_string(),
    }
}

/// openBD writes authors like `高田博行／著` or `Ａ／著 Ｂ／訳`; the role suffix of the author is dropped.
fn openbd_author(a: &str) -> String {
    a.split_whitespace().map(|p| p.trim_end_matches("／著").replace('／', " / ")).collect::<Vec<_>>().join(", ")
}

/// Applies openBD's `summary` (array aligned with the requested ISBNs) on top of the candidates.
pub fn merge_openbd(cands: &mut [Candidate], v: &Value) {
    let Some(arr) = v.as_array() else { return };
    for entry in arr {
        let sum = &entry["summary"];
        let Some(isbn) = s(&sum["isbn"]) else { continue };
        let isbn10 = isbn13_to_10(&isbn);
        for c in cands
            .iter_mut()
            .filter(|c| c.isbn.as_deref() == Some(isbn.as_str()) || (isbn10.is_some() && c.isbn.as_deref() == isbn10.as_deref()))
        {
            if let Some(t) = s(&sum["title"]) {
                c.title = t;
            }
            if let Some(p) = s(&sum["publisher"]) {
                c.publisher = Some(p);
            }
            let cov = s(&sum["cover"]);
            if cov.is_some() {
                c.cover_url = cov;
            }
            if c.author.is_none() {
                c.author = s(&sum["author"]).map(|a| openbd_author(&a));
            }
            if let Some(d) = s(&sum["pubdate"]) {
                c.published_date = Some(openbd_date(&d));
            }
        }
    }
}

async fn get_json(url: &str) -> Result<Value, String> {
    let resp = client()?.get(url).send().await.map_err(|e| format!("通信に失敗しました: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("検索サービスがエラーを返しました ({})", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// Queries CiNii Books with the given parameters.
async fn query_cinii(title: Option<&str>, author: Option<&str>, isbn: Option<&str>) -> Vec<Candidate> {
    let mut params = vec!["format=json".to_string(), "count=15".to_string()];
    if let Some(t) = title {
        if !t.is_empty() {
            params.push(format!("title={}", urlencoding::encode(t)));
        }
    }
    if let Some(a) = author {
        if !a.is_empty() {
            params.push(format!("author={}", urlencoding::encode(a)));
        }
    }
    if let Some(i) = isbn {
        if !i.is_empty() {
            params.push(format!("isbn={}", urlencoding::encode(i)));
        }
    }
    let url = format!("https://ci.nii.ac.jp/books/opensearch/search?{}", params.join("&"));
    if let Ok(json) = get_json(&url).await {
        parse_cinii(&json)
    } else {
        vec![]
    }
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

fn first_capture(re: &Regex, html: &str) -> Option<String> {
    let cap = re.captures(html)?;
    let m = cap.get(1).or_else(|| cap.get(2))?;
    let t = decode_html_entities(m.as_str().trim());
    (!t.is_empty()).then_some(t)
}

/// Candidate built only from what an Amazon / Audible URL says: the ASIN (an ISBN-10 for printed
/// books), the cover image URL derived from it, and the title slug. The page itself is not read.
pub fn candidate_from_amazon_url(url: &str) -> Candidate {
    let asin = extract_asin_from_url(url);
    let isbn = asin.as_deref().and_then(as_isbn).map(|i| if i.len() == 10 { isbn10_to_13(&i).unwrap_or(i) } else { i });
    Candidate {
        title: extract_title_from_amazon_url(url).unwrap_or_default(),
        isbn,
        cover_url: asin.as_deref().map(amazon_cover_url),
        link_url: Some(url.to_string()),
        ..Default::default()
    }
}

/// Book metadata from the OGP / meta tags of a general web page (no network access).
pub fn parse_html(url: &str, html: &str) -> Candidate {
    let title = first_capture(&RE_OG_TITLE, html).or_else(|| {
        let t = first_capture(&RE_TITLE_TAG, html)?;
        let cleaned = t.split(['|', '｜']).next().unwrap_or(&t).trim().to_string();
        (!cleaned.is_empty()).then_some(cleaned)
    });
    let cover_url = first_capture(&RE_OG_IMAGE, html).and_then(|img| {
        if img.starts_with("http://") || img.starts_with("https://") {
            Some(img)
        } else {
            reqwest::Url::parse(url).ok()?.join(&img).ok().map(|u| u.to_string())
        }
    });
    Candidate {
        title: title.unwrap_or_default(),
        author: first_capture(&RE_META_AUTHOR, html),
        cover_url,
        description: first_capture(&RE_META_DESC, html),
        link_url: Some(url.to_string()),
        ..Default::default()
    }
}

/// Fills the candidate from openBD, then CiNii Books, when it carries an ISBN.
async fn enrich_by_isbn(mut c: Candidate) -> Candidate {
    let Some(isbn) = c.isbn.clone() else { return c };
    if let Ok(v) = get_json(&format!("https://api.openbd.jp/v1/get?isbn={isbn}")).await {
        merge_openbd(std::slice::from_mut(&mut c), &v);
    }
    if c.title.is_empty() || c.author.is_none() {
        if let Some(found) = query_cinii(None, None, Some(&isbn)).await.into_iter().next() {
            if c.title.is_empty() {
                c.title = found.title;
                c.subtitle = c.subtitle.or(found.subtitle);
            }
            c.author = c.author.or(found.author);
            c.publisher = c.publisher.or(found.publisher);
            c.published_date = c.published_date.or(found.published_date);
        }
    }
    c
}

async fn fetch_html(url: &str) -> Result<String, String> {
    let resp = client()?
        .get(url)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| format!("ページを取得できません: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("ページを取得できません ({})", resp.status()));
    }
    if resp.content_length().is_some_and(|n| n as usize > MAX_HTML_BYTES) {
        return Err("ページが大きすぎます".into());
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_HTML_BYTES {
        return Err("ページが大きすぎます".into());
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Book metadata for a link URL. Amazon / Audible links are resolved from the URL alone
/// (ASIN / ISBN → openBD / CiNii); other pages are fetched once and read for OGP tags.
pub async fn fetch_from_link(url: &str) -> Result<Candidate, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("URLが空です".into());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("有効な URL を入力してください（http:// または https://）".into());
    }

    let c = if is_amazon_url(url) {
        let c = enrich_by_isbn(candidate_from_amazon_url(url)).await;
        if c.title.is_empty() && c.cover_url.is_none() {
            return Err("この Amazon の URL からは書誌情報が分かりません。ISBN かタイトルで検索してください。".into());
        }
        c
    } else {
        enrich_by_isbn(parse_html(url, &fetch_html(url).await?)).await
    };
    Ok(c)
}

pub async fn search(query: &str) -> Result<Vec<Candidate>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    // 1. Direct URL fetch if user query is a URL
    if query.starts_with("http://") || query.starts_with("https://") {
        if let Ok(c) = fetch_from_link(query).await {
            return Ok(vec![c]);
        }
    }

    let mut cands = Vec::new();

    // 2. Direct ISBN search
    if let Some(isbn) = as_isbn(query) {
        let isbn13 = if isbn.len() == 10 { isbn10_to_13(&isbn) } else { Some(isbn.clone()) };
        let isbn_to_query = isbn13.as_deref().unwrap_or(&isbn);
        let openbd_url = format!("https://api.openbd.jp/v1/get?isbn={isbn_to_query}");
        if let Ok(v) = get_json(&openbd_url).await {
            let mut temp = vec![Candidate { isbn: Some(isbn_to_query.to_string()), ..Default::default() }];
            merge_openbd(&mut temp, &v);
            if !temp[0].title.is_empty() {
                cands.extend(temp);
            }
        }
        if cands.is_empty() {
            cands.extend(query_cinii(None, None, Some(isbn_to_query)).await);
        }
        if cands.is_empty() {
            cands.push(Candidate { isbn: Some(isbn_to_query.to_string()), ..Default::default() });
        }
    } else {
        // 3. Keyword / Title search in CiNii Books
        cands.extend(query_cinii(Some(query), None, None).await);

        if cands.is_empty() {
            let parts: Vec<&str> = query.split([' ', '　', ':', '：']).filter(|s| !s.trim().is_empty()).collect();
            if parts.len() >= 2 {
                cands.extend(query_cinii(Some(parts[0]), Some(parts[1]), None).await);
                if cands.is_empty() {
                    cands.extend(query_cinii(Some(parts[0]), None, None).await);
                }
            }
        }
    }

    // 4. Enrich all candidates with openBD & fallback cover image URLs
    let isbns: Vec<String> = cands.iter().filter_map(|c| c.isbn.clone()).collect();
    if !isbns.is_empty() {
        let openbd_url = format!("https://api.openbd.jp/v1/get?isbn={}", isbns.join(","));
        if let Ok(v) = get_json(&openbd_url).await {
            merge_openbd(&mut cands, &v);
        }
    }

    cands.retain(|c| !c.title.is_empty());
    for c in cands.iter_mut() {
        c.link_url = Some(amazon_url(c.isbn.as_deref(), &c.title));
        if c.cover_url.is_none() {
            if let Some(isbn) = &c.isbn {
                c.cover_url = Some(google_cover_url(isbn));
            }
        }
    }

    Ok(cands)
}

/// Downloads an image (at most [`MAX_IMAGE_BYTES`]); returns the bytes and a file extension.
pub async fn download(url: &str) -> Result<(Vec<u8>, &'static str), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("有効な URL を入力してください（http:// または https://）".into());
    }
    let resp = client()?.get(url).send().await.map_err(|e| format!("表紙を取得できません: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("表紙を取得できません ({})", resp.status()));
    }
    let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let ext = image_ext(&ctype).ok_or_else(|| format!("画像ではありません ({ctype})"))?;
    if resp.content_length().is_some_and(|n| n as usize > MAX_IMAGE_BYTES) {
        return Err("画像が大きすぎます（10 MB まで）".into());
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("画像が大きすぎます（10 MB まで）".into());
    }
    // Amazon answers a missing cover with a 1x1 GIF (43 bytes); treat tiny images as missing.
    if bytes.len() < 100 {
        return Err("表紙画像がありません".into());
    }
    Ok((bytes.to_vec(), ext))
}

/// File extension for an image Content-Type; `None` when it is not an image.
/// A missing or generic type is accepted as JPEG, which some CDNs send as `application/octet-stream`.
pub fn image_ext(content_type: &str) -> Option<&'static str> {
    match content_type {
        t if t.contains("png") => Some("png"),
        t if t.contains("webp") => Some("webp"),
        t if t.contains("gif") => Some("gif"),
        t if t.contains("jpeg") || t.contains("jpg") => Some("jpg"),
        "" | "application/octet-stream" | "binary/octet-stream" => Some("jpg"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore]
    fn live_search() {
        let rt = tauri::async_runtime::block_on(async {
            (search("ヒトラー演説 熱狂の真実").await, search("具体と抽象").await, search("9784121022721").await)
        });
        let (a, b, c) = rt;
        assert!(!a.unwrap().is_empty());
        assert!(!b.unwrap().is_empty());
        assert!(!c.unwrap().is_empty());
    }

    #[test]
    #[ignore]
    fn live_fetch_link() {
        let rt = tauri::async_runtime::block_on(async {
            (fetch_from_link("https://www.amazon.co.jp/dp/4121022726").await, fetch_from_link("http://henshu-no-bunpo.com/").await)
        });
        let (amazon, site) = rt;
        let amazon = amazon.unwrap();
        assert!(amazon.title.contains("ヒトラー演説"));
        assert_eq!(amazon.isbn.as_deref(), Some("9784121022721"));

        let site = site.unwrap();
        assert!(site.title.contains("才能に頼らない文章術"));
        assert!(site.cover_url.is_some());
    }

    #[test]
    fn amazon_hosts_are_recognised() {
        assert!(is_amazon_url("https://www.amazon.co.jp/dp/4121022726"));
        assert!(is_amazon_url("https://amzn.asia/d/abc"));
        assert!(is_amazon_url("https://www.audible.co.jp/pd/B0CSK3YCLM"));
        assert!(!is_amazon_url("https://example.com/amazon.co.jp/dp/4121022726"));
        assert!(!is_amazon_url("https://notamazon.co.jp/"));
        assert!(!is_amazon_url("not a url"));
    }

    #[test]
    fn amazon_candidate_comes_from_the_url_only() {
        let c = candidate_from_amazon_url("https://www.amazon.co.jp/ヒトラー演説-熱狂の真実-中公新書-高田-博行/dp/4121022726/ref=sr_1_1");
        assert_eq!(c.isbn.as_deref(), Some("9784121022721"));
        assert_eq!(c.cover_url.as_deref(), Some("https://images-na.ssl-images-amazon.com/images/P/4121022726.01.LZZZZZZZ.jpg"));
        assert!(c.title.starts_with("ヒトラー演説 熱狂の真実"));

        let kindle = candidate_from_amazon_url("https://www.amazon.co.jp/dp/B09771DDFL");
        assert_eq!(kindle.isbn, None);
        assert!(kindle.title.is_empty());
        assert!(kindle.cover_url.is_some());

        let audible = candidate_from_amazon_url("https://www.audible.co.jp/pd/Some-Title-Audiobook/B0CSK3YCLM?ref=x");
        assert!(audible.cover_url.unwrap().contains("B0CSK3YCLM"));
    }

    #[test]
    fn html_ogp_is_parsed() {
        let html = r#"<html><head>
            <title>Fallback | Site</title>
            <meta property="og:title" content="才能に頼らない文章術 &amp; 編集">
            <meta property="og:image" content="/img/cover.jpg">
            <meta name="author" content="上阪 徹">
            <meta name="description" content="説明文">
        </head></html>"#;
        let c = parse_html("https://example.com/book/1", html);
        assert_eq!(c.title, "才能に頼らない文章術 & 編集");
        assert_eq!(c.cover_url.as_deref(), Some("https://example.com/img/cover.jpg"));
        assert_eq!(c.author.as_deref(), Some("上阪 徹"));
        assert_eq!(c.description.as_deref(), Some("説明文"));
        assert_eq!(c.link_url.as_deref(), Some("https://example.com/book/1"));
    }

    #[test]
    fn html_title_tag_is_the_fallback() {
        let c = parse_html("https://example.com/", "<title>本のタイトル ｜ 出版社</title>");
        assert_eq!(c.title, "本のタイトル");
        assert_eq!(c.cover_url, None);
        assert!(parse_html("https://example.com/", "<p>no title</p>").title.is_empty());
    }

    #[test]
    fn entities_are_decoded_once() {
        assert_eq!(decode_html_entities("&amp;lt;"), "&lt;");
        assert_eq!(decode_html_entities("A &amp; B &quot;C&quot;"), "A & B \"C\"");
    }

    #[test]
    fn image_content_types() {
        assert_eq!(image_ext("image/png"), Some("png"));
        assert_eq!(image_ext("image/jpeg"), Some("jpg"));
        assert_eq!(image_ext(""), Some("jpg"));
        assert_eq!(image_ext("text/html; charset=utf-8"), None);
    }

    #[test]
    fn openbd_summary_is_merged() {
        let mut cands = vec![Candidate { isbn: Some("4121022726".into()), ..Default::default() }];
        let v = json!([{ "summary": {
            "isbn": "9784121022721", "title": "ヒトラー演説", "publisher": "中央公論新社",
            "author": "高田博行／著", "pubdate": "20140620", "cover": "https://cover.openbd.jp/9784121022721.jpg"
        }}]);
        merge_openbd(&mut cands, &v);
        assert_eq!(cands[0].title, "ヒトラー演説");
        assert_eq!(cands[0].author.as_deref(), Some("高田博行"));
        assert_eq!(cands[0].published_date.as_deref(), Some("2014-06-20"));
        assert!(cands[0].cover_url.is_some());
    }

    #[test]
    fn asin_extraction() {
        assert_eq!(extract_asin_from_url("https://www.amazon.co.jp/dp/4121022726/ref=sr_1_1").as_deref(), Some("4121022726"));
        assert_eq!(extract_asin_from_url("https://www.amazon.co.jp/gp/product/B09771DDFL").as_deref(), Some("B09771DDFL"));
        assert_eq!(extract_asin_from_url("http://example.com/books/123"), None);
    }

    #[test]
    fn isbn_detection_and_conversion() {
        assert_eq!(as_isbn("978-4-12-102272-1").as_deref(), Some("9784121022721"));
        assert_eq!(as_isbn("412102272X").as_deref(), Some("412102272X"));
        assert_eq!(isbn13_to_10("9784121022721").as_deref(), Some("4121022726"));
        assert_eq!(isbn10_to_13("4121022726").as_deref(), Some("9784121022721"));
    }

    #[test]
    fn cinii_parsing() {
        let j = json!({
            "@graph": [{
                "items": [{
                    "title": "ヒトラー演説 : 熱狂の真実",
                    "dc:creator": "高田博行著",
                    "dc:publisher": ["中央公論新社"],
                    "prism:publicationDate": "2014",
                    "dcterms:hasPart": [{"@id": "urn:isbn:9784121022721"}]
                }]
            }]
        });
        let cands = parse_cinii(&j);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].title, "ヒトラー演説");
        assert_eq!(cands[0].subtitle.as_deref(), Some("熱狂の真実"));
        assert_eq!(cands[0].author.as_deref(), Some("高田博行"));
        assert_eq!(cands[0].isbn.as_deref(), Some("9784121022721"));
    }
}
