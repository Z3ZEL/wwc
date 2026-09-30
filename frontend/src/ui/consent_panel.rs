//! Consent panel (ARCHITECTURE §5.12), only while `consent.json` enables consent. It floats at
//! the bottom of the map until the visitor chooses, and again when they open "Privacy choices"
//! from the footer. Refusing is as easy as accepting (same buttons, side by side), nothing is
//! ticked in advance, and the map stays usable without choosing: no choice means no consent.

use egui::{Align, Align2, Area, Button, Frame, Id, Layout, Order, Rect, RichText, Stroke, Vec2};

use crate::actions::{Action, ConsentChoice};
use crate::consent;
use crate::documents::{self, PRIVACY};
use crate::state::{AppState, Panel};
use crate::ui::theme::{Theme, subheading};
use crate::ui::widgets::{link_button, muted, primary_button};

const TEXT: &str = "With your consent, we would like to measure how the site is used, to improve it. \
                    It's optional: nothing is collected unless you accept, and you can change your choice \
                    at any time with \u{201c}Privacy choices\u{201d} at the bottom of the map.";

/// `area`: the part of the map the panel may cover (above the footer). Gets `&mut AppState`
/// only to edit the panel's own buffers (customize view, ticked purposes).
pub fn show(ctx: &egui::Context, area: Rect, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    if !state.consent_panel_open() || area.width() < 1.0 {
        return;
    }
    let config = consent::config();
    let c = &theme.colors;
    let gap = theme.spacing.item_spacing[1];
    let margin = theme.panel_margin();
    let width = area.width().min(theme.layout.notice_max_width) - 2.0 * gap - margin.sum().x;
    let panel = &mut state.consent_panel;

    Area::new(Id::new("consent_panel"))
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
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Cookies and analytics").text_style(subheading()));
                        if panel.reopened {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(Button::new("×").frame(false)).on_hover_text("Close").clicked() {
                                    actions.push(Action::CloseConsent);
                                }
                            });
                        }
                    });
                    ui.label(TEXT);

                    if panel.customizing {
                        ui.add_space(gap);
                        for purpose in &config.purposes {
                            let mut on = panel.choices.contains(&purpose.id);
                            if ui.checkbox(&mut on, RichText::new(&purpose.title).strong()).changed() {
                                if on {
                                    panel.choices.insert(purpose.id.clone());
                                } else {
                                    panel.choices.remove(&purpose.id);
                                }
                            }
                            ui.indent(("consent_purpose", &purpose.id), |ui| muted(ui, theme, &purpose.description));
                        }
                    }

                    ui.add_space(gap);
                    ui.horizontal_wrapped(|ui| {
                        // Same style for both: refusing must not be harder than accepting.
                        if primary_button(ui, theme, "Accept all", true).clicked() {
                            actions.push(Action::SaveConsent(ConsentChoice::AcceptAll));
                        }
                        if primary_button(ui, theme, "Reject all", true).clicked() {
                            actions.push(Action::SaveConsent(ConsentChoice::RejectAll));
                        }
                        if panel.customizing {
                            if ui.button("Save my choices").clicked() {
                                actions.push(Action::SaveConsent(ConsentChoice::Selected));
                            }
                        } else if link_button(ui, theme, "Customize").clicked() {
                            panel.customizing = true;
                        }
                        let policy = documents::manifest().get(PRIVACY).map_or("Privacy Policy", |d| d.title.as_str());
                        if link_button(ui, theme, policy).clicked() {
                            actions.push(Action::OpenPanel(Panel::Document(PRIVACY.to_owned())));
                        }
                    });
                });
        });
}
