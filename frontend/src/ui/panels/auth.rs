//! Login and Register panels.

use egui::{Checkbox, Label, RichText, Sense, Ui, Vec2};

use crate::actions::Action;
use crate::documents::{self, PRIVACY, TERMS};
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

    ui.add_space(theme.spacing.item_spacing[1]);
    consent(ui, theme, &mut f.accepted_terms, actions);
    field_error(ui, theme, f.error.as_ref(), "terms");
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

/// Age and Terms of Use checkbox, and where to read about personal data. The Privacy Policy
/// is information, not something to accept: the account itself is the legal basis.
fn consent(ui: &mut Ui, theme: &Theme, accepted: &mut bool, actions: &mut Vec<Action>) {
    let doc_link = |ui: &mut Ui, id: &str, actions: &mut Vec<Action>| {
        let title = documents::manifest().get(id).map_or(id, |d| d.title.as_str());
        let text = RichText::new(title).color(theme.colors.link).underline();
        if ui.link(text).clicked() {
            actions.push(Action::OpenPanel(Panel::Document(id.to_owned())));
        }
    };
    let age = documents::manifest().var("min_age");
    ui.horizontal_top(|ui| {
        ui.add(Checkbox::without_text(accepted));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let lead = match age {
                Some(age) => format!("I am at least {age} years old and I accept the "),
                None => "I accept the ".to_owned(),
            };
            // The text toggles the box too, like a <label>.
            if ui.add(Label::new(lead).sense(Sense::click())).clicked() {
                *accepted = !*accepted;
            }
            doc_link(ui, TERMS, actions);
            ui.label(".");
        });
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        muted(ui, theme, "We use your data as described in the ");
        doc_link(ui, PRIVACY, actions);
        muted(ui, theme, ".");
    });
}
