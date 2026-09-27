//! PocketBase client, DTOs and errors. The only place that talks HTTP.

mod client;
mod error;
pub mod models;

pub use client::{
    ApiClient, BBox, COMMENTS_PER_PAGE, CampsiteFilter, Done, MARKERS_PER_REQUEST, PhotoSize, SEARCH_MAX_CHARS,
    SEARCH_RESULTS_PER_PAGE, clean_search, is_record_id, photo_url,
};
pub use error::ApiError;
