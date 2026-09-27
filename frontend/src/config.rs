//! Build-time configuration (ARCHITECTURE §6). Values are baked in by `trunk build`.

/// PocketBase base URL from `WWC_API_URL`, without a trailing `/`.
/// Empty = same origin (dev: Trunk proxies `/api` to PocketBase).
pub fn api_url() -> &'static str {
    trim_base(option_env!("WWC_API_URL").unwrap_or(""))
}

fn trim_base(url: &str) -> &str {
    url.trim().trim_end_matches('/')
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
}
