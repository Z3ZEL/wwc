//! Form drafts. UI code edits these input buffers directly; submitting goes through actions.

use std::collections::{BTreeMap, BTreeSet};

use crate::api::ApiError;
use crate::api::models::{Campsite, CampsiteInput, ImageBytes, PhotoUpload, ReportInput, ReportReason, ReportTarget};

/// `tent_capacity` 1–10, where 10 means "10+".
pub const TENT_CAPACITY_MAX: u8 = 10;

/// Photo limits. Mirror the `photos` field in backend/pb_migrations/1790452800_campsite_photos.js.
pub const MAX_PHOTOS: usize = 3;
pub const MAX_PHOTO_BYTES: u64 = 10 * 1024 * 1024;
pub const PHOTO_MIME_TYPES: [&str; 3] = ["image/jpeg", "image/png", "image/webp"];

/// Largest file the picker reads. Picked photos are shrunk before upload (ADR 0017), so an
/// original may exceed `MAX_PHOTO_BYTES`; this only keeps huge files out of wasm memory.
pub const MAX_SOURCE_BYTES: u64 = 40 * 1024 * 1024;

/// A file chosen in the browser's file picker, ready to upload: re-encoded without its
/// metadata (`media`, `web/photo.rs`). `name` and `mime` are the processed file's. `bytes` is
/// empty when the file was refused (`problem`, or see `source_problem`).
#[derive(Debug, Clone, PartialEq)]
pub struct PickedPhoto {
    pub name: String,
    pub mime: String,
    /// Size of the file as picked, before processing.
    pub original_size: u64,
    pub bytes: ImageBytes,
    /// Small image made by the browser for the form preview (`None` if that failed).
    pub preview: Option<ImageBytes>,
    /// Why the picker couldn't prepare the file (unreadable, metadata couldn't be removed…).
    pub problem: Option<String>,
}

/// A photo that will be uploaded on save.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPhoto {
    /// Unique for the app's lifetime: it keys the preview in egui's image cache.
    pub id: u64,
    pub upload: PhotoUpload,
    /// Size of the file as picked (`upload` is usually much smaller).
    pub original_size: u64,
    pub preview: Option<ImageBytes>,
}

/// A photo in the campsite form: already on the server (filename) or picked (id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoRef {
    Existing(String),
    New(u64),
}

/// Why a picked file can't even be read, if it can't (checked before processing it).
pub fn source_problem(name: &str, mime: &str, size: u64) -> Option<String> {
    if !PHOTO_MIME_TYPES.contains(&mime) {
        Some(format!("{name}: only JPEG, PNG and WebP images are allowed."))
    } else if size > MAX_SOURCE_BYTES {
        Some(format!("{name}: photos are limited to {} MB.", MAX_SOURCE_BYTES / (1024 * 1024)))
    } else {
        None
    }
}

/// Why a processed file can't be uploaded, if it can't.
pub fn photo_problem(name: &str, mime: &str, size: u64) -> Option<String> {
    if !PHOTO_MIME_TYPES.contains(&mime) {
        Some(format!("{name}: only JPEG, PNG and WebP images are allowed."))
    } else if size > MAX_PHOTO_BYTES {
        Some(format!("{name}: this photo is still over {} MB once compressed.", MAX_PHOTO_BYTES / (1024 * 1024)))
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub error: Option<ApiError>,
    pub submitting: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RegisterForm {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
    /// "I am old enough and I accept the Terms of Use" (checked in the browser only: the
    /// backend doesn't record it yet).
    pub accepted_terms: bool,
    pub error: Option<ApiError>,
    pub submitting: bool,
}

impl RegisterForm {
    /// What can be checked before asking the server.
    pub fn validate(&self) -> BTreeMap<String, String> {
        let mut errors = BTreeMap::new();
        if self.password != self.confirm {
            errors.insert("passwordConfirm".into(), "Passwords don't match.".into());
        }
        if !self.accepted_terms {
            errors.insert("terms".into(), "Please confirm your age and accept the Terms of Use.".into());
        }
        errors
    }
}

/// The consent panel (ARCHITECTURE §5.12). The saved choice is `AppState::consent`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConsentPanel {
    /// Opened again from the footer ("Privacy choices") after a choice was made.
    pub reopened: bool,
    /// Shows one checkbox per purpose.
    pub customizing: bool,
    /// Ticked purposes. Nothing is ticked until the visitor ticks it.
    pub choices: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileForm {
    pub name: String,
    pub name_error: Option<ApiError>,
    pub saving_name: bool,
    pub old_password: String,
    pub new_password: String,
    pub confirm: String,
    pub password_error: Option<ApiError>,
    pub changing_password: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CampsiteForm {
    /// `Some(id)` when editing an existing campsite.
    pub editing: Option<String>,
    pub title: String,
    pub description: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// Selected tag ids.
    pub tags: BTreeSet<String>,
    pub tent_capacity: u8,
    /// Filenames of the photos already on the server that are kept.
    pub existing_photos: Vec<String>,
    /// Filenames to delete on save.
    pub removed_photos: Vec<String>,
    pub new_photos: Vec<NewPhoto>,
    /// Server error, or local validation errors keyed by field.
    pub error: Option<ApiError>,
    pub submitting: bool,
    pub dirty: bool,
}

impl Default for CampsiteForm {
    fn default() -> Self {
        Self {
            editing: None,
            title: String::new(),
            description: String::new(),
            lat: None,
            lng: None,
            tags: BTreeSet::new(),
            tent_capacity: 2,
            existing_photos: Vec::new(),
            removed_photos: Vec::new(),
            new_photos: Vec::new(),
            error: None,
            submitting: false,
            dirty: false,
        }
    }
}

impl CampsiteForm {
    pub fn from_campsite(c: &Campsite) -> Self {
        Self {
            editing: Some(c.id.clone()),
            title: c.title.clone(),
            description: c.description.clone(),
            lat: Some(c.lat),
            lng: Some(c.lng),
            tags: c.tags.iter().cloned().collect(),
            tent_capacity: c.tent_capacity.clamp(1, TENT_CAPACITY_MAX),
            existing_photos: c.photos.clone(),
            ..Default::default()
        }
    }

    pub fn photo_count(&self) -> usize {
        self.existing_photos.len() + self.new_photos.len()
    }

    pub fn free_photo_slots(&self) -> usize {
        MAX_PHOTOS.saturating_sub(self.photo_count())
    }

    /// Adds the valid picked files while there is room. Returns a message per refused file.
    /// `next_id` is the app-wide preview id counter.
    pub fn add_picked(&mut self, picked: Vec<PickedPhoto>, next_id: &mut u64) -> Vec<String> {
        let mut problems = Vec::new();
        for p in picked {
            let size = u64::try_from(p.bytes.len()).unwrap_or(u64::MAX);
            if let Some(problem) = p.problem.or_else(|| source_problem(&p.name, &p.mime, p.original_size)) {
                problems.push(problem);
            } else if self.free_photo_slots() == 0 {
                // The picker doesn't read files beyond the free slots: they come without bytes.
                problems.push(format!("{}: a campsite can have at most {MAX_PHOTOS} photos.", p.name));
            } else if p.bytes.is_empty() {
                problems.push(format!("{}: the file could not be read.", p.name));
            } else if let Some(problem) = photo_problem(&p.name, &p.mime, size) {
                problems.push(problem);
            } else {
                *next_id += 1;
                let upload = PhotoUpload { name: p.name, mime: p.mime, bytes: p.bytes };
                let original_size = p.original_size;
                self.new_photos.push(NewPhoto { id: *next_id, upload, original_size, preview: p.preview });
                self.dirty = true;
            }
        }
        problems
    }

    pub fn remove_photo(&mut self, photo: &PhotoRef) {
        match photo {
            PhotoRef::Existing(file) => {
                if let Some(i) = self.existing_photos.iter().position(|f| f == file) {
                    self.removed_photos.push(self.existing_photos.remove(i));
                    self.dirty = true;
                }
            }
            PhotoRef::New(id) => {
                let before = self.new_photos.len();
                self.new_photos.retain(|p| p.id != *id);
                self.dirty |= self.new_photos.len() != before;
            }
        }
    }

    /// Total size of the new photos as picked, and as they will be uploaded.
    pub fn new_photo_sizes(&self) -> (u64, u64) {
        self.new_photos.iter().fold((0, 0), |(picked, upload), p| {
            (picked + p.original_size, upload + u64::try_from(p.upload.bytes.len()).unwrap_or(u64::MAX))
        })
    }

    /// The files to send with the save request.
    pub fn uploads(&self) -> Vec<PhotoUpload> {
        self.new_photos.iter().map(|p| p.upload.clone()).collect()
    }

    /// Mirrors the server-side constraints (backend/pb_migrations) for fast feedback.
    pub fn validate(&self) -> BTreeMap<String, String> {
        let mut errors = BTreeMap::new();
        let title_len = self.title.trim().chars().count();
        if !(3..=100).contains(&title_len) {
            errors.insert("title".into(), "Title must be between 3 and 100 characters.".into());
        }
        if self.description.chars().count() > 5000 {
            errors.insert("description".into(), "Description is limited to 5000 characters.".into());
        }
        if self.lat.is_none() || self.lng.is_none() {
            errors.insert("location".into(), "Click on the map to place the campsite.".into());
        }
        if !(1..=TENT_CAPACITY_MAX).contains(&self.tent_capacity) {
            errors.insert("tent_capacity".into(), "Tent capacity must be between 1 and 10+.".into());
        }
        if self.photo_count() > MAX_PHOTOS {
            errors.insert("photos".into(), format!("A campsite can have at most {MAX_PHOTOS} photos."));
        }
        let size = |p: &NewPhoto| u64::try_from(p.upload.bytes.len()).unwrap_or(u64::MAX);
        if let Some(problem) =
            self.new_photos.iter().find_map(|p| photo_problem(&p.upload.name, &p.upload.mime, size(p)))
        {
            errors.insert("photos".into(), problem);
        }
        errors
    }

    /// `None` if the location is missing. `author` is set on create only.
    pub fn to_input(&self, author: Option<String>) -> Option<CampsiteInput> {
        Some(CampsiteInput {
            title: self.title.trim().to_owned(),
            description: self.description.trim().to_owned(),
            lat: self.lat?,
            lng: self.lng?,
            tags: self.tags.iter().cloned().collect(),
            tent_capacity: self.tent_capacity,
            author,
            remove_photos: self.removed_photos.clone(),
        })
    }
}

/// Mirrors `reports.details` (backend/pb_migrations/1790539200_reports.js).
pub const REPORT_DETAILS_MAX: usize = 1000;

/// The Report panel. The target itself is in `Panel::Report`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReportForm {
    /// The campsite to go back to (the reported campsite, or the one holding the comment).
    pub campsite_id: String,
    pub reason: Option<ReportReason>,
    pub details: String,
    pub error: Option<ApiError>,
    pub sending: bool,
}

impl ReportForm {
    pub fn new(campsite_id: String) -> Self {
        Self { campsite_id, ..Default::default() }
    }

    pub fn validate(&self) -> BTreeMap<String, String> {
        let mut errors = BTreeMap::new();
        if self.reason.is_none() {
            errors.insert("reason".into(), "Choose a reason.".into());
        }
        let details = self.details.trim();
        if self.reason == Some(ReportReason::Other) && details.is_empty() {
            errors.insert("details".into(), "Tell us what is wrong.".into());
        }
        if details.chars().count() > REPORT_DETAILS_MAX {
            errors.insert("details".into(), format!("Details are limited to {REPORT_DETAILS_MAX} characters."));
        }
        errors
    }

    /// `None` until a reason is chosen.
    pub fn to_input(&self, reporter: String, target: &ReportTarget) -> Option<ReportInput> {
        Some(ReportInput::new(reporter, target, self.reason?, self.details.trim().to_owned()))
    }
}

/// "10+" for the maximum capacity.
pub fn tent_capacity_label(n: u8) -> String {
    if n >= TENT_CAPACITY_MAX { format!("{TENT_CAPACITY_MAX}+") } else { n.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_form_needs_title_and_location() {
        let errors = CampsiteForm::default().validate();
        assert!(errors.contains_key("title"));
        assert!(errors.contains_key("location"));
    }

    #[test]
    fn valid_form_builds_input() {
        let form = CampsiteForm {
            title: "  Lake spot ".into(),
            lat: Some(45.0),
            lng: Some(3.0),
            tent_capacity: 10,
            ..Default::default()
        };
        assert!(form.validate().is_empty());
        let input = form.to_input(None).expect("location is set");
        assert_eq!(input.title, "Lake spot");
        assert_eq!(input.tent_capacity, 10);
    }

    fn picked(name: &str, mime: &str, original_size: u64) -> PickedPhoto {
        let bytes = vec![0; 4].into();
        PickedPhoto { name: name.into(), mime: mime.into(), original_size, bytes, preview: None, problem: None }
    }

    #[test]
    fn picking_photos_respects_type_size_and_count() {
        let mut form = CampsiteForm { existing_photos: vec!["old.jpg".into()], ..Default::default() };
        let mut next_id = 0;
        let problems = form.add_picked(
            vec![
                picked("a.gif", "image/gif", 10),
                picked("huge.jpg", "image/jpeg", MAX_SOURCE_BYTES + 1),
                // Over the upload limit as picked, but small once processed.
                picked("b.jpg", "image/jpeg", MAX_PHOTO_BYTES * 2),
                picked("c.png", "image/png", 10),
                picked("d.webp", "image/webp", 10),
            ],
            &mut next_id,
        );
        assert_eq!(problems.len(), 3, "{problems:?}"); // gif, too big, one too many
        assert!(problems[2].starts_with("d.webp"));
        assert_eq!(form.photo_count(), MAX_PHOTOS);
        assert_eq!(form.free_photo_slots(), 0);
        assert_eq!(form.new_photos.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 2]);
        assert!(form.dirty);
    }

    #[test]
    fn photos_the_picker_could_not_prepare_are_refused() {
        let mut form = CampsiteForm::default();
        let unreadable = PickedPhoto { bytes: ImageBytes::default(), ..picked("a.jpg", "image/jpeg", 10) };
        let with_problem = PickedPhoto { problem: Some("b.jpg: metadata".into()), ..picked("b.jpg", "image/jpeg", 10) };
        let still_big = PickedPhoto {
            bytes: vec![0; usize::try_from(MAX_PHOTO_BYTES + 1).expect("fits")].into(),
            ..picked("c.webp", "image/webp", 10)
        };
        let problems = form.add_picked(vec![unreadable, with_problem, still_big], &mut 0);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert_eq!(problems[1], "b.jpg: metadata");
        assert!(form.new_photos.is_empty());
        assert!(!form.dirty);
    }

    #[test]
    fn new_photo_sizes_add_up_before_and_after_processing() {
        let mut form = CampsiteForm::default();
        assert_eq!(form.new_photo_sizes(), (0, 0));
        form.add_picked(vec![picked("a.jpg", "image/jpeg", 1000), picked("b.jpg", "image/jpeg", 500)], &mut 0);
        assert_eq!(form.new_photo_sizes(), (1500, 8));
    }

    #[test]
    fn removing_an_existing_photo_frees_a_slot_and_is_sent() {
        let c: Campsite =
            serde_json::from_str(include_str!("../../tests/fixtures/campsite.json")).expect("fixture parses");
        let mut form = CampsiteForm::from_campsite(&c);
        assert_eq!(form.free_photo_slots(), 1);
        form.remove_photo(&PhotoRef::Existing("lake_k2j4h1s9dq.jpg".into()));
        assert_eq!(form.free_photo_slots(), 2);
        assert!(form.dirty);
        let mut next_id = 7;
        assert!(form.add_picked(vec![picked("n.jpg", "image/jpeg", 4)], &mut next_id).is_empty());
        form.remove_photo(&PhotoRef::New(8));
        assert!(form.new_photos.is_empty());
        let input = form.to_input(None).expect("location is set");
        assert_eq!(input.remove_photos, ["lake_k2j4h1s9dq.jpg"]);
    }

    #[test]
    fn report_needs_a_reason_and_details_for_other() {
        let mut form = ReportForm::new("c1".into());
        assert!(form.validate().contains_key("reason"));
        assert_eq!(form.to_input("u".into(), &ReportTarget::Campsite("c1".into())), None);

        form.reason = Some(ReportReason::Other);
        form.details = "   ".into();
        assert!(form.validate().contains_key("details"));

        form.details = "x".repeat(REPORT_DETAILS_MAX + 1);
        assert!(form.validate().contains_key("details"));

        form.details = " Rude words \n".into();
        assert!(form.validate().is_empty());
        let input = form.to_input("u".into(), &ReportTarget::Comment("k1".into())).expect("reason is set");
        assert_eq!(input.details, "Rude words");
        assert_eq!(input.comment.as_deref(), Some("k1"));
    }

    #[test]
    fn register_needs_matching_passwords_and_accepted_terms() {
        let mut form = RegisterForm { password: "a".into(), confirm: "b".into(), ..Default::default() };
        let errors = form.validate();
        assert!(errors.contains_key("passwordConfirm") && errors.contains_key("terms"));
        form.confirm = "a".into();
        form.accepted_terms = true;
        assert!(form.validate().is_empty());
    }

    #[test]
    fn capacity_label() {
        assert_eq!(tent_capacity_label(3), "3");
        assert_eq!(tent_capacity_label(10), "10+");
    }
}
