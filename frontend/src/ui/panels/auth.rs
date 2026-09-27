//! Login and Register panels.

use egui::Ui;

use crate::actions::Action;
use crate::state::{AppState, Panel};
use crate::ui::theme::Theme;
use crate::ui::widgets::{field_error, form_error, link_button, muted, primary_button, submitted, text_field};

pub fn login(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let f = &mut state.login;
    if state.after_login.is_some() {
        muted(ui, theme, "Log in to continue.");
    }
    text_field(ui, "Email", &mut f.email, false);
    field_error(ui, theme, f.error.as_ref(), "identity");
    let pw = text_field(ui, "Password", &mut f.password, true);
    field_error(ui, theme, f.error.as_ref(), "password");
    form_error(ui, theme, f.error.as_ref());

    ui.add_space(theme.spacing.item_spacing[1]);
    let enabled = !f.submitting;
    if primary_button(ui, theme, if f.submitting { "Logging in…" } else { "Log in" }, enabled).clicked()
        || (enabled && submitted(ui, &pw))
    {
        actions.push(Action::Login);
    }

    ui.add_space(theme.spacing.section_gap);
    ui.horizontal(|ui| {
        muted(ui, theme, "No account yet?");
        if link_button(ui, theme, "Sign up").clicked() {
            actions.push(Action::OpenPanel(Panel::Register));
        }
    });
}

pub fn register(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let f = &mut state.register;
    text_field(ui, "Display name", &mut f.name, false);
    field_error(ui, theme, f.error.as_ref(), "name");
    text_field(ui, "Email", &mut f.email, false);
    field_error(ui, theme, f.error.as_ref(), "email");
    text_field(ui, "Password (8 characters min.)", &mut f.password, true);
    field_error(ui, theme, f.error.as_ref(), "password");
    let confirm = text_field(ui, "Confirm password", &mut f.confirm, true);
    field_error(ui, theme, f.error.as_ref(), "passwordConfirm");
    form_error(ui, theme, f.error.as_ref());

    ui.add_space(theme.spacing.item_spacing[1]);
    let enabled = !f.submitting;
    if primary_button(ui, theme, if f.submitting { "Creating account…" } else { "Create account" }, enabled).clicked()
        || (enabled && submitted(ui, &confirm))
    {
        actions.push(Action::Register);
    }

    ui.add_space(theme.spacing.section_gap);
    ui.horizontal(|ui| {
        muted(ui, theme, "Already have an account?");
        if link_button(ui, theme, "Log in").clicked() {
            actions.push(Action::OpenPanel(Panel::Login));
        }
    });
}
