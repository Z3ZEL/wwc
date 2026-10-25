//! Layer button at the bottom-right of the map, right above "Locate me": switches the map
//! background between Mapbox's outdoor map and aerial imagery (ARCHITECTURE §5.5, ADR 0024).
//! Only in builds with a Mapbox token: OpenStreetMap's tiles have a single layer.

use egui::{Align2, Area, Button, Color32, Id, Order, Pos2, Rect, Shape, Stroke, Ui, Vec2};

use crate::actions::Action;
use crate::map::{Basemap, Provider};
use crate::state::AppState;
use crate::ui::theme::Theme;

/// Icon geometry, as shares of the button side: a sheet's half width, its half height (in half
/// widths), and how far each sheet sits from the center (in half heights).
const SHEET: f32 = 0.26;
const FLAT: f32 = 0.5;
const STEP: f32 = 0.7;

/// `area`: the map above the footer. `locate`: the "Locate me" button; this one goes above it,
/// or isn't drawn when there is no room.
pub fn show(ctx: &egui::Context, area: Rect, locate: Rect, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    if !Provider::configured().has_layers() {
        return;
    }
    let size = theme.layout.map_button_size;
    let margin = theme.spacing.map_button_margin;
    let bottom = locate.top() - margin;
    if bottom - size < area.top() + margin {
        return;
    }

    Area::new(Id::new("layer_button"))
        .order(Order::Middle)
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(Pos2::new(locate.right(), bottom))
        .constrain_to(area)
        .show(ctx, |ui| {
            let response =
                ui.add(Button::new("").min_size(Vec2::splat(size))).on_hover_text(state.basemap.switch_hint());
            if response.clicked() {
                actions.push(Action::SetBasemap(state.basemap.other()));
            }
            let color = ui.style().interact(&response).fg_stroke.color;
            layers(ui, response.rect, color, state.basemap == Basemap::Aerial, theme);
        });
}

/// The usual "layers" icon: a sheet seen at an angle above the edge of another. The top sheet is
/// filled while aerial imagery is shown, like the dot of the "Locate me" icon once a position is
/// known. Same line width as that icon.
fn layers(ui: &Ui, rect: Rect, color: Color32, aerial: bool, theme: &Theme) {
    let painter = ui.painter();
    let w = rect.width().min(rect.height()) * SHEET;
    let h = w * FLAT;
    let step = Vec2::new(0.0, h * STEP);
    let stroke = Stroke::new(theme.layout.spinner_stroke, color);

    let top = rect.center() - step;
    let sheet =
        vec![top - Vec2::new(w, 0.0), top - Vec2::new(0.0, h), top + Vec2::new(w, 0.0), top + Vec2::new(0.0, h)];
    painter.add(Shape::convex_polygon(sheet, if aerial { color } else { Color32::TRANSPARENT }, stroke));

    let below = rect.center() + step;
    painter.add(Shape::line(
        vec![below - Vec2::new(w, 0.0), below + Vec2::new(0.0, h), below + Vec2::new(w, 0.0)],
        stroke,
    ));
}
