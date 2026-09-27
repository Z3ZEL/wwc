//! Report panel: flag a campsite or a comment for admin review (ARCHITECTURE §5.8).
//! Reports are read in the PocketBase dashboard for now.

use egui::{CursorIcon, RichText, TextEdit, Ui};

use crate::actions::Action;
use crate::api::models::{ReportReason, ReportTarget};
use crate::state::{AppState, Panel, REPORT_DETAILS_MAX};
use crate::ui::theme::Theme;
use crate::ui::widgets::{danger_button, field_error, form_error, link_button, muted, section_title};

/// Longest comment excerpt shown as the preview.
const PREVIEW_CHARS: usize = 120;

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, target: &ReportTarget, actions: &mut Vec<Action>) {
    preview(ui, state, theme, target);
    ui.add_space(theme.spacing.section_gap);

    let f = &mut state.report;
    section_title(ui, "Why are you reporting it?");
    for &reason in ReportReason::for_target(target) {
        if ui.radio(f.reason == Some(reason), reason.label()).on_hover_cursor(CursorIcon::PointingHand).clicked() {
            f.reason = Some(reason);
        }
    }
    field_error(ui, theme, f.error.as_ref(), "reason");

    ui.add_space(theme.spacing.item_spacing[1]);
    ui.label(if f.reason == Some(ReportReason::Other) { "Details" } else { "Details (optional)" });
    ui.add(
        TextEdit::multiline(&mut f.details)
            .hint_text("Anything that helps an admin understand the problem…")
            .desired_rows(4)
            .char_limit(REPORT_DETAILS_MAX)
            .desired_width(f32::INFINITY),
    );
    field_error(ui, theme, f.error.as_ref(), "details");
    form_error(ui, theme, f.error.as_ref());

    ui.add_space(theme.spacing.item_spacing[1]);
    ui.horizontal(|ui| {
        if danger_button(ui, theme, if f.sending { "Sending…" } else { "Send report" }, !f.sending).clicked() {
            actions.push(Action::SubmitReport);
        }
        if link_button(ui, theme, "Cancel").clicked() {
            actions.push(if f.campsite_id.is_empty() {
                Action::ClosePanel
            } else {
                Action::OpenPanel(Panel::Campsite(f.campsite_id.clone()))
            });
        }
    });
}

/// What is being reported, from the campsite that was open.
fn preview(ui: &mut Ui, state: &AppState, theme: &Theme, target: &ReportTarget) {
    let detail = state.detail.as_ref();
    match target {
        ReportTarget::Campsite(id) => {
            muted(ui, theme, "You are reporting the campsite");
            let title = detail.filter(|d| &d.id == id).and_then(|d| d.campsite.loaded()).map(|c| c.title.as_str());
            ui.label(RichText::new(title.unwrap_or("this campsite")).strong());
        }
        ReportTarget::Comment(id) => {
            let comment = detail.and_then(|d| d.comments.iter().find(|c| &c.id == id));
            match comment {
                Some(c) => {
                    muted(ui, theme, format!("You are reporting a comment by {}", c.author_name()));
                    ui.label(RichText::new(excerpt(&c.body)).italics());
                }
                None => {
                    muted(ui, theme, "You are reporting a comment.");
                }
            }
        }
    }
}

fn excerpt(body: &str) -> String {
    let body = body.trim();
    match body.char_indices().nth(PREVIEW_CHARS) {
        Some((i, _)) => format!("{}…", &body[..i]),
        None => body.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpt_cuts_on_a_char_boundary() {
        assert_eq!(excerpt(" short "), "short");
        let long = "é".repeat(PREVIEW_CHARS + 5);
        let cut = excerpt(&long);
        assert_eq!(cut.chars().count(), PREVIEW_CHARS + 1);
        assert!(cut.ends_with('…'));
    }
}
