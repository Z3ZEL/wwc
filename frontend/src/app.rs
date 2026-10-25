//! The eframe app: drains network events, draws the page, then handles the actions
//! collected during the frame (ARCHITECTURE §5.2).

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::actions::{Action, Event};
use crate::analytics::{self, Analytics, Change};
use crate::api::ApiClient;
use crate::api::models::Session;
use crate::config;
use crate::consent::{self, ConsentRecord};
use crate::controller::Controller;
use crate::documents;
use crate::map::MapView;
use crate::seo::SeoTitles;
use crate::state::{self, AppState};
use crate::ui::theme::Theme;
use crate::ui::{
    consent_panel, footer, layer_button, locate_button, notice, panels, photo_viewer, top_bar, welcome_card, widgets,
};

/// Browser storage keys (localStorage), all listed in the Privacy Policy. `wwc_consent` is
/// only used while consent is enabled (§5.12).
const SESSION_KEY: &str = "wwc_session";
const NOTICE_KEY: &str = "wwc_notice";
const CONSENT_KEY: &str = "wwc_consent";

pub struct WwcApp {
    state: AppState,
    theme: Theme,
    controller: Controller,
    events: Receiver<Event>,
    map: MapView,
    seo: SeoTitles,
    /// Last value written to `document.title`.
    page_title: String,
    /// Whether the text agent was last switched to a password input (ADR 0018).
    password_keyboard: bool,
    /// Where the welcome card was drawn last frame: the map's zoom buttons go on its right.
    welcome_card: Option<egui::Rect>,
    /// Umami analytics, while the visitor allows them (§5.12).
    analytics: Analytics,
}

impl WwcApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::load();
        theme.apply(&cc.egui_ctx);
        // Campsite photos: `egui::Image` fetches http(s) URLs and decodes JPEG/PNG/WebP (ADR 0010).
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let (tx, events) = mpsc::channel();
        // Empty API URL: same origin, /api is proxied to PocketBase (dev).
        let api_url = config::api_url();
        let controller = Controller::new(ApiClient::new(api_url), tx, cc.egui_ctx.clone());
        let api_base = match api_url {
            "" => web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default(),
            url => url.to_owned(),
        };
        let state = AppState {
            api_base,
            session: cc.storage.and_then(|s| eframe::get_value::<Option<Session>>(s, SESSION_KEY)).flatten(),
            notice_seen: cc.storage.and_then(|s| eframe::get_value::<Option<String>>(s, NOTICE_KEY)).flatten(),
            // An outdated or expired choice is dropped: the visitor is asked again.
            consent: cc
                .storage
                .filter(|_| consent::config().enabled)
                .and_then(|s| eframe::get_value::<Option<ConsentRecord>>(s, CONSENT_KEY))
                .flatten()
                .filter(|r| consent::config().is_current(r, crate::web::now_ms())),
            ..Default::default()
        };
        let map = MapView::new(&theme);

        let mut app = Self {
            state,
            theme,
            controller,
            events,
            map,
            seo: SeoTitles::load(),
            page_title: String::new(),
            password_keyboard: false,
            welcome_card: None,
            analytics: Analytics::default(),
        };
        sync_analytics(&mut app.analytics, &app.state);
        for action in [
            Action::RefreshSession,
            Action::RefreshCount,
            Action::LoadTags,
            Action::CheckLocationPermission,
            Action::LoadDocument(documents::WELCOME.to_owned()),
        ] {
            app.controller.handle(&mut app.state, action);
        }
        app
    }
}

impl eframe::App for WwcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let Self {
            state,
            theme,
            controller,
            events,
            map,
            seo,
            page_title,
            password_keyboard,
            welcome_card: card_rect,
            analytics,
        } = self;

        let mut actions = Vec::new();
        while let Ok(event) = events.try_recv() {
            if let Some(e) = analytics.for_event(&event) {
                crate::web::analytics::track(e);
            }
            actions.extend(state::apply(state, event));
        }
        // Only what the visitor does in the UI is measured, not the follow-ups of events.
        let from_ui = actions.len();

        top_bar::show(ui, state, theme, &mut actions);
        panels::show(ui, state, theme, &mut actions);
        let map_rect = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.colors.background))
            .show(ui, |ui| map.show(ui, state, theme, *card_rect, &mut actions))
            .response
            .rect;
        // On narrow screens an open panel covers the map (egui still leaves it a sliver).
        let narrow = ui.ctx().content_rect().width() < theme.layout.narrow_breakpoint;
        let mut welcome = None;
        if !(narrow && state.panel.is_some()) {
            let footer = footer::show(ui.ctx(), map_rect, state, theme, &mut actions);
            let above_footer = footer.map_or(map_rect, |f| map_rect.with_max_y(f.top()));
            // Once consent is enabled, its panel replaces the notice (which says there are no analytics).
            let card = if consent::config().enabled {
                consent_panel::show(ui.ctx(), above_footer, state, theme, &mut actions)
            } else {
                notice::show(ui.ctx(), above_footer, state, theme, &mut actions)
            };
            welcome = welcome_card::show(ui.ctx(), above_footer, card, state, theme, &mut actions);
            if let Some(locate) = locate_button::show(ui.ctx(), above_footer, card, state, theme, &mut actions) {
                layer_button::show(ui.ctx(), above_footer, locate, state, theme, &mut actions);
            }
        }
        // The zoom buttons follow the card one frame later: draw that frame now.
        if *card_rect != welcome {
            *card_rect = welcome;
            ui.ctx().request_repaint();
        }
        photo_viewer::show(ui.ctx(), state, theme);
        widgets::toasts(ui.ctx(), theme, &mut state.toasts);

        for (i, action) in actions.into_iter().enumerate() {
            if let Some(e) = analytics.for_action(state, &action).filter(|_| i >= from_ui) {
                crate::web::analytics::track(e);
            }
            controller.handle(state, action);
        }
        // A consent choice saved this frame starts or stops analytics at once.
        sync_analytics(analytics, state);

        let title = seo.page_title(state);
        if *page_title != title {
            crate::web::set_document_title(&title);
            *page_title = title;
        }

        // Phone keyboards compose words, which eframe ignores in password fields: give them a
        // password input instead. Touch screens only, desktop typing already works (ADR 0018).
        let password = ui.ctx().input(|i| i.has_touch_screen())
            && ui.ctx().output(|o| o.ime.is_some_and(|ime| ime.purpose == egui::IMEPurpose::Password));
        if *password_keyboard != password {
            crate::web::set_password_mode(password);
            *password_keyboard = password;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SESSION_KEY, &self.state.session);
        eframe::set_value(storage, NOTICE_KEY, &self.state.notice_seen);
        if consent::config().enabled {
            eframe::set_value(storage, CONSENT_KEY, &self.state.consent);
        }
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.theme.colors.background.to_normalized_gamma_f32()
    }
}

/// Loads Umami once the visitor accepts analytics, and stops sending when they withdraw.
fn sync_analytics(analytics: &mut Analytics, state: &AppState) {
    let Some(website_id) = config::umami_website_id() else { return };
    if analytics.sync(state.consent_allows(analytics::PURPOSE)) == Some(Change::Start) {
        crate::web::analytics::start(website_id, config::umami_script_url());
    }
}
