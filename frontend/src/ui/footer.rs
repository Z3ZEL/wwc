//! Strip at the bottom-right of the map: links to the legal documents, "Privacy choices"
//! while consent is enabled, and the attribution the map's tiles require: OpenStreetMap's, or
//! Mapbox's wordmark and links (ARCHITECTURE §5.6, ADR 0024).

use egui::{Align2, Area, Frame, Hyperlink, Id, Image, Link, Margin, Order, Rect, RichText, Vec2};

use crate::actions::Action;
use crate::map::Provider;
use crate::state::{AppState, Panel};
use crate::ui::theme::Theme;
use crate::{consent, documents};

/// Width / height of the wordmark (its SVG view box).
const MAPBOX_LOGO_RATIO: f32 = 800.0 / 180.0;

/// Draws over the map rect and returns the strip's rect. The app doesn't call it while a
/// panel covers the map (narrow screens).
pub fn show(
    ctx: &egui::Context,
    map: Rect,
    state: &AppState,
    theme: &Theme,
    actions: &mut Vec<Action>,
) -> Option<Rect> {
    if map.width() < 1.0 || map.height() < 1.0 {
        return None;
    }
    let c = &theme.map.colors;
    let [px, py] = theme.spacing.map_footer_padding;
    let small = |text: &str| RichText::new(text).small().color(c.attribution_text);
    let provider = Provider::configured();
    let area = Area::new(Id::new("map_footer"))
        .order(Order::Middle)
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(map.right_bottom())
        .constrain_to(map)
        .show(ctx, |ui| {
            Frame::NONE.fill(c.attribution_bg).inner_margin(Margin::symmetric(px as i8, py as i8)).show(ui, |ui| {
                ui.set_max_width(map.width() - 2.0 * px);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(px, 0.0);
                    if provider.has_wordmark() {
                        let height = theme.map.mapbox_logo_height;
                        let logo = Image::new(egui::include_image!("../../assets/map/mapbox-logo.svg"))
                            .fit_to_exact_size(Vec2::new(height * MAPBOX_LOGO_RATIO, height));
                        ui.add(logo).on_hover_text("Map by Mapbox");
                    }
                    for doc in documents::manifest().footer() {
                        if ui.add(Link::new(small(doc.short_title()))).on_hover_text(&doc.title).clicked() {
                            actions.push(Action::OpenPanel(Panel::Document(doc.id.clone())));
                        }
                        ui.label(small("·"));
                    }
                    // Withdrawing consent must be as easy as giving it (§5.12).
                    if consent::config().enabled {
                        let link = ui.add(Link::new(small("Privacy choices"))).on_hover_text("Change what you accept");
                        if link.clicked() {
                            actions.push(Action::OpenConsent);
                        }
                        ui.label(small("·"));
                    }
                    for credit in provider.credits(state.basemap) {
                        ui.add(Hyperlink::from_label_and_url(small(credit.text), credit.url).open_in_new_tab(true));
                    }
                });
            });
        });
    Some(area.response.rect)
}
