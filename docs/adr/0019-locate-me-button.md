# 0019 — "Locate me" button: the browser's position, kept in memory

Status: proposed (2026-10-09).

## Context
Visitors look for campsites near where they are, often on a phone and outdoors. The map opens on the
theme's `default_center` (France, zoom 6). Today the only ways to reach your own area are panning and the
name search. Browsers give the device's position through the Geolocation API, after asking the visitor.
egui has no binding for this API, and a position is personal data.

## Decision
- **A "Locate me" button in the bottom-right corner of the map**, above the footer. That is the usual
  place in map apps, and the zoom buttons already use the top-left. `ui/locate_button.rs` draws it as an
  `Area` constrained to the map, like the footer. On phones it moves above the privacy notice or the
  consent panel when they would overlap. It hides with the footer when a panel covers the map.
- **One position per click.** `web/geolocation.rs` calls `getCurrentPosition` (only from the controller)
  and the answer comes back as `Event::Located`. There is no `watchPosition`: the dot doesn't follow the
  visitor, and the next click updates it. Options: high accuracy (GPS, quick outdoors), a 15 s timeout,
  and a position up to 60 s old is accepted.
- **Permission is only asked after a click,** never on load. Browsers ignore or penalize prompts that
  don't follow a user gesture, and visitors who never click are never asked.
- **Locate on load once access is allowed.** At startup, `Action::CheckLocationPermission` asks the
  Permissions API (`navigator.permissions.query({name: "geolocation"})`), which never prompts. If the
  answer is `granted`, the reducer starts the same request as a click (`Event::LocationPermission` →
  `Action::Locate`), so a returning visitor lands on their position. `prompt`, `denied`, or a browser
  without the Permissions API: nothing happens. A failure of this automatic request is silent
  (`Locate::automatic`): the visitor didn't ask, so the map just stays on the default view. A click while
  the check is in flight wins, and the check is then ignored.
- **The position stays in memory** (`AppState::locate`). It centers the map (`map_focus`, the same as for
  a search result) and draws a "you are here" dot inside its accuracy circle, under the campsites. It is
  not sent to PocketBase, not saved in browser storage, and gone on reload. OpenStreetMap serves the map
  tiles for that area, the same as when panning there.
- **Errors** (denied, unavailable, timeout, no API or no HTTPS) show a toast and remove the dot, so the dot
  is never stale.
- **Stable web-sys bindings** (`Position`, `Coordinates`, `PositionError`): in web-sys 0.3.106 the
  `Geolocation*` names are behind `web_sys_unstable_apis`. Their getters are plain property reads, so they
  work on the objects current browsers pass.

## Consequences
- Finding campsites nearby takes one tap, and none for a returning visitor who allowed it. The browser's
  permission prompt and site settings are the only consent needed, because the operator never receives
  the position. Revoking access in the site settings stops the automatic locate too.
- Whether the automatic locate happens depends on how the visitor allowed access. "Allow this time" in
  Chrome, or an allow that isn't remembered in Firefox, reads as `prompt` on the next visit. Safari only
  reports geolocation permissions from version 16.
- The map can jump to the visitor's position a moment after it opens, even if they have started panning.
- Geolocation needs a secure context. It works over HTTPS and on `localhost`, but not on a plain-HTTP LAN
  address, where the toast says the browser can't share the position.
- The Privacy Policy should say in one sentence that the button only uses the position in the browser.
  Review this with the legal texts: bumping the policy's `updated` date shows the notice to everyone again.
- On each click, the callback that doesn't run is never freed (a few bytes). That is acceptable for a button.
- Not done: following the visitor (`watchPosition`), a heading arrow, sorting or searching "campsites near
  me", skipping the automatic locate once the visitor has moved the map, and remembering the last
  position between visits. That last one would be a new storage key to justify (§5.3).
