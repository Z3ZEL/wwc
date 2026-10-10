//! The welcome card at the top-left of the map (ARCHITECTURE §5.6, ADR 0020): a "Welcome" tab
//! with the `welcome` document, and an "Updates" tab with the latest releases (`changelog`).
//! It floats over the map like the privacy notice, semi-transparent, and collapses to a small
//! button. Nothing about it is saved: it opens on wide screens and starts collapsed on phones.

use egui::{
    Align, Align2, Area, Button, Frame, Hyperlink, Id, Layout, Margin, Order, Rangef, Rect, RichText, ScrollArea,
    Stroke, Ui, Vec2,
};

use crate::actions::Action;
use crate::documents::{WELCOME, format_date};
use crate::state::{AppState, Remote, WelcomeTab};
use crate::ui::panels::document_body;
use crate::ui::theme::{Theme, subheading};
use crate::ui::widgets::{badge, failed, loading, markdown, muted, tabs};

/// `map`: the map area. `avoid`: a card at the bottom of the map (the privacy notice or the
/// consent panel) that this one stops above when they would overlap, as on phones.
/// Returns the card's rect while it is shown: the zoom buttons go on its right.
pub fn show(
    ctx: &egui::Context,
    map: Rect,
    avoid: Option<Rect>,
    state: &AppState,
    theme: &Theme,
    actions: &mut Vec<Action>,
) -> Option<Rect> {
    let margin = theme.spacing.map_button_margin;
    let pad = theme.panel_margin();
    let icon = theme.layout.icon_button_size;
    // Leave room for the zoom buttons on the right.
    let width = theme.layout.welcome_width.min(map.width() - 3.0 * margin - theme.map.zoom_button_size);
    let origin = map.left_top() + Vec2::splat(margin);
    let bottom = match avoid.filter(|card| card.x_range().intersects(Rangef::new(origin.x, origin.x + width))) {
        Some(card) => map.bottom().min(card.top()),
        None => map.bottom(),
    };
    let height = theme.layout.welcome_max_height.min(bottom - margin - origin.y);
    if width < 2.0 * icon + pad.sum().x || height < icon {
        return None;
    }
    let collapsed = state.welcome.is_collapsed(theme.is_narrow(ctx));
    let c = &theme.colors;

    let shown = Area::new(Id::new("welcome_card"))
        .order(Order::Middle)
        .pivot(Align2::LEFT_TOP)
        .fixed_pos(origin)
        .constrain_to(map)
        .show(ctx, |ui| {
            Frame::NONE
                .fill(c.welcome_bg)
                .stroke(Stroke::new(theme.shape.border_width, c.border))
                .corner_radius(theme.radius())
                .inner_margin(if collapsed { Margin::ZERO } else { pad })
                .show(ui, |ui| {
                    if collapsed {
                        let label = format!("▶ {}", state.welcome.tab.title());
                        if ui.add(Button::new(label).frame(false).min_size(Vec2::splat(icon))).clicked() {
                            actions.push(Action::CollapseWelcome(false));
                        }
                    } else {
                        expanded(ui, width - pad.sum().x, height - pad.sum().y, state, theme, actions);
                    }
                });
        });
    Some(shown.response.rect)
}

/// The tabs and the collapse button, then the open tab, scrolling within `height`.
fn expanded(ui: &mut Ui, width: f32, height: f32, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    ui.set_width(width.max(0.0));
    let tab = state.welcome.tab;
    ui.horizontal(|ui| {
        let labels = WelcomeTab::ALL.map(WelcomeTab::title);
        let selected = WelcomeTab::ALL.iter().position(|t| *t == tab).unwrap_or_default();
        if let Some(clicked) = tabs(ui, theme, &labels, selected).and_then(|i| WelcomeTab::ALL.get(i)) {
            actions.push(Action::WelcomeTab(*clicked));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let collapse = Button::new("◀").frame(false).min_size(Vec2::splat(theme.layout.icon_button_size));
            if ui.add(collapse).on_hover_text("Hide").clicked() {
                actions.push(Action::CollapseWelcome(true));
            }
        });
    });
    let body = height - ui.min_rect().height() - theme.spacing.item_spacing[1];
    ScrollArea::vertical().id_salt(tab.title()).max_height(body.max(0.0)).auto_shrink([false, true]).show(ui, |ui| {
        match tab {
            WelcomeTab::Welcome => document_body(ui, state, theme, WELCOME, actions),
            WelcomeTab::Updates => updates(ui, state, theme, actions),
        }
    });
}

/// The latest releases, newest first: tag, title, date, notes and a link to GitHub.
fn updates(ui: &mut Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let releases = match &state.changelog {
        Remote::Loaded(releases) => releases,
        Remote::Failed(e) => {
            if failed(ui, theme, e) {
                actions.push(Action::LoadChangelog);
            }
            return;
        }
        Remote::Loading | Remote::NotAsked => {
            loading(ui, theme, "Loading…");
            return;
        }
    };
    if releases.is_empty() {
        muted(ui, theme, "No updates yet.");
        return;
    }
    let c = &theme.colors;
    for (i, release) in releases.iter().enumerate() {
        if i > 0 {
            ui.add_space(theme.spacing.item_spacing[1]);
            ui.separator();
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&release.tag).text_style(subheading()).color(c.primary));
            if release.title != release.tag {
                ui.label(RichText::new(&release.title).text_style(subheading()));
            }
            if release.prerelease {
                badge(ui, theme, "Pre-release", c.tag_bg, c.tag_text);
            }
        });
        muted(ui, theme, format_date(&release.date).unwrap_or_else(|| release.date.clone()));
        if let Some(href) = markdown(ui, theme, &release.notes) {
            log::warn!("release {}: relative link {href:?} ignored", release.tag);
        }
        if let Some(url) = &release.url {
            ui.add(Hyperlink::from_label_and_url("View on GitHub", url).open_in_new_tab(true));
        }
    }
}
