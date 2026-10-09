//! Pure event handling: `apply` updates the state from a network result and returns
//! follow-up actions for the controller (e.g. "log in" after a successful signup).
//! No I/O here, so it is unit-tested on the host.

use crate::actions::{Action, Event, ToastKind};
use crate::api::ApiError;
use crate::api::models::Session;
use crate::documents;

use super::{AppState, CampsiteForm, Panel, Remote};

pub fn apply(state: &mut AppState, event: Event) -> Vec<Action> {
    match event {
        Event::CampsiteCount(r) => state.campsite_count = r.into(),
        Event::Tags(r) => state.tags = r.into(),

        Event::Markers { generation, result } => {
            if generation != state.markers_generation {
                return vec![]; // a newer viewport request is in flight
            }
            state.markers_loading = false;
            match result {
                Ok(markers) => state.markers = markers,
                Err(e) => state.toast(ToastKind::Error, format!("Could not load campsites: {e}")),
            }
        }

        Event::SearchResults { generation, result } => {
            if generation != state.search.generation {
                return vec![]; // superseded by a newer query or filter change
            }
            match result {
                Ok(list) => {
                    state.search.total = list.total_items;
                    state.search.results = Remote::Loaded(list.items);
                }
                Err(e) => state.search.results = Remote::Failed(e),
            }
        }

        Event::Campsite { id, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.campsite = result.into();
            }
        }
        Event::Stats { id, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.stats = result.into();
            }
        }
        Event::MyRating { id, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.my_rating = result.into();
            }
        }
        Event::Comments { id, page, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.comments_loading = false;
                match result {
                    Ok(list) => {
                        if page <= 1 {
                            d.comments = list.items;
                        } else {
                            d.comments.extend(list.items);
                        }
                        d.comments_page = page;
                        d.comments_total_pages = list.total_pages;
                        d.comments_error = None;
                    }
                    Err(e) => d.comments_error = Some(e),
                }
            }
        }

        Event::LoggedIn(result) => {
            state.login.submitting = false;
            match result {
                Ok(auth) => {
                    let session = Session::from(auth);
                    state.toast(ToastKind::Success, format!("Welcome, {}!", session.user.display_name()));
                    state.session = Some(session);
                    state.login = Default::default();
                    let next = state.after_login.take();
                    let mut follow = vec![Action::RefreshMarkers];
                    follow.push(match next {
                        Some(panel) => Action::OpenPanel(panel),
                        None => Action::ClosePanel,
                    });
                    return follow;
                }
                Err(e) => {
                    state.login.password.clear();
                    state.login.error = Some(e);
                }
            }
        }

        Event::SessionRefreshed(result) => match result {
            Ok(auth) => state.session = Some(auth.into()),
            // Network trouble: keep the session and try again next time.
            Err(e) if e.status == 0 => log::warn!("session refresh failed: {e}"),
            Err(_) => {
                state.session = None;
                state.toast(ToastKind::Info, "Your session has expired. Please log in again.");
            }
        },

        Event::Registered { email, password, result } => {
            state.register.submitting = false;
            match result {
                Ok(_) => {
                    state.register = Default::default();
                    state.login.email = email.clone();
                    state.login.password = password;
                    return vec![Action::Login, Action::RequestVerification { email }];
                }
                Err(e) => state.register.error = Some(e),
            }
        }

        Event::VerificationRequested(result) => match result {
            Ok(()) => state.toast(ToastKind::Info, "We sent you a confirmation email. Open its link to start posting."),
            Err(e) => state.toast(ToastKind::Error, format!("Couldn't send the confirmation email: {e}")),
        },

        Event::ProfileNameSaved(result) => {
            state.profile.saving_name = false;
            match result {
                Ok(user) => {
                    if let Some(s) = &mut state.session {
                        s.user.name = user.name;
                    }
                    state.profile.name_error = None;
                    state.toast(ToastKind::Success, "Profile saved.");
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        state.profile.name_error = Some(e);
                        vec![]
                    });
                }
            }
        }

        Event::PasswordChanged { email, password, result } => {
            state.profile.changing_password = false;
            match result {
                Ok(_) => {
                    // PocketBase revoked the old token: log in again with the new password.
                    state.profile.old_password.clear();
                    state.profile.new_password.clear();
                    state.profile.confirm.clear();
                    state.profile.password_error = None;
                    state.toast(ToastKind::Success, "Password changed.");
                    state.login.email = email;
                    state.login.password = password;
                    state.after_login = Some(Panel::Profile);
                    return vec![Action::Login];
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        state.profile.password_error = Some(e);
                        vec![]
                    });
                }
            }
        }

        Event::PhotosPicked(picked) => {
            if !state.is_campsite_form_open() {
                return vec![]; // the form was closed while the files were being read
            }
            let problems = state.campsite_form.add_picked(picked, &mut state.photo_seq);
            for problem in problems {
                state.toast(ToastKind::Error, problem);
            }
        }

        Event::Located(result) => {
            state.locate.pending = false;
            let automatic = std::mem::take(&mut state.locate.automatic);
            match result {
                Ok(location) => {
                    state.locate.found = Some(location);
                    state.map_focus = Some((location.lat, location.lng));
                }
                Err(e) => {
                    state.locate.found = None;
                    // On page load nobody clicked: the map just stays where it is.
                    if !automatic {
                        state.toast(ToastKind::Error, e.message());
                    }
                }
            }
        }
        Event::LocationPermission(granted) => {
            // Skip it if the visitor clicked "Locate me" in the meantime.
            if granted && !state.locate.pending && state.locate.found.is_none() {
                state.locate.automatic = true;
                return vec![Action::Locate];
            }
        }

        Event::CampsiteSaved { created, result } => {
            state.campsite_form.submitting = false;
            match result {
                Ok(c) => {
                    state.campsite_form = CampsiteForm::default();
                    state.toast(ToastKind::Success, if created { "Campsite added!" } else { "Campsite updated." });
                    let mut follow = vec![Action::RefreshMarkers];
                    if created {
                        follow.push(Action::RefreshCount);
                    }
                    follow.push(Action::OpenPanel(Panel::Campsite(c.id)));
                    return follow;
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        state.campsite_form.error = Some(e);
                        vec![]
                    });
                }
            }
        }

        Event::CampsiteDeleted { id, result } => match result {
            Ok(()) => {
                if state.selected_campsite() == Some(id.as_str()) {
                    state.panel = None;
                    state.detail = None;
                }
                state.toast(ToastKind::Success, "Campsite deleted.");
                return vec![Action::RefreshMarkers, Action::RefreshCount];
            }
            Err(e) => {
                if let Some(d) = detail_for(state, &id) {
                    d.deleting = false;
                    d.confirm_delete = false;
                }
                return auth_error(state, e).unwrap_or_else(|e| {
                    state.toast(ToastKind::Error, format!("Could not delete: {e}"));
                    vec![]
                });
            }
        },

        Event::RatingSaved { id, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.rating_saving = false;
            }
            match result {
                Ok(r) => {
                    if let Some(d) = detail_for(state, &id) {
                        d.my_rating = Remote::Loaded(Some(r));
                    }
                    return vec![Action::RefreshStats(id)];
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        state.toast(ToastKind::Error, format!("Could not save rating: {e}"));
                        vec![]
                    });
                }
            }
        }

        Event::CommentPosted { id, result } => {
            if let Some(d) = detail_for(state, &id) {
                d.posting_comment = false;
            }
            match result {
                Ok(c) => {
                    if let Some(d) = detail_for(state, &id) {
                        d.comments.insert(0, c);
                        d.comment_draft.clear();
                        d.comment_error = None;
                    }
                    return vec![Action::RefreshStats(id)];
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        if let Some(d) = detail_for(state, &id) {
                            d.comment_error = Some(e);
                        }
                        vec![]
                    });
                }
            }
        }

        Event::CommentDeleted { id, comment_id, result } => match result {
            Ok(()) => {
                if let Some(d) = detail_for(state, &id) {
                    d.comments.retain(|c| c.id != comment_id);
                }
                return vec![Action::RefreshStats(id)];
            }
            Err(e) => {
                return auth_error(state, e).unwrap_or_else(|e| {
                    state.toast(ToastKind::Error, format!("Could not delete comment: {e}"));
                    vec![]
                });
            }
        },

        Event::ReportSent(result) => {
            state.report.sending = false;
            match result {
                Ok(_) => {
                    state.toast(ToastKind::Success, "Thanks — an admin will review your report.");
                    // Back to the campsite, unless the user already went somewhere else.
                    if matches!(state.panel, Some(Panel::Report(_))) {
                        let id = std::mem::take(&mut state.report.campsite_id);
                        return vec![if id.is_empty() {
                            Action::ClosePanel
                        } else {
                            Action::OpenPanel(Panel::Campsite(id))
                        }];
                    }
                }
                Err(e) => {
                    return auth_error(state, e).unwrap_or_else(|e| {
                        state.report.error = Some(e);
                        vec![]
                    });
                }
            }
        }

        Event::Document { id, result } => {
            let doc = result.map(|text| documents::manifest().prepare(&text));
            state.documents.insert(id, doc.into());
        }
    }
    vec![]
}

fn detail_for<'a>(state: &'a mut AppState, id: &str) -> Option<&'a mut super::CampsiteDetail> {
    state.detail.as_mut().filter(|d| d.id == id)
}

/// A 401 on an authenticated call means the session is gone: log out and ask to log in,
/// coming back to the current panel afterwards. Other errors are handed back.
fn auth_error(state: &mut AppState, e: ApiError) -> Result<Vec<Action>, ApiError> {
    if !e.is_unauthorized() {
        return Err(e);
    }
    state.session = None;
    state.after_login = state.panel.take();
    state.toast(ToastKind::Info, "Your session has expired. Please log in again.");
    Ok(vec![Action::OpenPanel(Panel::Login)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{AuthResponse, CampsiteMarker, User};
    use crate::state::CampsiteDetail;

    fn user() -> User {
        User {
            id: "u1".into(),
            name: "Alice".into(),
            email: "a@example.com".into(),
            role: String::new(),
            verified: true,
        }
    }

    fn logged_in() -> AppState {
        AppState { session: Some(Session { token: "t".into(), user: user() }), ..Default::default() }
    }

    fn marker(id: &str) -> CampsiteMarker {
        CampsiteMarker { id: id.into(), title: "x".into(), lat: 1.0, lng: 2.0, author: "u2".into() }
    }

    #[test]
    fn stale_marker_responses_are_dropped() {
        let mut s = AppState { markers_generation: 2, markers_loading: true, ..Default::default() };
        apply(&mut s, Event::Markers { generation: 1, result: Ok(vec![marker("old")]) });
        assert!(s.markers.is_empty());
        assert!(s.markers_loading);
        apply(&mut s, Event::Markers { generation: 2, result: Ok(vec![marker("new")]) });
        assert_eq!(s.markers[0].id, "new");
        assert!(!s.markers_loading);
    }

    #[test]
    fn stale_search_results_are_dropped() {
        use crate::api::models::ListResponse;
        let list = |id: &str| ListResponse { page: 1, total_items: 7, total_pages: 1, items: vec![marker(id)] };
        let mut s = AppState::default();
        s.search.generation = 2;
        s.search.results = Remote::Loading;
        apply(&mut s, Event::SearchResults { generation: 1, result: Ok(list("old")) });
        assert!(s.search.results.is_loading());
        apply(&mut s, Event::SearchResults { generation: 2, result: Ok(list("new")) });
        assert_eq!(s.search.results.loaded().map(|r| r[0].id.as_str()), Some("new"));
        assert_eq!(s.search.total, 7);
    }

    #[test]
    fn login_opens_the_pending_panel() {
        let mut s = AppState { after_login: Some(Panel::NewCampsite), ..Default::default() };
        let follow = apply(&mut s, Event::LoggedIn(Ok(AuthResponse { token: "t".into(), record: user() })));
        assert_eq!(s.user_id(), Some("u1"));
        assert!(follow.contains(&Action::OpenPanel(Panel::NewCampsite)));
        assert!(s.after_login.is_none());
    }

    #[test]
    fn sent_report_goes_back_to_the_campsite() {
        use crate::api::models::{Report, ReportTarget};
        let mut s = logged_in();
        s.panel = Some(Panel::Report(ReportTarget::Comment("k1".into())));
        s.report = crate::state::ReportForm { campsite_id: "c1".into(), sending: true, ..Default::default() };
        let follow = apply(&mut s, Event::ReportSent(Ok(Report { id: "r1".into() })));
        assert!(!s.report.sending);
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(follow, vec![Action::OpenPanel(Panel::Campsite("c1".into()))]);
    }

    #[test]
    fn report_error_stays_on_the_form() {
        use crate::api::models::ReportTarget;
        let mut s = logged_in();
        s.panel = Some(Panel::Report(ReportTarget::Campsite("c1".into())));
        s.report.sending = true;
        let body = br#"{"status":400,"message":"You already reported this.","data":{}}"#;
        let follow = apply(&mut s, Event::ReportSent(Err(ApiError::from_response(400, body))));
        assert!(follow.is_empty());
        assert!(!s.report.sending);
        assert_eq!(s.report.error.as_ref().map(|e| e.message.as_str()), Some("You already reported this."));
        assert_eq!(s.panel, Some(Panel::Report(ReportTarget::Campsite("c1".into()))));
    }

    #[test]
    fn signup_logs_in_automatically() {
        let mut s = AppState::default();
        let follow = apply(
            &mut s,
            Event::Registered { email: "a@example.com".into(), password: "pw".into(), result: Ok(user()) },
        );
        assert_eq!(follow, vec![Action::Login, Action::RequestVerification { email: "a@example.com".into() }]);
        assert_eq!(s.login.email, "a@example.com");
    }

    #[test]
    fn verification_request_result_is_a_toast() {
        let mut s = logged_in();
        apply(&mut s, Event::VerificationRequested(Ok(())));
        apply(&mut s, Event::VerificationRequested(Err(ApiError::network("offline"))));
        let kinds: Vec<_> = s.toasts.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![ToastKind::Info, ToastKind::Error]);
        assert!(s.session.is_some(), "a failed email request doesn't log out");
    }

    #[test]
    fn needs_verification_only_when_logged_in_and_unconfirmed() {
        let mut s = logged_in();
        assert!(!s.needs_verification());
        if let Some(session) = &mut s.session {
            session.user.verified = false;
        }
        assert!(s.needs_verification());
        assert!(!AppState::default().needs_verification());
    }

    #[test]
    fn unauthorized_logs_out_and_returns_to_panel() {
        let mut s = logged_in();
        s.panel = Some(Panel::NewCampsite);
        let follow =
            apply(&mut s, Event::CampsiteSaved { created: true, result: Err(ApiError::from_response(401, b"{}")) });
        assert!(s.session.is_none());
        assert_eq!(s.after_login, Some(Panel::NewCampsite));
        assert_eq!(follow, vec![Action::OpenPanel(Panel::Login)]);
    }

    #[test]
    fn validation_error_stays_on_the_form() {
        let mut s = logged_in();
        s.campsite_form.submitting = true;
        let e = ApiError::from_response(400, br#"{"message":"Failed","data":{"title":{"message":"too short"}}}"#);
        apply(&mut s, Event::CampsiteSaved { created: true, result: Err(e) });
        assert!(!s.campsite_form.submitting);
        assert_eq!(s.campsite_form.error.as_ref().and_then(|e| e.field("title")), Some("too short"));
        assert!(s.session.is_some());
    }

    #[test]
    fn events_for_another_campsite_are_ignored() {
        let mut s = AppState { detail: Some(CampsiteDetail::new("a".into())), ..Default::default() };
        apply(&mut s, Event::Stats { id: "b".into(), result: Ok(None) });
        assert_eq!(s.detail.as_ref().map(|d| &d.stats), Some(&Remote::NotAsked));
    }

    #[test]
    fn picked_photos_go_to_the_open_form_and_problems_are_toasted() {
        use crate::state::PickedPhoto;
        let photo = |name: &str, mime: &str| PickedPhoto {
            name: name.into(),
            mime: mime.into(),
            original_size: 3,
            bytes: vec![1, 2, 3].into(),
            preview: None,
            problem: None,
        };
        let mut s = logged_in();
        s.photo_seq = 41;
        apply(&mut s, Event::PhotosPicked(vec![photo("a.jpg", "image/jpeg")]));
        assert!(s.campsite_form.new_photos.is_empty(), "no form open: dropped");

        s.panel = Some(Panel::NewCampsite);
        apply(&mut s, Event::PhotosPicked(vec![photo("a.jpg", "image/jpeg"), photo("b.gif", "image/gif")]));
        assert_eq!(s.campsite_form.new_photos.len(), 1);
        assert_eq!(s.campsite_form.new_photos[0].id, 42);
        assert_eq!(s.photo_seq, 42);
        assert!(s.campsite_form.dirty);
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].kind, ToastKind::Error);
    }

    #[test]
    fn documents_are_parsed_with_their_vars() {
        let mut s = AppState::default();
        apply(&mut s, Event::Document { id: "terms".into(), result: Ok("# {{site_name}}".into()) });
        let name = crate::documents::manifest().var("site_name").unwrap_or_default().to_owned();
        let Some(Remote::Loaded(blocks)) = s.documents.get("terms") else { panic!("{:?}", s.documents) };
        assert!(matches!(&blocks[..], [crate::documents::Block::Heading(1, spans)] if spans[0].text == name));

        apply(&mut s, Event::Document { id: "terms".into(), result: Err(ApiError::network("offline")) });
        assert!(matches!(s.documents.get("terms"), Some(Remote::Failed(_))));
    }

    #[test]
    fn deleting_the_open_campsite_closes_the_panel() {
        let mut s = logged_in();
        s.panel = Some(Panel::Campsite("a".into()));
        s.detail = Some(CampsiteDetail::new("a".into()));
        let follow = apply(&mut s, Event::CampsiteDeleted { id: "a".into(), result: Ok(()) });
        assert!(s.panel.is_none());
        assert!(follow.contains(&Action::RefreshCount));
    }

    #[test]
    fn a_found_position_centers_the_map_and_a_failure_clears_it() {
        use crate::state::{LocateError, MyLocation};

        let mut s = AppState::default();
        s.locate.pending = true;
        let here = MyLocation { lat: 45.0, lng: 6.0, accuracy_m: 30.0 };
        apply(&mut s, Event::Located(Ok(here)));
        assert!(!s.locate.pending);
        assert_eq!(s.locate.found, Some(here));
        assert_eq!(s.map_focus, Some((45.0, 6.0)));

        s.locate.pending = true;
        s.map_focus = None;
        apply(&mut s, Event::Located(Err(LocateError::Denied)));
        assert!(!s.locate.pending);
        assert_eq!(s.locate.found, None, "no stale dot after a failed request");
        assert_eq!(s.map_focus, None);
        assert_eq!(s.toasts.last().map(|t| t.kind), Some(ToastKind::Error));
    }

    #[test]
    fn page_load_locates_only_when_access_was_already_granted() {
        let mut s = AppState::default();
        assert!(apply(&mut s, Event::LocationPermission(false)).is_empty());
        assert!(!s.locate.automatic);

        assert_eq!(apply(&mut s, Event::LocationPermission(true)), vec![Action::Locate]);
        assert!(s.locate.automatic);

        // The visitor clicked "Locate me" before the check came back: no second request.
        let mut s = AppState::default();
        s.locate.pending = true;
        assert!(apply(&mut s, Event::LocationPermission(true)).is_empty());
        assert!(!s.locate.automatic);
    }

    #[test]
    fn a_failed_automatic_locate_is_silent() {
        use crate::state::LocateError;

        let mut s = AppState::default();
        s.locate.pending = true;
        s.locate.automatic = true;
        apply(&mut s, Event::Located(Err(LocateError::Timeout)));
        assert!(s.toasts.is_empty());
        assert!(!s.locate.automatic, "the next click reports its errors");
        assert!(!s.locate.pending);
    }
}
