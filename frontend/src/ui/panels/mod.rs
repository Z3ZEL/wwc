//! Side panel pages (ARCHITECTURE §5.6). Each takes the state (to edit its own form
//! buffers) and pushes `Action`s for anything else.

mod auth;
mod campsite_form;
mod campsite_view;
mod filters;
mod profile;
mod report;
mod search;

use egui::{RichText, Ui};

use crate::actions::Action;
use crate::state::{AppState, Panel};
use crate::ui::theme::Theme;
use crate::ui::widgets::{danger_button, panel_frame};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let Some(panel) = state.panel.clone() else { return };
    panel_frame(ui, theme, panel.title(), state.panel_collapsed, actions, |ui, actions| {
        if state.confirm_discard.is_some() {
            discard_prompt(ui, theme, actions);
            return;
        }
        match panel {
            Panel::Login => auth::login(ui, state, theme, actions),
            Panel::Register => auth::register(ui, state, theme, actions),
            Panel::Profile => profile::show(ui, state, theme, actions),
            Panel::NewCampsite | Panel::EditCampsite(_) => campsite_form::show(ui, state, theme, actions),
            Panel::Campsite(_) => campsite_view::show(ui, state, theme, actions),
            Panel::Search => search::show(ui, state, theme, actions),
            Panel::Filters => filters::show(ui, state, theme, actions),
            Panel::Report(target) => report::show(ui, state, theme, &target, actions),
        }
    });
}

fn discard_prompt(ui: &mut Ui, theme: &Theme, actions: &mut Vec<Action>) {
    ui.label(RichText::new("You have unsaved changes. Discard them?").strong());
    ui.horizontal(|ui| {
        if danger_button(ui, theme, "Discard", true).clicked() {
            actions.push(Action::DiscardChanges);
        }
        if ui.button("Keep editing").clicked() {
            actions.push(Action::KeepEditing);
        }
    });
}
