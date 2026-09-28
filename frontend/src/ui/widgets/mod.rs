//! Reusable widgets. They know nothing about `AppState`: plain inputs, plain outputs.

mod panel_frame;
mod photo_strip;
mod range_slider;
mod spinner;
mod toasts;

pub use panel_frame::panel_frame;
pub use photo_strip::{PhotoStripResponse, StripStyle, Thumb, photo_strip};
pub use range_slider::range_slider;
pub use spinner::{Spinner, SpinnerSize, loading, loading_block, spinner};
pub use toasts::toasts;

use egui::{Button, Color32, Response, RichText, Sense, Stroke, TextStyle, TextWrapMode, Ui, Vec2, WidgetText};

use crate::actions::Action;
use crate::api::ApiError;
use crate::ui::theme::{Theme, subheading};

pub fn primary_button(ui: &mut Ui, theme: &Theme, text: &str, enabled: bool) -> Response {
    let c = &theme.colors;
    let fg = if enabled { c.on_primary } else { c.text_disabled };
    let bg = if enabled { c.primary } else { c.button_bg };
    ui.add_enabled(enabled, Button::new(RichText::new(text).color(fg)).fill(bg).stroke(Stroke::NONE))
}

pub fn danger_button(ui: &mut Ui, theme: &Theme, text: &str, enabled: bool) -> Response {
    let c = &theme.colors;
    let fg = if enabled { c.on_primary } else { c.text_disabled };
    let bg = if enabled { c.danger } else { c.button_bg };
    ui.add_enabled(enabled, Button::new(RichText::new(text).color(fg)).fill(bg).stroke(Stroke::NONE))
}

/// Borderless text button, used for secondary actions and links between panels.
pub fn link_button(ui: &mut Ui, theme: &Theme, text: &str) -> Response {
    ui.add(Button::new(RichText::new(text).color(theme.colors.link)).frame(false))
}

/// Shown instead of a create form while the user's email is not confirmed.
pub fn verify_prompt(ui: &mut Ui, theme: &Theme, email: &str, what: &str, actions: &mut Vec<Action>) {
    muted(ui, theme, format!("Confirm your email ({email}) {what}"));
    ui.horizontal(|ui| {
        if link_button(ui, theme, "Resend email").clicked() {
            actions.push(Action::RequestVerification { email: email.to_owned() });
        }
        if link_button(ui, theme, "I've confirmed it").clicked() {
            actions.push(Action::RefreshSession);
        }
    });
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).text_style(subheading()));
}

pub fn muted(ui: &mut Ui, theme: &Theme, text: impl Into<String>) -> Response {
    ui.label(RichText::new(text.into()).color(theme.colors.text_muted))
}

pub fn tag_chip(ui: &mut Ui, theme: &Theme, label: &str, selected: bool) -> Response {
    let c = &theme.colors;
    let (bg, fg) = if selected { (c.tag_selected_bg, c.tag_selected_text) } else { (c.tag_bg, c.tag_text) };
    ui.add(
        Button::new(RichText::new(label).color(fg).small())
            .fill(bg)
            .stroke(Stroke::NONE)
            .corner_radius(theme.tag_radius()),
    )
}

/// Small colored label, e.g. "Hidden by moderation".
pub fn badge(ui: &mut Ui, theme: &Theme, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::NONE
        .fill(bg)
        .corner_radius(theme.tag_radius())
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| ui.label(RichText::new(text).color(fg).small()));
}

/// Read-only star rating, e.g. 4.2 → ★★★★☆.
pub fn stars(ui: &mut Ui, theme: &Theme, value: f64) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        for i in 1..=5 {
            let filled = value >= f64::from(i) - 0.5;
            let color = if filled { theme.colors.star_filled } else { theme.colors.star_empty };
            ui.label(RichText::new("★").color(color).size(theme.typography.size_subheading));
        }
    });
}

/// Clickable stars. Returns the clicked score (1–5).
pub fn stars_input(ui: &mut Ui, theme: &Theme, current: Option<u8>, enabled: bool) -> Option<u8> {
    let size = theme.typography.size_heading;
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        // Hover previews the score under the pointer.
        let hovered = ui.memory(|m| m.data.get_temp::<u8>(ui.id().with("hover")));
        let shown = hovered.or(current).unwrap_or(0);
        let mut now_hovered = None;
        for i in 1..=5u8 {
            let color = if i <= shown { theme.colors.star_filled } else { theme.colors.star_empty };
            let sense = if enabled { Sense::click() } else { Sense::hover() };
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), sense);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "★", egui::FontId::proportional(size), color);
            if enabled && resp.hovered() {
                now_hovered = Some(i);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        let id = ui.id().with("hover");
        ui.memory_mut(|m| {
            if let Some(i) = now_hovered {
                m.data.insert_temp(id, i);
            } else {
                m.data.remove::<u8>(id);
            }
        });
    });
    clicked
}

/// A form field error from the server (or client-side validation).
pub fn field_error(ui: &mut Ui, theme: &Theme, error: Option<&ApiError>, field: &str) {
    if let Some(msg) = error.and_then(|e| e.field(field)) {
        ui.label(RichText::new(msg).color(theme.colors.danger).small());
    }
}

/// The general error message of a form, if any.
pub fn form_error(ui: &mut Ui, theme: &Theme, error: Option<&ApiError>) {
    if let Some(e) = error {
        ui.label(RichText::new(e.to_string()).color(theme.colors.danger));
    }
}

/// Error with a retry button, for `Remote::Failed`. Returns true when retry is clicked.
pub fn failed(ui: &mut Ui, theme: &Theme, error: &ApiError) -> bool {
    ui.label(RichText::new(error.to_string()).color(theme.colors.danger));
    ui.button("Retry").clicked()
}

/// Labeled single-line text input.
pub fn text_field(ui: &mut Ui, label: &str, value: &mut String, password: bool) -> Response {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).password(password).desired_width(f32::INFINITY))
}

/// Width of a one-line button label, without the button's padding.
pub fn text_width(ui: &Ui, text: &RichText) -> f32 {
    let galley =
        WidgetText::from(text.clone()).into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
    galley.size().x
}

/// True when Enter was pressed while `response` had focus.
pub fn submitted(ui: &Ui, response: &Response) -> bool {
    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}
