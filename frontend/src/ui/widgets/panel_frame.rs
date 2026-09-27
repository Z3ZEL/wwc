use egui::{Align, Button, Id, Layout, Panel, RichText, ScrollArea, Stroke, Ui};

use crate::actions::Action;
use crate::ui::theme::Theme;

/// The side panel shared by every page: header (title, collapse, close) and a
/// scrollable body. Docked right on wide screens; full width on narrow ones.
///
/// `title` is `None` when no panel is open: the panel then slides out (with the last
/// title and an empty body) and stops drawing. Opening, closing, collapsing and
/// expanding all slide over `style.animation_time` (`layout.panel_animation_ms`).
pub fn panel_frame(
    ui: &mut Ui,
    theme: &Theme,
    title: Option<&str>,
    collapsed: bool,
    actions: &mut Vec<Action>,
    body: impl FnOnce(&mut Ui, &mut Vec<Action>),
) {
    let layout = &theme.layout;
    let c = &theme.colors;
    let frame = egui::Frame::NONE.fill(c.panel_bg).stroke(Stroke::new(theme.shape.border_width, c.border));

    // Show and collapse share one animation value (egui keys both on the expanded
    // panel's id), so every transition picks up where the previous one stopped.
    let narrow = ui.ctx().content_rect().width() < layout.narrow_breakpoint;
    let expanded = Panel::right("side_panel").frame(frame);
    let expanded = if narrow {
        expanded.resizable(false).exact_size(ui.available_width())
    } else {
        expanded
            .resizable(true)
            .default_size(layout.side_panel_width)
            .size_range(layout.side_panel_min_width..=layout.side_panel_max_width)
    };

    let last_title = Id::new("side_panel").with("last_title");
    let Some(title) = title else {
        let title = ui.data(|d| d.get_temp::<String>(last_title)).unwrap_or_default();
        expanded.resizable(false).drag_to_open(false).show_collapsible(ui, &mut false, |ui| {
            header(ui, theme, &title, narrow, &mut Vec::new());
        });
        return;
    };
    ui.data_mut(|d| d.insert_temp(last_title, title.to_owned()));

    let strip = Panel::right("side_panel_collapsed")
        .resizable(false)
        .exact_size(layout.side_panel_collapsed_width)
        .frame(frame.fill(c.panel_header_bg));

    let mut is_expanded = !collapsed;
    Panel::show_switched(ui, &mut is_expanded, strip, expanded, |ui, expanded| {
        if !expanded {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                if ui.add(Button::new("◀").frame(false)).on_hover_text(format!("Show {title}")).clicked() {
                    actions.push(Action::ToggleCollapse);
                }
            });
            return;
        }
        header(ui, theme, title, narrow, actions);
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(theme.panel_margin()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                body(ui, actions);
            });
        });
    });
    // Dragging or double-clicking the resize edge collapses the panel.
    if is_expanded == collapsed {
        actions.push(Action::ToggleCollapse);
    }
}

fn header(ui: &mut Ui, theme: &Theme, title: &str, narrow: bool, actions: &mut Vec<Action>) {
    egui::Frame::NONE
        .fill(theme.colors.panel_header_bg)
        .inner_margin(egui::Margin::symmetric(theme.panel_margin().left, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).heading());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(Button::new("×").frame(false)).on_hover_text("Close").clicked() {
                        actions.push(Action::ClosePanel);
                    }
                    if !narrow && ui.add(Button::new("▶").frame(false)).on_hover_text("Collapse").clicked() {
                        actions.push(Action::ToggleCollapse);
                    }
                });
            });
        });
}
