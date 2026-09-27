use egui::{Align, Button, Layout, Panel, RichText, ScrollArea, Stroke, Ui};

use crate::actions::Action;
use crate::ui::theme::Theme;

/// The side panel shared by every page: header (title, collapse, close) and a
/// scrollable body. Docked right on wide screens; full width on narrow ones.
pub fn panel_frame(
    ui: &mut Ui,
    theme: &Theme,
    title: &str,
    collapsed: bool,
    actions: &mut Vec<Action>,
    body: impl FnOnce(&mut Ui, &mut Vec<Action>),
) {
    let layout = &theme.layout;
    let c = &theme.colors;
    let frame = egui::Frame::NONE.fill(c.panel_bg).stroke(Stroke::new(theme.shape.border_width, c.border));

    if collapsed {
        Panel::right("side_panel_collapsed")
            .resizable(false)
            .exact_size(layout.side_panel_collapsed_width)
            .frame(frame.fill(c.panel_header_bg))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    if ui.add(Button::new("◀").frame(false)).on_hover_text(format!("Show {title}")).clicked() {
                        actions.push(Action::ToggleCollapse);
                    }
                });
            });
        return;
    }

    let narrow = ui.ctx().content_rect().width() < layout.narrow_breakpoint;
    let panel = Panel::right("side_panel").frame(frame);
    let panel = if narrow {
        panel.resizable(false).exact_size(ui.available_width())
    } else {
        panel
            .resizable(true)
            .default_size(layout.side_panel_width)
            .size_range(layout.side_panel_min_width..=layout.side_panel_max_width)
    };

    panel.show(ui, |ui| {
        egui::Frame::NONE
            .fill(c.panel_header_bg)
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

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(theme.panel_margin()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                body(ui, actions);
            });
        });
    });
}
