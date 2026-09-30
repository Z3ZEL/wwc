//! Renders `frontend/assets/seo.json` into the static page (docs/ARCHITECTURE.md §5.10).
//!
//! The UI is an egui canvas, so crawlers and link-preview scrapers only see what is in the
//! HTML before the wasm runs. Everything here is plain string building: parse, validate,
//! render. File I/O lives in `main.rs`.

use serde::Deserialize;
use serde_json::json;

/// The `site_url` shipped until a production domain exists.
pub const PLACEHOLDER_SITE_URL: &str = "https://example.com";

/// Marks a `frontend/assets/documents/documents.json` var that still needs its real value
/// (the frontend's `documents::PLACEHOLDER`).
pub const DOCUMENT_PLACEHOLDER: &str = "TODO";

/// Markers in `frontend/index.html`, replaced by [`inject`].
pub const HEAD_MARKER: &str = "<!-- seo:head -->";
pub const BODY_MARKER: &str = "<!-- seo:body -->";
pub const LANG_MARKER: &str = "seo:lang";

const TITLE_MAX: usize = 60;
const DESCRIPTION_RANGE: std::ops::RangeInclusive<usize> = 70..=160;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seo {
    /// Absolute https URL of the site root, without a trailing slash.
    pub site_url: String,
    pub site_name: String,
    pub lang: String,
    pub locale: String,
    pub title: String,
    pub description: String,
    pub robots: String,
    /// File in `assets/seo/`, served at the site root.
    pub favicon: String,
    pub open_graph: OpenGraph,
    pub twitter_card: String,
    pub structured_data: StructuredData,
    pub page: Page,
    /// Paths listed in sitemap.xml, each starting with `/`.
    pub sitemap: Vec<String>,
    pub robots_disallow: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenGraph {
    #[serde(rename = "type")]
    pub kind: String,
    /// File in `assets/seo/`, served at the site root. PNG or JPEG: scrapers don't read SVG.
    pub image: String,
    pub image_width: u32,
    pub image_height: u32,
    pub image_alt: String,
}

/// schema.org `WebApplication` fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredData {
    pub application_category: String,
    pub operating_system: String,
    pub price: String,
    pub price_currency: String,
}

/// The crawlable `<main id="about">` block, covered by the canvas once the app runs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub heading: String,
    pub intro: String,
    pub features: Vec<String>,
}

/// The few `theme.json` colors the static page needs, so no color is defined twice.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub text: String,
    pub top_bar_bg: String,
}

#[derive(Deserialize)]
struct ThemeFile {
    colors: ThemeColors,
}

/// The documents' vars that still hold a placeholder: the legal pages would show them
/// (ARCHITECTURE §5.11). Only `vars` is read here; the frontend's tests check the rest.
pub fn document_placeholders(documents_json: &str) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct Manifest {
        vars: std::collections::BTreeMap<String, String>,
    }
    let manifest: Manifest = serde_json::from_str(documents_json).map_err(|e| format!("documents.json: {e}"))?;
    Ok(manifest.vars.into_iter().filter(|(_, v)| v.contains(DOCUMENT_PLACEHOLDER)).map(|(k, _)| k).collect())
}

impl Seo {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("seo.json: {e}"))
    }

    /// Hard errors fail the build; the returned warnings are only printed.
    pub fn validate(&self) -> Result<Vec<String>, String> {
        let mut errors = Vec::new();
        let url = &self.site_url;
        if !url.starts_with("https://") || url.len() <= "https://".len() || url.ends_with('/') {
            errors.push(format!("site_url must be an absolute https URL without a trailing slash, got {url:?}"));
        }
        let title = self.title.chars().count();
        if title == 0 || title > TITLE_MAX {
            errors.push(format!("title must be 1–{TITLE_MAX} characters, got {title}"));
        }
        let description = self.description.chars().count();
        if !DESCRIPTION_RANGE.contains(&description) {
            errors.push(format!(
                "description must be {}–{} characters, got {description}",
                DESCRIPTION_RANGE.start(),
                DESCRIPTION_RANGE.end()
            ));
        }
        if let Some(p) = self.sitemap.iter().chain(&self.robots_disallow).find(|p| !p.starts_with('/')) {
            errors.push(format!("sitemap and robots_disallow paths must start with '/', got {p:?}"));
        }
        let image = self.open_graph.image.to_ascii_lowercase();
        if ![".png", ".jpg", ".jpeg"].iter().any(|ext| image.ends_with(ext)) {
            errors.push(format!("open_graph.image must be a PNG or JPEG, got {:?}", self.open_graph.image));
        }
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        let mut warnings = Vec::new();
        if self.site_url == PLACEHOLDER_SITE_URL {
            warnings.push(format!(
                "site_url is still the placeholder {PLACEHOLDER_SITE_URL}: set the real domain before deploying"
            ));
        }
        Ok(warnings)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.site_url)
    }

    /// Everything that goes in `<head>`.
    pub fn render_head(&self, theme: &ThemeColors) -> String {
        let og = &self.open_graph;
        let image = self.url(&format!("/{}", og.image));
        let canonical = self.url("/");
        let mut h = String::new();
        let mut meta = |attr: &str, key: &str, value: &str| {
            h.push_str(&format!("  <meta {attr}=\"{key}\" content=\"{}\" />\n", escape(value)));
        };
        meta("name", "description", &self.description);
        meta("name", "robots", &self.robots);
        meta("name", "theme-color", &theme.top_bar_bg);
        meta("property", "og:type", &og.kind);
        meta("property", "og:site_name", &self.site_name);
        meta("property", "og:locale", &self.locale);
        meta("property", "og:url", &canonical);
        meta("property", "og:title", &self.title);
        meta("property", "og:description", &self.description);
        meta("property", "og:image", &image);
        meta("property", "og:image:width", &og.image_width.to_string());
        meta("property", "og:image:height", &og.image_height.to_string());
        meta("property", "og:image:alt", &og.image_alt);
        meta("name", "twitter:card", &self.twitter_card);
        meta("name", "twitter:title", &self.title);
        meta("name", "twitter:description", &self.description);
        meta("name", "twitter:image", &image);
        meta("name", "twitter:image:alt", &og.image_alt);

        let mut head = format!("<title>{}</title>\n", escape(&self.title));
        head.push_str(&h);
        head.push_str(&format!("  <link rel=\"canonical\" href=\"{}\" />\n", escape(&canonical)));
        head.push_str(&format!("  <link rel=\"icon\" href=\"/{}\" type=\"image/svg+xml\" />\n", escape(&self.favicon)));
        head.push_str(&format!(
            "  <script type=\"application/ld+json\">{}</script>\n",
            self.json_ld().replace("</", "<\\/")
        ));
        head.push_str(&format!(
            "  <style>html, body {{ background: {}; color: {}; }}</style>",
            escape(&theme.background),
            escape(&theme.text)
        ));
        head
    }

    /// schema.org `WebSite` + `WebApplication`.
    pub fn json_ld(&self) -> String {
        let s = &self.structured_data;
        let url = self.url("/");
        json!({
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "WebSite",
                    "name": self.site_name,
                    "url": url,
                    "description": self.description,
                    "inLanguage": self.lang,
                },
                {
                    "@type": "WebApplication",
                    "name": self.site_name,
                    "url": url,
                    "description": self.description,
                    "image": self.url(&format!("/{}", self.open_graph.image)),
                    "applicationCategory": s.application_category,
                    "operatingSystem": s.operating_system,
                    "browserRequirements": "Requires JavaScript and WebAssembly",
                    "inLanguage": self.lang,
                    "isAccessibleForFree": s.price == "0",
                    "offers": { "@type": "Offer", "price": s.price, "priceCurrency": s.price_currency },
                },
            ],
        })
        .to_string()
    }

    /// The `<main id="about">` block.
    pub fn render_body(&self) -> String {
        let p = &self.page;
        let mut body = String::from("<main id=\"about\">\n");
        body.push_str(&format!("    <h1>{}</h1>\n", escape(&p.heading)));
        body.push_str(&format!("    <p>{}</p>\n", escape(&p.intro)));
        body.push_str("    <ul>\n");
        for f in &p.features {
            body.push_str(&format!("      <li>{}</li>\n", escape(f)));
        }
        body.push_str("    </ul>\n");
        body.push_str("    <noscript><p>This map needs JavaScript and WebAssembly.</p></noscript>\n");
        body.push_str("  </main>");
        body
    }

    pub fn render_robots(&self) -> String {
        let mut r = String::from("User-agent: *\n");
        for path in &self.robots_disallow {
            r.push_str(&format!("Disallow: {path}\n"));
        }
        r.push_str(&format!("\nSitemap: {}\n", self.url("/sitemap.xml")));
        r
    }

    pub fn render_sitemap(&self) -> String {
        let mut s = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for path in &self.sitemap {
            s.push_str(&format!("  <url><loc>{}</loc></url>\n", escape(&self.url(path))));
        }
        s.push_str("</urlset>\n");
        s
    }
}

impl ThemeColors {
    /// Reads the colors from the full `theme.json`, ignoring everything else.
    pub fn parse(theme_json: &str) -> Result<Self, String> {
        serde_json::from_str::<ThemeFile>(theme_json).map(|t| t.colors).map_err(|e| format!("theme.json: {e}"))
    }
}

/// Fills the markers of Trunk's generated `index.html`. A missing marker is an error, so
/// a template edit can't silently drop the tags.
pub fn inject(html: &str, seo: &Seo, theme: &ThemeColors) -> Result<String, String> {
    for marker in [HEAD_MARKER, BODY_MARKER, LANG_MARKER] {
        if !html.contains(marker) {
            return Err(format!("index.html has no {marker} marker"));
        }
    }
    let html = html.replacen(HEAD_MARKER, &seo.render_head(theme), 1);
    let html = html.replacen(BODY_MARKER, &seo.render_body(), 1);
    Ok(html.replacen(LANG_MARKER, &escape(&seo.lang), 1))
}

/// Escapes text for HTML text and double-quoted attributes.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEO_JSON: &str = include_str!("../../../frontend/assets/seo.json");
    const THEME_JSON: &str = include_str!("../../../frontend/assets/theme.json");

    fn seo() -> Seo {
        Seo::parse(SEO_JSON).expect("seo.json parses")
    }

    fn theme() -> ThemeColors {
        ThemeColors::parse(THEME_JSON).expect("theme.json parses")
    }

    #[test]
    fn shipped_files_are_valid() {
        seo().validate().expect("seo.json is valid");
        theme();
    }

    #[test]
    fn rejects_bad_values() {
        let mut s = seo();
        s.site_url = "http://example.com/".into();
        s.title = "x".repeat(TITLE_MAX + 1);
        s.description = "too short".into();
        s.sitemap = vec!["about".into()];
        s.open_graph.image = "og.svg".into();
        let err = s.validate().expect_err("invalid");
        for part in ["site_url", "title", "description", "paths", "open_graph.image"] {
            assert!(err.contains(part), "missing {part} in {err}");
        }
    }

    #[test]
    fn warns_on_placeholder_url() {
        let mut s = seo();
        s.site_url = PLACEHOLDER_SITE_URL.into();
        assert_eq!(s.validate().map(|w| w.len()), Ok(1));
        s.site_url = "https://wwc.test".into();
        assert_eq!(s.validate().map(|w| w.len()), Ok(0));
    }

    #[test]
    fn lists_document_placeholders() {
        let json = r#"{"vars": {"a": "TODO name", "b": "Ann", "c": "x TODO"}, "documents": []}"#;
        assert_eq!(document_placeholders(json), Ok(vec!["a".to_owned(), "c".to_owned()]));
        assert!(document_placeholders("{}").is_err());
    }

    #[test]
    fn head_has_every_tag() {
        let head = seo().render_head(&theme());
        for tag in [
            "<title>",
            "name=\"description\"",
            "name=\"robots\"",
            "name=\"theme-color\"",
            "rel=\"canonical\"",
            "rel=\"icon\"",
            "og:title",
            "og:description",
            "og:url",
            "og:image\"",
            "og:image:alt",
            "twitter:card",
            "application/ld+json",
        ] {
            assert!(head.contains(tag), "missing {tag}");
        }
    }

    #[test]
    fn escapes_html_and_script_end() {
        let mut s = seo();
        s.title = "A <b>\"camp\"</b> & co".into();
        s.description = format!("</script>{}", "x".repeat(80));
        let head = s.render_head(&theme());
        assert!(head.contains("<title>A &lt;b&gt;&quot;camp&quot;&lt;/b&gt; &amp; co</title>"));
        let ld = &head[head.find("ld+json").unwrap_or(0)..];
        assert!(!ld[..ld.find("</script>").unwrap_or(ld.len())].contains("</"));
    }

    #[test]
    fn inject_fills_markers_and_requires_them() {
        let (s, t) = (seo(), theme());
        let html = format!("<html lang=\"{LANG_MARKER}\"><head>{HEAD_MARKER}</head><body>{BODY_MARKER}</body></html>");
        let out = inject(&html, &s, &t).expect("markers present");
        assert!(out.starts_with(&format!("<html lang=\"{}\">", s.lang)));
        assert!(out.contains("<main id=\"about\">") && out.contains("<title>"));
        assert!(!out.contains("seo:"));
        assert!(inject("<html></html>", &s, &t).is_err());
    }

    #[test]
    fn robots_and_sitemap_use_site_url() {
        let s = seo();
        assert!(s.render_robots().contains(&format!("Sitemap: {}/sitemap.xml", s.site_url)));
        assert!(s.render_sitemap().contains(&format!("<loc>{}/</loc>", s.site_url)));
    }
}
