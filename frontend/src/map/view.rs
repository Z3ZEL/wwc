//! The full-screen map: tiles (walkers + OpenStreetMap), campsite markers and clusters,
//! the draft pin of the campsite form, and viewport → bbox reporting (ARCHITECTURE §5.5).

use egui::{Align2, Button, FontId, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2};
use walkers::sources::OpenStreetMap;
use walkers::{HttpTiles, Map, MapMemory, Position, Projector, lat_lon};

use super::cluster::{cluster, cluster_radius};
use crate::actions::Action;
use crate::api::BBox;
use crate::state::AppState;
use crate::ui::theme::Theme;

/// Wait this long after the last pan/zoom before querying the new viewport.
const VIEWPORT_DEBOUNCE_SECS: f64 = 0.3;
/// Extra click tolerance around markers, in points.
const HIT_SLOP: f32 = 4.0;

pub struct MapView {
    tiles: HttpTiles,
    memory: MapMemory,
    home: Position,
    /// Last bbox sent to the controller, and a changed bbox waiting for the debounce.
    reported: Option<BBox>,
    pending: Option<(BBox, f64)>,
}

impl MapView {
    pub fn new(ctx: &egui::Context, theme: &Theme) -> Self {
        let mut memory = MapMemory::default();
        if let Err(e) = memory.set_zoom(theme.map.default_zoom) {
            log::warn!("invalid default zoom: {e:?}");
        }
        Self {
            tiles: HttpTiles::new(OpenStreetMap, ctx.clone()),
            memory,
            home: lat_lon(theme.map.default_center.lat, theme.map.default_center.lon),
            reported: None,
            pending: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
        if let Some((lat, lng)) = state.map_focus.take() {
            self.memory.center_at(lat_lon(lat, lng));
            if self.memory.zoom() < theme.map.focus_zoom {
                let _ = self.memory.set_zoom(theme.map.focus_zoom);
            }
        }

        let map = Map::new(Some(&mut self.tiles), &mut self.memory, self.home).double_click_to_zoom(true);
        let (bbox, center, rect, zoom_into) = map
            .show(ui, |ui, response, projector, memory| {
                let zoom_into = draw_markers(ui, response, projector, memory.zoom(), state, theme, actions);
                attribution(ui, response.rect, theme);
                let center = projector.unproject(response.rect.center().to_vec2());
                (viewport(projector, response.rect), center, response.rect, zoom_into)
            })
            .inner;

        if let Some(pos) = zoom_into {
            self.memory.center_at(pos);
            let _ = self.memory.set_zoom((self.memory.zoom() + theme.map.cluster_zoom_step).min(theme.map.max_zoom));
        }
        state.map_center = (center.y(), center.x());
        self.zoom_buttons(ui, rect, theme);
        self.report_viewport(ui, bbox, actions);
    }

    fn zoom_buttons(&mut self, ui: &mut Ui, rect: Rect, theme: &Theme) {
        let size = Vec2::splat(28.0);
        let origin = rect.left_top() + Vec2::splat(12.0);
        let zoom_in = ui.put(Rect::from_min_size(origin, size), Button::new(RichText::new("+").strong()));
        let zoom_out = ui.put(
            Rect::from_min_size(origin + Vec2::new(0.0, size.y + 4.0), size),
            Button::new(RichText::new("−").strong()),
        );
        if zoom_in.clicked() && self.memory.zoom() < theme.map.max_zoom {
            let _ = self.memory.zoom_in();
        }
        if zoom_out.clicked() {
            let _ = self.memory.zoom_out();
        }
        if self.memory.zoom() > theme.map.max_zoom {
            let _ = self.memory.set_zoom(theme.map.max_zoom);
        }
    }

    /// Emit `ViewportChanged` once the map has been still for `VIEWPORT_DEBOUNCE_SECS`.
    fn report_viewport(&mut self, ui: &Ui, bbox: BBox, actions: &mut Vec<Action>) {
        let now = ui.input(|i| i.time);
        if Some(bbox) != self.reported && self.pending.is_none_or(|(b, _)| b != bbox) {
            self.pending = Some((bbox, now));
        }
        if let Some((b, since)) = self.pending {
            if now - since >= VIEWPORT_DEBOUNCE_SECS && !self.memory.animating() {
                self.reported = Some(b);
                self.pending = None;
                actions.push(Action::ViewportChanged(b));
            } else {
                ui.ctx().request_repaint_after_secs(VIEWPORT_DEBOUNCE_SECS as f32);
            }
        }
    }
}

fn viewport(projector: &Projector, rect: Rect) -> BBox {
    let nw = projector.unproject(rect.left_top().to_vec2());
    let se = projector.unproject(rect.right_bottom().to_vec2());
    // Rounded so tiny sub-pixel jitter doesn't count as a viewport change.
    let r = |v: f64| (v * 1e5).round() / 1e5;
    BBox { north: r(nw.y()), west: r(nw.x()), south: r(se.y()), east: r(se.x()) }
}

/// What is under the pointer: one campsite (index into `state.markers`) or a cluster.
#[derive(Clone, Copy)]
enum Hit {
    Campsite(usize),
    Cluster { count: usize, center: Pos2 },
}

/// Draws campsites, clustered below `cluster_max_zoom`, and handles clicks. Returns the
/// position to zoom into when a cluster was clicked.
fn draw_markers(
    ui: &mut Ui,
    response: &Response,
    projector: &Projector,
    zoom: f64,
    state: &AppState,
    theme: &Theme,
    actions: &mut Vec<Action>,
) -> Option<Position> {
    let m = &theme.map;
    let painter = ui.painter().with_clip_rect(response.rect);
    let selected = state.selected_campsite();
    let me = state.user_id();
    let outline = Stroke::new(m.marker_outline_width, m.colors.marker_outline);

    // On-screen campsites. The selected one is never hidden in a cluster: it is drawn alone, on top.
    let visible = response.rect.expand(m.cluster_radius_max);
    let mut on_screen: Vec<(usize, Pos2)> = Vec::new();
    let mut selected_marker = None;
    for (i, c) in state.markers.iter().enumerate() {
        let pos = projector.project(lat_lon(c.lat, c.lng)).to_pos2();
        if !visible.contains(pos) {
            continue;
        }
        if selected == Some(c.id.as_str()) {
            selected_marker = Some((i, pos));
        } else {
            on_screen.push((i, pos));
        }
    }
    let distance = if zoom >= m.cluster_max_zoom { 0.0 } else { m.cluster_distance };
    let points: Vec<Pos2> = on_screen.iter().map(|&(_, p)| p).collect();

    // (hit, position, radius) of everything drawn, for hover detection.
    let mut drawn: Vec<(Hit, Pos2, f32)> = Vec::new();
    for group in cluster(&points, distance) {
        if let [only] = group.members[..] {
            let (i, pos) = on_screen[only];
            let color =
                if me == Some(state.markers[i].author.as_str()) { m.colors.marker_own } else { m.colors.marker };
            painter.circle(pos, m.marker_radius, color, outline);
            drawn.push((Hit::Campsite(i), pos, m.marker_radius));
        } else {
            let count = group.members.len();
            let radius = cluster_radius(count, m.cluster_radius, m.cluster_radius_max);
            painter.circle(group.center, radius, m.colors.cluster, outline);
            painter.text(
                group.center,
                Align2::CENTER_CENTER,
                count.to_string(),
                FontId::proportional(theme.typography.size_small),
                m.colors.cluster_text,
            );
            drawn.push((Hit::Cluster { count, center: group.center }, group.center, radius));
        }
    }
    if let Some((i, pos)) = selected_marker {
        painter.circle(pos, m.marker_radius_selected, m.colors.marker_selected, outline);
        drawn.push((Hit::Campsite(i), pos, m.marker_radius_selected));
    }

    // Draft pin of the campsite form.
    if let (true, Some(lat), Some(lng)) =
        (state.is_campsite_form_open(), state.campsite_form.lat, state.campsite_form.lng)
    {
        let pos = projector.project(lat_lon(lat, lng)).to_pos2();
        painter.circle(pos, m.marker_radius_selected, m.colors.marker_draft, outline);
        painter.line_segment([pos, pos + Vec2::new(0.0, m.marker_radius_selected * 2.0)], outline);
    }

    // The closest marker under the pointer; later (top-most) ones win ties.
    let hovered = response.hover_pos().and_then(|p| {
        drawn
            .iter()
            .filter(|(_, pos, radius)| p.distance(*pos) <= radius + HIT_SLOP)
            .min_by(|a, b| p.distance(a.1).total_cmp(&p.distance(b.1)))
            .map(|&(hit, pos, radius)| (hit, pos, radius))
    });

    if let Some((hit, pos, radius)) = hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let text = match hit {
            Hit::Campsite(i) => state.markers[i].title.clone(),
            Hit::Cluster { count, .. } => format!("{count} campsites · click to zoom in"),
        };
        label(&painter, theme, pos + Vec2::new(radius + 4.0, 0.0), &text);
    }

    if response.clicked() {
        match hovered.filter(|_| !state.is_campsite_form_open()) {
            Some((Hit::Campsite(i), ..)) => {
                actions.push(Action::OpenPanel(crate::state::Panel::Campsite(state.markers[i].id.clone())));
            }
            Some((Hit::Cluster { center, .. }, ..)) => return Some(projector.unproject(center.to_vec2())),
            None => {
                if let Some(p) = response.interact_pointer_pos() {
                    let pos = projector.unproject(p.to_vec2());
                    actions.push(Action::MapClicked { lat: pos.y(), lng: pos.x() });
                }
            }
        }
    }
    None
}

fn label(painter: &egui::Painter, theme: &Theme, at: Pos2, text: &str) {
    let c = &theme.map.colors;
    let galley =
        painter.layout_no_wrap(text.to_owned(), FontId::proportional(theme.typography.size_small), c.label_text);
    let rect = Align2::LEFT_CENTER.anchor_size(at, galley.size()).expand(4.0);
    painter.rect_filled(rect, theme.radius(), c.label_bg);
    painter.galley(rect.min + Vec2::splat(4.0), galley, c.label_text);
}

fn attribution(ui: &mut Ui, map_rect: Rect, theme: &Theme) {
    let c = &theme.map.colors;
    let size = Vec2::new(190.0, 18.0);
    let rect = Rect::from_min_size(map_rect.right_bottom() - size, size);
    ui.painter().rect_filled(rect, 0.0, c.attribution_bg);
    ui.put(
        rect,
        egui::Hyperlink::from_label_and_url(
            RichText::new("© OpenStreetMap contributors").small().color(c.attribution_text),
            "https://www.openstreetmap.org/copyright",
        ),
    );
}
