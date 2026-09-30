use egui::{Ui, Vec2};

use crate::actions::Action;
use crate::documents::{self, PRIVACY};
use crate::state::{AppState, Panel};
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    field_error, form_error, link_button, muted, primary_button, section_title, text_field, verify_prompt,
};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let Some(session) = &state.session else { return };
    muted(ui, theme, format!("Signed in as {}", session.user.email));
    if !session.user.verified {
        verify_prompt(ui, theme, &session.user.email, "to post, rate and comment.", actions);
    }
    ui.add_space(theme.spacing.section_gap);

    let f = &mut state.profile;
    section_title(ui, "Public profile");
    text_field(ui, "Display name", &mut f.name, false);
    field_error(ui, theme, f.name_error.as_ref(), "name");
    form_error(ui, theme, f.name_error.as_ref());
    if primary_button(ui, theme, if f.saving_name { "Saving…" } else { "Save" }, !f.saving_name).clicked() {
        actions.push(Action::SaveProfileName);
    }

    ui.add_space(theme.spacing.section_gap);
    ui.separator();
    section_title(ui, "Change password");
    text_field(ui, "Current password", &mut f.old_password, true);
    field_error(ui, theme, f.password_error.as_ref(), "oldPassword");
    text_field(ui, "New password", &mut f.new_password, true);
    field_error(ui, theme, f.password_error.as_ref(), "password");
    text_field(ui, "Confirm new password", &mut f.confirm, true);
    field_error(ui, theme, f.password_error.as_ref(), "passwordConfirm");
    form_error(ui, theme, f.password_error.as_ref());
    let busy = f.changing_password;
    if primary_button(ui, theme, if busy { "Changing…" } else { "Change password" }, !busy).clicked() {
        actions.push(Action::ChangePassword);
    }

    ui.add_space(theme.spacing.section_gap);
    ui.separator();
    section_title(ui, "Your data");
    // No self-service export or deletion yet: the policy explains how to ask for them.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        muted(ui, theme, "To get a copy of your data or delete your account, see the ");
        let title = documents::manifest().get(PRIVACY).map_or("Privacy Policy", |d| d.title.as_str());
        if link_button(ui, theme, title).clicked() {
            actions.push(Action::OpenPanel(Panel::Document(PRIVACY.to_owned())));
        }
        muted(ui, theme, ".");
    });

    ui.add_space(theme.spacing.section_gap);
    ui.separator();
    if ui.button("Log out").clicked() {
        actions.push(Action::Logout);
    }
}
