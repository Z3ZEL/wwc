//! Full-page photo viewer: a modal over the whole app (map, top bar and panel) with a
//! carousel of the open campsite's photos. It is open while `CampsiteDetail::photo_open` is set.
//! Keys: ← / → browse, Esc closes; clicking the backdrop closes too.

use std::borrow::Cow;

use egui::{Align, Button, Id, ImageSource, Key, Layout, Modal, RichText, Ui, vec2};

use crate::api::models::Campsite;
use crate::api::{PhotoSize, photo_url};
use crate::state::{AppState, CampsiteDetail, Panel};
use crate::ui::theme::Theme;
use crate::ui::widgets::{StripStyle, Thumb, photo_strip};

pub fn show(ctx: &egui::Context, state: &mut AppState, theme: &Theme) {
    // Only over the Campsite panel: the detail also lives on while editing.
    if !matches!(state.panel, Some(Panel::Campsite(_))) {
        return;
    }
    let origin = state.origin.as_str();
    let Some(d) = state.detail.as_mut() else { return };
    let Some(c) = d.campsite.loaded().cloned() else { return };
    let count = c.photos.len();
    let Some(index) = d.photo_open.filter(|&i| i < count) else {
        d.photo_open = None;
        return;
    };

    let screen = ctx.content_rect().shrink(theme.layout.photo_viewer_margin);
    let frame = egui::Frame::NONE
        .fill(theme.colors.photo_viewer_bg)
        .corner_radius(theme.radius())
        .inner_margin(theme.panel_margin());
    let modal = Modal::new(Id::new("photo_viewer"))
        .backdrop_color(theme.colors.photo_viewer_backdrop)
        .frame(frame)
        .show(ctx, |ui| {
            // The frame's inner margin is added around this size.
            let inner = screen.size() - theme.panel_margin().sum();
            ui.set_min_size(inner);
            ui.set_max_size(inner);
            carousel(ui, theme, &c, d, index, origin)
        });
    if modal.inner || modal.should_close() {
        d.photo_open = None;
    }
}

/// Returns true when the close button was clicked.
fn carousel(ui: &mut Ui, theme: &Theme, c: &Campsite, d: &mut CampsiteDetail, index: usize, origin: &str) -> bool {
    let count = c.photos.len();
    let text = theme.colors.photo_viewer_text;
    let mut close = false;

    if count > 1 {
        let step = ui.input_mut(|i| {
            i32::from(i.consume_key(egui::Modifiers::NONE, Key::ArrowRight))
                - i32::from(i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft))
        });
        if step != 0 {
            d.step_photo(step as isize, count);
        }
    }
    let index = d.photo_open.unwrap_or(index);

    // Header: title, position, close.
    ui.horizontal(|ui| {
        ui.label(RichText::new(&c.title).color(text).strong());
        ui.label(RichText::new(format!("{} / {count}", index + 1)).color(text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            close = ui.add(nav_button(theme, "×")).on_hover_text("Close (Esc)").clicked();
        });
    });

    // Thumbnails take the bottom row when there is more than one photo.
    let thumbs_height =
        if count > 1 { theme.layout.photo_viewer_thumb_height + theme.spacing.item_spacing[1] * 2.0 } else { 0.0 };
    let main_height = (ui.available_height() - thumbs_height).max(0.0);

    ui.allocate_ui_with_layout(vec2(ui.available_width(), main_height), Layout::left_to_right(Align::Center), |ui| {
        let arrow_width = if count > 1 { theme.typography.size_heading * 2.0 } else { 0.0 };
        if count > 1 && ui.add_sized(vec2(arrow_width, main_height), nav_button(theme, "◀")).clicked() {
            d.step_photo(-1, count);
        }
        let image_size = vec2((ui.available_width() - arrow_width).max(0.0), main_height);
        ui.allocate_ui_with_layout(image_size, Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
            match photo_url(origin, c, &c.photos[index], PhotoSize::Large) {
                Some(url) => {
                    ui.add(
                        egui::Image::new(ImageSource::Uri(Cow::Owned(url)))
                            .fit_to_exact_size(image_size)
                            .maintain_aspect_ratio(true)
                            .corner_radius(theme.radius()),
                    );
                }
                None => {
                    ui.label(RichText::new("Photo unavailable.").color(text));
                }
            }
        });
        if count > 1 && ui.add_sized(vec2(arrow_width, main_height), nav_button(theme, "▶")).clicked() {
            d.step_photo(1, count);
        }
    });

    if count > 1 {
        ui.add_space(theme.spacing.item_spacing[1]);
        let thumbs = c
            .photos
            .iter()
            .map(|file| Thumb {
                source: photo_url(origin, c, file, PhotoSize::Thumb).map(|u| ImageSource::Uri(Cow::Owned(u))),
                label: file,
            })
            .collect();
        let style =
            StripStyle { height: theme.layout.photo_viewer_thumb_height, removable: false, selected: Some(index) };
        ui.vertical_centered(|ui| {
            // Centre the row: the strip wraps left-aligned, so give it just its own width.
            let row_width = count as f32 * (style.height * 4.0 / 3.0 + theme.spacing.photo_gap);
            ui.set_max_width(row_width);
            if let Some(i) = photo_strip(ui, theme, thumbs, style).clicked {
                d.photo_open = Some(i);
            }
        });
    }
    close
}

/// Flat, frameless button in the viewer's text color.
fn nav_button<'a>(theme: &Theme, text: &'a str) -> Button<'a> {
    Button::new(RichText::new(text).color(theme.colors.photo_viewer_text).size(theme.typography.size_heading))
        .frame(false)
}
