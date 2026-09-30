use egui::{Align2, Area, Button, Frame, Id, Label, Margin, Order, RichText, Stroke, Vec2};

use crate::actions::ToastKind;
use crate::state::Toast;
use crate::ui::theme::Theme;
use crate::ui::widgets::text_width;

/// Toasts stacked at the bottom-left of the map. Each one disappears
/// `layout.toast_duration_ms` after it was first shown, or when closed.
pub fn toasts(ctx: &egui::Context, theme: &Theme, toasts: &mut Vec<Toast>) {
    let now = ctx.input(|i| i.time);
    let ttl = f64::from(theme.layout.toast_duration_ms) / 1000.0;
    toasts.retain(|t| t.shown_at.is_none_or(|shown| now - shown <= ttl));
    if toasts.is_empty() {
        return;
    }
    let c = &theme.colors;
    let mut closed = None;

    Area::new(Id::new("toasts")).order(Order::Foreground).anchor(Align2::LEFT_BOTTOM, [16.0, -24.0]).show(ctx, |ui| {
        for (i, toast) in toasts.iter_mut().enumerate() {
            toast.shown_at.get_or_insert(now);
            let accent = match toast.kind {
                ToastKind::Info => c.info,
                ToastKind::Success => c.success,
                ToastKind::Error => c.danger,
            };
            Frame::NONE
                .fill(c.toast_bg)
                .stroke(Stroke::new(2.0, accent))
                .corner_radius(theme.radius())
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    // Never wider than the window (16 offset and 12 padding on each side).
                    ui.set_max_width((ctx.content_rect().width() - 2.0 * (16.0 + 12.0)).min(360.0));
                    ui.horizontal(|ui| {
                        // Long messages wrap, leaving room for the ×.
                        let close = RichText::new("×").color(c.toast_text);
                        let text_max = ui.available_width() - text_width(ui, &close) - ui.spacing().item_spacing.x;
                        ui.allocate_ui(Vec2::new(text_max, 0.0), |ui| {
                            ui.add(Label::new(RichText::new(&toast.text).color(c.toast_text)).wrap());
                        });
                        if ui.add(Button::new(close).frame(false)).clicked() {
                            closed = Some(i);
                        }
                    });
                });
        }
    });
    if let Some(i) = closed {
        toasts.remove(i);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
}
