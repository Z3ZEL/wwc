//! Global map filters: tags (all must match) and a tent capacity range. They apply to the
//! markers and to search, and stay on when the panel is closed.

use egui::Ui;

use crate::actions::Action;
use crate::state::{AppState, Remote, TENT_CAPACITY_MAX, tent_capacity_label};
use crate::ui::theme::Theme;
use crate::ui::widgets::{failed, loading, muted, primary_button, range_slider, section_title, tag_chip};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let filters = &state.filters;
    muted(ui, theme, "Filters apply to the map and to search.");

    ui.add_space(theme.spacing.section_gap);
    section_title(ui, "Tags");
    match &state.tags {
        Remote::Loaded(tags) if tags.is_empty() => {
            muted(ui, theme, "No tags yet.");
        }
        Remote::Loaded(tags) => {
            muted(ui, theme, "Campsites with all the selected tags.");
            ui.horizontal_wrapped(|ui| {
                for tag in tags {
                    if tag_chip(ui, theme, &tag.label, filters.tags.contains(&tag.id)).clicked() {
                        actions.push(Action::ToggleTagFilter(tag.id.clone()));
                    }
                }
            });
        }
        Remote::Failed(e) => {
            if failed(ui, theme, e) {
                actions.push(Action::LoadTags);
            }
        }
        Remote::Loading | Remote::NotAsked => loading(ui, theme, "Loading tags…"),
    }

    ui.add_space(theme.spacing.section_gap);
    section_title(ui, "Tent capacity");
    let (min, max) = (filters.min_tents, filters.max_tents);
    ui.label(if min == max {
        format!("{} tents", tent_capacity_label(min))
    } else {
        format!("{} to {} tents", tent_capacity_label(min), tent_capacity_label(max))
    });
    if let Some((min, max)) = range_slider(ui, theme, 1..=TENT_CAPACITY_MAX, min, max) {
        actions.push(Action::SetTentRange { min, max });
    }

    ui.add_space(theme.spacing.section_gap);
    if primary_button(ui, theme, "Reset filters", filters.is_active()).clicked() {
        actions.push(Action::ResetFilters);
    }
}
