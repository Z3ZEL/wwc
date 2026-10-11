//! Audience measurement with Umami Cloud, only with the visitor's consent (ARCHITECTURE §5.12,
//! ADR 0022). This module decides what is sent; `web::analytics` loads the script and sends it.
//!
//! Nothing is sent before the visitor accepts the `analytics` purpose, and nothing after they
//! withdraw it. Events carry no personal data: no ids, search text, emails or positions.

use crate::actions::{Action, Event};
use crate::state::{AppState, Panel};

/// The consent purpose (`assets/consent.json`) that analytics need.
pub const PURPOSE: &str = "analytics";

/// A custom event: a fixed name and at most one small, non-personal property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackEvent {
    pub name: &'static str,
    pub prop: Option<(&'static str, Prop)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prop {
    Text(&'static str),
    Count(u32),
}

impl TrackEvent {
    const fn named(name: &'static str) -> Self {
        Self { name, prop: None }
    }

    const fn with(name: &'static str, key: &'static str, value: Prop) -> Self {
        Self { name, prop: Some((key, value)) }
    }
}

/// What the browser side has to do after a consent change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Load the script (once) and count the visit.
    Start,
    /// Stop sending. The script stays loaded but sends nothing on its own (auto-track is off).
    Stop,
}

/// Whether events may be sent right now. Follows `AppState::consent_allows(PURPOSE)`.
#[derive(Debug, Default)]
pub struct Analytics {
    active: bool,
}

impl Analytics {
    /// Call at startup and after every consent change.
    pub fn sync(&mut self, allowed: bool) -> Option<Change> {
        if self.active == allowed {
            return None;
        }
        self.active = allowed;
        Some(if allowed { Change::Start } else { Change::Stop })
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// The event to send for an action the visitor took in the UI, given the state before it
    /// is handled. Actions the app emits by itself (after an event) don't count.
    pub fn for_action(&self, state: &AppState, action: &Action) -> Option<TrackEvent> {
        if !self.active {
            return None;
        }
        // Reopening the campsite already shown (Retry, Cancel, back from a report) isn't a new view.
        let new_campsite = |id: &String| state.detail.as_ref().is_none_or(|d| &d.id != id);
        match action {
            Action::OpenPanel(Panel::Campsite(id)) if new_campsite(id) => {
                Some(TrackEvent::with("campsite-open", "from", Prop::Text("map")))
            }
            Action::FocusCampsite { id, .. } if new_campsite(id) => {
                Some(TrackEvent::with("campsite-open", "from", Prop::Text("search")))
            }
            Action::Search => Some(TrackEvent::named("search")),
            Action::ToggleTagFilter(_) => Some(TrackEvent::with("filter", "kind", Prop::Text("tag"))),
            Action::SetTentRange { .. } => Some(TrackEvent::with("filter", "kind", Prop::Text("tents"))),
            Action::Locate => Some(TrackEvent::named("locate")),
            _ => None,
        }
    }

    /// The event to send for a result, when it means the visitor completed something.
    pub fn for_event(&self, event: &Event) -> Option<TrackEvent> {
        if !self.active {
            return None;
        }
        match event {
            Event::Registered { result: Ok(_), .. } => Some(TrackEvent::named("signup")),
            Event::CampsiteSaved { created: true, result: Ok(_) } => Some(TrackEvent::named("campsite-create")),
            Event::PhotosPicked(photos) => {
                let added = photos.iter().filter(|p| p.problem.is_none()).count();
                (added > 0).then(|| {
                    TrackEvent::with("photos-add", "count", Prop::Count(u32::try_from(added).unwrap_or(u32::MAX)))
                })
            }
            Event::RatingSaved { result: Ok(_), .. } => Some(TrackEvent::named("rating")),
            Event::CommentPosted { result: Ok(_), .. } => Some(TrackEvent::named("comment")),
            Event::ReportSent(Ok(_)) => Some(TrackEvent::named("report")),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiError;
    use crate::api::models::Report;
    use crate::state::CampsiteDetail;

    fn report() -> Report {
        Report { id: "r1".into() }
    }

    fn active() -> Analytics {
        let mut a = Analytics::default();
        assert_eq!(a.sync(true), Some(Change::Start));
        a
    }

    #[test]
    fn starts_once_and_stops_on_withdrawal() {
        let mut a = Analytics::default();
        assert_eq!(a.sync(false), None, "nothing to stop before consent");
        assert_eq!(a.sync(true), Some(Change::Start));
        assert_eq!(a.sync(true), None, "already started");
        assert_eq!(a.sync(false), Some(Change::Stop));
        assert!(!a.active());
        assert_eq!(a.sync(true), Some(Change::Start), "accepted again");
    }

    #[test]
    fn nothing_is_sent_without_consent() {
        let mut a = Analytics::default();
        let state = AppState::default();
        assert_eq!(a.for_action(&state, &Action::Search), None);
        assert_eq!(a.for_event(&Event::ReportSent(Ok(report()))), None);

        a.sync(true);
        assert!(a.for_action(&state, &Action::Search).is_some());
        a.sync(false);
        assert_eq!(a.for_action(&state, &Action::Search), None, "withdrawn");
    }

    #[test]
    fn campsite_views_count_once() {
        let a = active();
        let mut state = AppState::default();
        let open = Action::OpenPanel(Panel::Campsite("c1".into()));
        assert_eq!(a.for_action(&state, &open), Some(TrackEvent::with("campsite-open", "from", Prop::Text("map"))));
        let focus = Action::FocusCampsite { id: "c2".into(), lat: 0.0, lng: 0.0 };
        assert_eq!(a.for_action(&state, &focus), Some(TrackEvent::with("campsite-open", "from", Prop::Text("search"))));

        state.detail = Some(CampsiteDetail::new("c1".into()));
        assert_eq!(a.for_action(&state, &open), None, "already open (Retry, Cancel, back)");
        assert!(a.for_action(&state, &focus).is_some(), "another campsite");
    }

    #[test]
    fn only_completed_actions_count() {
        let a = active();
        assert_eq!(a.for_event(&Event::ReportSent(Ok(report()))), Some(TrackEvent::named("report")));
        assert_eq!(a.for_event(&Event::ReportSent(Err(ApiError::validation(Default::default())))), None);
        assert_eq!(a.for_event(&Event::PhotosPicked(vec![])), None, "nothing added");
        assert_eq!(a.for_action(&AppState::default(), &Action::Login), None, "not measured");
    }
}
