use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

/// An error from PocketBase (or the network), shaped for display.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    /// HTTP status, or 0 for network / decoding failures.
    pub status: u16,
    pub message: String,
    /// Per-field validation messages, keyed by field name.
    pub fields: BTreeMap<String, String>,
}

impl ApiError {
    pub fn network(message: impl Into<String>) -> Self {
        Self { status: 0, message: message.into(), fields: BTreeMap::new() }
    }

    /// Client-side validation failure, displayed like a PocketBase 400.
    pub fn validation(fields: BTreeMap<String, String>) -> Self {
        Self { status: 400, message: "Please fix the highlighted fields.".into(), fields }
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == 401
    }

    pub fn is_not_found(&self) -> bool {
        self.status == 404
    }

    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    /// Parse PocketBase's `{status, message, data: {field: {code, message}}}` error body.
    pub fn from_response(status: u16, body: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct FieldError {
            message: String,
        }
        #[derive(Deserialize)]
        struct Body {
            #[serde(default)]
            message: String,
            #[serde(default)]
            data: BTreeMap<String, FieldError>,
        }

        match serde_json::from_slice::<Body>(body) {
            Ok(b) => Self {
                status,
                message: if b.message.is_empty() { format!("Request failed ({status})") } else { b.message },
                fields: b.data.into_iter().map(|(k, v)| (k, v.message)).collect(),
            },
            Err(_) => Self { status, message: format!("Request failed ({status})"), fields: BTreeMap::new() },
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.status == 0 { write!(f, "Network error: {}", self.message) } else { f.write_str(&self.message) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_errors() {
        let body = br#"{"status":400,"message":"Failed to create record.","data":{"title":{"code":"validation_length_out_of_range","message":"Must be between 3 and 100."}}}"#;
        let e = ApiError::from_response(400, body);
        assert_eq!(e.message, "Failed to create record.");
        assert_eq!(e.field("title"), Some("Must be between 3 and 100."));
    }

    #[test]
    fn non_json_body_still_gives_a_message() {
        let e = ApiError::from_response(502, b"Bad gateway");
        assert_eq!(e.status, 502);
        assert!(e.message.contains("502"));
    }
}
