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

fn trim_base(url: &str) -> &str {
    url.trim().trim_end_matches('/')
}

/// The value goes into a query string: anything but letters, digits, `.`, `_` and `-` is refused.
fn url_safe_or_dev(id: Option<&str>) -> &str {
    id.map(str::trim)
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)))
        .unwrap_or("dev")
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
}
