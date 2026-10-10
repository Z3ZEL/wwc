//! The browser's Geolocation API, for the "Locate me" button (ADR 0019). One position per call
//! (`getCurrentPosition`). The browser asks the visitor for permission the first time, so the app
//! only asks after a click on the button. On page load it checks, without asking, whether access
//! was already allowed (Permissions API), and only then locates on its own.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
// The old names of `GeolocationPosition` and `GeolocationPositionError`: web-sys only has the new
// ones behind `web_sys_unstable_apis`. Their getters are plain property reads, so they work on
// what today's browsers pass to the callbacks.
use web_sys::{
    PermissionDescriptor, PermissionName, PermissionState, PermissionStatus, Position, PositionError, PositionOptions,
};

use crate::state::{LocateError, MyLocation};

/// Give up after this long. Browsers start counting once the permission prompt is answered.
const TIMEOUT_MS: u32 = 15_000;
/// A position found this recently is good enough, and comes back at once.
const MAX_AGE_MS: u32 = 60_000;

type Done = Box<dyn FnOnce(Result<MyLocation, LocateError>)>;

/// `done`, shared by the success and error callbacks: whichever runs takes it.
#[derive(Clone)]
struct Once(Rc<Cell<Option<Done>>>);

impl Once {
    fn call(&self, result: Result<MyLocation, LocateError>) {
        if let Some(done) = self.0.take() {
            done(result);
        }
    }
}

/// Asks the browser for the current position. `done` is called once, with the position or
/// the reason there is none.
pub fn locate(done: impl FnOnce(Result<MyLocation, LocateError>) + 'static) {
    let done = Once(Rc::new(Cell::new(Some(Box::new(done)))));
    let Some(geolocation) = web_sys::window().and_then(|w| w.navigator().geolocation().ok()) else {
        return done.call(Err(LocateError::Unsupported));
    };

    // `once_into_js` frees a callback when it runs. Only one of the two ever runs: the other one
    // stays allocated (a few bytes per click).
    let on_success = {
        let done = done.clone();
        Closure::once_into_js(move |position: Position| done.call(to_location(&position)))
    };
    let on_error = {
        let done = done.clone();
        Closure::once_into_js(move |e: PositionError| done.call(Err(LocateError::from_code(e.code()))))
    };
    let options = PositionOptions::new();
    // GPS on phones: campers are mostly outdoors, where it is quick and precise.
    options.set_enable_high_accuracy(true);
    options.set_timeout(TIMEOUT_MS);
    options.set_maximum_age(MAX_AGE_MS);

    if let Err(e) = geolocation.get_current_position_with_error_callback_and_options(
        on_success.unchecked_ref(),
        Some(on_error.unchecked_ref()),
        &options,
    ) {
        log::error!("geolocation unavailable: {e:?}");
        done.call(Err(LocateError::Unsupported));
    }
}

/// Whether the visitor already allowed this site to use their position, checked without asking.
/// `done` gets false when access isn't granted (not decided yet, or refused), and when the browser
/// can't tell (no Permissions API).
pub fn location_allowed(done: impl FnOnce(bool) + 'static) {
    spawn_local(async move { done(query_granted().await.unwrap_or(false)) });
}

async fn query_granted() -> Result<bool, JsValue> {
    let permissions = web_sys::window().ok_or("no window")?.navigator().permissions()?;
    let descriptor = PermissionDescriptor::new(PermissionName::Geolocation);
    let status: PermissionStatus = JsFuture::from(permissions.query(&descriptor)?).await?.dyn_into()?;
    Ok(status.state() == PermissionState::Granted)
}

fn to_location(position: &Position) -> Result<MyLocation, LocateError> {
    let c = position.coords();
    let (lat, lng) = (c.latitude(), c.longitude());
    if !(lat.is_finite() && lng.is_finite()) {
        return Err(LocateError::Unavailable);
    }
    let accuracy_m = Some(c.accuracy()).filter(|a| a.is_finite()).map_or(0.0, |a| a.max(0.0));
    Ok(MyLocation { lat, lng, accuracy_m })
}
