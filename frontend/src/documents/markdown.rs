//! Markdown → the small block tree drawn by `ui::widgets::markdown`. A document is parsed
//! once when it arrives, not every frame. Supported: CommonMark (headings, paragraphs,
//! emphasis, strong, inline code, links, nested lists, block quotes, code blocks, rules)
//! plus strikethrough. Tables are not enabled (their text shows as a paragraph), images
//! show their alt text, and HTML is dropped, so `<!-- comments -->` stay out of the page.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// Level 1–6.
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// `start` is the first number of an ordered list, `None` for a bullet list.
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Quote(Vec<Block>),
    Code(String),
    Rule,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub strong: bool,
    pub emphasis: bool,
    pub strikethrough: bool,
    pub code: bool,
}

/// A run of text with one style. `link` is the raw destination (`https://…`, `mailto:…`,
/// or a relative file name for another document).
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
    pub link: Option<String>,
}

pub fn parse(markdown: &str) -> Vec<Block> {
    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH) {
        builder.event(event);
    }
    builder.finish()
}

/// Applies `f` to every piece of text: span texts, link destinations and code blocks.
pub fn map_text(blocks: &mut [Block], f: &impl Fn(&str) -> String) {
    let spans = |spans: &mut Vec<Span>| {
        for span in spans {
            span.text = f(&span.text);
            if let Some(link) = &mut span.link {
                *link = f(link);
            }
        }
    };
    for block in blocks {
        match block {
            Block::Heading(_, s) | Block::Paragraph(s) => spans(s),
            Block::List { items, .. } => items.iter_mut().for_each(|item| map_text(item, f)),
            Block::Quote(inner) => map_text(inner, f),
            Block::Code(code) => *code = f(code),
            Block::Rule => {}
        }
    }
}

enum FrameKind {
    Root,
    Quote,
    List(Option<u64>),
    Item,
}

/// An open container. Blocks go to `blocks`; a list collects its finished items in `items`.
struct Frame {
    kind: FrameKind,
    blocks: Vec<Block>,
    items: Vec<Vec<Block>>,
}

impl Frame {
    fn new(kind: FrameKind) -> Self {
        Self { kind, blocks: Vec::new(), items: Vec::new() }
    }
}

struct Builder {
    /// `frames[0]` is the root and is never popped.
    frames: Vec<Frame>,
    /// Text of the paragraph or heading being built.
    inline: Vec<Span>,
    strong: u32,
    emphasis: u32,
    strikethrough: u32,
    link: Option<String>,
    heading: Option<u8>,
    code: Option<String>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            frames: vec![Frame::new(FrameKind::Root)],
            inline: Vec::new(),
            strong: 0,
            emphasis: 0,
            strikethrough: 0,
            link: None,
            heading: None,
            code: None,
        }
    }
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => match &mut self.code {
                Some(code) => code.push_str(&text),
                None => self.text(&text, false),
            },
            Event::Code(text) => self.text(&text, true),
            Event::SoftBreak => self.text(" ", false),
            Event::HardBreak => self.text("\n", false),
            Event::Rule => {
                self.flush();
                self.push(Block::Rule);
            }
            Event::TaskListMarker(done) => self.text(if done { "[x] " } else { "[ ] " }, false),
            // HTML, and extensions that are not enabled.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.flush(),
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
            }
            Tag::BlockQuote(_) => self.open(FrameKind::Quote),
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
            }
            Tag::List(start) => self.open(FrameKind::List(start)),
            Tag::Item => self.open(FrameKind::Item),
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strong => self.strong += 1,
            Tag::Strikethrough => self.strikethrough += 1,
            Tag::Link { dest_url, .. } => self.link = Some(dest_url.into_string()),
            // Images: the alt text arrives as text and is shown as such.
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                let spans = std::mem::take(&mut self.inline);
                if let Some(level) = self.heading.take() {
                    self.push(Block::Heading(level, spans));
                }
            }
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.push(Block::Code(code.trim_end_matches('\n').to_owned()));
                }
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item => self.close(),
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Strikethrough => self.strikethrough = self.strikethrough.saturating_sub(1),
            TagEnd::Link => self.link = None,
            _ => {}
        }
    }

    fn text(&mut self, text: &str, code: bool) {
        let style = SpanStyle {
            strong: self.strong > 0,
            emphasis: self.emphasis > 0,
            strikethrough: self.strikethrough > 0,
            code,
        };
        match self.inline.last_mut().filter(|s| s.style == style && s.link == self.link) {
            Some(last) => last.text.push_str(text),
            None => self.inline.push(Span { text: text.to_owned(), style, link: self.link.clone() }),
        }
    }

    fn push(&mut self, block: Block) {
        if let Some(frame) = self.frames.last_mut() {
            frame.blocks.push(block);
        }
    }

    /// Ends the current run of text: a paragraph, or the text of a tight list item.
    fn flush(&mut self) {
        if !self.inline.is_empty() {
            let spans = std::mem::take(&mut self.inline);
            self.push(Block::Paragraph(spans));
        }
    }

    fn open(&mut self, kind: FrameKind) {
        self.flush();
        self.frames.push(Frame::new(kind));
    }

    fn close(&mut self) {
        self.flush();
        if self.frames.len() < 2 {
            return;
        }
        let Some(frame) = self.frames.pop() else { return };
        match frame.kind {
            FrameKind::Item => {
                if let Some(list) = self.frames.last_mut() {
                    list.items.push(frame.blocks);
                }
            }
            FrameKind::List(start) => {
                let mut items = frame.items;
                if !frame.blocks.is_empty() {
                    items.push(frame.blocks);
                }
                self.push(Block::List { start, items });
            }
            FrameKind::Quote => self.push(Block::Quote(frame.blocks)),
            FrameKind::Root => self.frames.push(frame),
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush();
        while self.frames.len() > 1 {
            self.close();
        }
        self.frames.pop().map(|root| root.blocks).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Span {
        Span { text: text.into(), style: SpanStyle::default(), link: None }
    }

    #[test]
    fn headings_and_paragraphs() {
        let blocks = parse("# Title\n\nFirst line\nsame paragraph.\n\n## Part\n\nSecond.");
        assert_eq!(
            blocks,
            [
                Block::Heading(1, vec![plain("Title")]),
                Block::Paragraph(vec![plain("First line same paragraph.")]),
                Block::Heading(2, vec![plain("Part")]),
                Block::Paragraph(vec![plain("Second.")]),
            ]
        );
    }

    #[test]
    fn inline_styles_and_links() {
        let blocks = parse("Read **the [Privacy Policy](privacy.md)** or `mail` *us*.");
        let Some(Block::Paragraph(spans)) = blocks.first() else { panic!("paragraph expected: {blocks:?}") };
        let strong = SpanStyle { strong: true, ..Default::default() };
        assert_eq!(
            spans,
            &[
                plain("Read "),
                Span { text: "the ".into(), style: strong, link: None },
                Span { text: "Privacy Policy".into(), style: strong, link: Some("privacy.md".into()) },
                plain(" or "),
                Span { text: "mail".into(), style: SpanStyle { code: true, ..Default::default() }, link: None },
                plain(" "),
                Span { text: "us".into(), style: SpanStyle { emphasis: true, ..Default::default() }, link: None },
                plain("."),
            ]
        );
    }

    #[test]
    fn tight_and_loose_nested_lists() {
        let blocks = parse("- one\n- two\n  1. a\n  2. b\n\n3. x\n\n4. y\n");
        assert_eq!(
            blocks,
            [
                Block::List {
                    start: None,
                    items: vec![
                        vec![Block::Paragraph(vec![plain("one")])],
                        vec![
                            Block::Paragraph(vec![plain("two")]),
                            Block::List {
                                start: Some(1),
                                items: vec![
                                    vec![Block::Paragraph(vec![plain("a")])],
                                    vec![Block::Paragraph(vec![plain("b")])],
                                ],
                            },
                        ],
                    ],
                },
                Block::List {
                    start: Some(3),
                    items: vec![vec![Block::Paragraph(vec![plain("x")])], vec![Block::Paragraph(vec![plain("y")])]],
                },
            ]
        );
    }

    #[test]
    fn quotes_code_rules_and_html() {
        let blocks = parse("> **Note:** hi\n\n<!-- hidden -->\n\n---\n\n```\nlet x;\n```\n");
        let strong = SpanStyle { strong: true, ..Default::default() };
        assert_eq!(
            blocks,
            [
                Block::Quote(vec![Block::Paragraph(vec![
                    Span { text: "Note:".into(), style: strong, link: None },
                    plain(" hi"),
                ])]),
                Block::Rule,
                Block::Code("let x;".into()),
            ]
        );
    }

    #[test]
    fn map_text_reaches_every_text() {
        let mut blocks = parse("# A\n\n- [A](mailto:A)\n\n> A\n\n```\nA\n```");
        map_text(&mut blocks, &|t| t.replace('A', "B"));
        assert_eq!(format!("{blocks:?}").matches('A').count(), 0, "{blocks:?}");
    }

    #[test]
    fn unclosed_containers_are_kept() {
        assert_eq!(parse("> - item"), parse("> - item\n\n"));
        assert!(parse("").is_empty());
    }
}
