use egui::{Align, Button, Layout, Margin, Panel, RichText, Stroke, TextEdit, Ui};

use crate::actions::Action;
use crate::api::SEARCH_MAX_CHARS;
use crate::state::{AppState, Panel as SidePanel, Remote};
use crate::ui::theme::Theme;
use crate::ui::widgets::{Spinner, SpinnerSize, submitted, text_width};

/// Wide: one row. Narrow: logo, count and a ☰ account menu, then a second row with the
/// search field and Filters (ARCHITECTURE §5.6).
///
/// Gets `&mut AppState` only to edit the search field buffer; everything else is an `Action`.
pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let [px, py] = theme.spacing.top_bar_padding;
    let frame = egui::Frame::NONE.fill(theme.colors.top_bar_bg).inner_margin(Margin::symmetric(px as i8, py as i8));
    let layout = &theme.layout;
    let narrow = theme.is_narrow(ui.ctx());
    let height = if narrow { layout.top_bar_height_narrow } else { layout.top_bar_height };

    Panel::top("top_bar").exact_size(height).frame(frame).show(ui, |ui| {
        if narrow {
            ui.spacing_mut().item_spacing.y = 0.0;
            let row = egui::vec2(ui.available_width(), layout.top_bar_height);
            ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                brand(ui, state, theme, actions);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| account_menu(ui, state, theme, actions));
            });
            ui.allocate_ui_with_layout(ui.available_size(), Layout::left_to_right(Align::Center), |ui| {
                search_and_filters(ui, state, theme, f32::INFINITY, actions);
            });
            return;
        }
        ui.horizontal_centered(|ui| {
            brand(ui, state, theme, actions);
            ui.add_space(theme.spacing.section_gap);
            // Account buttons first, so the search field only gets the space left between.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                account_buttons(ui, state, theme, actions);
                ui.add_space(theme.spacing.section_gap);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    search_and_filters(ui, state, theme, layout.search_width, actions);
                });
            });
        });
    });
}

/// Logo (closes the panel), campsite count and the markers spinner.
fn brand(ui: &mut Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let c = &theme.colors;
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
}

/// Unfilled buttons (and menu buttons) blend into the bar.
fn blend_into_bar(ui: &mut Ui) {
    let w = &mut ui.visuals_mut().widgets;
    for state in [&mut w.inactive, &mut w.open] {
        state.weak_bg_fill = egui::Color32::TRANSPARENT;
        state.bg_stroke = Stroke::NONE;
    }
}

/// Wide screens, right to left: the name menu and + New campsite, or Sign up and Log in.
fn account_buttons(ui: &mut Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let c = &theme.colors;
    ui.scope(|ui| {
        blend_into_bar(ui);
        let on_bar = |text: &str| RichText::new(text).color(c.top_bar_text);
        match &state.session {
            Some(session) => {
                ui.menu_button(on_bar(session.user.display_name()), |ui| account_items(ui, false, actions));
                let new =
                    Button::new(RichText::new("+ New campsite").color(c.on_accent)).fill(c.accent).stroke(Stroke::NONE);
                if ui.add(new).on_hover_text("Add a campsite").clicked() {
                    actions.push(Action::OpenPanel(SidePanel::NewCampsite));
                }
            }
            None => {
                let signup =
                    Button::new(RichText::new("Sign up").color(c.on_primary)).fill(c.primary).stroke(Stroke::NONE);
                if ui.add(signup).clicked() {
                    actions.push(Action::OpenPanel(SidePanel::Register));
                }
                if ui.add(Button::new(on_bar("Log in")).frame(false)).clicked() {
                    actions.push(Action::OpenPanel(SidePanel::Login));
                }
            }
        }
    });
}

/// Narrow screens: every account button goes into one ☰ menu.
fn account_menu(ui: &mut Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    blend_into_bar(ui);
    let icon = RichText::new("☰").color(theme.colors.top_bar_text).size(theme.typography.size_heading);
    ui.menu_button(icon, |ui| match &state.session {
        Some(session) => {
            ui.label(RichText::new(session.user.display_name()).strong());
            account_items(ui, true, actions);
        }
        None => {
            if ui.button("Log in").clicked() {
                actions.push(Action::OpenPanel(SidePanel::Login));
            }
            if ui.button("Sign up").clicked() {
                actions.push(Action::OpenPanel(SidePanel::Register));
            }
        }
    });
}

fn account_items(ui: &mut Ui, with_new: bool, actions: &mut Vec<Action>) {
    if with_new && ui.button("+ New campsite").clicked() {
        actions.push(Action::OpenPanel(SidePanel::NewCampsite));
    }
    if ui.button("Profile").clicked() {
        actions.push(Action::OpenPanel(SidePanel::Profile));
    }
    if ui.button("Log out").clicked() {
        actions.push(Action::Logout);
    }
}

/// Search field, × (clear) and Filters. The field takes the width the buttons leave,
/// up to `max_width`.
fn search_and_filters(ui: &mut Ui, state: &mut AppState, theme: &Theme, max_width: f32, actions: &mut Vec<Action>) {
    let c = &theme.colors;
    let active = state.filters.active_count();
    let label = if active > 0 { format!("Filters ({active})") } else { "Filters".to_owned() };
    let (fg, bg) = if active > 0 {
        (c.tag_selected_text, c.tag_selected_bg)
    } else {
        (c.top_bar_text, egui::Color32::TRANSPARENT)
    };
    let filters = RichText::new(label).color(fg);
    let clear = RichText::new("×").color(c.top_bar_text);
    // The × slot is reserved even when hidden, so the field keeps its width while typing.
    let padding = 2.0 * ui.spacing().button_padding.x;
    let reserved = text_width(ui, &filters) + padding + text_width(ui, &clear) + 2.0 * ui.spacing().item_spacing.x;
    let width = (ui.available_width() - reserved).min(max_width);

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
    let show_clear = !search.query.is_empty() || search.is_active();
    if ui.add_visible(show_clear, Button::new(clear).frame(false)).on_hover_text("Clear search").clicked() {
        actions.push(Action::ClearSearch);
    }

    let button = Button::new(filters).fill(bg).stroke(Stroke::NONE);
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
