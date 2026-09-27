//! A wrapping row of photo thumbnails, optionally with a remove button on each.

use egui::{Button, ImageSource, Rect, RichText, Sense, Ui, Vec2, vec2};

use crate::ui::theme::Theme;

/// One thumbnail. `source` is `None` when there is nothing to draw (e.g. no preview):
/// `label` is shown instead.
pub struct Thumb<'a> {
    pub source: Option<ImageSource<'a>>,
    pub label: &'a str,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PhotoStripResponse {
    pub clicked: Option<usize>,
    pub removed: Option<usize>,
}

/// Options for `photo_strip`.
pub struct StripStyle {
    /// Box height; thumbnails are 4:3 (the `320x240` server thumb).
    pub height: f32,
    /// Show a remove button on each thumbnail.
    pub removable: bool,
    /// Index drawn with the `photo_selected` outline.
    pub selected: Option<usize>,
}

pub fn photo_strip(ui: &mut Ui, theme: &Theme, thumbs: Vec<Thumb<'_>>, style: StripStyle) -> PhotoStripResponse {
    let StripStyle { height, removable, selected } = style;
    let size = vec2(height * 4.0 / 3.0, height);
    let mut out = PhotoStripResponse::default();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(theme.spacing.photo_gap);
        for (i, thumb) in thumbs.into_iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
            ui.painter().rect_filled(rect, theme.radius(), theme.colors.surface_alt);
            match thumb.source {
                Some(source) => {
                    // Centered in the box, aspect ratio kept (letterboxed on the surface color).
                    ui.put(rect, egui::Image::new(source).corner_radius(theme.radius()).max_size(size));
                }
                None => {
                    ui.put(rect, egui::Label::new(RichText::new(thumb.label).color(theme.colors.text_muted).small()));
                }
            }
            if selected == Some(i) {
                let stroke = egui::Stroke::new(theme.shape.focus_border_width, theme.colors.photo_selected);
                ui.painter().rect_stroke(rect, theme.radius(), stroke, egui::StrokeKind::Outside);
            }
            let resp = resp.on_hover_text(thumb.label);
            if resp.clicked() {
                out.clicked = Some(i);
            }
            if resp.hovered() && !removable {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if removable {
                let side = theme.typography.size_body + theme.spacing.button_padding[1] * 2.0;
                let button_rect = Rect::from_min_size(rect.right_top() - vec2(side, 0.0), Vec2::splat(side));
                let remove = Button::new(RichText::new("×").color(theme.colors.on_primary))
                    .fill(theme.colors.toast_bg)
                    .corner_radius(theme.radius());
                if ui.put(button_rect, remove).on_hover_text("Remove photo").clicked() {
                    out.removed = Some(i);
                }
            }
        }
    });
    out
}
