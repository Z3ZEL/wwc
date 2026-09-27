//! The eframe app: drains network events, draws the page, then handles the actions
//! collected during the frame (ARCHITECTURE §5.2).

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::actions::{Action, Event};
use crate::api::ApiClient;
use crate::api::models::Session;
use crate::controller::Controller;
use crate::map::MapView;
use crate::seo::SeoTitles;
use crate::state::{self, AppState};
use crate::ui::theme::Theme;
use crate::ui::{panels, photo_viewer, top_bar, widgets};

const SESSION_KEY: &str = "wwc_session";

pub struct WwcApp {
    state: AppState,
    theme: Theme,
    controller: Controller,
    events: Receiver<Event>,
    map: MapView,
    seo: SeoTitles,
    /// Last value written to `document.title`.
    page_title: String,
}

impl WwcApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::load();
        theme.apply(&cc.egui_ctx);
        // Campsite photos: `egui::Image` fetches http(s) URLs and decodes JPEG/PNG/WebP (ADR 0010).
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let (tx, events) = mpsc::channel();
        // Empty base URL: same origin, /api is proxied to PocketBase.
        let controller = Controller::new(ApiClient::new(""), tx, cc.egui_ctx.clone());
        let state = AppState {
            origin: web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default(),
            session: cc.storage.and_then(|s| eframe::get_value::<Option<Session>>(s, SESSION_KEY)).flatten(),
            ..Default::default()
        };
        let map = MapView::new(&cc.egui_ctx, &theme);

        let mut app = Self { state, theme, controller, events, map, seo: SeoTitles::load(), page_title: String::new() };
        for action in [Action::RefreshSession, Action::RefreshCount, Action::LoadTags] {
            app.controller.handle(&mut app.state, action);
        }
        app
    }
}

impl eframe::App for WwcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let Self { state, theme, controller, events, map, seo, page_title } = self;

        let mut actions = Vec::new();
        while let Ok(event) = events.try_recv() {
            actions.extend(state::apply(state, event));
        }

        top_bar::show(ui, state, theme, &mut actions);
        panels::show(ui, state, theme, &mut actions);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.colors.background))
            .show(ui, |ui| map.show(ui, state, theme, &mut actions));
        photo_viewer::show(ui.ctx(), state, theme);
        widgets::toasts(ui.ctx(), theme, &mut state.toasts);

        for action in actions {
            controller.handle(state, action);
        }

        let title = seo.page_title(state);
        if *page_title != title {
            crate::web::set_document_title(&title);
            *page_title = title;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SESSION_KEY, &self.state.session);
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
