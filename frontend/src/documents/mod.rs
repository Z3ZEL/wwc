//! In-app documents: legal pages and any other long text, written in Markdown in
//! `assets/documents/` (ARCHITECTURE §5.11).
//!
//! The manifest (`documents.json`: titles, files, dates, and the `{{vars}}` the texts use) is
//! compiled in. The Markdown files are copied next to the app by Trunk and fetched the first
//! time a document is opened, so they don't grow the wasm bundle however long they get.

pub mod markdown;

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

pub use markdown::{Block, Span, SpanStyle};

const MANIFEST_JSON: &str = include_str!("../../assets/documents/documents.json");
const SEO_JSON: &str = include_str!("../../assets/seo.json");

/// Where the folder is served, relative to the page (`copy-dir` in index.html).
const SERVED_DIR: &str = "documents";

/// Documents the app links to from its own screens.
pub const TERMS: &str = "terms";
pub const PRIVACY: &str = "privacy";
/// The Welcome tab of the welcome card on the map.
pub const WELCOME: &str = "welcome";

/// A var whose value contains this still needs its real value before going live
/// (`tools/seo-gen` warns about them on every build).
pub const PLACEHOLDER: &str = "TODO";

/// `assets/documents/documents.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `{{name}}` in a document is replaced by `vars[name]`. `site_name` and `site_url`
    /// come from `seo.json`.
    pub vars: BTreeMap<String, String>,
    /// In the order the map footer lists them.
    pub documents: Vec<DocumentInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentInfo {
    /// Stable id, used in code (`TERMS`, `PRIVACY`) and in `Panel::Document`.
    pub id: String,
    pub title: String,
    /// Link text in the map footer; the title if unset.
    #[serde(default)]
    pub short_title: Option<String>,
    /// File name in `assets/documents/`. Documents link to each other by it.
    pub file: String,
    /// Date of the last change (`YYYY-MM-DD`). Shown above the text, and part of the URL,
    /// so browsers and CDNs never serve an older version. Bump it with every change.
    pub updated: String,
    /// Linked from the map footer.
    #[serde(default)]
    pub footer: bool,
}

impl DocumentInfo {
    pub fn short_title(&self) -> &str {
        self.short_title.as_deref().unwrap_or(&self.title)
    }

    /// URL of the Markdown file, relative to the page.
    pub fn url(&self) -> String {
        format!("{SERVED_DIR}/{}?v={}", self.file, self.updated)
    }

    /// "29 September 2026".
    pub fn updated_label(&self) -> String {
        format_date(&self.updated).unwrap_or_else(|| self.updated.clone())
    }
}

static MANIFEST: LazyLock<Manifest> = LazyLock::new(|| {
    let manifest = Manifest::load(MANIFEST_JSON, SEO_JSON).unwrap_or_else(|e| {
        // The unit tests parse the real file, so this only happens if they were skipped.
        log::error!("assets/documents/documents.json: {e}");
        Manifest::default()
    });
    let todo = manifest.placeholders();
    if !todo.is_empty() {
        log::warn!("documents.json: placeholder values left in {}", todo.join(", "));
    }
    manifest
});

/// The compiled-in manifest.
pub fn manifest() -> &'static Manifest {
    &MANIFEST
}

impl Manifest {
    /// Parses the manifest and adds the vars that come from `seo.json`.
    pub fn load(manifest_json: &str, seo_json: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Site {
            site_name: String,
            site_url: String,
        }
        let mut manifest: Manifest = serde_json::from_str(manifest_json).map_err(|e| e.to_string())?;
        let site: Site = serde_json::from_str(seo_json).map_err(|e| format!("seo.json: {e}"))?;
        for (name, value) in [("site_name", site.site_name), ("site_url", site.site_url)] {
            if manifest.vars.insert(name.to_owned(), value).is_some() {
                return Err(format!("var {name:?} comes from seo.json; remove it from documents.json"));
            }
        }
        Ok(manifest)
    }

    pub fn get(&self, id: &str) -> Option<&DocumentInfo> {
        self.documents.iter().find(|d| d.id == id)
    }

    /// The document a relative Markdown link points to: `privacy.md`, `./privacy.md#rights`.
    pub fn by_link(&self, href: &str) -> Option<&DocumentInfo> {
        let path = href.split(['#', '?']).next().unwrap_or_default();
        let path = path.strip_prefix("./").unwrap_or(path);
        self.documents.iter().find(|d| d.file == path)
    }

    pub fn footer(&self) -> impl Iterator<Item = &DocumentInfo> {
        self.documents.iter().filter(|d| d.footer)
    }

    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// Replaces each `{{name}}` with its value. Unknown names stay as they are, so they show.
    pub fn fill(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some((before, after)) = rest.split_once("{{") {
            out.push_str(before);
            let Some((name, tail)) = after.split_once("}}") else {
                out.push_str("{{");
                rest = after;
                break;
            };
            match self.var(name.trim()) {
                Some(value) => out.push_str(value),
                None => {
                    out.push_str("{{");
                    out.push_str(name);
                    out.push_str("}}");
                }
            }
            rest = tail;
        }
        out.push_str(rest);
        out
    }

    /// A fetched document, ready to draw. Vars are filled in after parsing, so their values
    /// are plain text and never Markdown.
    pub fn prepare(&self, markdown: &str) -> Vec<Block> {
        let mut blocks = markdown::parse(markdown);
        markdown::map_text(&mut blocks, &|text| self.fill(text));
        blocks
    }

    /// Names of the vars that still hold a placeholder.
    pub fn placeholders(&self) -> Vec<&str> {
        self.vars.iter().filter(|(_, v)| v.contains(PLACEHOLDER)).map(|(k, _)| k.as_str()).collect()
    }
}

/// `YYYY-MM-DD` → "29 September 2026"; `None` if it isn't a valid date.
pub fn format_date(date: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut parts = date.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let year: u32 = y.parse().ok()?;
    let month: usize = m.parse().ok()?;
    let day: u32 = d.parse().ok()?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    let name = MONTHS.get(month.checked_sub(1)?)?;
    (1..=days).contains(&day).then(|| format!("{day} {name} {year}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;

    fn real() -> Manifest {
        Manifest::load(MANIFEST_JSON, SEO_JSON).expect("documents.json and seo.json parse")
    }

    fn read_document(file: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/documents").join(file);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// `{{name}}` occurrences in a text.
    fn var_names(text: &str) -> Vec<String> {
        text.split("{{").skip(1).filter_map(|s| s.split_once("}}")).map(|(name, _)| name.trim().to_owned()).collect()
    }

    #[test]
    fn manifest_is_consistent() {
        let m = real();
        let mut ids = BTreeSet::new();
        let mut files = BTreeSet::new();
        for d in &m.documents {
            assert!(ids.insert(&d.id), "duplicate id {:?}", d.id);
            assert!(files.insert(&d.file), "duplicate file {:?}", d.file);
            assert!(!d.id.is_empty() && d.id.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'), "id {:?}", d.id);
            assert!(
                d.file.ends_with(".md") && !d.file.contains('/') && !d.file.starts_with('.'),
                "{:?}: a .md file directly in assets/documents/",
                d.file
            );
            assert!(format_date(&d.updated).is_some(), "{}: updated {:?} is not YYYY-MM-DD", d.id, d.updated);
            assert!(!d.title.is_empty() && !d.short_title().is_empty());
        }
        for id in [TERMS, PRIVACY] {
            assert!(m.get(id).is_some_and(|d| d.footer), "{id} must exist and be in the footer");
        }
        assert!(m.get(WELCOME).is_some_and(|d| !d.footer), "the welcome text is in the map card, not the footer");
        assert!(m.var("min_age").and_then(|a| a.parse::<u8>().ok()).is_some(), "min_age is a number");
    }

    #[test]
    fn documents_exist_and_use_known_vars_and_links() {
        let m = real();
        for d in &m.documents {
            let text = read_document(&d.file);
            for name in var_names(&text) {
                assert!(m.var(&name).is_some(), "{}: unknown var {{{{{name}}}}}", d.file);
            }
            let blocks = m.prepare(&text);
            assert!(!blocks.is_empty(), "{} is empty", d.file);
            // Every link either leaves the app or opens another document.
            let mut links = Vec::new();
            collect_links(&blocks, &mut links);
            for link in links {
                let external = ["https://", "http://", "mailto:"].iter().any(|s| link.starts_with(s));
                assert!(external || m.by_link(&link).is_some(), "{}: link {link:?} goes nowhere", d.file);
            }
        }
    }

    fn collect_links(blocks: &[Block], out: &mut Vec<String>) {
        for block in blocks {
            match block {
                Block::Heading(_, spans) | Block::Paragraph(spans) => {
                    out.extend(spans.iter().filter_map(|s| s.link.clone()));
                }
                Block::List { items, .. } => items.iter().for_each(|item| collect_links(item, out)),
                Block::Quote(inner) => collect_links(inner, out),
                Block::Code(_) | Block::Rule => {}
            }
        }
    }

    #[test]
    fn retention_matches_the_backend() {
        // The Privacy Policy states how long request logs are kept.
        let settings: serde_json::Value =
            serde_json::from_str(include_str!("../../../backend/pb_settings.json")).expect("pb_settings.json parses");
        let days = settings["logs"]["maxDays"].as_u64().map(|d| d.to_string());
        assert_eq!(real().var("log_retention_days"), days.as_deref());
    }

    #[test]
    fn seo_vars_cannot_be_redefined() {
        let json = r#"{"vars": {"site_name": "x"}, "documents": []}"#;
        assert!(Manifest::load(json, SEO_JSON).is_err());
    }

    #[test]
    fn fill_replaces_known_vars_only() {
        let m = Manifest { vars: [("a".to_owned(), "1".to_owned())].into(), documents: vec![] };
        assert_eq!(m.fill("x {{a}} {{ a }} {{b}} {{a"), "x 1 1 {{b}} {{a");
        assert_eq!(m.fill("no vars"), "no vars");
    }

    #[test]
    fn vars_are_plain_text() {
        let m = Manifest { vars: [("mail".to_owned(), "a_b*c*@x.eu".to_owned())].into(), documents: vec![] };
        let blocks = m.prepare("Write to [{{mail}}](mailto:{{mail}}).");
        let Some(Block::Paragraph(spans)) = blocks.first() else { panic!("{blocks:?}") };
        assert_eq!(spans[1].text, "a_b*c*@x.eu");
        assert_eq!(spans[1].link.as_deref(), Some("mailto:a_b*c*@x.eu"));
        assert_eq!(spans[1].style, SpanStyle::default());
    }

    #[test]
    fn links_resolve_to_documents() {
        let m = real();
        assert_eq!(m.by_link("privacy.md").map(|d| d.id.as_str()), Some(PRIVACY));
        assert_eq!(m.by_link("./terms.md#accounts").map(|d| d.id.as_str()), Some(TERMS));
        assert!(m.by_link("../privacy.md").is_none());
        assert!(m.by_link("https://example.com/privacy.md").is_none());
    }

    #[test]
    fn urls_change_with_the_date() {
        let m = real();
        let d = m.get(PRIVACY).expect("privacy exists");
        assert_eq!(d.url(), format!("documents/privacy.md?v={}", d.updated));
    }

    #[test]
    fn placeholders_are_listed() {
        let m = Manifest {
            vars: [("a".to_owned(), "TODO name".to_owned()), ("b".to_owned(), "Ann".to_owned())].into(),
            documents: vec![],
        };
        assert_eq!(m.placeholders(), ["a"]);
    }

    #[test]
    fn dates() {
        assert_eq!(format_date("2026-09-29").as_deref(), Some("29 September 2026"));
        assert_eq!(format_date("2028-02-29").as_deref(), Some("29 February 2028"));
        assert_eq!(format_date("2026-02-29"), None);
        assert_eq!(format_date("2026-13-01"), None);
        assert_eq!(format_date("2026-00-01"), None);
        assert_eq!(format_date("2026-9-29"), None);
        assert_eq!(format_date("29/09/2026"), None);
    }
}
