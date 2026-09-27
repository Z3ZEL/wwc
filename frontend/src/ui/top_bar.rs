use egui::{Align, Button, Layout, Margin, Panel, RichText, Stroke, TextEdit, Ui};

use crate::actions::Action;
use crate::api::SEARCH_MAX_CHARS;
use crate::state::{AppState, Panel as SidePanel, Remote};
use crate::ui::theme::Theme;
use crate::ui::widgets::{Spinner, SpinnerSize, submitted};

/// Gets `&mut AppState` only to edit the search field buffer; everything else is an `Action`.
pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let c = &theme.colors;
    let [px, py] = theme.spacing.top_bar_padding;
    let frame = egui::Frame::NONE.fill(c.top_bar_bg).inner_margin(Margin::symmetric(px as i8, py as i8));
    let narrow = ui.ctx().content_rect().width() < theme.layout.narrow_breakpoint;

    Panel::top("top_bar").exact_size(theme.layout.top_bar_height).frame(frame).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            let logo = RichText::new("⛺ WWC").heading().strong().color(c.top_bar_text);
            if ui.add(Button::new(logo).frame(false)).on_hover_text("Close panel").clicked() {
                actions.push(Action::ClosePanel);
            }
            ui.add_space(theme.spacing.section_gap);
            // No track on the dark bar: the theme's track color is meant for panels.
            let bar_spinner = Spinner::new(SpinnerSize::Inline).color(c.top_bar_text_muted).track(None);
            let count = match &state.campsite_count {
                Remote::Loaded(n) => format!("{} campsite{}", group_thousands(*n), if *n == 1 { "" } else { "s" }),
                Remote::Failed(_) => "— campsites".to_owned(),
                Remote::Loading | Remote::NotAsked => {
                    bar_spinner.show(ui, theme);
                    "campsites".to_owned()
                }
            };
            ui.label(RichText::new(count).color(c.top_bar_text_muted));
            // Always takes its space, so the search field doesn't jump when it comes and goes.
            let markers = bar_spinner.visible(state.markers_loading).show(ui, theme);
            if state.markers_loading {
                markers.on_hover_text("Loading campsites on the map…");
            }
            ui.add_space(theme.spacing.section_gap);
            search_and_filters(ui, state, theme, narrow, actions);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Unfilled buttons blend into the bar.
                let w = &mut ui.visuals_mut().widgets;
                for state in [&mut w.inactive, &mut w.open] {
                    state.weak_bg_fill = egui::Color32::TRANSPARENT;
                    state.bg_stroke = Stroke::NONE;
                }
                let on_bar = |text: &str| RichText::new(text).color(c.top_bar_text);
                match &state.session {
                    Some(session) => {
                        ui.menu_button(on_bar(session.user.display_name()), |ui| {
                            if ui.button("Profile").clicked() {
                                actions.push(Action::OpenPanel(SidePanel::Profile));
                            }
                            if ui.button("Log out").clicked() {
                                actions.push(Action::Logout);
                            }
                        });
                        let label = if narrow { "+" } else { "+ New campsite" };
                        let new =
                            Button::new(RichText::new(label).color(c.on_accent)).fill(c.accent).stroke(Stroke::NONE);
                        if ui.add(new).on_hover_text("Add a campsite").clicked() {
                            actions.push(Action::OpenPanel(SidePanel::NewCampsite));
                        }
                    }
                    None => {
                        let signup = Button::new(RichText::new("Sign up").color(c.on_primary))
                            .fill(c.primary)
                            .stroke(Stroke::NONE);
                        if ui.add(signup).clicked() {
                            actions.push(Action::OpenPanel(SidePanel::Register));
                        }
                        if ui.add(Button::new(on_bar("Log in")).frame(false)).clicked() {
                            actions.push(Action::OpenPanel(SidePanel::Login));
                        }
                    }
                }
            });
        });
    });
}

fn search_and_filters(ui: &mut Ui, state: &mut AppState, theme: &Theme, narrow: bool, actions: &mut Vec<Action>) {
    let c = &theme.colors;
    let width = if narrow { theme.layout.search_width_narrow } else { theme.layout.search_width };
    let search = &mut state.search;
    let field = ui.add(
        TextEdit::singleline(&mut search.query)
            .hint_text("Search campsites…")
            .char_limit(SEARCH_MAX_CHARS)
            .desired_width(width),
    );
    if submitted(ui, &field) {
        actions.push(Action::Search);
    }
    if (!search.query.is_empty() || search.is_active())
        && ui
            .add(Button::new(RichText::new("×").color(c.top_bar_text)).frame(false))
            .on_hover_text("Clear search")
            .clicked()
    {
        actions.push(Action::ClearSearch);
    }

    let active = state.filters.active_count();
    let label = if active > 0 { format!("Filters ({active})") } else { "Filters".to_owned() };
    let (fg, bg) = if active > 0 {
        (c.tag_selected_text, c.tag_selected_bg)
    } else {
        (c.top_bar_text, egui::Color32::TRANSPARENT)
    };
    let button = Button::new(RichText::new(label).color(fg)).fill(bg).stroke(Stroke::NONE);
    if ui.add(button).on_hover_text("Filter campsites by tags and tent capacity").clicked() {
        let open = state.panel == Some(SidePanel::Filters) && !state.panel_collapsed;
        actions.push(if open { Action::ClosePanel } else { Action::OpenPanel(SidePanel::Filters) });
    }
}

/// 1284 → "1 284".
fn group_thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    #[test]
    fn thousands() {
        assert_eq!(super::group_thousands(0), "0");
        assert_eq!(super::group_thousands(999), "999");
        assert_eq!(super::group_thousands(1284), "1 284");
        assert_eq!(super::group_thousands(1_000_000), "1 000 000");
    }
}
