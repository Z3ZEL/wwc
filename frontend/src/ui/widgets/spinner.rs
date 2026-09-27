//! Themed loading spinner: a track circle and a rotating arc. Use it instead of `ui.spinner()`,
//! whose size and color don't come from the theme.

use std::f32::consts::TAU;

use egui::{Color32, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2};

use super::muted;
use crate::ui::theme::Theme;

/// Share of the circle covered by the arc.
const ARC_FRACTION: f32 = 0.75;
const ARC_POINTS: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerSize {
    /// Next to a label or inside a line of text.
    Inline,
    /// The main load of a panel.
    Block,
}

/// Builder for a spinner that isn't drawn on a panel background, e.g. on the top bar.
#[derive(Debug, Clone, Copy)]
pub struct Spinner {
    size: SpinnerSize,
    color: Option<Color32>,
    track: Option<Option<Color32>>,
    visible: bool,
}

impl Spinner {
    pub fn new(size: SpinnerSize) -> Self {
        Self { size, color: None, track: None, visible: true }
    }

    /// Arc color (default `colors.spinner`).
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Track color; `None` draws no track (default `colors.spinner_track`).
    pub fn track(mut self, track: Option<Color32>) -> Self {
        self.track = Some(track);
        self
    }

    /// When false, the spinner keeps its space but draws nothing, so a spinner that comes
    /// and goes doesn't shift the widgets after it.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Draws the spinner and keeps egui repainting while it is visible.
    pub fn show(self, ui: &mut Ui, theme: &Theme) -> Response {
        let l = &theme.layout;
        let diameter = match self.size {
            SpinnerSize::Inline => l.spinner_size_inline,
            SpinnerSize::Block => l.spinner_size_block,
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
        if !self.visible {
            return response;
        }
        response.widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, "Loading"));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        // Only repaint while a spinner is on screen: egui goes idle again once it is gone.
        ui.ctx().request_repaint();

        let center = rect.center();
        let radius = (diameter - l.spinner_stroke) / 2.0;
        let painter = ui.painter();
        if let Some(track) = self.track.unwrap_or(Some(theme.colors.spinner_track)) {
            painter.circle_stroke(center, radius, Stroke::new(l.spinner_stroke, track));
        }
        let period = f64::from(l.spinner_period_ms.max(1)) / 1000.0;
        let start = (ui.input(|i| i.time) / period).fract() as f32 * TAU;
        let points = (0..=ARC_POINTS)
            .map(|i| {
                let a = start + ARC_FRACTION * TAU * i as f32 / ARC_POINTS as f32;
                pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        let color = self.color.unwrap_or(theme.colors.spinner);
        painter.add(Shape::line(points, Stroke::new(l.spinner_stroke, color)));
        response
    }
}

/// A spinner with the theme colors.
pub fn spinner(ui: &mut Ui, theme: &Theme, size: SpinnerSize) -> Response {
    Spinner::new(size).show(ui, theme)
}

/// Inline spinner with a label, for a section still loading (`Remote::Loading`).
pub fn loading(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.horizontal(|ui| {
        spinner(ui, theme, SpinnerSize::Inline);
        muted(ui, theme, text);
    });
}

/// Centered large spinner with a label, for the main content of a panel.
pub fn loading_block(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.add_space(theme.spacing.section_gap);
    ui.vertical_centered(|ui| {
        spinner(ui, theme, SpinnerSize::Block);
        muted(ui, theme, text);
    });
    ui.add_space(theme.spacing.section_gap);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use egui::{Context, RawInput, ViewportId};

    use super::*;

    /// Runs a few passes (egui repaints on its own at startup) and returns the spinner size
    /// and the repaint delay egui asked for after the last one.
    fn render(spinner: Option<Spinner>) -> (Option<Vec2>, Duration) {
        let theme = Theme::load();
        let ctx = Context::default();
        let mut drawn = None;
        let mut delay = Duration::MAX;
        for _ in 0..5 {
            let mut out = ctx.run_ui(RawInput::default(), |ui| {
                drawn = spinner.map(|s| s.show(ui, &theme).rect.size());
            });
            out.textures_delta.clear(); // no renderer here to upload the font atlas to
            delay = out.viewport_output.get(&ViewportId::ROOT).map_or(Duration::MAX, |v| v.repaint_delay);
        }
        (drawn, delay)
    }

    #[test]
    fn sizes_come_from_the_theme() {
        let l = Theme::load().layout;
        assert_eq!(render(Some(Spinner::new(SpinnerSize::Inline))).0, Some(Vec2::splat(l.spinner_size_inline)));
        assert_eq!(render(Some(Spinner::new(SpinnerSize::Block))).0, Some(Vec2::splat(l.spinner_size_block)));
    }

    #[test]
    fn repaints_only_while_shown() {
        assert_eq!(render(Some(Spinner::new(SpinnerSize::Inline))).1, Duration::ZERO);
        assert_ne!(render(None).1, Duration::ZERO, "no spinner, no busy repaint");
    }

    #[test]
    fn hidden_spinner_keeps_its_space_without_repainting() {
        let l = Theme::load().layout;
        let (size, delay) = render(Some(Spinner::new(SpinnerSize::Inline).visible(false)));
        assert_eq!(size, Some(Vec2::splat(l.spinner_size_inline)));
        assert_ne!(delay, Duration::ZERO);
    }
}
