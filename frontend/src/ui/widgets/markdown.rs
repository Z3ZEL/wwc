use egui::{Align, Frame, Hyperlink, Label, Layout, Link, Margin, RichText, TextStyle, Ui, Vec2};

use crate::documents::{Block, Span};
use crate::ui::theme::{Theme, subheading};

/// Draws a parsed Markdown document with the theme's text styles, colors and spacing.
/// Web and mail links open by themselves (web links in a new tab). Returns the destination
/// of a clicked relative link (another document) for the caller to resolve.
pub fn markdown(ui: &mut Ui, theme: &Theme, blocks: &[Block]) -> Option<String> {
    let mut clicked = None;
    blocks_ui(ui, theme, blocks, &mut clicked);
    clicked
}

fn blocks_ui(ui: &mut Ui, theme: &Theme, blocks: &[Block], clicked: &mut Option<String>) {
    let c = &theme.colors;
    let box_margin =
        Margin::symmetric(theme.spacing.item_spacing[0] as i8, (theme.spacing.item_spacing[1] / 2.0) as i8);
    for (i, block) in blocks.iter().enumerate() {
        match block {
            Block::Heading(level, spans) => {
                let (style, gap) = match level {
                    1 => (TextStyle::Heading, theme.spacing.section_gap),
                    2 => (subheading(), theme.spacing.section_gap),
                    _ => (TextStyle::Body, theme.spacing.item_spacing[1]),
                };
                if i > 0 {
                    ui.add_space(gap);
                }
                inline(ui, theme, spans, &style, *level >= 3, clicked);
            }
            Block::Paragraph(spans) => inline(ui, theme, spans, &TextStyle::Body, false, clicked),
            Block::List { start, items } => list(ui, theme, *start, items, clicked),
            Block::Quote(inner) => {
                Frame::NONE.fill(c.surface_alt).corner_radius(theme.radius()).inner_margin(box_margin).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    blocks_ui(ui, theme, inner, clicked);
                });
            }
            Block::Code(code) => {
                Frame::NONE.fill(c.surface_alt).corner_radius(theme.radius()).inner_margin(box_margin).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(Label::new(RichText::new(code).monospace()).wrap());
                });
            }
            Block::Rule => {
                ui.separator();
            }
        }
    }
}

fn list(ui: &mut Ui, theme: &Theme, start: Option<u64>, items: &[Vec<Block>], clicked: &mut Option<String>) {
    let indent = theme.spacing.list_indent;
    for (i, item) in items.iter().enumerate() {
        let marker = match start {
            Some(n) => format!("{}.", n.saturating_add(i as u64)),
            None => "•".to_owned(),
        };
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(indent, 0.0), Layout::top_down(Align::Max), |ui| {
                ui.set_min_width(indent);
                ui.label(marker);
            });
            ui.vertical(|ui| blocks_ui(ui, theme, item, clicked));
        });
    }
}

/// A paragraph or heading: styled runs that wrap together like one text.
fn inline(ui: &mut Ui, theme: &Theme, spans: &[Span], style: &TextStyle, strong: bool, clicked: &mut Option<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        for span in spans {
            let mut text = RichText::new(&span.text).text_style(style.clone());
            // egui's default font has no bold weight: strong text (and small headings) use the
            // primary color instead, so the emphasis stays visible.
            if strong || span.style.strong {
                text = text.strong().color(theme.colors.primary);
            }
            if span.style.emphasis {
                text = text.italics();
            }
            if span.style.strikethrough {
                text = text.strikethrough();
            }
            if span.style.code {
                text = text.code();
            }
            let Some(url) = &span.link else {
                ui.add(Label::new(text).wrap());
                continue;
            };
            // Underlined, so links don't rely on color alone.
            let text = text.color(theme.colors.link).underline();
            let web = url.starts_with("https://") || url.starts_with("http://");
            if web || url.starts_with("mailto:") {
                // Web pages open in a new tab so the app stays open; mail links don't navigate.
                ui.add(Hyperlink::from_label_and_url(text, url).open_in_new_tab(web));
            } else if ui.add(Link::new(text)).clicked() {
                *clicked = Some(url.clone());
            }
        }
    });
}
