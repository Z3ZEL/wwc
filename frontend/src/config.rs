//! Build-time configuration (ARCHITECTURE §6). Values are baked in by `trunk build`.

/// PocketBase base URL from `WWC_API_URL`, without a trailing `/`.
/// Empty = same origin (dev: Trunk proxies `/api` to PocketBase).
pub fn api_url() -> &'static str {
    trim_base(option_env!("WWC_API_URL").unwrap_or(""))
}

/// Identifies the build, from `WWC_BUILD_ID` (`scripts/build-frontend.sh` sets the build time).
/// Files written at build time are fetched with `?v=<build id>`, so a new deploy is never
/// answered from an older cache. `dev` when unset.
pub fn build_id() -> &'static str {
    url_safe_or_dev(option_env!("WWC_BUILD_ID"))
}

/// Umami Cloud website id from `WWC_UMAMI_WEBSITE_ID` (ADR 0022). `None` when unset or not a
/// UUID: the build then has no analytics and no consent panel.
pub fn umami_website_id() -> Option<&'static str> {
    uuid_or_none(option_env!("WWC_UMAMI_WEBSITE_ID"))
}

/// The Umami tracker script, from `WWC_UMAMI_SCRIPT_URL` (copy it from the Umami dashboard,
/// e.g. for the EU region). Defaults to Umami Cloud's.
pub fn umami_script_url() -> &'static str {
    match option_env!("WWC_UMAMI_SCRIPT_URL").map(str::trim) {
        Some(url) if url.starts_with("https://") => url,
        _ => "https://cloud.umami.is/script.js",
    }
}

/// Mapbox public access token from `WWC_MAPBOX_TOKEN` (ADR 0024). The map then uses Mapbox's
/// Outdoors and Aerial layers; `None` (unset or not a public token) keeps OpenStreetMap's tiles.
pub fn mapbox_token() -> Option<&'static str> {
    public_token_or_none(option_env!("WWC_MAPBOX_TOKEN"))
}

fn trim_base(url: &str) -> &str {
    url.trim().trim_end_matches('/')
}

/// The value goes into a query string: anything but letters, digits, `.`, `_` and `-` is refused.
fn url_safe_or_dev(id: Option<&str>) -> &str {
    id.map(str::trim)
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)))
        .unwrap_or("dev")
}

/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (hex digits), or `None`.
fn uuid_or_none(id: Option<&str>) -> Option<&str> {
    id.map(str::trim).filter(|id| {
        id.len() == 36
            && id.char_indices().all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            })
    })
}

/// A Mapbox public token (`pk.…`), or `None`. The token is baked into the wasm, where anyone can
/// read it: a secret token (`sk.…`) is refused. It goes into tile URLs, so only letters, digits,
/// `.`, `_` and `-` are accepted.
fn public_token_or_none(token: Option<&str>) -> Option<&str> {
    token.map(str::trim).filter(|t| {
        t.len() > "pk.".len()
            && t.starts_with("pk.")
            && t.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_trailing_slashes_and_space() {
        assert_eq!(trim_base("https://api.example.com/"), "https://api.example.com");
        assert_eq!(trim_base(" https://api.example.com// "), "https://api.example.com");
        assert_eq!(trim_base(""), "");
    }

    #[test]
    fn build_ids_are_url_safe() {
        assert_eq!(url_safe_or_dev(Some("20261010093000")), "20261010093000");
        assert_eq!(url_safe_or_dev(Some(" v1.0-rc_2 ")), "v1.0-rc_2");
        assert_eq!(url_safe_or_dev(None), "dev");
        assert_eq!(url_safe_or_dev(Some("")), "dev");
        assert_eq!(url_safe_or_dev(Some("a&b=c")), "dev");
        assert_eq!(url_safe_or_dev(Some("a b")), "dev");
        assert!(!build_id().is_empty());
    }

    #[test]
    fn umami_ids_are_uuids() {
        let id = "94db1cb1-74f4-4a40-ad6c-962362670409";
        assert_eq!(uuid_or_none(Some(id)), Some(id));
        assert_eq!(
            uuid_or_none(Some(" 94DB1CB1-74F4-4A40-AD6C-962362670409 ")),
            Some("94DB1CB1-74F4-4A40-AD6C-962362670409")
        );
        assert_eq!(uuid_or_none(None), None);
        assert_eq!(uuid_or_none(Some("")), None);
        assert_eq!(uuid_or_none(Some("94db1cb1-74f4-4a40-ad6c-96236267040")), None, "too short");
        assert_eq!(uuid_or_none(Some("94db1cb1-74f4-4a40-ad6c-96236267040\"")), None);
        assert!(umami_script_url().starts_with("https://"));
    }

    #[test]
    fn mapbox_tokens_are_public() {
        let token = "pk.eyJ1IjoiZXhhbXBsZSIsImEiOiJjbGV4YW1wbGUifQ.abc-DEF_123";
        assert_eq!(public_token_or_none(Some(token)), Some(token));
        assert_eq!(public_token_or_none(Some(&format!(" {token}\n"))), Some(token));
        assert_eq!(public_token_or_none(None), None);
        assert_eq!(public_token_or_none(Some("")), None);
        assert_eq!(public_token_or_none(Some("pk.")), None);
        assert_eq!(public_token_or_none(Some("sk.eyJ1IjoiZXhhbXBsZSJ9.secret")), None, "secret tokens stay out");
        assert_eq!(public_token_or_none(Some("pk.abc&style=x")), None);
        assert_eq!(public_token_or_none(Some("pk.abc def")), None);
    }
}
