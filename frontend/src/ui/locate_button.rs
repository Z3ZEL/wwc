//! "Locate me" button at the bottom-right of the map, above the footer: centers the map on the
//! visitor and shows where they are (ARCHITECTURE §5.5, ADR 0019). The controller asks the
//! browser for the position; this only draws the button.

use egui::{Align2, Area, Button, Color32, Id, Order, Pos2, Rangef, Rect, Stroke, Ui, Vec2};

use crate::actions::Action;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widgets::{Spinner, SpinnerSize};

/// Icon geometry, as shares of the button side: the ring's radius, the ticks' outer end
/// (from the center, in ring radii) and the center dot's radius (in ring radii).
const RING: f32 = 0.2;
const TICK_END: f32 = 1.6;
const DOT: f32 = 0.45;

/// `area`: the map above the footer. `avoid`: a card at the bottom of the map (the privacy
/// notice or the consent panel) that the button goes above when they would overlap, as on phones.
pub fn show(
    ctx: &egui::Context,
    area: Rect,
    avoid: Option<Rect>,
    state: &AppState,
    theme: &Theme,
    actions: &mut Vec<Action>,
) {
    let size = theme.layout.map_button_size;
    let margin = theme.spacing.map_button_margin;
    if area.width() < size + 2.0 * margin || area.height() < size + 2.0 * margin {
        return;
    }
    let right = area.right() - margin;
    let columns = Rangef::new(right - size, right);
    let bottom = match avoid.filter(|card| card.x_range().intersects(columns)) {
        Some(card) => area.bottom().min(card.top()),
        None => area.bottom(),
    };

    Area::new(Id::new("locate_button"))
        .order(Order::Middle)
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(Pos2::new(right, bottom - margin))
        .constrain_to(area)
        .show(ctx, |ui| {
            let pending = state.locate.pending;
            let response = ui
                .add_enabled(!pending, Button::new("").min_size(Vec2::splat(size)))
                .on_hover_text("Show my location")
                .on_disabled_hover_text("Finding your position…");
            if response.clicked() {
                actions.push(Action::Locate);
            }
            if pending {
                let spinner = Rect::from_center_size(response.rect.center(), Vec2::splat(size));
                ui.put(spinner, |ui: &mut Ui| Spinner::new(SpinnerSize::Inline).show(ui, theme));
            } else {
                let color = ui.style().interact(&response).fg_stroke.color;
                crosshair(ui, response.rect, color, state.locate.found.is_some(), theme);
            }
        });
}

/// The usual "my location" icon: a ring with four ticks, plus a dot once a position is known.
/// Same line width as the spinner that replaces it while locating.
fn crosshair(ui: &Ui, rect: Rect, color: Color32, found: bool, theme: &Theme) {
    let painter = ui.painter();
    let center = rect.center();
    let ring = rect.width().min(rect.height()) * RING;
    let stroke = Stroke::new(theme.layout.spinner_stroke, color);
    painter.circle_stroke(center, ring, stroke);
    for dir in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        painter.line_segment([center + dir * ring, center + dir * ring * TICK_END], stroke);
    }
    if found {
        painter.circle_filled(center, ring * DOT, color);
    }
}
