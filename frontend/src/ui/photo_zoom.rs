//! Zoom and pan of the photo in the full-page viewer (`ui::photo_viewer`). Pure geometry, so it
//! is tested on the host. At scale 1 the photo is fitted to the viewport and centered; `offset`
//! moves its center, and is clamped so a zoomed photo always covers the viewport on the axes
//! where it is larger than it.

use egui::{Pos2, Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoZoom {
    /// 1 = fitted to the viewport.
    pub scale: f32,
    /// Photo center minus viewport center, in points.
    pub offset: Vec2,
}

impl Default for PhotoZoom {
    fn default() -> Self {
        Self { scale: 1.0, offset: Vec2::ZERO }
    }
}

impl PhotoZoom {
    pub fn is_zoomed(&self) -> bool {
        self.scale > 1.0
    }

    /// Where the photo is drawn. `fitted` is its size at scale 1.
    pub fn image_rect(&self, viewport: Rect, fitted: Vec2) -> Rect {
        Rect::from_center_size(viewport.center() + self.offset, fitted * self.scale)
    }

    /// Multiplies the scale by `factor` (kept in `1..=max`), keeping the photo point under
    /// `focus` where it is.
    pub fn zoom_about(&mut self, factor: f32, focus: Pos2, viewport: Rect, fitted: Vec2, max: f32) {
        let scale = (self.scale * factor).clamp(1.0, max.max(1.0));
        let applied = scale / self.scale;
        let from_center = focus - (viewport.center() + self.offset);
        self.offset = focus - viewport.center() - from_center * applied;
        self.scale = scale;
        self.clamp(viewport, fitted);
    }

    pub fn pan(&mut self, delta: Vec2, viewport: Rect, fitted: Vec2) {
        self.offset += delta;
        self.clamp(viewport, fitted);
    }

    /// Double click / tap: zooms in to `zoom` on `focus`, or back to fitted when zoomed.
    pub fn toggle(&mut self, focus: Pos2, viewport: Rect, fitted: Vec2, zoom: f32) {
        if self.is_zoomed() {
            *self = Self::default();
        } else {
            self.zoom_about(zoom, focus, viewport, fitted, zoom);
        }
    }

    fn clamp(&mut self, viewport: Rect, fitted: Vec2) {
        let slack = ((fitted * self.scale - viewport.size()) / 2.0).max(Vec2::ZERO);
        self.offset = self.offset.clamp(-slack, slack);
    }
}

/// Photo step for a finished horizontal swipe of `dx` points: a swipe to the left shows the
/// next photo, to the right the previous one, and a short one nothing.
pub fn swipe_step(dx: f32, threshold: f32) -> isize {
    if dx <= -threshold {
        1
    } else if dx >= threshold {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    fn viewport() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 800.0))
    }

    /// A landscape photo fitted to the portrait viewport: full width, letterboxed.
    const FITTED: Vec2 = vec2(400.0, 300.0);

    #[test]
    fn fitted_photo_is_centered_and_cannot_pan() {
        let mut z = PhotoZoom::default();
        assert_eq!(z.image_rect(viewport(), FITTED), Rect::from_center_size(pos2(200.0, 400.0), FITTED));
        z.pan(vec2(50.0, -30.0), viewport(), FITTED);
        assert_eq!(z, PhotoZoom::default());
    }

    #[test]
    fn zoom_keeps_the_focus_point_still() {
        let mut z = PhotoZoom::default();
        // On the vertical center line: a 600-point tall photo stays centered in the 800 viewport.
        let focus = pos2(300.0, 400.0);
        let before = z.image_rect(viewport(), FITTED);
        let rel = (focus - before.min) / before.size();
        z.zoom_about(2.0, focus, viewport(), FITTED, 4.0);
        let after = z.image_rect(viewport(), FITTED);
        assert_eq!(z.scale, 2.0);
        let moved = after.min + rel * after.size();
        assert!((moved - focus).length() < 0.01, "{moved:?}");
    }

    #[test]
    fn scale_is_clamped() {
        let mut z = PhotoZoom::default();
        z.zoom_about(10.0, pos2(200.0, 400.0), viewport(), FITTED, 4.0);
        assert_eq!(z.scale, 4.0);
        z.zoom_about(0.01, pos2(0.0, 0.0), viewport(), FITTED, 4.0);
        assert_eq!(z, PhotoZoom::default(), "back to fitted, recentered");
    }

    #[test]
    fn pan_stops_at_the_photo_edges() {
        let mut z = PhotoZoom { scale: 2.0, offset: Vec2::ZERO };
        z.pan(vec2(1000.0, 1000.0), viewport(), FITTED);
        // 800×600 photo: 200 of slack each side horizontally; it is shorter than the viewport.
        assert_eq!(z.offset, vec2(200.0, 0.0));
        let rect = z.image_rect(viewport(), FITTED);
        assert_eq!(rect.min.x, 0.0);

        z.scale = 4.0; // 1600×1200: taller than the viewport now
        z.pan(vec2(-5000.0, -5000.0), viewport(), FITTED);
        assert_eq!(z.offset, vec2(-600.0, -200.0));
    }

    #[test]
    fn toggle_zooms_in_then_back_out() {
        let mut z = PhotoZoom::default();
        z.toggle(pos2(100.0, 400.0), viewport(), FITTED, 2.5);
        assert_eq!(z.scale, 2.5);
        assert!(z.is_zoomed());
        z.toggle(pos2(100.0, 400.0), viewport(), FITTED, 2.5);
        assert_eq!(z, PhotoZoom::default());
    }

    #[test]
    fn swipes() {
        assert_eq!(swipe_step(-80.0, 60.0), 1);
        assert_eq!(swipe_step(80.0, 60.0), -1);
        assert_eq!(swipe_step(-59.0, 60.0), 0);
        assert_eq!(swipe_step(0.0, 60.0), 0);
    }
}
