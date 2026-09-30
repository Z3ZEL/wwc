//! `Action`: what the user asked for (emitted by UI code).
//! `Event`: what came back from the network (sent by API callbacks).
//! See ARCHITECTURE §5.2.

use crate::api::models::{
    AuthResponse, Campsite, CampsiteMarker, CampsiteStats, Comment, ListResponse, Rating, Report, Tag, User,
};
use crate::api::{ApiError, BBox};
use crate::state::{Panel, PhotoRef, PickedPhoto};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    OpenPanel(Panel),
    /// Close button: asks for confirmation when a form has unsaved changes.
    ClosePanel,
    /// Answer to "discard unsaved changes?": continue to the requested panel (or close).
    DiscardChanges,
    KeepEditing,
    ToggleCollapse,

    RefreshCount,
    LoadTags,
    ViewportChanged(BBox),
    RefreshMarkers,
    /// A click on the map background (sets the draft pin while the campsite form is open).
    MapClicked {
        lat: f64,
        lng: f64,
    },
    /// Center the map on a campsite and open it (e.g. a search result).
    FocusCampsite {
        id: String,
        lat: f64,
        lng: f64,
    },

    /// Run the query typed in the top bar search field.
    Search,
    ClearSearch,
    /// Filters: each change reloads the markers and the search results.
    ToggleTagFilter(String),
    SetTentRange {
        min: u8,
        max: u8,
    },
    ResetFilters,

    RefreshSession,
    Login,
    Register,
    /// Send (again) the email confirmation link.
    RequestVerification {
        email: String,
    },
    Logout,
    SaveProfileName,
    ChangePassword,

    SaveCampsite,
    /// Open the browser's file dialog to add photos to the campsite form.
    PickPhotos,
    RemovePhoto(PhotoRef),
    DeleteCampsite(String),
    RefreshStats(String),
    Rate(u8),
    PostComment,
    DeleteComment(String),
    LoadMoreComments,
    /// Send the Report panel's form.
    SubmitReport,

    /// Fetch a document again (Retry). Opening a Document panel fetches it the first time.
    LoadDocument(String),
    /// "Got it" on the privacy notice.
    DismissNotice,
    /// "Privacy choices" in the map footer: show the consent panel again.
    OpenConsent,
    /// Close the reopened consent panel without changing anything.
    CloseConsent,
    SaveConsent(ConsentChoice),

    Toast(String, ToastKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentChoice {
    AcceptAll,
    RejectAll,
    /// The purposes ticked in the consent panel.
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum Event {
    CampsiteCount(ApiResult<i64>),
    Tags(ApiResult<Vec<Tag>>),
    /// `generation` lets the reducer drop responses to superseded viewport requests.
    Markers {
        generation: u64,
        result: ApiResult<Vec<CampsiteMarker>>,
    },
    SearchResults {
        generation: u64,
        result: ApiResult<ListResponse<CampsiteMarker>>,
    },

    Campsite {
        id: String,
        result: ApiResult<Campsite>,
    },
    Stats {
        id: String,
        result: ApiResult<Option<CampsiteStats>>,
    },
    MyRating {
        id: String,
        result: ApiResult<Option<Rating>>,
    },
    Comments {
        id: String,
        page: u32,
        result: ApiResult<ListResponse<Comment>>,
    },

    LoggedIn(ApiResult<AuthResponse>),
    SessionRefreshed(ApiResult<AuthResponse>),
    Registered {
        email: String,
        password: String,
        result: ApiResult<User>,
    },
    VerificationRequested(ApiResult<()>),
    ProfileNameSaved(ApiResult<User>),
    PasswordChanged {
        email: String,
        password: String,
        result: ApiResult<User>,
    },

    /// Files chosen in the photo picker (not a network result, but it arrives the same way).
    PhotosPicked(Vec<PickedPhoto>),
    CampsiteSaved {
        created: bool,
        result: ApiResult<Campsite>,
    },
    CampsiteDeleted {
        id: String,
        result: ApiResult<()>,
    },
    RatingSaved {
        id: String,
        result: ApiResult<Rating>,
    },
    CommentPosted {
        id: String,
        result: ApiResult<Comment>,
    },
    CommentDeleted {
        id: String,
        comment_id: String,
        result: ApiResult<()>,
    },
    ReportSent(ApiResult<Report>),

    /// The Markdown text of a document.
    Document {
        id: String,
        result: ApiResult<String>,
    },
}
