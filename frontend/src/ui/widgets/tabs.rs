use egui::{Button, RichText, Stroke, Ui, Vec2};

use crate::ui::theme::Theme;

/// A row of tabs. The selected one is filled like a selected tag; the others show a frame on
/// hover only. Tabs are at least `icon_button_size` tall, for touch. Returns the index of a
/// clicked tab other than the selected one.
pub fn tabs(ui: &mut Ui, theme: &Theme, labels: &[&str], selected: usize) -> Option<usize> {
    let c = &theme.colors;
    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let on = i == selected;
        let mut tab = Button::new(RichText::new(*label).color(if on { c.tag_selected_text } else { c.text }))
            .corner_radius(theme.tag_radius())
            .min_size(Vec2::new(0.0, theme.layout.icon_button_size));
        tab = if on { tab.fill(c.tag_selected_bg).stroke(Stroke::NONE) } else { tab.frame_when_inactive(false) };
        if ui.add(tab).clicked() && !on {
            clicked = Some(i);
        }
    }
    clicked
}
