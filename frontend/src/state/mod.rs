//! Application state. Mutated only by `controller::Controller::handle` (actions) and
//! `reduce::apply` (network events). UI code reads it, and may edit form input buffers.

mod forms;
mod reduce;

pub use forms::{
    CampsiteForm, ConsentPanel, LoginForm, MAX_PHOTO_BYTES, MAX_PHOTOS, MAX_SOURCE_BYTES, NewPhoto, PHOTO_MIME_TYPES,
    PhotoRef, PickedPhoto, ProfileForm, REPORT_DETAILS_MAX, RegisterForm, ReportForm, TENT_CAPACITY_MAX, photo_problem,
    source_problem, tent_capacity_label,
};
pub use reduce::apply;

use std::collections::BTreeMap;

use crate::actions::ToastKind;
use crate::api::models::{Campsite, CampsiteMarker, CampsiteStats, Comment, Rating, ReportTarget, Session, Tag};
use crate::api::{ApiError, BBox, CampsiteFilter};
use crate::consent::{self, ConsentRecord};
use crate::documents::Block;

/// The side panel shown over the map (ARCHITECTURE §5.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    Login,
    Register,
    Profile,
    NewCampsite,
    Campsite(String),
    EditCampsite(String),
    /// Results of the top bar search.
    Search,
    /// Global map filters (tags, tent capacity).
    Filters,
    /// Report form for a campsite or a comment of the open campsite.
    Report(ReportTarget),
    /// A Markdown document from `assets/documents/` (legal pages…), by manifest id.
    Document(String),
}

impl Panel {
    pub fn requires_auth(&self) -> bool {
        matches!(self, Panel::Profile | Panel::NewCampsite | Panel::EditCampsite(_) | Panel::Report(_))
    }

    pub fn is_campsite_form(&self) -> bool {
        matches!(self, Panel::NewCampsite | Panel::EditCampsite(_))
    }

    pub fn title(&self) -> &'static str {
        match self {
            Panel::Login => "Log in",
            Panel::Register => "Create an account",
            Panel::Profile => "Profile",
            Panel::NewCampsite => "New campsite",
            Panel::Campsite(_) => "Campsite",
            Panel::EditCampsite(_) => "Edit campsite",
            Panel::Search => "Search results",
            Panel::Filters => "Filters",
            Panel::Report(_) => "Report",
            Panel::Document(id) => crate::documents::manifest().get(id).map_or("Document", |d| d.title.as_str()),
        }
    }
}

/// Remote data, with every state a screen must render.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Remote<T> {
    #[default]
    NotAsked,
    Loading,
    Loaded(T),
    Failed(ApiError),
}

impl<T> Remote<T> {
    pub fn loaded(&self) -> Option<&T> {
        match self {
            Remote::Loaded(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Remote::Loading)
    }
}

impl<T> From<Result<T, ApiError>> for Remote<T> {
    fn from(r: Result<T, ApiError>) -> Self {
        match r {
            Ok(v) => Remote::Loaded(v),
            Err(e) => Remote::Failed(e),
        }
    }
}

/// The top bar search: the input buffer and the results of the last submitted query.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Search {
    /// Text field buffer (edited by the top bar).
    pub query: String,
    /// The query the results are for. Empty = no search.
    pub submitted: String,
    pub results: Remote<Vec<CampsiteMarker>>,
    /// Matches on the server; can be more than `results.len()`.
    pub total: i64,
    /// Guards against stale responses, like `markers_generation`.
    pub generation: u64,
}

impl Search {
    pub fn is_active(&self) -> bool {
        !self.submitted.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    /// Set by the app the first time the toast is drawn (egui time, seconds).
    pub shown_at: Option<f64>,
}

/// Everything the Campsite panel shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CampsiteDetail {
    pub id: String,
    pub campsite: Remote<Campsite>,
    pub stats: Remote<Option<CampsiteStats>>,
    pub my_rating: Remote<Option<Rating>>,
    pub rating_saving: bool,
    pub comments: Vec<Comment>,
    pub comments_page: u32,
    pub comments_total_pages: i64,
    pub comments_loading: bool,
    pub comments_error: Option<ApiError>,
    pub comment_draft: String,
    pub comment_error: Option<ApiError>,
    pub posting_comment: bool,
    pub confirm_delete: bool,
    pub deleting: bool,
    /// Index of the photo shown in the full-page viewer, if it is open.
    pub photo_open: Option<usize>,
}

impl CampsiteDetail {
    pub fn new(id: String) -> Self {
        Self { id, ..Default::default() }
    }

    pub fn has_more_comments(&self) -> bool {
        i64::from(self.comments_page) < self.comments_total_pages
    }

    /// Carousel navigation in the photo viewer: moves by `delta` and wraps around.
    pub fn step_photo(&mut self, delta: isize, count: usize) {
        let (Some(i), Ok(n)) = (self.photo_open, isize::try_from(count)) else { return };
        if n == 0 {
            self.photo_open = None;
            return;
        }
        let i = isize::try_from(i).unwrap_or(0);
        self.photo_open = usize::try_from((i + delta).rem_euclid(n)).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_carousel_wraps_around() {
        let mut d = CampsiteDetail { photo_open: Some(0), ..Default::default() };
        d.step_photo(-1, 3);
        assert_eq!(d.photo_open, Some(2));
        d.step_photo(1, 3);
        assert_eq!(d.photo_open, Some(0));
        d.step_photo(1, 1);
        assert_eq!(d.photo_open, Some(0));
        d.step_photo(1, 0);
        assert_eq!(d.photo_open, None);
        d.step_photo(1, 3);
        assert_eq!(d.photo_open, None, "a closed viewer stays closed");
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    /// PocketBase base URL, e.g. `https://api.example.com` (the page origin when same-origin).
    /// Photo URLs must be absolute (see `api::photo_url`).
    pub api_base: String,
    pub session: Option<Session>,

    pub panel: Option<Panel>,
    pub panel_collapsed: bool,
    /// `Some(target)` while asking "discard unsaved changes?"; target `None` = just close.
    pub confirm_discard: Option<Option<Panel>>,
    /// Panel to open once the user has logged in.
    pub after_login: Option<Panel>,
    /// Where a Document panel was opened from (e.g. the sign-up form), for its back link.
    pub document_back: Option<Panel>,

    pub campsite_count: Remote<i64>,
    pub tags: Remote<Vec<Tag>>,

    pub viewport: Option<BBox>,
    /// Map center (lat, lng), kept up to date by the map for "Use map center".
    pub map_center: (f64, f64),
    pub markers: Vec<CampsiteMarker>,
    pub markers_generation: u64,
    pub markers_loading: bool,
    /// Global filters for markers and search (ARCHITECTURE §5.5).
    pub filters: CampsiteFilter,
    pub search: Search,
    /// A position (lat, lng) the map should center on; taken by the map on its next frame.
    pub map_focus: Option<(f64, f64)>,

    pub detail: Option<CampsiteDetail>,

    /// Fetched documents by id, parsed. Kept for the whole session.
    pub documents: BTreeMap<String, Remote<Vec<Block>>>,
    /// The Privacy Policy version (`updated` date) the privacy notice was dismissed for.
    /// Saved in the browser; the notice shows again when the policy changes.
    pub notice_seen: Option<String>,
    /// The visitor's consent choice (saved in the browser), `None` until they choose. Only
    /// used while `consent.json` enables consent (§5.12).
    pub consent: Option<ConsentRecord>,
    pub consent_panel: ConsentPanel,

    pub login: LoginForm,
    pub register: RegisterForm,
    pub profile: ProfileForm,
    pub campsite_form: CampsiteForm,
    pub report: ReportForm,
    /// Last id given to a picked photo (never reset: ids key egui's image cache).
    pub photo_seq: u64,

    pub toasts: Vec<Toast>,
}

impl AppState {
    pub fn user_id(&self) -> Option<&str> {
        self.session.as_ref().map(|s| s.user.id.as_str())
    }

    /// Logged in with an unconfirmed email: reading works, creating content doesn't.
    pub fn needs_verification(&self) -> bool {
        self.session.as_ref().is_some_and(|s| !s.user.verified)
    }

    /// The campsite currently shown or edited, if any.
    pub fn selected_campsite(&self) -> Option<&str> {
        match &self.panel {
            Some(Panel::Campsite(id) | Panel::EditCampsite(id)) => Some(id),
            Some(Panel::Report(_)) if !self.report.campsite_id.is_empty() => Some(&self.report.campsite_id),
            _ => None,
        }
    }

    /// Whether an optional purpose (e.g. `"analytics"`) may run: the visitor accepted it.
    /// Always false while consent is disabled in `consent.json`.
    pub fn consent_allows(&self, purpose: &str) -> bool {
        consent::config().allows(self.consent.as_ref(), purpose)
    }

    /// The consent panel is on screen: no choice yet, or reopened from the footer.
    pub fn consent_panel_open(&self) -> bool {
        consent::config().enabled && (self.consent.is_none() || self.consent_panel.reopened)
    }

    pub fn is_campsite_form_open(&self) -> bool {
        self.panel.as_ref().is_some_and(Panel::is_campsite_form)
    }

    pub fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), kind, shown_at: None });
    }
}
