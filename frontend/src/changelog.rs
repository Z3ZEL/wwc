//! Release notes for the Updates tab of the welcome card (ARCHITECTURE §5.6, ADR 0020).
//!
//! `tools/changelog-gen` writes `releases.json` next to the app at build time, from the
//! repository's GitHub Releases. The app fetches it the first time the tab is shown, so
//! visitors never contact GitHub, and parses each release's notes once, like a document.

use serde::Deserialize;

use crate::api::ApiError;
use crate::documents::{Block, markdown};

/// Served next to `index.html` (written into Trunk's staging dir by the hook).
const FILE: &str = "releases.json";

/// URL of the release notes, relative to the page. `?v=` changes with every build.
pub fn url() -> String {
    format!("{FILE}?v={}", crate::config::build_id())
}

/// One release, newest first in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub tag: String,
    /// The release name, or the tag when it has none.
    pub title: String,
    /// Publication day, `YYYY-MM-DD`.
    pub date: String,
    pub prerelease: bool,
    /// The release page on GitHub (`https://` only).
    pub url: Option<String>,
    pub notes: Vec<Block>,
}

/// `releases.json`, as written by `tools/changelog-gen`.
#[derive(Deserialize)]
struct File {
    releases: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    tag: String,
    title: String,
    date: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    notes: String,
}

/// Parse `releases.json`. Notes are Markdown, rendered with the documents' subset; they
/// are not given the documents' `{{vars}}`.
pub fn parse(json: &str) -> Result<Vec<Release>, ApiError> {
    let file: File =
        serde_json::from_str(json).map_err(|e| ApiError::network(format!("Unreadable release notes ({e}).")))?;
    Ok(file
        .releases
        .into_iter()
        .map(|e| Release {
            notes: markdown::parse(&e.notes),
            url: e.url.filter(|u| u.starts_with("https://")),
            tag: e.tag,
            title: e.title,
            date: e.date,
            prerelease: e.prerelease,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Written by `tools/changelog-gen` from its fixture; its tests check it stays in sync.
    const CONTRACT: &str = include_str!("../tests/fixtures/releases.json");

    #[test]
    fn reads_the_generated_file() {
        let releases = parse(CONTRACT).expect("contract parses");
        assert!(releases.len() >= 2, "{releases:?}");
        let first = &releases[0];
        assert!(!first.tag.is_empty() && !first.title.is_empty());
        assert!(crate::documents::format_date(&first.date).is_some(), "{}", first.date);
        assert!(first.url.as_deref().is_some_and(|u| u.starts_with("https://github.com/")));
        assert!(matches!(first.notes.first(), Some(Block::Paragraph(_))), "{:?}", first.notes);
        assert!(releases.iter().any(|r| r.prerelease));
    }

    #[test]
    fn empty_and_broken_files() {
        assert_eq!(parse(r#"{"releases": []}"#), Ok(vec![]));
        assert!(parse("<!doctype html>").is_err());
        assert!(parse(r#"{"releases": [{"tag": "v1"}]}"#).is_err());
    }

    #[test]
    fn only_https_links_are_kept() {
        let json = r#"{"releases": [{"tag": "v1", "title": "v1", "date": "2026-01-01", "url": "javascript:x"}]}"#;
        let releases = parse(json).expect("parses");
        assert_eq!(releases[0].url, None);
        assert!(releases[0].notes.is_empty());
    }

    #[test]
    fn the_url_is_versioned() {
        assert_eq!(url(), format!("releases.json?v={}", crate::config::build_id()));
    }
}
