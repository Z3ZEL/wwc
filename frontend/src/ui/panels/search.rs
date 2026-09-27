//! Search results: campsites whose name matches the top bar query (global filters apply).

use egui::{Button, RichText, Ui, Vec2};

use crate::actions::Action;
use crate::state::{AppState, Panel, Remote};
use crate::ui::theme::Theme;
use crate::ui::widgets::{failed, link_button, loading_block, muted};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let search = &state.search;
    if !search.is_active() {
        muted(ui, theme, "Type a campsite name in the search bar and press Enter.");
        return;
    }
    ui.label(RichText::new(format!("Names containing \u{201c}{}\u{201d}", search.submitted)).strong());
    filters_note(ui, state, theme, actions);
    ui.add_space(theme.spacing.item_spacing[1]);

    match &search.results {
        Remote::Loaded(results) if results.is_empty() => {
            muted(ui, theme, "No campsite matches.");
        }
        Remote::Loaded(results) => {
            let shown = results.len();
            let summary = match usize::try_from(search.total) {
                Ok(total) if total > shown => format!("Showing {shown} of {total}. Refine your search to see more."),
                _ => format!("{shown} campsite{}", if shown == 1 { "" } else { "s" }),
            };
            muted(ui, theme, summary);
            let selected = state.selected_campsite();
            let width = ui.available_width();
            for c in results {
                let row = Button::selectable(selected == Some(c.id.as_str()), c.title.as_str())
                    .right_text("")
                    .truncate()
                    .min_size(Vec2::new(width, 0.0));
                if ui.add(row).clicked() {
                    actions.push(Action::FocusCampsite { id: c.id.clone(), lat: c.lat, lng: c.lng });
                }
            }
        }
        Remote::Failed(e) => {
            if failed(ui, theme, e) {
                actions.push(Action::Search);
            }
        }
        Remote::Loading | Remote::NotAsked => loading_block(ui, theme, "Searching…"),
    }

    ui.add_space(theme.spacing.section_gap);
    if link_button(ui, theme, "Clear search").clicked() {
        actions.push(Action::ClearSearch);
    }
}

fn filters_note(ui: &mut Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let n = state.filters.active_count();
    if n == 0 {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        muted(ui, theme, format!("{n} filter{} applied.", if n == 1 { "" } else { "s" }));
        if link_button(ui, theme, "Edit filters").clicked() {
            actions.push(Action::OpenPanel(Panel::Filters));
        }
    });
}
