//! A Markdown document from `assets/documents/`: Terms of Use, Privacy Policy… (ARCHITECTURE §5.11).

use egui::Ui;

use crate::actions::Action;
use crate::documents;
use crate::state::{AppState, Panel, Remote};
use crate::ui::theme::Theme;
use crate::ui::widgets::{failed, link_button, loading_block, markdown, muted};

pub fn show(ui: &mut Ui, state: &AppState, theme: &Theme, id: &str, actions: &mut Vec<Action>) {
    let manifest = documents::manifest();
    if let Some(back) = &state.document_back
        && link_button(ui, theme, &format!("◀ {}", back.title())).clicked()
    {
        actions.push(Action::OpenPanel(back.clone()));
    }
    if let Some(info) = manifest.get(id) {
        muted(ui, theme, format!("Last updated: {}", info.updated_label()));
        ui.add_space(theme.spacing.item_spacing[1]);
    }

    body(ui, state, theme, id, actions);

    // The other documents of the footer, so they can be read one after another.
    ui.add_space(theme.spacing.section_gap);
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        for doc in manifest.footer().filter(|d| d.id != id) {
            if link_button(ui, theme, &doc.title).clicked() {
                actions.push(Action::OpenPanel(Panel::Document(doc.id.clone())));
            }
        }
    });
}

/// The text of a document, with its loading and error states. A link to another document
/// opens it in the Document panel. Also used by the welcome card.
pub fn body(ui: &mut Ui, state: &AppState, theme: &Theme, id: &str, actions: &mut Vec<Action>) {
    match state.documents.get(id) {
        Some(Remote::Loaded(blocks)) => {
            if let Some(href) = markdown(ui, theme, blocks) {
                match documents::manifest().by_link(&href) {
                    Some(doc) => actions.push(Action::OpenPanel(Panel::Document(doc.id.clone()))),
                    None => log::warn!("{id}: link to {href:?} matches no document"),
                }
            }
        }
        Some(Remote::Failed(e)) if e.is_not_found() => {
            muted(ui, theme, "This page doesn't exist.");
        }
        Some(Remote::Failed(e)) => {
            if failed(ui, theme, e) {
                actions.push(Action::LoadDocument(id.to_owned()));
            }
        }
        Some(Remote::Loading | Remote::NotAsked) | None => loading_block(ui, theme, "Loading…"),
    }
}
