//! DTOs mirroring PocketBase JSON. Field names match the collections in
//! backend/pb_migrations (see docs/API.md).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// PocketBase list endpoint envelope.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub page: u32,
    pub total_items: i64,
    pub total_pages: i64,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Only present for the logged-in user's own record.
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub role: String,
}

impl User {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() { "Anonymous camper" } else { &self.name }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub record: User,
}

impl From<AuthResponse> for Session {
    fn from(a: AuthResponse) -> Self {
        Self { token: a.token, user: a.record }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Tag {
    pub id: String,
    pub slug: String,
    pub label: String,
    #[serde(default)]
    pub icon: String,
}

/// The few fields the map needs to draw a marker.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CampsiteMarker {
    pub id: String,
    pub title: String,
    pub lat: f64,
    pub lng: f64,
    pub author: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Campsite {
    pub id: String,
    /// Needed to build file URLs (`/api/files/{collectionId}/{id}/{file}`).
    #[serde(rename = "collectionId", default)]
    pub collection_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub lat: f64,
    pub lng: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub tent_capacity: u8,
    /// Photo filenames, at most 3. See `api::photo_url`.
    #[serde(default)]
    pub photos: Vec<String>,
    #[serde(default)]
    pub hidden: bool,
    pub author: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub expand: CampsiteExpand,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct CampsiteExpand {
    pub author: Option<User>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

/// Body for creating / updating a campsite.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CampsiteInput {
    pub title: String,
    pub description: String,
    pub lat: f64,
    pub lng: f64,
    pub tags: Vec<String>,
    pub tent_capacity: u8,
    /// Set on create only: the update rule rejects any change to `author`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Photo filenames to delete (PocketBase `field-` modifier).
    #[serde(rename = "photos-", skip_serializing_if = "Vec::is_empty")]
    pub remove_photos: Vec<String>,
}

/// A new photo to upload with a campsite save (multipart `photos+` part).
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoUpload {
    pub name: String,
    pub mime: String,
    pub bytes: ImageBytes,
}

/// Image file content. Shared, so cloning a form or an event never copies the image,
/// and `Debug` prints the size instead of megabytes of numbers.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ImageBytes(pub Arc<[u8]>);

impl std::ops::Deref for ImageBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8]> for ImageBytes {
    fn from(b: &[u8]) -> Self {
        Self(b.into())
    }
}

impl From<Vec<u8>> for ImageBytes {
    fn from(b: Vec<u8>) -> Self {
        Self(b.into())
    }
}

impl std::fmt::Debug for ImageBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} bytes>", self.0.len())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct CampsiteStats {
    pub avg_score: f64,
    pub rating_count: u32,
    pub comment_count: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rating {
    pub id: String,
    pub campsite: String,
    pub author: String,
    pub score: u8,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Comment {
    pub id: String,
    pub campsite: String,
    pub author: String,
    pub body: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub expand: CommentExpand,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct CommentExpand {
    pub author: Option<User>,
}

impl Comment {
    pub fn author_name(&self) -> &str {
        self.expand.author.as_ref().map_or("Unknown", User::display_name)
    }
}

/// What a report is about. Keep in sync with the `reports` collection (backend/pb_migrations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportTarget {
    Campsite(String),
    Comment(String),
}

impl ReportTarget {
    pub fn id(&self) -> &str {
        match self {
            ReportTarget::Campsite(id) | ReportTarget::Comment(id) => id,
        }
    }
}

/// Values of the `reports.reason` select field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    Spam,
    Inappropriate,
    WrongLocation,
    Dangerous,
    Other,
}

impl ReportReason {
    pub fn label(self) -> &'static str {
        match self {
            ReportReason::Spam => "Spam or advertising",
            ReportReason::Inappropriate => "Inappropriate or offensive",
            ReportReason::WrongLocation => "Wrong location",
            ReportReason::Dangerous => "Dangerous or illegal",
            ReportReason::Other => "Something else",
        }
    }

    /// The reasons offered for a target: only a campsite has a location.
    pub fn for_target(target: &ReportTarget) -> &'static [ReportReason] {
        use ReportReason::*;
        match target {
            ReportTarget::Campsite(_) => &[Spam, Inappropriate, WrongLocation, Dangerous, Other],
            ReportTarget::Comment(_) => &[Spam, Inappropriate, Dangerous, Other],
        }
    }
}

/// Body of a report create request. Exactly one of `campsite` / `comment` is set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportInput {
    pub reporter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campsite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub reason: ReportReason,
    pub details: String,
}

impl ReportInput {
    pub fn new(reporter: String, target: &ReportTarget, reason: ReportReason, details: String) -> Self {
        let (campsite, comment) = match target {
            ReportTarget::Campsite(id) => (Some(id.clone()), None),
            ReportTarget::Comment(id) => (None, Some(id.clone())),
        };
        Self { reporter, campsite, comment, reason, details }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Report {
    pub id: String,
}

/// `2026-09-25 20:07:56.273Z` → `2026-09-25`.
pub fn date_part(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAMPSITE: &str = include_str!("../../tests/fixtures/campsite.json");
    const COMMENTS: &str = include_str!("../../tests/fixtures/comments.json");
    const AUTH: &str = include_str!("../../tests/fixtures/auth.json");

    #[test]
    fn campsite_with_expand() {
        let c: Campsite = serde_json::from_str(CAMPSITE).expect("fixture parses");
        assert_eq!(c.tent_capacity, 1);
        assert_eq!(c.expand.author.as_ref().map(User::display_name), Some("Bob"));
        assert_eq!(c.expand.tags[0].slug, "safe_water");
        assert_eq!(c.collection_id, "pbc_1626116840");
        assert_eq!(c.photos, ["lake_k2j4h1s9dq.jpg", "tent_8fh2k1l0aa.png"]);
    }

    #[test]
    fn comment_list() {
        let l: ListResponse<Comment> = serde_json::from_str(COMMENTS).expect("fixture parses");
        assert_eq!(l.total_items, 2);
        assert_eq!(l.items[0].author_name(), "Carol");
    }

    #[test]
    fn auth_response_to_session() {
        let a: AuthResponse = serde_json::from_str(AUTH).expect("fixture parses");
        let s = Session::from(a);
        assert!(s.user.is_admin());
        assert_eq!(s.user.email, "alice@example.com");
    }

    #[test]
    fn campsite_input_omits_author_on_update() {
        let input = CampsiteInput {
            title: "t".into(),
            description: String::new(),
            lat: 1.0,
            lng: 2.0,
            tags: vec![],
            tent_capacity: 2,
            author: None,
            remove_photos: vec![],
        };
        let json = serde_json::to_string(&input).expect("serializes");
        assert!(!json.contains("author"));
        assert!(!json.contains("photos"));
    }

    #[test]
    fn campsite_input_sends_removed_photos() {
        let input = CampsiteInput {
            title: "t".into(),
            description: String::new(),
            lat: 1.0,
            lng: 2.0,
            tags: vec![],
            tent_capacity: 2,
            author: None,
            remove_photos: vec!["a.jpg".into()],
        };
        let json = serde_json::to_value(&input).expect("serializes");
        assert_eq!(json["photos-"], serde_json::json!(["a.jpg"]));
    }

    #[test]
    fn report_input_sets_one_target() {
        let target = ReportTarget::Comment("k1".into());
        let input = ReportInput::new("u1".into(), &target, ReportReason::Spam, String::new());
        let json = serde_json::to_value(&input).expect("serializes");
        assert_eq!(json, serde_json::json!({ "reporter": "u1", "comment": "k1", "reason": "spam", "details": "" }));
        let target = ReportTarget::Campsite("c1".into());
        let input = ReportInput::new("u1".into(), &target, ReportReason::WrongLocation, "x".into());
        let json = serde_json::to_value(&input).expect("serializes");
        assert_eq!(json["campsite"], "c1");
        assert_eq!(json["reason"], "wrong_location");
        assert!(json.get("comment").is_none());
    }

    #[test]
    fn comments_have_no_wrong_location() {
        let comment = ReportReason::for_target(&ReportTarget::Comment("k".into()));
        assert!(!comment.contains(&ReportReason::WrongLocation));
        let campsite = ReportReason::for_target(&ReportTarget::Campsite("c".into()));
        assert!(campsite.contains(&ReportReason::WrongLocation));
    }

    #[test]
    fn date_part_is_safe_on_short_input() {
        assert_eq!(date_part("2026-09-25 20:07:56.273Z"), "2026-09-25");
        assert_eq!(date_part("x"), "x");
    }
}
