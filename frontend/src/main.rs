#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();
    if eframe::WebLogger::init(log::LevelFilter::Info).is_err() {
        web_sys::console::warn_1(&"logger already initialized".into());
    }

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("app"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        let Some(canvas) = canvas else {
            log::error!("missing <canvas id=\"app\"> in index.html");
            return;
        };
        let result = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(frontend::app::WwcApp::new(cc)))))
            .await;
        if let Err(e) = result {
            log::error!("failed to start the app: {e:?}");
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("The frontend is web-only: run it with `trunk serve` (see README.md).");
}
