//! Turns `Action`s into state changes and API requests. API callbacks send an `Event`
//! into the channel drained by the app every frame (ARCHITECTURE §5.2).

use std::sync::mpsc::Sender;

use crate::actions::{Action, ApiResult, Event, ToastKind};
use crate::api::models::ReportTarget;
use crate::api::{ApiClient, ApiError, CampsiteFilter, Done, clean_search};
use crate::state::{AppState, CampsiteDetail, CampsiteForm, Panel, Remote, ReportForm, Search, TENT_CAPACITY_MAX};

pub struct Controller {
    api: ApiClient,
    tx: Sender<Event>,
    ctx: egui::Context,
}

impl Controller {
    pub fn new(api: ApiClient, tx: Sender<Event>, ctx: egui::Context) -> Self {
        Self { api, tx, ctx }
    }

    /// Callback that forwards an API result as an `Event` and wakes up the UI.
    fn done<T: 'static>(&self, wrap: impl FnOnce(ApiResult<T>) -> Event + Send + 'static) -> Done<T> {
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        Box::new(move |result| {
            // The receiver only disappears when the app is shutting down.
            let _ = tx.send(wrap(result));
            ctx.request_repaint();
        })
    }

    pub fn handle(&mut self, state: &mut AppState, action: Action) {
        self.api.set_token(state.session.as_ref().map(|s| s.token.as_str()));

        match action {
            Action::OpenPanel(panel) => self.request_panel(state, Some(panel)),
            Action::ClosePanel => self.request_panel(state, None),
            Action::DiscardChanges => {
                let target = state.confirm_discard.take().flatten();
                state.campsite_form = CampsiteForm::default();
                self.set_panel(state, target);
            }
            Action::KeepEditing => state.confirm_discard = None,
            Action::ToggleCollapse => state.panel_collapsed = !state.panel_collapsed,

            Action::RefreshCount => {
                // Keep the last count on screen while refreshing: no spinner, no layout shift.
                if !matches!(state.campsite_count, Remote::Loaded(_)) {
                    state.campsite_count = Remote::Loading;
                }
                self.api.count_campsites(self.done(Event::CampsiteCount));
            }
            Action::LoadTags => {
                state.tags = Remote::Loading;
                self.api.list_tags(self.done(Event::Tags));
            }
            Action::ViewportChanged(bbox) => {
                state.viewport = Some(bbox);
                self.handle(state, Action::RefreshMarkers);
            }
            Action::RefreshMarkers => {
                if let Some(bbox) = state.viewport {
                    state.markers_generation += 1;
                    state.markers_loading = true;
                    let generation = state.markers_generation;
                    self.api.list_campsites_in_bbox(
                        bbox,
                        &state.filters,
                        self.done(move |result| Event::Markers { generation, result }),
                    );
                }
            }
            Action::MapClicked { lat, lng } => {
                if state.is_campsite_form_open() {
                    let form = &mut state.campsite_form;
                    form.lat = Some(lat);
                    form.lng = Some(lng);
                    form.dirty = true;
                }
            }

            Action::FocusCampsite { id, lat, lng } => {
                state.map_focus = Some((lat, lng));
                self.request_panel(state, Some(Panel::Campsite(id)));
            }

            Action::Search => {
                let query = clean_search(&state.search.query);
                if query.is_empty() {
                    return self.handle(state, Action::ClearSearch);
                }
                state.search.submitted = query;
                self.run_search(state);
                self.request_panel(state, Some(Panel::Search));
            }
            Action::ClearSearch => {
                // Keep the generation so a response still in flight is dropped.
                state.search = Search { generation: state.search.generation + 1, ..Default::default() };
                if state.panel == Some(Panel::Search) {
                    self.set_panel(state, None);
                }
            }
            Action::ToggleTagFilter(id) => {
                if !state.filters.tags.remove(&id) {
                    state.filters.tags.insert(id);
                }
                self.filters_changed(state);
            }
            Action::SetTentRange { min, max } => {
                let min = min.clamp(1, TENT_CAPACITY_MAX);
                state.filters.min_tents = min;
                state.filters.max_tents = max.clamp(min, TENT_CAPACITY_MAX);
                self.filters_changed(state);
            }
            Action::ResetFilters => {
                state.filters = CampsiteFilter::default();
                self.filters_changed(state);
            }

            Action::RefreshSession => {
                if state.session.is_some() {
                    self.api.auth_refresh(self.done(Event::SessionRefreshed));
                }
            }
            Action::Login => {
                let f = &mut state.login;
                if f.email.trim().is_empty() || f.password.is_empty() {
                    f.error = Some(ApiError::validation(Default::default()));
                    return;
                }
                f.submitting = true;
                f.error = None;
                self.api.auth_with_password(f.email.trim(), &f.password, self.done(Event::LoggedIn));
            }
            Action::Register => {
                let f = &mut state.register;
                if f.password != f.confirm {
                    let fields = [("passwordConfirm".to_owned(), "Passwords don't match.".to_owned())].into();
                    f.error = Some(ApiError::validation(fields));
                    return;
                }
                f.submitting = true;
                f.error = None;
                let (email, password) = (f.email.trim().to_owned(), f.password.clone());
                let done = {
                    let (email, password) = (email.clone(), password.clone());
                    self.done(move |result| Event::Registered { email, password, result })
                };
                self.api.register(f.name.trim(), &email, &password, &f.confirm, done);
            }
            Action::RequestVerification { email } => {
                self.api.request_verification(&email, self.done(Event::VerificationRequested));
            }
            Action::Logout => {
                state.session = None;
                state.toast(ToastKind::Info, "You are logged out.");
                if state.panel.as_ref().is_some_and(Panel::requires_auth) {
                    state.campsite_form = CampsiteForm::default();
                    self.set_panel(state, None);
                }
                if let Some(d) = &mut state.detail {
                    d.my_rating = Remote::Loaded(None);
                }
                self.handle(state, Action::RefreshMarkers);
            }
            Action::SaveProfileName => {
                let Some(user_id) = state.user_id().map(str::to_owned) else { return };
                state.profile.saving_name = true;
                state.profile.name_error = None;
                self.api.update_name(&user_id, state.profile.name.trim(), self.done(Event::ProfileNameSaved));
            }
            Action::ChangePassword => {
                let Some(session) = &state.session else { return };
                let (user_id, email) = (session.user.id.clone(), session.user.email.clone());
                let f = &mut state.profile;
                if f.new_password != f.confirm {
                    let fields = [("passwordConfirm".to_owned(), "Passwords don't match.".to_owned())].into();
                    f.password_error = Some(ApiError::validation(fields));
                    return;
                }
                f.changing_password = true;
                f.password_error = None;
                let password = f.new_password.clone();
                self.api.change_password(
                    &user_id,
                    &f.old_password,
                    &f.new_password,
                    &f.confirm,
                    self.done(move |result| Event::PasswordChanged { email, password, result }),
                );
            }

            Action::SaveCampsite => self.save_campsite(state),
            Action::PickPhotos => {
                let slots = state.campsite_form.free_photo_slots();
                if state.is_campsite_form_open() && slots > 0 {
                    self.pick_photos(slots);
                }
            }
            Action::RemovePhoto(photo) => state.campsite_form.remove_photo(&photo),
            Action::DeleteCampsite(id) => {
                if let Some(d) = state.detail.as_mut().filter(|d| d.id == id) {
                    d.deleting = true;
                }
                let cb_id = id.clone();
                self.api.delete_campsite(&id, self.done(move |result| Event::CampsiteDeleted { id: cb_id, result }));
            }
            Action::RefreshStats(id) => {
                let cb_id = id.clone();
                self.api.get_stats(&id, self.done(move |result| Event::Stats { id: cb_id, result }));
            }
            Action::Rate(score) => {
                let (Some(user_id), Some(d)) = (state.user_id().map(str::to_owned), state.detail.as_mut()) else {
                    return;
                };
                d.rating_saving = true;
                let id = d.id.clone();
                let cb_id = id.clone();
                let done = self.done(move |result| Event::RatingSaved { id: cb_id, result });
                match d.my_rating.loaded().and_then(Option::as_ref) {
                    Some(existing) => self.api.update_rating(&existing.id, score, done),
                    None => self.api.create_rating(&id, &user_id, score, done),
                }
            }
            Action::PostComment => {
                let (Some(user_id), Some(d)) = (state.user_id().map(str::to_owned), state.detail.as_mut()) else {
                    return;
                };
                let body = d.comment_draft.trim().to_owned();
                if body.is_empty() {
                    return;
                }
                d.posting_comment = true;
                d.comment_error = None;
                let id = d.id.clone();
                let cb_id = id.clone();
                self.api.create_comment(
                    &id,
                    &user_id,
                    &body,
                    self.done(move |result| Event::CommentPosted { id: cb_id, result }),
                );
            }
            Action::DeleteComment(comment_id) => {
                let Some(d) = &state.detail else { return };
                let (id, cid) = (d.id.clone(), comment_id.clone());
                self.api.delete_comment(
                    &comment_id,
                    self.done(move |result| Event::CommentDeleted { id, comment_id: cid, result }),
                );
            }
            Action::LoadMoreComments => {
                let Some(d) = &mut state.detail else { return };
                if d.comments_loading || !d.has_more_comments() {
                    return;
                }
                let page = d.comments_page + 1;
                d.comments_loading = true;
                self.fetch_comments(&d.id.clone(), page);
            }
            Action::SubmitReport => {
                let (Some(user_id), Some(Panel::Report(target))) = (state.user_id().map(str::to_owned), &state.panel)
                else {
                    return;
                };
                let form = &mut state.report;
                if form.sending {
                    return;
                }
                let errors = form.validate();
                let Some(input) = form.to_input(user_id, target).filter(|_| errors.is_empty()) else {
                    form.error = Some(ApiError::validation(errors));
                    return;
                };
                form.sending = true;
                form.error = None;
                self.api.create_report(&input, self.done(Event::ReportSent));
            }

            Action::Toast(text, kind) => state.toast(kind, text),
        }
    }

    /// Filters apply to the markers and to the current search results.
    fn filters_changed(&mut self, state: &mut AppState) {
        self.handle(state, Action::RefreshMarkers);
        if state.search.is_active() {
            self.run_search(state);
        }
    }

    fn run_search(&mut self, state: &mut AppState) {
        let search = &mut state.search;
        search.generation += 1;
        search.results = Remote::Loading;
        let generation = search.generation;
        self.api.search_campsites(
            &search.submitted,
            &state.filters,
            self.done(move |result| Event::SearchResults { generation, result }),
        );
    }

    /// Switch panels, asking first if that would lose unsaved campsite form changes.
    fn request_panel(&mut self, state: &mut AppState, target: Option<Panel>) {
        if state.is_campsite_form_open() && state.campsite_form.dirty && state.panel != target {
            state.confirm_discard = Some(target);
            state.panel_collapsed = false;
            return;
        }
        self.set_panel(state, target);
    }

    fn set_panel(&mut self, state: &mut AppState, target: Option<Panel>) {
        state.confirm_discard = None;
        state.panel_collapsed = false;

        let target = match target {
            Some(p) if p.requires_auth() && state.session.is_none() => {
                state.after_login = Some(p);
                Some(Panel::Login)
            }
            other => other,
        };

        if !matches!(target, Some(Panel::Login | Panel::Register)) {
            state.after_login = None; // the login detour was abandoned
        }

        match &target {
            Some(Panel::Campsite(id)) => self.open_campsite(state, id),
            Some(Panel::NewCampsite) => {
                if state.panel != target {
                    state.campsite_form = CampsiteForm::default();
                    state.toast(ToastKind::Info, "Tip: click on the map to place your campsite.");
                }
            }
            Some(Panel::EditCampsite(id)) => {
                match state.detail.as_ref().filter(|d| &d.id == id).and_then(|d| d.campsite.loaded()) {
                    Some(c) => state.campsite_form = CampsiteForm::from_campsite(c),
                    None => return self.set_panel(state, Some(Panel::Campsite(id.clone()))),
                }
            }
            Some(Panel::Profile) => {
                state.profile = Default::default();
                state.profile.name = state.session.as_ref().map(|s| s.user.name.clone()).unwrap_or_default();
            }
            Some(Panel::Login) => {
                state.login.error = None;
                // Logging in from a campsite ("Log in to rate") comes back to it.
                if let (None, Some(Panel::Campsite(id))) = (&state.after_login, &state.panel) {
                    state.after_login = Some(Panel::Campsite(id.clone()));
                }
            }
            Some(Panel::Register) => state.register.error = None,
            Some(Panel::Report(t)) => {
                // Go back to the campsite afterwards: the reported one, or the one holding the comment.
                let campsite_id = match t {
                    ReportTarget::Campsite(id) => id.clone(),
                    ReportTarget::Comment(_) => state.detail.as_ref().map(|d| d.id.clone()).unwrap_or_default(),
                };
                state.report = ReportForm::new(campsite_id);
            }
            Some(Panel::Search | Panel::Filters) | None => {}
        }

        // The Report panel shows a preview of the target from the open campsite.
        if !matches!(target, Some(Panel::Campsite(_) | Panel::EditCampsite(_) | Panel::Report(_))) {
            state.detail = None;
        }
        state.panel = target;
    }

    fn open_campsite(&mut self, state: &mut AppState, id: &str) {
        let mut d = CampsiteDetail::new(id.to_owned());
        d.campsite = Remote::Loading;
        d.stats = Remote::Loading;
        d.comments_loading = true;

        let cb = id.to_owned();
        self.api.get_campsite(id, self.done(move |result| Event::Campsite { id: cb, result }));
        let cb = id.to_owned();
        self.api.get_stats(id, self.done(move |result| Event::Stats { id: cb, result }));
        self.fetch_comments(id, 1);

        match state.user_id() {
            Some(user_id) => {
                d.my_rating = Remote::Loading;
                let cb = id.to_owned();
                self.api.my_rating(id, user_id, self.done(move |result| Event::MyRating { id: cb, result }));
            }
            None => d.my_rating = Remote::Loaded(None),
        }
        state.detail = Some(d);
    }

    fn fetch_comments(&self, id: &str, page: u32) {
        let cb = id.to_owned();
        self.api.list_comments(id, page, self.done(move |result| Event::Comments { id: cb, page, result }));
    }

    fn save_campsite(&mut self, state: &mut AppState) {
        let Some(user_id) = state.user_id().map(str::to_owned) else { return };
        let form = &mut state.campsite_form;
        let errors = form.validate();
        if !errors.is_empty() {
            form.error = Some(ApiError::validation(errors));
            return;
        }
        let editing = form.editing.clone();
        let Some(input) = form.to_input(editing.is_none().then_some(user_id)) else { return };
        let photos = form.uploads();
        form.submitting = true;
        form.error = None;
        match editing {
            Some(id) => self.api.update_campsite(
                &id,
                &input,
                &photos,
                self.done(|result| Event::CampsiteSaved { created: false, result }),
            ),
            None => self.api.create_campsite(
                &input,
                &photos,
                self.done(|result| Event::CampsiteSaved { created: true, result }),
            ),
        }
    }

    /// The picked files come back as `Event::PhotosPicked`, like an API result.
    fn pick_photos(&self, max: usize) {
        // The picker is a browser API; host builds (unit tests) have no file dialog.
        #[cfg(target_arch = "wasm32")]
        {
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            crate::web::pick_photos(max, move |picked| {
                let _ = tx.send(Event::PhotosPicked(picked));
                ctx.request_repaint();
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = max;
    }
}
