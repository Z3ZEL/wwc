//! A slider with two handles for an inclusive integer range. The handles can't cross.
//! Drag (or click) moves the handle closest to the pointer; ← / → move the last one used.

use std::ops::RangeInclusive;

use egui::{Id, Key, Rect, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2};

use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handle {
    Low,
    High,
}

/// Returns the new `(low, high)` when the user changed it.
pub fn range_slider(ui: &mut Ui, theme: &Theme, bounds: RangeInclusive<u8>, low: u8, high: u8) -> Option<(u8, u8)> {
    let (min, max) = (*bounds.start(), *bounds.end());
    let s = &theme.layout;
    let radius = s.range_slider_handle_radius;
    let size = Vec2::new(ui.available_width(), radius * 2.0 + theme.shape.focus_border_width * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let (left, right) = (rect.left() + radius, rect.right() - radius);
    let x_of = |v: u8| x_at(v, left, right, min, max);

    // The handle being moved is remembered across frames (for dragging and the arrow keys).
    let active_id: Id = response.id.with("active");
    let mut active = ui.data(|d| d.get_temp::<Handle>(active_id)).unwrap_or(Handle::Low);
    let mut new = (low.clamp(min, max), high.clamp(min, max));

    if let Some(p) = response.interact_pointer_pos() {
        // Choose the handle on the press itself: `drag_started` only fires after the pointer
        // has moved a few points, and until then the previous handle would follow the pointer.
        if ui.input(|i| i.pointer.any_pressed()) {
            active = nearest_handle(p.x, x_of(new.0), x_of(new.1));
        }
        new = move_handle(new, active, value_at(p.x, left, right, min, max));
    }
    if response.has_focus() {
        let step = ui.input(|i| i32::from(i.key_pressed(Key::ArrowRight)) - i32::from(i.key_pressed(Key::ArrowLeft)));
        let current = if active == Handle::Low { new.0 } else { new.1 };
        let target = (i32::from(current) + step).clamp(i32::from(min), i32::from(max));
        new = move_handle(new, active, u8::try_from(target).unwrap_or(current));
    }
    ui.data_mut(|d| d.insert_temp(active_id, active));

    // Track, selected span, handles.
    let c = &theme.colors;
    let painter = ui.painter_at(rect.expand(theme.shape.focus_border_width));
    let y = rect.center().y;
    let half = s.range_slider_track_height / 2.0;
    let track = Rect::from_min_max(pos2(left, y - half), pos2(right, y + half));
    painter.rect_filled(track, theme.radius(), c.border);
    let span = Rect::from_min_max(pos2(x_of(new.0), y - half), pos2(x_of(new.1), y + half));
    painter.rect_filled(span, theme.radius(), c.primary);
    for handle in [Handle::Low, Handle::High] {
        let x = x_of(if handle == Handle::Low { new.0 } else { new.1 });
        let focused = response.has_focus() && handle == active;
        let (width, color) = if focused {
            (theme.shape.focus_border_width, c.input_border_focus)
        } else {
            (theme.shape.border_width, c.primary)
        };
        painter.circle(pos2(x, y), radius, c.surface, Stroke::new(width, color));
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response.widget_info(|| WidgetInfo::labeled(WidgetType::Slider, true, format!("{} to {}", new.0, new.1)));
    (new != (low, high)).then_some(new)
}

fn x_at(v: u8, left: f32, right: f32, min: u8, max: u8) -> f32 {
    if max <= min {
        return left;
    }
    left + (right - left) * f32::from(v.saturating_sub(min)) / f32::from(max - min)
}

/// The value under `x`, rounded to the nearest integer and clamped to `min..=max`.
fn value_at(x: f32, left: f32, right: f32, min: u8, max: u8) -> u8 {
    if max <= min || right <= left {
        return min;
    }
    let t = ((x - left) / (right - left)).clamp(0.0, 1.0);
    let v = f32::from(min) + t * f32::from(max - min);
    (v.round() as u8).clamp(min, max)
}

/// Handle closest to `x`. When both are at the same place, the side of the pointer decides,
/// so a range squeezed to one value can still be opened in both directions.
fn nearest_handle(x: f32, x_low: f32, x_high: f32) -> Handle {
    let (d_low, d_high) = ((x - x_low).abs(), (x - x_high).abs());
    if d_low < d_high || (d_low == d_high && x < x_low) { Handle::Low } else { Handle::High }
}

/// Moves one handle to `value`, stopping at the other handle.
fn move_handle((low, high): (u8, u8), handle: Handle, value: u8) -> (u8, u8) {
    match handle {
        Handle::Low => (value.min(high), high),
        Handle::High => (low, value.max(low)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_and_values_round_trip() {
        for v in 1..=10 {
            let x = x_at(v, 100.0, 280.0, 1, 10);
            assert_eq!(value_at(x, 100.0, 280.0, 1, 10), v);
        }
        assert_eq!(value_at(0.0, 100.0, 280.0, 1, 10), 1, "left of the track");
        assert_eq!(value_at(999.0, 100.0, 280.0, 1, 10), 10, "right of the track");
        assert_eq!(value_at(108.0, 100.0, 280.0, 1, 10), 1, "rounds to the nearest step");
        assert_eq!(value_at(112.0, 100.0, 280.0, 1, 10), 2);
        assert_eq!(value_at(150.0, 100.0, 100.0, 1, 10), 1, "zero-width track");
    }

    #[test]
    fn handles_cannot_cross() {
        assert_eq!(move_handle((3, 6), Handle::Low, 9), (6, 6));
        assert_eq!(move_handle((3, 6), Handle::High, 1), (3, 3));
        assert_eq!(move_handle((3, 6), Handle::Low, 2), (2, 6));
        assert_eq!(move_handle((3, 6), Handle::High, 10), (3, 10));
    }

    #[test]
    fn nearest_handle_picks_by_distance_then_side() {
        assert_eq!(nearest_handle(10.0, 0.0, 100.0), Handle::Low);
        assert_eq!(nearest_handle(90.0, 0.0, 100.0), Handle::High);
        assert_eq!(nearest_handle(40.0, 50.0, 50.0), Handle::Low);
        assert_eq!(nearest_handle(60.0, 50.0, 50.0), Handle::High);
    }
}
