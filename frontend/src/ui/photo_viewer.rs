//! Full-page photo viewer: a modal over the whole app (map, top bar and panel) with a
//! carousel of the open campsite's photos. It is open while `CampsiteDetail::photo_open` is set.
//! Keys: ← / → browse, Esc unzooms then closes; clicking the backdrop closes too.
//! The photo zooms (pinch, Ctrl/⌘ + wheel or wheel, double click / tap) and pans when zoomed
//! (`ui::photo_zoom`); an unzoomed photo swipes to the next / previous one.
//! Phones and portrait screens get a full-screen layout without the side arrow columns.

use std::borrow::Cow;

use egui::{
    Align, Button, CornerRadius, CursorIcon, Id, Image, ImageSource, Key, Layout, Margin, Modal, Rect, RichText,
    ScrollArea, Sense, Ui, UiBuilder, Vec2, vec2,
};

use crate::api::models::Campsite;
use crate::api::{PhotoSize, photo_url};
use crate::state::{AppState, CampsiteDetail, Panel};
use crate::ui::photo_zoom::{PhotoZoom, swipe_step};
use crate::ui::theme::Theme;
use crate::ui::widgets::{StripStyle, Thumb, photo_strip};

/// Viewer state that only lives while it is on screen, in egui's memory: it starts over for each
/// photo and each time the viewer opens.
#[derive(Clone, Default)]
struct ViewerMemory {
    index: usize,
    /// Frame this was last saved, to tell a viewer that stayed open from a reopened one.
    frame: u64,
    zoom: PhotoZoom,
    /// Horizontal distance of the drag in progress on an unzoomed photo.
    swipe: f32,
    /// The drag in progress included a pinch: it neither pans nor swipes.
    pinched: bool,
    /// Scroll the thumbnail row to the current photo on the next frame.
    scroll_thumbs: bool,
}

pub fn show(ctx: &egui::Context, state: &mut AppState, theme: &Theme) {
    // Only over the Campsite panel: the detail also lives on while editing.
    if !matches!(state.panel, Some(Panel::Campsite(_))) {
        return;
    }
    let api_base = state.api_base.as_str();
    let Some(d) = state.detail.as_mut() else { return };
    let Some(c) = d.campsite.loaded().cloned() else { return };
    let count = c.photos.len();
    let Some(index) = d.photo_open.filter(|&i| i < count) else {
        d.photo_open = None;
        return;
    };

    let layout = &theme.layout;
    let content = ctx.content_rect();
    // Phones and portrait screens: full screen, every point goes to the photo.
    let compact = theme.is_narrow(ctx) || content.height() > content.width();
    let margin = if compact { layout.photo_viewer_margin_narrow } else { layout.photo_viewer_margin };
    let screen = content.shrink(margin);
    let (radius, inner_margin) = if compact {
        let padding = theme.spacing.item_spacing[0].round().clamp(0.0, 127.0) as i8;
        (CornerRadius::ZERO, Margin::same(padding))
    } else {
        (theme.radius(), theme.panel_margin())
    };
    let frame = egui::Frame::NONE.fill(theme.colors.photo_viewer_bg).corner_radius(radius).inner_margin(inner_margin);
    let modal = Modal::new(Id::new("photo_viewer"))
        .backdrop_color(theme.colors.photo_viewer_backdrop)
        .frame(frame)
        .show(ctx, |ui| {
            // The frame's inner margin is added around this size.
            let inner = screen.size() - inner_margin.sum();
            ui.set_min_size(inner);
            ui.set_max_size(inner);
            carousel(ui, theme, &c, d, index, api_base, compact)
        });
    if modal.inner || modal.should_close() {
        d.photo_open = None;
    }
}

/// Returns true when the close button was clicked.
fn carousel(
    ui: &mut Ui,
    theme: &Theme,
    c: &Campsite,
    d: &mut CampsiteDetail,
    index: usize,
    api_base: &str,
    compact: bool,
) -> bool {
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

    let memory_id = Id::new("photo_viewer_memory");
    let frame_nr = ui.ctx().cumulative_frame_nr();
    let mut mem = ui.data(|data| data.get_temp::<ViewerMemory>(memory_id)).unwrap_or_default();
    if mem.index != index || mem.frame + 1 < frame_nr {
        mem = ViewerMemory { index, scroll_thumbs: true, ..Default::default() };
    }
    // Esc first unzooms; the modal only closes on the next one.
    if mem.zoom.is_zoomed() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
        mem.zoom = PhotoZoom::default();
    }

    // Header: title, then position, "Fit" (when zoomed) and close on the right. A long title is
    // truncated, so the × stays on screen (narrow screens, titles up to 100 chars).
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let close_button = nav_button(theme, "×").min_size(Vec2::splat(theme.layout.icon_button_size));
            close = ui.add(close_button).on_hover_text("Close (Esc)").clicked();
            ui.label(RichText::new(format!("{} / {count}", index + 1)).color(text));
            if mem.zoom.is_zoomed()
                && ui.add(nav_button(theme, "Fit")).on_hover_text("Fit to screen (double click)").clicked()
            {
                mem.zoom = PhotoZoom::default();
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(&c.title).color(text).strong()).truncate());
            });
        });
    });

    // Thumbnails take the bottom row when there is more than one photo.
    let thumbs_height =
        if count > 1 { theme.layout.photo_viewer_thumb_height + theme.spacing.item_spacing[1] * 2.0 } else { 0.0 };
    let main_height = (ui.available_height() - thumbs_height).max(0.0);

    ui.allocate_ui_with_layout(vec2(ui.available_width(), main_height), Layout::left_to_right(Align::Center), |ui| {
        // Wide screens: ◀ ▶ columns beside the photo. Compact: buttons over it (below).
        let side_arrows = count > 1 && !compact;
        let arrow_width = if side_arrows { theme.typography.size_heading * 2.0 } else { 0.0 };
        if side_arrows && ui.add_sized(vec2(arrow_width, main_height), nav_button(theme, "◀")).clicked() {
            d.step_photo(-1, count);
        }
        let image_size = vec2((ui.available_width() - arrow_width).max(0.0), main_height);
        let (viewport, response) = ui.allocate_exact_size(image_size, Sense::click_and_drag());
        match photo_url(api_base, c, &c.photos[index], PhotoSize::Large) {
            Some(url) => {
                let step = photo(ui, theme, &mut mem, url, viewport, &response, count);
                if step != 0 {
                    d.step_photo(step, count);
                }
            }
            None => {
                ui.put(viewport, egui::Label::new(RichText::new("Photo unavailable.").color(text)));
            }
        }
        // Compact screens have no arrow columns: small ◀ ▶ buttons over the photo's sides
        // instead, hidden while zoomed so they don't cover it.
        if compact && count > 1 && !mem.zoom.is_zoomed() {
            let side = Vec2::splat(theme.layout.icon_button_size);
            let inset = theme.spacing.item_spacing[0];
            let left = Rect::from_min_size(viewport.left_center() + vec2(inset, -side.y / 2.0), side);
            let right = Rect::from_min_size(viewport.right_center() - vec2(inset + side.x, side.y / 2.0), side);
            if ui.put(left, overlay_button(theme, "◀")).clicked() {
                d.step_photo(-1, count);
            }
            if ui.put(right, overlay_button(theme, "▶")).clicked() {
                d.step_photo(1, count);
            }
        }
        if side_arrows && ui.add_sized(vec2(arrow_width, main_height), nav_button(theme, "▶")).clicked() {
            d.step_photo(1, count);
        }
    });

    if count > 1 {
        ui.add_space(theme.spacing.item_spacing[1]);
        let thumbs = c
            .photos
            .iter()
            .map(|file| Thumb {
                source: photo_url(api_base, c, file, PhotoSize::Thumb).map(|u| ImageSource::Uri(Cow::Owned(u))),
                label: file,
            })
            .collect();
        let style = StripStyle {
            height: theme.layout.photo_viewer_thumb_height,
            removable: false,
            selected: Some(index),
            wrap: false,
        };
        ui.vertical_centered(|ui| {
            // Centre the row when it fits; otherwise it scrolls sideways (one row, swipeable).
            let row_width = count as f32 * (style.height * 4.0 / 3.0 + theme.spacing.photo_gap);
            ui.set_max_width(row_width.min(ui.available_width()));
            ScrollArea::horizontal().id_salt("photo_viewer_thumbs").show(ui, |ui| {
                let strip = photo_strip(ui, theme, thumbs, style);
                if let Some(i) = strip.clicked {
                    d.photo_open = Some(i);
                }
                if mem.scroll_thumbs
                    && let Some(rect) = strip.selected_rect
                {
                    ui.scroll_to_rect(rect, Some(Align::Center));
                    mem.scroll_thumbs = false;
                }
            });
        });
    }

    // A photo change resets the memory on the next frame (it is keyed by `index`).
    mem.frame = frame_nr;
    ui.data_mut(|data| data.insert_temp(memory_id, mem));
    close
}

/// The zoomable photo in `viewport`, with its gestures. Returns the photo step of a finished
/// swipe (0 if none).
fn photo(
    ui: &mut Ui,
    theme: &Theme,
    mem: &mut ViewerMemory,
    url: String,
    viewport: Rect,
    response: &egui::Response,
    count: usize,
) -> isize {
    let layout = &theme.layout;
    let image = Image::new(ImageSource::Uri(Cow::Owned(url)))
        .fit_to_exact_size(viewport.size())
        .maintain_aspect_ratio(true)
        .corner_radius(theme.radius());
    // Size of the photo fitted to the viewport; `None` while it loads or if it failed.
    let Some(fitted) = image.load_and_calc_size(ui, viewport.size()) else {
        ui.put(viewport, image); // egui's spinner or error
        return 0;
    };

    let (pinch, wheel_zoom, scroll, hover) =
        ui.input(|i| (i.multi_touch(), i.zoom_delta(), i.smooth_scroll_delta(), i.pointer.hover_pos()));
    let zoom = &mut mem.zoom;
    if let Some(touch) = pinch.filter(|t| viewport.contains(t.start_pos)) {
        mem.pinched = true;
        zoom.zoom_about(touch.zoom_delta, touch.center_pos, viewport, fitted, layout.photo_viewer_max_zoom);
        zoom.pan(touch.translation_delta, viewport, fitted);
    } else if let Some(pos) = hover.filter(|p| viewport.contains(*p)) {
        // Ctrl/⌘ + wheel and trackpad pinches come as `zoom_delta`, a plain wheel as a scroll.
        let factor = wheel_zoom * (scroll.y * layout.photo_viewer_wheel_zoom_speed).exp();
        if factor != 1.0 {
            zoom.zoom_about(factor, pos, viewport, fitted, layout.photo_viewer_max_zoom);
        }
    }

    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        zoom.toggle(pos, viewport, fitted, layout.photo_viewer_double_tap_zoom);
    }
    if response.drag_started() {
        mem.swipe = 0.0;
        mem.pinched = pinch.is_some();
    }
    if response.dragged() && !mem.pinched {
        if zoom.is_zoomed() {
            zoom.pan(response.drag_delta(), viewport, fitted);
        } else if count > 1 {
            mem.swipe += response.drag_delta().x;
        }
    }
    let mut step = 0;
    if response.drag_stopped() {
        if !mem.pinched && !zoom.is_zoomed() && count > 1 {
            step = swipe_step(mem.swipe, layout.photo_viewer_swipe_threshold);
        }
        mem.swipe = 0.0;
        mem.pinched = false;
    }
    if response.hovered() && zoom.is_zoomed() {
        let icon = if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab };
        ui.ctx().set_cursor_icon(icon);
    }

    // An unzoomed photo follows the finger while swiping.
    let rect = zoom.image_rect(viewport, fitted).translate(vec2(mem.swipe, 0.0));
    let mut clipped = ui.new_child(UiBuilder::new().max_rect(viewport));
    clipped.set_clip_rect(viewport.intersect(ui.clip_rect()));
    image.paint_at(&clipped, rect);
    step
}

/// Flat, frameless button in the viewer's text color.
fn nav_button<'a>(theme: &Theme, text: &'a str) -> Button<'a> {
    Button::new(RichText::new(text).color(theme.colors.photo_viewer_text).size(theme.typography.size_heading))
        .frame(false)
}

/// A button drawn over the photo: on the backdrop color so it shows on light photos too.
fn overlay_button<'a>(theme: &Theme, text: &'a str) -> Button<'a> {
    Button::new(RichText::new(text).color(theme.colors.photo_viewer_text).size(theme.typography.size_body))
        .fill(theme.colors.photo_viewer_backdrop)
        .corner_radius(theme.radius())
}
