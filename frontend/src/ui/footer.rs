//! Strip at the bottom-right of the map: links to the legal documents, "Privacy choices"
//! while consent is enabled, and the map data attribution OpenStreetMap requires
//! (ARCHITECTURE §5.6).

use egui::{Align2, Area, Frame, Hyperlink, Id, Link, Margin, Order, Rect, RichText, Vec2};

use crate::actions::Action;
use crate::state::Panel;
use crate::ui::theme::Theme;
use crate::{consent, documents};

const OSM_COPYRIGHT: &str = "https://www.openstreetmap.org/copyright";

/// Draws over the map rect and returns the strip's rect. The app doesn't call it while a
/// panel covers the map (narrow screens).
pub fn show(ctx: &egui::Context, map: Rect, theme: &Theme, actions: &mut Vec<Action>) -> Option<Rect> {
    if map.width() < 1.0 || map.height() < 1.0 {
        return None;
    }
    let c = &theme.map.colors;
    let [px, py] = theme.spacing.map_footer_padding;
    let small = |text: &str| RichText::new(text).small().color(c.attribution_text);
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
                    ui.add(
                        Hyperlink::from_label_and_url(small("© OpenStreetMap contributors"), OSM_COPYRIGHT)
                            .open_in_new_tab(true),
                    );
                });
            });
        });
    Some(area.response.rect)
}
