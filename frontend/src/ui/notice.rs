//! Privacy notice at the bottom of the map, until dismissed. It informs, it doesn't ask:
//! the app stores nothing that needs consent (only the login session and this notice's
//! state), so there is no cookie banner to accept or refuse (ARCHITECTURE §5.11).
//! It comes back once when the Privacy Policy's `updated` date changes.

use egui::{Align2, Area, Frame, Id, Order, Rect, RichText, Stroke, Vec2};

use crate::actions::Action;
use crate::documents::{self, PRIVACY};
use crate::state::{AppState, Panel};
use crate::ui::theme::{Theme, subheading};
use crate::ui::widgets::{link_button, primary_button};

const TEXT: &str = "No ads, no analytics, no tracking cookies. Your browser only keeps your login session \
                    and this notice's state. Map images are loaded from OpenStreetMap's servers.";

/// `area`: the part of the map the notice may cover (above the footer).
pub fn show(ctx: &egui::Context, area: Rect, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let Some(policy) = documents::manifest().get(PRIVACY) else { return };
    if state.notice_seen.as_deref() == Some(policy.updated.as_str()) || area.width() < 1.0 {
        return;
    }
    let c = &theme.colors;
    let gap = theme.spacing.item_spacing[1];
    let margin = theme.panel_margin();
    let width = area.width().min(theme.layout.notice_max_width) - 2.0 * gap - margin.sum().x;
    let title = match state.notice_seen {
        Some(_) => format!("Our {} changed on {}", policy.title, policy.updated_label()),
        None => "Your privacy".to_owned(),
    };

    Area::new(Id::new("privacy_notice"))
        .order(Order::Middle)
        .pivot(Align2::CENTER_BOTTOM)
        .fixed_pos(area.center_bottom() - Vec2::new(0.0, gap))
        .constrain_to(area)
        .show(ctx, |ui| {
            Frame::NONE
                .fill(c.surface)
                .stroke(Stroke::new(theme.shape.border_width, c.border))
                .corner_radius(theme.radius())
                .inner_margin(margin)
                .show(ui, |ui| {
                    ui.set_width(width.max(0.0));
                    ui.label(RichText::new(title).text_style(subheading()));
                    ui.label(TEXT);
                    ui.horizontal(|ui| {
                        if primary_button(ui, theme, "Got it", true).clicked() {
                            actions.push(Action::DismissNotice);
                        }
                        if link_button(ui, theme, &format!("Read the {}", policy.title)).clicked() {
                            actions.push(Action::OpenPanel(Panel::Document(PRIVACY.to_owned())));
                        }
                    });
                });
        });
}
