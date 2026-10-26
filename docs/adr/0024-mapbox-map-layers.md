# 0024 — Mapbox tiles: Outdoors by default, Aerial with a layer button

Status: proposed (2026-10-25).

## Context
The map uses the public OpenStreetMap tile server (ADR 0003). Its usage policy rules it out for production,
and its standard style is a street map: it shows little of what campers look for (trails, contour lines,
forests, water), and it has no aerial imagery to check a pitch, a clearing or road access. Mapbox has an
Outdoors style and satellite imagery, both as raster tiles that walkers already knows how to draw, and a
free tier sized for a small site.

## Decision
- **Mapbox Static Tiles API, configured at build time.** `WWC_MAPBOX_TOKEN` is baked into the wasm like
  `WWC_API_URL` (`config::mapbox_token`). Only public tokens (`pk.…`) are accepted: the token ends up in the
  page, where anyone can read it, so a secret token (`sk.…`) is refused and the build falls back to OSM.
  The token should be URL-restricted to the site in the Mapbox dashboard.
- **Without a token** (dev, forks, CI), nothing changes: OSM tiles, OSM attribution, no layer button.
- **Two layers, with walkers' own `Mapbox` source:** Outdoors by default, and Aerial (`SatelliteStreets`:
  imagery with roads and place names, to find your way on it). `map/view.rs` keeps one `HttpTiles` per
  layer, so switching back is instant. Tiles are 512 px (a quarter of the requests of 256 px tiles), @2x on
  screens with more than one pixel per point.
- **A layer button right above "Locate me"**, drawn in the same `Area` (`ui/locate_button.rs`), so it
  follows it above the privacy notice on phones. Same size. One click switches to the other layer
  (`Action::ToggleAerial`): with two layers a toggle is one tap, where a menu would be two. Its icon is the
  layer a click shows (🗺 / 🌍, both in egui's bundled fonts), and the hover text says it too. The choice
  is kept in memory only (`AppState::aerial`): every visit opens on Outdoors, and there is no new browser
  storage key to justify (§5.3).
- **Attribution, as Mapbox requires** (docs.mapbox.com/help/dive-deeper/attribution): the Mapbox wordmark
  (unstyled, the black version on the footer's light background), "© Mapbox", "© OpenStreetMap",
  "© Maxar" on the aerial layer, and "Improve this map", each linking to Mapbox's pages. They live in the
  map footer with the legal links, so the bottom-right corner holds everything and no new card has to avoid
  the privacy notice on phones. The wordmark SVG comes from walkers' assets and is drawn by egui_extras'
  SVG loader, which walkers already enables.
- **Legal texts:** the Privacy Policy names Mapbox, Inc. (United States) as the independent controller that
  receives map image requests (IP address kept 30 days, EU–US Data Privacy Framework), and the Credits page
  credits Mapbox and Maxar. The privacy notice names the provider the build actually uses.
- Rejected:
  - Keeping the public OSM server in production: against its usage policy, and no aerial imagery.
  - Mapbox GL JS or vector tiles: a JavaScript map next to the egui canvas, or a vector renderer in wasm, for
    the same pictures.
  - Other providers (MapTiler, Thunderforest, Esri, IGN): workable, but Mapbox covers both layers with one
    account and walkers already ships a source for it.
  - A token fetched at runtime from PocketBase: a public token is public anyway, and one build per
    environment already exists for `WWC_API_URL`.
  - Remembering the layer between visits: a new storage key in the policy for a one-tap choice.

## Consequences
- Each visitor's browser contacts api.mapbox.com. The Privacy Policy changed (its `updated` date too), so
  returning visitors of builds without analytics see the privacy notice again. Consent isn't asked again:
  map images were already loaded from a third party, under legitimate interest.
- Usage counts against the Mapbox account: the free tier covers a limited number of tile requests a month.
  Keep the token URL-restricted, and watch the dashboard. 512 px tiles keep the count down; @2x tiles weigh
  more but count the same.
- Build hosts need `WWC_MAPBOX_TOKEN` set (Render: environment variable). docker compose passes it from
  `.env` to the dev frontend build.
- A Content-Security-Policy, if one is added, must allow `api.mapbox.com` for images.
- Not done: more layers (e.g. a Streets style), a layer menu with previews, a dark map, remembering the layer.
