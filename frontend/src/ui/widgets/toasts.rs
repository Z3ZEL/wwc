use egui::{Align2, Area, Button, Frame, Id, Margin, Order, RichText, Stroke};

use crate::actions::ToastKind;
use crate::state::Toast;
use crate::ui::theme::Theme;

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
                    ui.set_max_width(360.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&toast.text).color(c.toast_text));
                        if ui.add(Button::new(RichText::new("×").color(c.toast_text)).frame(false)).clicked() {
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
