//! Typed PocketBase calls. Every call is non-blocking: the result is handed to
//! `on_done`, which is called from the browser event loop (see controller.rs).
//! Filter strings are only ever built here, from validated ids and numbers.

use std::collections::BTreeSet;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::error::ApiError;
use super::models::{
    AuthResponse, Campsite, CampsiteInput, CampsiteMarker, CampsiteStats, Comment, ListResponse, PhotoUpload, Rating,
    Report, ReportInput, Tag, User,
};

pub type Done<T> = Box<dyn FnOnce(Result<T, ApiError>) + Send>;

pub const MARKERS_PER_REQUEST: u32 = 200;
pub const COMMENTS_PER_PAGE: u32 = 20;
pub const SEARCH_RESULTS_PER_PAGE: u32 = 50;
/// Longest search query sent to the server (campsite titles are at most 100 chars).
pub const SEARCH_MAX_CHARS: usize = 100;
/// Tent capacity range of the `campsites.tent_capacity` field; 10 means "10+".
const TENT_MIN: u8 = 1;
const TENT_MAX: u8 = 10;

/// Map viewport in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

/// Global map filters (ARCHITECTURE §5.5). They apply to the markers and to search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampsiteFilter {
    /// Tag record ids. A campsite must have all of them.
    pub tags: BTreeSet<String>,
    /// Inclusive tent capacity range, 1–10 where 10 means "10+".
    pub min_tents: u8,
    pub max_tents: u8,
}

impl Default for CampsiteFilter {
    fn default() -> Self {
        Self { tags: BTreeSet::new(), min_tents: TENT_MIN, max_tents: TENT_MAX }
    }
}

impl CampsiteFilter {
    /// Number of active criteria (each tag counts, the tent range counts once).
    pub fn active_count(&self) -> usize {
        self.tags.len() + usize::from(self.min_tents > TENT_MIN || self.max_tents < TENT_MAX)
    }

    pub fn is_active(&self) -> bool {
        self.active_count() > 0
    }

    /// Filter clauses, AND-combined by the caller. `tags ~ 'id'` (and not `tags ?= 'id'`):
    /// with the pinned PocketBase, several `?=` on the same relation never all match.
    fn clauses(&self) -> Vec<String> {
        let mut out: Vec<String> =
            self.tags.iter().filter(|id| is_record_id(id)).map(|id| format!("tags ~ '{id}'")).collect();
        if self.min_tents > TENT_MIN {
            out.push(format!("tent_capacity >= {}", self.min_tents));
        }
        // 10 means "10+": no upper bound.
        if self.max_tents < TENT_MAX {
            out.push(format!("tent_capacity <= {}", self.max_tents));
        }
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct ApiClient {
    /// Empty = same origin (Trunk proxies /api in dev); otherwise `config::api_url()`.
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), token: None }
    }

    pub fn set_token(&mut self, token: Option<&str>) {
        self.token = token.map(str::to_owned);
    }

    // ---- public data

    pub fn count_campsites(&self, on_done: Done<i64>) {
        let url = self.url("/api/collections/campsites/records", &[("perPage", "1"), ("fields", "id")]);
        self.send(ehttp::Request::get(url), move |r: Result<ListResponse<serde_json::Value>, _>| {
            on_done(r.map(|l| l.total_items))
        });
    }

    pub fn list_tags(&self, on_done: Done<Vec<Tag>>) {
        let url = self.url(
            "/api/collections/tags/records",
            &[("perPage", "200"), ("sort", "sort_order,label"), ("filter", "active = true")],
        );
        self.send(ehttp::Request::get(url), move |r: Result<ListResponse<Tag>, _>| on_done(r.map(|l| l.items)));
    }

    pub fn list_campsites_in_bbox(&self, bbox: BBox, filter: &CampsiteFilter, on_done: Done<Vec<CampsiteMarker>>) {
        let per_page = MARKERS_PER_REQUEST.to_string();
        let url = self.url(
            "/api/collections/campsites/records",
            &[
                ("filter", &and_all(std::iter::once(bbox_filter(bbox)).chain(filter.clauses()))),
                ("fields", "id,title,lat,lng,author"),
                ("perPage", &per_page),
                ("skipTotal", "true"),
            ],
        );
        self.send(ehttp::Request::get(url), move |r: Result<ListResponse<CampsiteMarker>, _>| {
            on_done(r.map(|l| l.items))
        });
    }

    /// Campsites whose title contains `query`, anywhere on the map, sorted by title.
    pub fn search_campsites(&self, query: &str, filter: &CampsiteFilter, on_done: Done<ListResponse<CampsiteMarker>>) {
        let per_page = SEARCH_RESULTS_PER_PAGE.to_string();
        let url = self.url(
            "/api/collections/campsites/records",
            &[
                ("filter", &and_all(std::iter::once(search_filter(query)).chain(filter.clauses()))),
                ("fields", "id,title,lat,lng,author"),
                ("sort", "title"),
                ("perPage", &per_page),
            ],
        );
        self.send(ehttp::Request::get(url), on_done);
    }

    pub fn get_campsite(&self, id: &str, on_done: Done<Campsite>) {
        if let Err(e) = check_id(id) {
            return on_done(Err(e));
        }
        let url = self.url(&format!("/api/collections/campsites/records/{id}"), &[("expand", "author,tags")]);
        self.send(ehttp::Request::get(url), on_done);
    }

    /// `None` when the campsite has no stats row (e.g. it is hidden).
    pub fn get_stats(&self, id: &str, on_done: Done<Option<CampsiteStats>>) {
        if let Err(e) = check_id(id) {
            return on_done(Err(e));
        }
        let url = self.url(&format!("/api/collections/campsite_stats/records/{id}"), &[]);
        self.send(ehttp::Request::get(url), move |r: Result<CampsiteStats, ApiError>| {
            on_done(match r {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.is_not_found() => Ok(None),
                Err(e) => Err(e),
            })
        });
    }

    pub fn list_comments(&self, campsite_id: &str, page: u32, on_done: Done<ListResponse<Comment>>) {
        if let Err(e) = check_id(campsite_id) {
            return on_done(Err(e));
        }
        let id = campsite_id;
        let (page, per_page) = (page.to_string(), COMMENTS_PER_PAGE.to_string());
        let url = self.url(
            "/api/collections/comments/records",
            &[
                ("filter", &format!("campsite = '{id}'")),
                ("sort", "-created"),
                ("expand", "author"),
                ("page", &page),
                ("perPage", &per_page),
            ],
        );
        self.send(ehttp::Request::get(url), on_done);
    }

    // ---- auth & profile

    pub fn auth_with_password(&self, email: &str, password: &str, on_done: Done<AuthResponse>) {
        let url = self.url("/api/collections/users/auth-with-password", &[]);
        self.send_json(ehttp::Method::POST, url, &json!({ "identity": email, "password": password }), on_done);
    }

    pub fn auth_refresh(&self, on_done: Done<AuthResponse>) {
        let url = self.url("/api/collections/users/auth-refresh", &[]);
        self.send(ehttp::Request::post(url, Vec::new()), on_done);
    }

    pub fn register(&self, name: &str, email: &str, password: &str, confirm: &str, on_done: Done<User>) {
        let url = self.url("/api/collections/users/records", &[]);
        let body = json!({ "name": name, "email": email, "password": password, "passwordConfirm": confirm });
        self.send_json(ehttp::Method::POST, url, &body, on_done);
    }

    /// Sends the confirmation email. PocketBase answers 204 whether or not the email exists.
    pub fn request_verification(&self, email: &str, on_done: Done<()>) {
        let url = self.url("/api/collections/users/request-verification", &[]);
        let body = json!({ "email": email }).to_string().into_bytes();
        let mut req = ehttp::Request::post(url, body);
        req.headers.insert("Content-Type", "application/json");
        self.send_empty(req, on_done);
    }

    pub fn update_name(&self, user_id: &str, name: &str, on_done: Done<User>) {
        if let Err(e) = check_id(user_id) {
            return on_done(Err(e));
        }
        let id = user_id;
        let url = self.url(&format!("/api/collections/users/records/{id}"), &[]);
        self.send_json(ehttp::Method::PATCH, url, &json!({ "name": name }), on_done);
    }

    /// PocketBase invalidates existing tokens on password change: log in again afterwards.
    pub fn change_password(&self, user_id: &str, old: &str, new: &str, confirm: &str, on_done: Done<User>) {
        if let Err(e) = check_id(user_id) {
            return on_done(Err(e));
        }
        let id = user_id;
        let url = self.url(&format!("/api/collections/users/records/{id}"), &[]);
        let body = json!({ "oldPassword": old, "password": new, "passwordConfirm": confirm });
        self.send_json(ehttp::Method::PATCH, url, &body, on_done);
    }

    // ---- campsites, ratings, comments

    /// With new photos the request is multipart (see `send_multipart`); otherwise plain JSON.
    pub fn create_campsite(&self, input: &CampsiteInput, photos: &[PhotoUpload], on_done: Done<Campsite>) {
        let url = self.url("/api/collections/campsites/records", &[("expand", "author,tags")]);
        self.send_campsite(ehttp::Method::POST, url, input, photos, on_done);
    }

    pub fn update_campsite(&self, id: &str, input: &CampsiteInput, photos: &[PhotoUpload], on_done: Done<Campsite>) {
        if let Err(e) = check_id(id) {
            return on_done(Err(e));
        }
        let url = self.url(&format!("/api/collections/campsites/records/{id}"), &[("expand", "author,tags")]);
        self.send_campsite(ehttp::Method::PATCH, url, input, photos, on_done);
    }

    fn send_campsite(
        &self,
        method: ehttp::Method,
        url: String,
        input: &CampsiteInput,
        photos: &[PhotoUpload],
        on_done: Done<Campsite>,
    ) {
        if photos.is_empty() {
            self.send_json(method, url, input, on_done);
        } else {
            self.send_multipart(method, url, input, "photos+", photos, on_done);
        }
    }

    pub fn delete_campsite(&self, id: &str, on_done: Done<()>) {
        if let Err(e) = check_id(id) {
            return on_done(Err(e));
        }
        let url = self.url(&format!("/api/collections/campsites/records/{id}"), &[]);
        self.send_empty(ehttp::Request::delete(&url), on_done);
    }

    pub fn my_rating(&self, campsite_id: &str, user_id: &str, on_done: Done<Option<Rating>>) {
        if let Err(e) = check_id(campsite_id).and_then(|()| check_id(user_id)) {
            return on_done(Err(e));
        }
        let (cid, uid) = (campsite_id, user_id);
        let url = self.url(
            "/api/collections/ratings/records",
            &[("filter", &format!("campsite = '{cid}' && author = '{uid}'")), ("perPage", "1")],
        );
        self.send(ehttp::Request::get(url), move |r: Result<ListResponse<Rating>, _>| {
            on_done(r.map(|l| l.items.into_iter().next()))
        });
    }

    pub fn create_rating(&self, campsite_id: &str, user_id: &str, score: u8, on_done: Done<Rating>) {
        let url = self.url("/api/collections/ratings/records", &[]);
        let body = json!({ "campsite": campsite_id, "author": user_id, "score": score });
        self.send_json(ehttp::Method::POST, url, &body, on_done);
    }

    pub fn update_rating(&self, rating_id: &str, score: u8, on_done: Done<Rating>) {
        if let Err(e) = check_id(rating_id) {
            return on_done(Err(e));
        }
        let id = rating_id;
        let url = self.url(&format!("/api/collections/ratings/records/{id}"), &[]);
        self.send_json(ehttp::Method::PATCH, url, &json!({ "score": score }), on_done);
    }

    pub fn create_comment(&self, campsite_id: &str, user_id: &str, body: &str, on_done: Done<Comment>) {
        let url = self.url("/api/collections/comments/records", &[("expand", "author")]);
        let body = json!({ "campsite": campsite_id, "author": user_id, "body": body });
        self.send_json(ehttp::Method::POST, url, &body, on_done);
    }

    pub fn delete_comment(&self, id: &str, on_done: Done<()>) {
        if let Err(e) = check_id(id) {
            return on_done(Err(e));
        }
        let url = self.url(&format!("/api/collections/comments/records/{id}"), &[]);
        self.send_empty(ehttp::Request::delete(&url), on_done);
    }

    // ---- reports

    pub fn create_report(&self, input: &ReportInput, on_done: Done<Report>) {
        let target = input.campsite.as_deref().or(input.comment.as_deref()).unwrap_or_default();
        if let Err(e) = check_id(target) {
            return on_done(Err(e));
        }
        let url = self.url("/api/collections/reports/records", &[]);
        self.send_json(ehttp::Method::POST, url, input, on_done);
    }

    // ---- plumbing

    fn url(&self, path: &str, query: &[(&str, &str)]) -> String {
        let mut url = format!("{}{}", self.base_url, path);
        for (i, (k, v)) in query.iter().enumerate() {
            url.push(if i == 0 { '?' } else { '&' });
            url.push_str(k);
            url.push('=');
            url.push_str(&encode_component(v));
        }
        url
    }

    fn with_auth(&self, mut req: ehttp::Request) -> ehttp::Request {
        if let Some(token) = &self.token {
            req.headers.insert("Authorization", token);
        }
        req
    }

    fn send_json<B: Serialize, T: DeserializeOwned + 'static>(
        &self,
        method: ehttp::Method,
        url: String,
        body: &B,
        on_done: impl FnOnce(Result<T, ApiError>) + Send + 'static,
    ) {
        let bytes = match serde_json::to_vec(body) {
            Ok(b) => b,
            Err(e) => return on_done(Err(ApiError::network(e.to_string()))),
        };
        let mut req = ehttp::Request::post(url, bytes);
        req.method = method;
        req.headers.insert("Content-Type", "application/json");
        self.send(req, on_done);
    }

    /// One `multipart/form-data` request: the JSON fields go in PocketBase's `@jsonPayload`
    /// part (so the collection rules see them exactly like a JSON body) and each file in a
    /// `file_field` part.
    fn send_multipart<B: Serialize, T: DeserializeOwned + 'static>(
        &self,
        method: ehttp::Method,
        url: String,
        body: &B,
        file_field: &str,
        files: &[PhotoUpload],
        on_done: impl FnOnce(Result<T, ApiError>) + Send + 'static,
    ) {
        let json = match serde_json::to_vec(body) {
            Ok(b) => b,
            Err(e) => return on_done(Err(ApiError::network(e.to_string()))),
        };
        let boundary = multipart_boundary(&json, files);
        let mut req = ehttp::Request::post(url, multipart_body(&boundary, &json, file_field, files));
        req.method = method;
        req.headers.insert("Content-Type", format!("multipart/form-data; boundary={boundary}"));
        self.send(req, on_done);
    }

    fn send<T: DeserializeOwned + 'static>(
        &self,
        req: ehttp::Request,
        on_done: impl FnOnce(Result<T, ApiError>) + Send + 'static,
    ) {
        ehttp::fetch(self.with_auth(req), move |result| {
            on_done(result.map_err(ApiError::network).and_then(|resp| {
                if resp.ok {
                    serde_json::from_slice(&resp.bytes).map_err(|e| ApiError::network(format!("bad response: {e}")))
                } else {
                    Err(ApiError::from_response(resp.status, &resp.bytes))
                }
            }))
        });
    }

    fn send_empty(&self, req: ehttp::Request, on_done: Done<()>) {
        ehttp::fetch(self.with_auth(req), move |result| {
            on_done(
                result.map_err(ApiError::network).and_then(|resp| {
                    if resp.ok { Ok(()) } else { Err(ApiError::from_response(resp.status, &resp.bytes)) }
                }),
            )
        });
    }
}

/// PocketBase record ids are short alphanumeric strings. Anything else is refused
/// before it can reach a URL path or a filter expression.
pub fn is_record_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn check_id(id: &str) -> Result<(), ApiError> {
    if is_record_id(id) { Ok(()) } else { Err(ApiError::network(format!("invalid record id {id:?}"))) }
}

/// Server-side image sizes, declared as `thumbs` on the `photos` field (backend/pb_migrations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    /// Panel thumbnails.
    Thumb,
    /// The full-page viewer. Never the original: it can be bigger than a GPU texture.
    Large,
}

impl PhotoSize {
    fn thumb(self) -> &'static str {
        match self {
            PhotoSize::Thumb => "320x240",
            PhotoSize::Large => "1200x1200f",
        }
    }
}

/// GET a text file served next to the app (a document), not a PocketBase call: no auth header.
/// An HTML answer means the host served its fallback page instead of the file.
pub fn fetch_text(url: &str, on_done: Done<String>) {
    ehttp::fetch(ehttp::Request::get(url), move |result| {
        on_done(result.map_err(ApiError::network).and_then(|resp| {
            if !resp.ok {
                return Err(ApiError::from_response(resp.status, &resp.bytes));
            }
            if resp.content_type().is_some_and(|t| t.starts_with("text/html")) {
                return Err(ApiError::from_response(404, b""));
            }
            String::from_utf8(resp.bytes).map_err(|e| ApiError::network(format!("bad response: {e}")))
        }))
    });
}

/// Absolute URL of a campsite photo. `api_base` is the absolute PocketBase base URL (`AppState::api_base`):
/// egui's image loader only fetches `http(s)://` URIs. `None` if an id or the filename is
/// not safe to put in a URL path.
pub fn photo_url(api_base: &str, campsite: &Campsite, file: &str, size: PhotoSize) -> Option<String> {
    let safe_file = !file.is_empty()
        && file.len() <= 255
        && !file.starts_with('.')
        && file.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'));
    // Collection ids look like `pbc_1626116840`: record-id characters plus `_`.
    let c = &campsite.collection_id;
    let safe_collection = !c.is_empty() && c.len() <= 64 && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    (safe_collection && is_record_id(&campsite.id) && safe_file).then(|| {
        format!("{api_base}/api/files/{}/{}/{file}?thumb={}", campsite.collection_id, campsite.id, size.thumb())
    })
}

/// A boundary that appears in none of the parts. Deterministic, so the body is testable.
fn multipart_boundary(json: &[u8], files: &[PhotoUpload]) -> String {
    let contains = |haystack: &[u8], needle: &str| haystack.windows(needle.len()).any(|w| w == needle.as_bytes());
    (0u32..)
        .map(|n| format!("----wwc-boundary-{n}"))
        .find(|b| !contains(json, b) && files.iter().all(|f| !contains(&f.bytes, b)))
        .unwrap_or_default()
}

/// RFC 7578 body: one `@jsonPayload` part, then one part per file.
fn multipart_body(boundary: &str, json: &[u8], file_field: &str, files: &[PhotoUpload]) -> Vec<u8> {
    // Header values must not break out of their quotes or line.
    let clean = |s: &str| s.chars().filter(|c| !matches!(c, '"' | '\\' | '\r' | '\n')).collect::<String>();
    let mut body = Vec::with_capacity(json.len() + files.iter().map(|f| f.bytes.len() + 200).sum::<usize>() + 200);
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"@jsonPayload\"\r\n").as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
    body.extend_from_slice(json);
    for f in files {
        let (name, mime) = (clean(&f.name), clean(&f.mime));
        body.extend_from_slice(
            format!(
                "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"{file_field}\"; filename=\"{name}\"\r\n\
                 Content-Type: {mime}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&f.bytes);
    }
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

pub fn bbox_filter(b: BBox) -> String {
    format!("lat >= {} && lat <= {} && lng >= {} && lng <= {}", b.south, b.north, b.west, b.east)
}

/// Search text as it goes into a filter: characters that could end the quoted string or
/// escape out of it are dropped, whitespace is collapsed, and the length is capped.
pub fn clean_search(query: &str) -> String {
    let kept: String = query.chars().filter(|c| !matches!(c, '\'' | '"' | '\\' | '`') && !c.is_control()).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(SEARCH_MAX_CHARS).collect()
}

fn search_filter(query: &str) -> String {
    format!("title ~ '{}'", clean_search(query))
}

fn and_all(clauses: impl IntoIterator<Item = String>) -> String {
    clauses.into_iter().map(|c| format!("({c})")).collect::<Vec<_>>().join(" && ")
}

/// Percent-encode a URL query component (RFC 3986 unreserved characters kept).
pub fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_ids_are_validated() {
        assert!(is_record_id("trtz3f8rta9yrh7"));
        assert!(!is_record_id(""));
        assert!(!is_record_id("abc' || 1=1"));
        assert!(!is_record_id("../users"));
    }

    #[test]
    fn bbox_filter_uses_all_edges() {
        let f = bbox_filter(BBox { south: 43.5, west: -1.0, north: 50.0, east: 7.25 });
        assert_eq!(f, "lat >= 43.5 && lat <= 50 && lng >= -1 && lng <= 7.25");
    }

    #[test]
    fn default_filter_adds_nothing() {
        let f = CampsiteFilter::default();
        assert!(f.clauses().is_empty());
        assert!(!f.is_active());
    }

    #[test]
    fn filter_clauses_and_tags_and_tent_range() {
        let f = CampsiteFilter {
            tags: ["river".to_owned(), "abc' || 1=1".to_owned(), "flat".to_owned()].into(),
            min_tents: 2,
            max_tents: 10,
        };
        assert_eq!(f.clauses(), ["tags ~ 'flat'", "tags ~ 'river'", "tent_capacity >= 2"]);
        assert_eq!(f.active_count(), 4, "the invalid id still counts: it came from the tag list");
        let f = CampsiteFilter { max_tents: 4, ..Default::default() };
        assert_eq!(f.clauses(), ["tent_capacity <= 4"]);
        assert_eq!(f.active_count(), 1);
        assert_eq!(
            and_all(["a = 1".to_owned(), "b = 2".to_owned()]),
            "(a = 1) && (b = 2)",
            "clauses are parenthesized so an || can't leak"
        );
    }

    #[test]
    fn search_text_cannot_escape_the_string() {
        assert_eq!(search_filter("Lake"), "title ~ 'Lake'");
        assert_eq!(search_filter("  o'Brien's \\ \"camp\"\n  "), "title ~ 'oBriens camp'");
        assert_eq!(search_filter("x' || hidden = true || title ~ '"), "title ~ 'x || hidden = true || title ~'");
        assert_eq!(clean_search(&"a".repeat(300)).len(), SEARCH_MAX_CHARS);
        assert_eq!(clean_search("   "), "");
    }

    fn photo(name: &str, bytes: &[u8]) -> PhotoUpload {
        PhotoUpload { name: name.into(), mime: "image/png".into(), bytes: bytes.into() }
    }

    #[test]
    fn multipart_body_has_json_payload_and_files() {
        let json = br#"{"title":"x"}"#;
        let files = [photo("a.png", b"AAA"), photo("b\"\r\n.png", b"BBB")];
        let boundary = multipart_boundary(json, &files);
        let body = String::from_utf8(multipart_body(&boundary, json, "photos+", &files)).expect("ascii body");
        let expected = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"@jsonPayload\"\r\nContent-Type: application/json\r\n\r\n\
             {{\"title\":\"x\"}}\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"photos+\"; filename=\"a.png\"\r\nContent-Type: image/png\r\n\r\nAAA\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"photos+\"; filename=\"b.png\"\r\nContent-Type: image/png\r\n\r\nBBB\r\n\
             --{b}--\r\n",
            b = boundary
        );
        assert_eq!(body, expected);
    }

    #[test]
    fn multipart_boundary_avoids_file_content() {
        let files = [photo("a.png", b"xx----wwc-boundary-0yy")];
        assert_eq!(multipart_boundary(b"{}", &files), "----wwc-boundary-1");
    }

    #[test]
    fn photo_urls_are_validated() {
        let c: Campsite =
            serde_json::from_str(include_str!("../../tests/fixtures/campsite.json")).expect("fixture parses");
        assert_eq!(
            photo_url("http://localhost:8080", &c, "lake_k2j4h1s9dq.jpg", PhotoSize::Thumb).as_deref(),
            Some("http://localhost:8080/api/files/pbc_1626116840/trtz3f8rta9yrh7/lake_k2j4h1s9dq.jpg?thumb=320x240")
        );
        assert!(photo_url("", &c, "a.jpg", PhotoSize::Large).is_some_and(|u| u.ends_with("?thumb=1200x1200f")));
        assert_eq!(photo_url("", &c, "../secret", PhotoSize::Thumb), None);
        assert_eq!(photo_url("", &c, "a.jpg?x=1", PhotoSize::Thumb), None);
        assert_eq!(photo_url("", &c, "", PhotoSize::Thumb), None);
        let bad_collection = Campsite { collection_id: "../users".into(), ..c };
        assert_eq!(photo_url("", &bad_collection, "a.jpg", PhotoSize::Thumb), None);
    }

    #[test]
    fn query_components_are_encoded() {
        assert_eq!(encode_component("campsite = 'abc'"), "campsite%20%3D%20%27abc%27");
        assert_eq!(encode_component("a&b"), "a%26b");
        let c = ApiClient::new("");
        assert_eq!(c.url("/x", &[("a", "1 2"), ("b", "ok")]), "/x?a=1%202&b=ok");
    }
}
