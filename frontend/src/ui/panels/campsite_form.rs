//! New / Edit campsite form. While it is open, clicks on the map move the draft pin.

use std::borrow::Cow;

use egui::{CursorIcon, ImageSource, Slider, TextEdit, Ui};

use crate::actions::Action;
use crate::api::models::Campsite;
use crate::api::{PhotoSize, photo_url};
use crate::state::{
    AppState, CampsiteForm, MAX_PHOTO_BYTES, MAX_PHOTOS, Panel, PhotoRef, Remote, TENT_CAPACITY_MAX,
    tent_capacity_label,
};
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    StripStyle, Thumb, field_error, form_error, loading, muted, photo_strip, primary_button, section_title, tag_chip,
};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let (center_lat, center_lng) = state.map_center;
    // When editing, the campsite is the one loaded in the detail (it builds the photo URLs).
    let editing = state.detail.as_ref().and_then(|d| d.campsite.loaded());
    let f = &mut state.campsite_form;
    let err = f.error.as_ref();
    let mut changed = false;

    ui.label("Title");
    changed |= ui.add(TextEdit::singleline(&mut f.title).desired_width(f32::INFINITY)).changed();
    field_error(ui, theme, err, "title");

    ui.label("Description");
    changed |= ui.add(TextEdit::multiline(&mut f.description).desired_rows(5).desired_width(f32::INFINITY)).changed();
    field_error(ui, theme, err, "description");

    ui.add_space(theme.spacing.section_gap);
    section_title(ui, "Location");
    match (f.lat, f.lng) {
        (Some(lat), Some(lng)) => {
            ui.monospace(format!("{lat:.5}, {lng:.5}"));
            muted(ui, theme, "Click on the map to move the pin.");
        }
        _ => {
            muted(ui, theme, "Click on the map to place the campsite.");
        }
    }
    if ui.button("Use map center").clicked() {
        actions.push(Action::MapClicked { lat: center_lat, lng: center_lng });
    }
    muted(ui, theme, "The exact location is public: anyone can see it on the map.");
    field_error(ui, theme, err, "location");
    field_error(ui, theme, err, "lat");
    field_error(ui, theme, err, "lng");

    ui.add_space(theme.spacing.section_gap);
    section_title(ui, "Tags");
    match &state.tags {
        Remote::Loaded(tags) => {
            ui.horizontal_wrapped(|ui| {
                for tag in tags {
                    let selected = f.tags.contains(&tag.id);
                    if tag_chip(ui, theme, &tag.label, selected).clicked() {
                        if selected {
                            f.tags.remove(&tag.id);
                        } else {
                            f.tags.insert(tag.id.clone());
                        }
                        changed = true;
                    }
                }
            });
        }
        Remote::Failed(e) => {
            muted(ui, theme, format!("Tags unavailable: {e}"));
        }
        Remote::Loading | Remote::NotAsked => loading(ui, theme, "Loading tags…"),
    }
    field_error(ui, theme, err, "tags");

    ui.add_space(theme.spacing.section_gap);
    section_title(ui, "Tent capacity");
    changed |= ui
        .add(
            Slider::new(&mut f.tent_capacity, 1..=TENT_CAPACITY_MAX)
                .custom_formatter(|v, _| tent_capacity_label(v as u8))
                .text("tents"),
        )
        .on_hover_cursor(CursorIcon::PointingHand)
        .changed();
    field_error(ui, theme, err, "tent_capacity");

    ui.add_space(theme.spacing.section_gap);
    photos(ui, theme, f, editing, &state.api_base, actions);

    if changed {
        f.dirty = true;
    }

    ui.add_space(theme.spacing.section_gap);
    form_error(ui, theme, f.error.as_ref());
    ui.horizontal(|ui| {
        let label = match (f.submitting, f.editing.is_some()) {
            (true, _) => "Saving…",
            (false, true) => "Save changes",
            (false, false) => "Add campsite",
        };
        if primary_button(ui, theme, label, !f.submitting).clicked() {
            actions.push(Action::SaveCampsite);
        }
        if ui.button("Cancel").clicked() {
            actions.push(match &f.editing {
                Some(id) => Action::OpenPanel(Panel::Campsite(id.clone())),
                None => Action::ClosePanel,
            });
        }
    });
}

fn photos(
    ui: &mut Ui,
    theme: &Theme,
    f: &CampsiteForm,
    campsite: Option<&Campsite>,
    api_base: &str,
    actions: &mut Vec<Action>,
) {
    section_title(ui, "Photos");
    let existing = f.existing_photos.iter().map(|file| Thumb {
        source: campsite
            .and_then(|c| photo_url(api_base, c, file, PhotoSize::Thumb))
            .map(|url| ImageSource::Uri(Cow::Owned(url))),
        label: file,
    });
    // Previews are keyed by the photo's app-wide id, so egui's cache never mixes them up.
    let new = f.new_photos.iter().map(|p| Thumb {
        source: p.preview.as_ref().map(|bytes| ImageSource::Bytes {
            uri: Cow::Owned(format!("bytes://wwc-new-photo-{}.jpg", p.id)),
            bytes: egui::load::Bytes::Shared(bytes.0.clone()),
        }),
        label: &p.upload.name,
    });
    let thumbs: Vec<Thumb<'_>> = existing.chain(new).collect();
    if !thumbs.is_empty() {
        let style = StripStyle { height: theme.layout.photo_thumb_height, removable: !f.submitting, selected: None };
        let removed = photo_strip(ui, theme, thumbs, style).removed;
        let photo = removed.and_then(|i| match f.existing_photos.get(i) {
            Some(file) => Some(PhotoRef::Existing(file.clone())),
            None => f.new_photos.get(i - f.existing_photos.len()).map(|p| PhotoRef::New(p.id)),
        });
        if let Some(photo) = photo {
            actions.push(Action::RemovePhoto(photo));
        }
    }

    let count = f.photo_count();
    let can_add = count < MAX_PHOTOS && !f.submitting;
    if ui.add_enabled(can_add, egui::Button::new(format!("Add photos ({count}/{MAX_PHOTOS})"))).clicked() {
        actions.push(Action::PickPhotos);
    }
    muted(
        ui,
        theme,
        format!("Up to {MAX_PHOTOS} photos: JPEG, PNG or WebP, {} MB each.", MAX_PHOTO_BYTES / (1024 * 1024)),
    );
    field_error(ui, theme, f.error.as_ref(), "photos");
}
