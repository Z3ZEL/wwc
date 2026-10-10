//! The visitor's position, for the "Locate me" button at the bottom-right of the map
//! (ARCHITECTURE §5.5, ADR 0019). It only lives in memory, to center the map and draw the
//! "you are here" dot: it is never sent to the server and never saved.

/// A position given by the browser's Geolocation API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MyLocation {
    pub lat: f64,
    pub lng: f64,
    /// Radius of the circle the visitor is in (95 % confidence), in meters.
    pub accuracy_m: f64,
}

/// Why the browser gave no position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocateError {
    /// The visitor, or a browser setting, refused location access.
    Denied,
    /// The device couldn't find its position (location services off, no signal…).
    Unavailable,
    Timeout,
    /// No Geolocation API: an old browser, or a page not served over HTTPS.
    Unsupported,
}

impl LocateError {
    /// From `GeolocationPositionError.code`.
    pub fn from_code(code: u16) -> Self {
        match code {
            1 => Self::Denied,
            3 => Self::Timeout,
            _ => Self::Unavailable,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Denied => {
                "Location access is blocked. Allow it in your browser's site settings to see where you are."
            }
            Self::Unavailable => "Your position couldn't be found. Check that location services are on.",
            Self::Timeout => "Finding your position took too long. Try again.",
            Self::Unsupported => "Your browser can't share your position with this site.",
        }
    }
}

/// State of the "Locate me" button.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Locate {
    /// Waiting for the browser, which may be asking the visitor for permission.
    pub pending: bool,
    /// The pending request was started on page load because access was already allowed, not by
    /// a click: a failure isn't worth a toast the visitor didn't ask for.
    pub automatic: bool,
    /// The last position found. Cleared when a new request fails, so the dot is never stale.
    pub found: Option<MyLocation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_follow_the_geolocation_spec() {
        assert_eq!(LocateError::from_code(1), LocateError::Denied);
        assert_eq!(LocateError::from_code(2), LocateError::Unavailable);
        assert_eq!(LocateError::from_code(3), LocateError::Timeout);
        assert_eq!(LocateError::from_code(42), LocateError::Unavailable);
    }
}
