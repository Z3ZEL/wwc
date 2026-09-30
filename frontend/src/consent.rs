//! Consent to optional processing (analytics…), ePrivacy art. 5(3) and GDPR art. 7
//! (ARCHITECTURE §5.12). Configured by `assets/consent.json`, compiled in.
//!
//! **Off for now** (`"enabled": false`): nothing needs consent yet. While it is off, the consent
//! panel never shows, nothing is stored, and `AppState::consent_allows` is always false. The
//! privacy notice (`ui/notice.rs`) is shown instead.
//!
//! Before setting `"enabled": true`, together with the analytics code:
//! - describe the analytics in `assets/documents/privacy.md` (purpose, provider, data, retention,
//!   what it stores in the browser, and the `wwc_consent` key), and bump its `updated` date;
//! - load or call the analytics only when `state.consent_allows("analytics")` is true, and stop
//!   when it turns false (the choice can be changed at any time from the map footer);
//! - bump `version` here whenever the purposes change, so everyone is asked again.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

const CONSENT_JSON: &str = include_str!("../assets/consent.json");

const DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// `assets/consent.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentConfig {
    /// Shows the consent panel and the footer's "Privacy choices" link.
    pub enabled: bool,
    /// Identifies the list of purposes. A saved choice for another version is asked again.
    pub version: String,
    /// A saved choice (accept or refuse) is asked again after this many days.
    pub max_age_days: u32,
    /// What visitors can accept, one by one. Each is off until they accept it.
    pub purposes: Vec<Purpose>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Purpose {
    /// Used in code: `state.consent_allows("analytics")`.
    pub id: String,
    pub title: String,
    pub description: String,
}

static CONFIG: LazyLock<ConsentConfig> = LazyLock::new(|| {
    ConsentConfig::parse(CONSENT_JSON).unwrap_or_else(|e| {
        // The unit tests parse the real file, so this only happens if they were skipped.
        // Default = disabled: nothing is ever allowed.
        log::error!("assets/consent.json: {e}");
        ConsentConfig::default()
    })
});

/// The compiled-in configuration.
pub fn config() -> &'static ConsentConfig {
    &CONFIG
}

impl ConsentConfig {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn purpose_ids(&self) -> BTreeSet<String> {
        self.purposes.iter().map(|p| p.id.clone()).collect()
    }

    /// A saved choice still applies: same purposes, and not older than `max_age_days`.
    pub fn is_current(&self, record: &ConsentRecord, now_ms: f64) -> bool {
        record.version == self.version
            && record.decided_at_ms <= now_ms
            && now_ms - record.decided_at_ms < f64::from(self.max_age_days) * DAY_MS
    }

    /// Only a current choice that includes the purpose allows it, and only while enabled.
    pub fn allows(&self, record: Option<&ConsentRecord>, purpose: &str) -> bool {
        self.enabled && record.is_some_and(|r| r.version == self.version && r.granted.contains(purpose))
    }
}

/// The visitor's choice, saved in the browser (`wwc_consent`). Keeping it is exempt from
/// consent itself: it is needed to respect the choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// `ConsentConfig::version` at the time of the choice.
    pub version: String,
    /// When the choice was made, in ms since the Unix epoch.
    pub decided_at_ms: f64,
    /// Accepted purposes. Everything else is refused.
    pub granted: BTreeSet<String>,
}

impl ConsentRecord {
    /// Keeps only purposes that exist in `config`.
    pub fn new(config: &ConsentConfig, granted: &BTreeSet<String>, now_ms: f64) -> Self {
        let known = config.purpose_ids();
        Self {
            version: config.version.clone(),
            decided_at_ms: now_ms,
            granted: granted.intersection(&known).cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(enabled: bool) -> ConsentConfig {
        ConsentConfig {
            enabled,
            version: "v2".into(),
            max_age_days: 10,
            purposes: vec![Purpose { id: "analytics".into(), title: "A".into(), description: "B".into() }],
        }
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn consent_json_is_valid() {
        let c = ConsentConfig::parse(CONSENT_JSON).expect("consent.json parses");
        assert!(!c.version.is_empty());
        // Choices are asked again within 13 months at most (CNIL guidance).
        assert!((1..=395).contains(&c.max_age_days), "max_age_days = {}", c.max_age_days);
        let ids = c.purpose_ids();
        assert_eq!(ids.len(), c.purposes.len(), "purpose ids are unique");
        for p in &c.purposes {
            assert!(!p.id.is_empty() && p.id.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'), "id {:?}", p.id);
            assert!(!p.title.is_empty() && !p.description.is_empty(), "{} needs a title and a description", p.id);
        }
    }

    #[test]
    fn records_keep_known_purposes_only() {
        let r = ConsentRecord::new(&config(true), &set(&["analytics", "ads"]), 5.0);
        assert_eq!(r.granted, set(&["analytics"]));
        assert_eq!(r.version, "v2");
        assert_eq!(r.decided_at_ms, 5.0);
    }

    #[test]
    fn choices_expire_and_follow_the_version() {
        let c = config(true);
        let r = ConsentRecord::new(&c, &set(&["analytics"]), 1000.0);
        assert!(c.is_current(&r, 1000.0 + 9.0 * DAY_MS));
        assert!(!c.is_current(&r, 1000.0 + 10.0 * DAY_MS), "too old");
        assert!(!c.is_current(&r, 999.0), "made in the future: clock changed");
        let newer = ConsentConfig { version: "v3".into(), ..c };
        assert!(!newer.is_current(&r, 1000.0), "purposes changed");
    }

    #[test]
    fn nothing_is_allowed_without_consent_or_while_disabled() {
        let accepted = ConsentRecord::new(&config(true), &set(&["analytics"]), 0.0);
        let refused = ConsentRecord::new(&config(true), &set(&[]), 0.0);
        assert!(config(true).allows(Some(&accepted), "analytics"));
        assert!(!config(true).allows(Some(&refused), "analytics"));
        assert!(!config(true).allows(None, "analytics"));
        assert!(!config(true).allows(Some(&accepted), "other"));
        assert!(!config(false).allows(Some(&accepted), "analytics"));
        let newer = ConsentConfig { version: "v3".into(), ..config(true) };
        assert!(!newer.allows(Some(&accepted), "analytics"));
    }
}
