//! Browser tab title, built from `assets/seo.json` (the rest of that file is rendered into
//! the static page at build time by `tools/seo-gen`, ARCHITECTURE §5.10).

use serde::Deserialize;

use crate::state::{AppState, Panel};

const SEO_JSON: &str = include_str!("../assets/seo.json");

/// The two `seo.json` fields the running app needs; the others are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct SeoTitles {
    pub site_name: String,
    pub title: String,
}

impl SeoTitles {
    /// The file is checked by the unit tests below (and by `seo-gen`); fall back to the
    /// static page's title rather than failing at runtime.
    pub fn load() -> Self {
        serde_json::from_str(SEO_JSON).unwrap_or_else(|e| {
            log::warn!("assets/seo.json: {e}");
            Self { site_name: String::new(), title: String::new() }
        })
    }

    /// "Lake Spot · WWC" for an open campsite, "Log in · WWC" for other panels, and the
    /// site title with no panel open.
    pub fn page_title(&self, state: &AppState) -> String {
        let Some(panel) = &state.panel else { return self.title.clone() };
        let campsite = match panel {
            Panel::Campsite(_) => state.detail.as_ref().and_then(|d| d.campsite.loaded()).map(|c| c.title.as_str()),
            _ => None,
        };
        format!("{} · {}", campsite.unwrap_or(panel.title()), self.site_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seo_json_parses() {
        let s: SeoTitles = serde_json::from_str(SEO_JSON).expect("seo.json has site_name and title");
        assert!(!s.site_name.is_empty() && !s.title.is_empty());
    }

    #[test]
    fn title_follows_panel() {
        let seo = SeoTitles { site_name: "WWC".into(), title: "WWC — map".into() };
        let mut state = AppState::default();
        assert_eq!(seo.page_title(&state), "WWC — map");
        state.panel = Some(Panel::Login);
        assert_eq!(seo.page_title(&state), "Log in · WWC");
        // Not loaded yet: generic panel title.
        state.panel = Some(Panel::Campsite("a".into()));
        assert_eq!(seo.page_title(&state), "Campsite · WWC");
    }
}
