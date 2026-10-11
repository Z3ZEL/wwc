//! The pure part of the `changelog-gen` hook: the settings in `frontend/assets/updates.json`,
//! the choice of source, and the GitHub Releases → `releases.json` transform. I/O lives in
//! `main.rs`, so everything here is unit-tested.

use serde::{Deserialize, Serialize};

/// Recorded GitHub answer used by dev builds (`trunk serve` rebuilds on every save, so it
/// never calls GitHub) and by the tests.
pub const FIXTURE: &str = include_str!("../tests/fixtures/github_releases.json");

/// The file written into Trunk's staging dir, served next to `index.html`.
pub const OUTPUT: &str = "releases.json";

/// GitHub's largest page. Drafts and (optionally) prereleases are dropped after fetching, so
/// one full page leaves room for them.
const PER_PAGE: usize = 100;

/// `frontend/assets/updates.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// `owner/name` of the GitHub repository whose releases are listed.
    pub repo: String,
    /// How many of the newest releases the Updates tab shows.
    pub max_releases: usize,
    pub include_prereleases: bool,
    /// Longer release notes are cut; the full text stays on GitHub ("View on GitHub").
    pub max_notes_chars: usize,
}

impl Settings {
    /// Parse and validate. A bad value is a hard error: it fails the build.
    pub fn parse(json: &str) -> Result<Self, String> {
        let settings: Self = serde_json::from_str(json).map_err(|e| format!("assets/updates.json: {e}"))?;
        settings.validate().map_err(|e| format!("assets/updates.json: {e}"))?;
        Ok(settings)
    }

    fn validate(&self) -> Result<(), String> {
        let part = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
        if !self.repo.split_once('/').is_some_and(|(owner, name)| part(owner) && part(name)) {
            return Err(format!("repo {:?} must be \"owner/name\"", self.repo));
        }
        if !(1..=PER_PAGE).contains(&self.max_releases) {
            return Err(format!("max_releases must be 1–{PER_PAGE}"));
        }
        if self.max_notes_chars == 0 {
            return Err("max_notes_chars must be at least 1".into());
        }
        Ok(())
    }

    /// The repository's releases, newest first (GitHub REST API, public repositories need no token).
    pub fn api_url(&self) -> String {
        format!("https://api.github.com/repos/{}/releases?per_page={PER_PAGE}", self.repo)
    }
}

/// Where the release notes come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The GitHub API.
    Fetch,
    /// `FIXTURE`, no network.
    Fixture,
    /// An empty list.
    Off,
}

impl Source {
    /// `WWC_CHANGELOG` when set; otherwise GitHub for release builds (`TRUNK_PROFILE=release`:
    /// scripts/build-frontend.sh, the Render deploy) and the fixture for dev builds.
    pub fn choose(var: Option<&str>, profile: Option<&str>) -> Result<Self, String> {
        match var.map(str::trim).filter(|v| !v.is_empty()) {
            Some("fetch") => Ok(Self::Fetch),
            Some("fixture") => Ok(Self::Fixture),
            Some("off") => Ok(Self::Off),
            Some(other) => Err(format!("WWC_CHANGELOG={other:?}: expected fetch, fixture or off")),
            None if profile == Some("release") => Ok(Self::Fetch),
            None => Ok(Self::Fixture),
        }
    }
}

/// The fields used from GitHub's release object.
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    /// `null` for drafts.
    #[serde(default)]
    published_at: Option<String>,
}

/// One entry of `releases.json`: what the app's Updates tab shows (`frontend/src/changelog.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Release {
    pub tag: String,
    /// The release name, or the tag when it has none.
    pub title: String,
    /// Publication day (UTC), `YYYY-MM-DD`.
    pub date: String,
    pub prerelease: bool,
    /// The release page on GitHub; `None` unless it is an `https://` URL.
    pub url: Option<String>,
    /// Markdown, with `\n` line ends, cut at `max_notes_chars`.
    pub notes: String,
}

#[derive(Serialize)]
struct Changelog<'a> {
    releases: &'a [Release],
}

/// GitHub's answer → the newest published releases. Drafts (listed for tokens with push
/// access) are dropped, and prereleases too unless the settings include them.
pub fn transform(json: &str, settings: &Settings) -> Result<Vec<Release>, String> {
    let all: Vec<GithubRelease> = serde_json::from_str(json).map_err(|e| format!("unexpected GitHub answer: {e}"))?;
    let mut published: Vec<(String, GithubRelease)> = all
        .into_iter()
        .filter(|r| !r.draft && (settings.include_prereleases || !r.prerelease))
        .filter_map(|r| Some((r.published_at.clone()?, r)))
        .collect();
    // GitHub's timestamps are UTC (`2026-09-27T14:49:11Z`), so they sort as text.
    published.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(published
        .into_iter()
        .take(settings.max_releases)
        .map(|(published_at, r)| {
            let title = r.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).unwrap_or(&r.tag_name).to_owned();
            let notes = r.body.unwrap_or_default().replace("\r\n", "\n");
            Release {
                title,
                date: published_at.get(..10).unwrap_or(&published_at).to_owned(),
                prerelease: r.prerelease,
                url: Some(r.html_url).filter(|u| u.starts_with("https://")),
                notes: cap(notes.trim(), settings.max_notes_chars),
                tag: r.tag_name,
            }
        })
        .collect())
}

/// The content of `releases.json`.
pub fn render(releases: &[Release]) -> Result<String, String> {
    serde_json::to_string_pretty(&Changelog { releases }).map(|json| json + "\n").map_err(|e| e.to_string())
}

/// Cuts `text` to `max` characters and adds "…". The cut goes back to the last line break
/// when there is one in the second half, so a list item isn't cut in the middle.
fn cap(text: &str, max: usize) -> String {
    let Some((end, _)) = text.char_indices().nth(max) else {
        return text.to_owned();
    };
    let cut = &text[..end];
    let cut = match cut.rfind('\n') {
        Some(line) if line >= end / 2 => &cut[..line],
        _ => cut,
    };
    format!("{}\n\n…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape the app parses (`frontend/src/changelog.rs` reads the same file).
    const CONTRACT: &str = include_str!("../../../frontend/tests/fixtures/releases.json");

    fn settings() -> Settings {
        Settings { repo: "Z3ZEL/wwc".into(), max_releases: 10, include_prereleases: true, max_notes_chars: 4000 }
    }

    fn release(tag: &str, published_at: Option<&str>, extra: &str) -> String {
        let published_at = published_at.map_or("null".into(), |p| format!("\"{p}\""));
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/o/r/releases/tag/{tag}","published_at":{published_at}{extra}}}"#
        )
    }

    #[test]
    fn the_fixture_gives_the_app_contract() {
        let releases = transform(FIXTURE, &settings()).expect("fixture parses");
        assert_eq!(render(&releases).expect("renders"), CONTRACT);
    }

    #[test]
    fn the_shipped_settings_are_valid() {
        let s = Settings::parse(include_str!("../../../frontend/assets/updates.json")).expect("valid");
        assert!(s.api_url().starts_with("https://api.github.com/repos/"));
    }

    #[test]
    fn bad_settings_fail() {
        let json = |repo: &str, max: usize, chars: usize| {
            format!(r#"{{"repo":"{repo}","max_releases":{max},"include_prereleases":true,"max_notes_chars":{chars}}}"#)
        };
        assert!(Settings::parse(&json("Z3ZEL/wwc", 10, 100)).is_ok());
        for bad in ["wwc", "Z3ZEL/", "/wwc", "a/b/c", "Z3ZEL/wwc?x=1", "Z3ZEL/w w"] {
            assert!(Settings::parse(&json(bad, 10, 100)).is_err(), "{bad}");
        }
        assert!(Settings::parse(&json("Z3ZEL/wwc", 0, 100)).is_err());
        assert!(Settings::parse(&json("Z3ZEL/wwc", 101, 100)).is_err());
        assert!(Settings::parse(&json("Z3ZEL/wwc", 10, 0)).is_err());
        assert!(Settings::parse(r#"{"repo":"Z3ZEL/wwc"}"#).is_err());
    }

    #[test]
    fn drafts_are_dropped_and_prereleases_optional() {
        let json = format!(
            "[{},{},{}]",
            release("v2", None, r#","draft":true"#),
            release("v1", Some("2026-01-02T00:00:00Z"), ""),
            release("v1-rc", Some("2026-01-01T00:00:00Z"), r#","prerelease":true"#),
        );
        let tags = |s: &Settings| transform(&json, s).expect("parses").into_iter().map(|r| r.tag).collect::<Vec<_>>();
        assert_eq!(tags(&settings()), ["v1", "v1-rc"]);
        assert_eq!(tags(&Settings { include_prereleases: false, ..settings() }), ["v1"]);
    }

    #[test]
    fn newest_first_and_capped() {
        let json = format!(
            "[{},{},{}]",
            release("a", Some("2026-01-01T00:00:00Z"), ""),
            release("c", Some("2026-03-01T00:00:00Z"), ""),
            release("b", Some("2026-02-01T00:00:00Z"), ""),
        );
        let releases = transform(&json, &Settings { max_releases: 2, ..settings() }).expect("parses");
        assert_eq!(releases.iter().map(|r| r.tag.as_str()).collect::<Vec<_>>(), ["c", "b"]);
        assert_eq!(releases[0].date, "2026-03-01");
    }

    #[test]
    fn titles_urls_and_notes_are_cleaned() {
        let json = format!(
            "[{}]",
            release("v1", Some("2026-01-01T00:00:00Z"), r#","name":"  ","body":"line 1\r\nline 2\r\n""#)
        );
        let r = &transform(&json, &settings()).expect("parses")[0];
        assert_eq!(r.title, "v1", "an empty name falls back to the tag");
        assert_eq!(r.notes, "line 1\nline 2");
        assert_eq!(r.url.as_deref(), Some("https://github.com/o/r/releases/tag/v1"));

        let json = r#"[{"tag_name":"v1","html_url":"javascript:alert(1)","published_at":"2026-01-01T00:00:00Z"}]"#;
        assert_eq!(transform(json, &settings()).expect("parses")[0].url, None);
    }

    #[test]
    fn long_notes_are_cut() {
        assert_eq!(cap("short", 10), "short");
        assert_eq!(cap("abcdefghij", 10), "abcdefghij");
        assert_eq!(cap("abcdefghijk", 10), "abcdefghij\n\n…");
        // Back to the last line break in the second half.
        assert_eq!(cap("* one\n* two\n* three", 15), "* one\n* two\n\n…");
        // Multi-byte characters are counted, not bytes.
        assert_eq!(cap("ééééé", 3), "ééé\n\n…");
    }

    #[test]
    fn a_bad_answer_is_an_error() {
        assert!(transform(r#"{"message":"API rate limit exceeded"}"#, &settings()).is_err());
        assert!(transform("<html>", &settings()).is_err());
        assert_eq!(transform("[]", &settings()), Ok(vec![]));
    }

    #[test]
    fn the_source_follows_the_profile_unless_overridden() {
        assert_eq!(Source::choose(None, Some("release")), Ok(Source::Fetch));
        assert_eq!(Source::choose(None, Some("debug")), Ok(Source::Fixture));
        assert_eq!(Source::choose(None, None), Ok(Source::Fixture));
        assert_eq!(Source::choose(Some(""), Some("release")), Ok(Source::Fetch));
        assert_eq!(Source::choose(Some("fetch"), Some("debug")), Ok(Source::Fetch));
        assert_eq!(Source::choose(Some("fixture"), Some("release")), Ok(Source::Fixture));
        assert_eq!(Source::choose(Some(" off "), Some("release")), Ok(Source::Off));
        assert!(Source::choose(Some("github"), None).is_err());
    }

    #[test]
    fn no_releases_render_an_empty_list() {
        assert_eq!(render(&[]).expect("renders"), "{\n  \"releases\": []\n}\n");
    }
}
