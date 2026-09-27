# Architecture Guidelines

> Status: **v0.6** (2026-09-26). This is the reference that agents and humans follow when building the app.
> The MVP is implemented. §13 lists what exists and what is deferred.
> If you need to break a rule here, update this document in the same change and explain why in an ADR (see §12).

---

## 1. Product summary

A campsite-sharing web app built around an **interactive map**.

| Actor                  | Can do                                                                                                                                                                                                                                                                                              |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Anonymous visitor**  | Browse the map, see every campsite, open details, read ratings and comments                                                                                                                                                                                                                         |
| **Authenticated user** | Everything above, plus: create campsites, edit/delete _their own_ campsites (owner-only, never other people's), rate a campsite (once per campsite, editable), comment, delete their own comments, **report** a campsite, comment or user. Users **cannot rate or comment on their own campsites**. |
| **Admin**              | A user with `role = "admin"`. Uses a **minimal in-app moderation screen** (report queue, hide/restore content). Everything else (tags, users, raw data) goes through the PocketBase dashboard (`/_/`) for now.                                                                                      |

Core domain objects: **User**, **Campsite**, **Tag**, **Rating**, **Comment**, **Report**, **Photo** (a file field on Campsite).

Product decisions (v0.2):

- Exact campsite locations are **public**, including for anonymous visitors.
- Campsite editing is **owner-only**; there is no wiki-style editing.
- You can't rate or comment on your own campsite.
- Moderation is **community reports + admin review**. Content is hidden by an admin, never automatically.

---

## 2. Stack at a glance

| Layer           | Choice                                                                         | Notes                                                                                                            |
| --------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Frontend        | **Rust + egui/eframe**, compiled to **WebAssembly** (`wasm32-unknown-unknown`) | **Web only.** No native desktop target is supported or tested.                                                   |
| Frontend build  | **Trunk**                                                                      | Dev server, asset pipeline, dev proxy to the backend, `wasm-opt` in release                                      |
| Map widget      | **`walkers`** crate (egui slippy map)                                          | OpenStreetMap raster tiles in dev                                                                                |
| HTTP (wasm)     | **`ehttp`** (callback based, no async runtime)                                 | One API client module, see §5.4                                                                                  |
| Backend         | **PocketBase** (single binary, SQLite)                                         | Auth, REST API, file storage, access rules, admin UI                                                             |
| Backend logic   | PocketBase **JS migrations** (`pb_migrations/`) and **JS hooks** (`pb_hooks/`) | No custom Go build unless an ADR justifies it                                                                    |
| Dev environment | **docker compose**                                                             | Development only (§7)                                                                                            |
| Production      | Static bundle + PocketBase container                                           | Frontend on any static host, PocketBase image with a `/pb_data` volume, files and backups on S3 (§7.1, ADR 0012) |

```
┌───────────────────────────── Browser ─────────────────────────────┐
│  index.html + app.wasm  (egui canvas)                              │
│   ├─ UI (screens / widgets)  ──► Actions                           │
│   ├─ Controller (update loop) ──► ApiClient ──┐                    │
│   └─ AppState  ◄── Events (channel) ◄─────────┘                    │
│   Map tiles ◄──── OSM tile server (HTTP)                           │
└─────────────────────────────┬─────────────────────────────────────┘
                              │ dev: same origin, /api/* (Trunk proxy) · prod: WWC_API_URL + CORS
                              ▼
┌──────────────────────── PocketBase :8090 ─────────────────────────┐
│  /api/collections/*   REST + auth                                  │
│  /api/files/*         campsite photos                              │
│  /_/                  admin dashboard                              │
│  pb_migrations/  (schema as code)    pb_hooks/ (server logic)      │
│  pb_data/        (SQLite + uploads, docker volume, NOT committed)  │
│  prod: uploads + backups on S3, SQLite on a persistent volume      │
└───────────────────────────────────────────────────────────────────┘
```

---

## 3. Repository layout

```
wwc/
├── CLAUDE.md                  # Agent entry point (short, points here)
├── docker-compose.yml         # Dev stack (+ mailpit)
├── scripts/build-frontend.sh  # Production static build (WWC_API_URL=… → frontend/dist/)
├── .env.example               # Every env var, documented, no secrets
├── Cargo.toml                 # Cargo workspace root (+ release profile)
├── rust-toolchain.toml        # Pinned toolchain + wasm32-unknown-unknown target
├── rustfmt.toml               # max_width = 120
├── frontend/
│   ├── Cargo.toml             # eframe/walkers are wasm-only deps; the rest also builds on the host
│   ├── Dockerfile.dev         # rust + trunk image for docker compose
│   ├── Trunk.toml             # Build config (the /api proxy comes from TRUNK_SERVE_PROXY_* env vars) + seo-gen hook
│   ├── index.html             # Page shell with SEO markers, filled by tools/seo-gen
│   ├── assets/theme.json      # ALL colors, fonts, sizes, spacing — see §5.7
│   ├── assets/seo.json        # ALL SEO text and settings — see §5.10
│   ├── assets/seo/            # favicon.svg, og-image.svg/.png (social preview)
│   ├── tests/fixtures/        # Recorded PocketBase JSON for DTO tests
│   └── src/
│       ├── main.rs            # wasm entry: start eframe WebRunner
│       ├── lib.rs             # module tree; `app` and `map` are wasm-only
│       ├── app.rs             # impl eframe::App — frame loop + session persistence
│       ├── config.rs          # build-time config (WWC_API_URL)
│       ├── actions.rs         # Action (UI intent) + Event (API result) enums
│       ├── controller.rs      # Action -> state change / API request
│       ├── state/             # AppState, Panel, Remote, form drafts, pure `apply(event)` + tests
│       ├── api/               # PocketBase client, DTOs, errors (the only HTTP code)
│       ├── map/               # view.rs: walkers map, markers, draft pin, viewport → bbox (wasm only)
│       │                      # cluster.rs: pure screen-space marker clustering (host-tested)
│       ├── seo.rs             # browser tab title from seo.json
│       ├── web/               # browser APIs egui lacks (photo file picker, document title), wasm only
│       └── ui/
│           ├── theme.rs       # Theme struct (serde) + apply to egui::Style
│           ├── top_bar.rs
│           ├── panels/        # auth (login/register), profile, campsite_form, campsite_view, search, filters
│           └── widgets/       # panel_frame, toasts, buttons, stars, tag chips, form errors, range_slider
├── tools/seo-gen/           # host binary run by Trunk post_build: seo.json → index.html, robots.txt, sitemap.xml
├── backend/
│   ├── Dockerfile             # *Pinned*, checksum-verified PocketBase + migrations/hooks; dev and prod image
│   ├── entrypoint.sh          # migrate, upsert superuser, serve (automigrate only with PB_DEV=1, CORS from PB_ORIGINS)
│   ├── pb_migrations/         # Schema as code — committed
│   ├── pb_hooks/              # *.pb.js server hooks — committed (settings.pb.js: settings from files + env)
│   ├── pb_settings.json       # Instance settings as code (rate limits, logs, batch…); pb_settings.dev.json on top in dev
│   ├── seed/seed.sh           # Dev seed data (curl + jq)
│   └── tests/                 # rules.sh: API rule tests (curl + jq); fixtures/ (test image)
└── docs/
    ├── ARCHITECTURE.md        # This file
    ├── API.md                 # Collections, fields, rules, example requests
    └── adr/                   # Architecture Decision Records
```

Rules:

- `pb_data/` is **never** committed (add to `.gitignore`); it lives in a docker volume.
- Add a `fonts/` or `icons/` folder under `frontend/assets/` when the first custom font or icon is needed.
- API DTOs live in `frontend/src/api/models.rs`. If a second Rust consumer appears (e.g. a CLI/seed tool), extract them into a `crates/api-types` crate — not before.

---

## 4. Backend (PocketBase)

### 4.1 Principles

1. **PocketBase is the source of truth for authorization.** Every permission is expressed as a collection **API rule**. The frontend hiding a button is UX, not security.
2. **Schema is code.** All collection changes go through files in `backend/pb_migrations/`. Changes made by clicking in the admin UI in dev must be exported as a migration (PocketBase's automigrate does this when running with `--dev`/automigrate enabled) and committed.
3. **Prefer rules and view collections over hooks.** Reach for `pb_hooks` only when rules can't express it (e.g. side effects, derived fields, custom endpoints).
4. **Pin the PocketBase version** (`ARG PB_VERSION` in `backend/Dockerfile`). PocketBase is pre-1.0 and has breaking changes between minors; upgrades are deliberate and get their own change.

### 4.2 Data model

**`users`** (built-in auth collection)

| Field                     | Type                    | Notes                                                                                                                   |
| ------------------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `username` / `name`       | text                    | Public display name                                                                                                     |
| `avatar`                  | file                    | optional, image, size-limited                                                                                           |
| `role`                    | select: `user`, `admin` | default `user`. **Users can never change it themselves** (see §4.3). Admins are promoted from the PocketBase dashboard. |
| email, password, verified | built-in                | Email is **not** publicly visible                                                                                       |

**`campsites`**

| Field                 | Type                       | Notes                                                                                                                                             |
| --------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`               | text                       | required, 3–100 chars                                                                                                                             |
| `description`         | editor/text                | max length enforced                                                                                                                               |
| `lat`, `lng`          | number                     | two plain number fields (simpler bbox filters and indexes than `geoPoint`). Not `required`: PocketBase treats 0 as empty.                         |
| `photos`              | file (multiple)            | at most **3**, JPEG/PNG/WebP, 10 MB each, thumbs `320x240` + `1200x1200f`, not protected (ADR 0010). Owner-only, like every other campsite field. |
| `tags`                | relation (multiple) → tags | e.g. safe water, river, flat                                                                                                                      |
| `tent_capacity`       | number                     | required, integer **1–10**, where **10 means "10+"**. The UI always shows `10` as `10+`.                                                          |
| `hidden`              | bool                       | default false; set by admins only (moderation)                                                                                                    |
| `author`              | relation → users           | required, set from auth (see rules)                                                                                                               |
| `created` / `updated` | autodate                   |                                                                                                                                                   |

> Campsite properties will change. **New "yes/no" properties become tags** (a data change, not a migration). Add a dedicated field only for values that are numeric, ranged, or have to be validated (like `tent_capacity`).

**`tags`** (admin-managed lookup table)

| Field        | Type   | Notes                                                                              |
| ------------ | ------ | ---------------------------------------------------------------------------------- |
| `slug`       | text   | unique, stable identifier used in code/filters, e.g. `safe_water`, `river`, `flat` |
| `label`      | text   | display name                                                                       |
| `icon`       | text   | optional icon key for the frontend                                                 |
| `sort_order` | number | display order                                                                      |
| `active`     | bool   | inactive tags are hidden from the picker but kept on existing campsites            |

Initial tags come from a migration: `safe_water` (Safe water), `river` (River), `flat` (Flat ground). The frontend **must not hard-code the tag list**. It loads tags from the API at startup.

**`ratings`**

| Field            | Type                   | Notes                                                |
| ---------------- | ---------------------- | ---------------------------------------------------- |
| `campsite`       | relation → campsites   | required, cascade delete                             |
| `author`         | relation → users       | required                                             |
| `score`          | number                 | integer 1–5                                          |
| **unique index** | (`campsite`, `author`) | one rating per user per campsite; re-rating = update |

**`comments`**

| Field      | Type                 | Notes                      |
| ---------- | -------------------- | -------------------------- |
| `campsite` | relation → campsites | required, cascade delete   |
| `author`   | relation → users     | required                   |
| `body`     | text                 | required, 1–2000 chars     |
| `hidden`   | bool                 | default false; admins only |

**`reports`**

| Field                  | Type                                                                                   | Notes                                                                                                  |
| ---------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `reporter`             | relation → users                                                                       | required, = auth user                                                                                  |
| `campsite` / `comment` | relation (single, optional each)                                                       | **exactly one** must be set, enforced by a hook (§4.5). A `user` target (report a person) is deferred. |
| `reason`               | select                                                                                 | `spam`, `inappropriate`, `wrong_location` (campsites only), `dangerous`, `other`                       |
| `details`              | text                                                                                   | optional, max 1000 chars                                                                               |
| `status`               | select                                                                                 | `open` (set by the hook on create), `resolved`, `dismissed`. Only admins can change it.                |
| **unique indexes**     | (`reporter`, `campsite`), (`reporter`, `comment`), both partial (`WHERE target != ''`) | one report per user per target                                                                         |

`resolved_by` (who acted) is deferred to the moderation panel: for now admins resolve reports in the dashboard as a superuser, which isn't a `users` record.

**`campsite_stats`** (**view collection**, read-only, SQL)
Aggregates per campsite: `avg_score`, `rating_count`, `comment_count`. The frontend reads this instead of computing averages client-side. Avoid denormalized counters maintained by hooks unless performance proves the view insufficient.

### 4.3 API rules (the security contract)

Shorthands used below (write them out in full in the real rules):

- `AUTH` = `@request.auth.id != ""`
- `ADMIN` = `@request.auth.role = "admin"`
- `NOT_OWN_SITE` = `@request.body.campsite.author != @request.auth.id`
- `VERIFIED` = `@request.auth.verified = true` — prefixes every create rule on `campsites`, `ratings`, `comments` and `reports` (§4.6); left out of the table for brevity.

| Collection       | list / view                                                | create                                                                                    | update                                                                                                                                                                    | delete                                 |
| ---------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `campsites`      | `hidden = false \|\| author = @request.auth.id \|\| ADMIN` | `AUTH && @request.body.author = @request.auth.id && @request.body.hidden:isset = false`   | `author = @request.auth.id && @request.body.author:isset = false && @request.body.hidden:isset = false` **or** `ADMIN` (admin may only toggle `hidden`, enforced by hook) | `author = @request.auth.id \|\| ADMIN` |
| `tags`           | `""`                                                       | `null` (dashboard only)                                                                   | `null`                                                                                                                                                                    | `null`                                 |
| `ratings`        | `""`                                                       | `AUTH && @request.body.author = @request.auth.id && NOT_OWN_SITE`                         | `author = @request.auth.id` (can't change `campsite`/`author`)                                                                                                            | `author = @request.auth.id`            |
| `comments`       | `hidden = false \|\| author = @request.auth.id \|\| ADMIN` | `AUTH && @request.body.author = @request.auth.id && NOT_OWN_SITE`                         | `author = @request.auth.id` (body only)                                                                                                                                   | `author = @request.auth.id \|\| ADMIN` |
| `reports`        | `reporter = @request.auth.id \|\| ADMIN`                   | `AUTH && @request.body.reporter = @request.auth.id && @request.body.status:isset = false` | `ADMIN`                                                                                                                                                                   | `ADMIN`                                |
| `campsite_stats` | `""` (exclude hidden campsites in the view SQL)            | —                                                                                         | —                                                                                                                                                                         | —                                      |
| `users`          | view limited to public fields                              | public signup, `@request.body.role:isset = false`                                         | self only, `@request.body.role:isset = false`                                                                                                                             | self only                              |

- `null` rule = superuser only. Empty string `""` = everyone. Be explicit — never leave a rule accidentally `""`.
- **Never let a user set their own `role`, `hidden`, or `status`.** Each of these is guarded by a rule and a rule test.
- "Can't rate or comment on your own campsite" is enforced by `NOT_OWN_SITE` in the rules **and** by a hook (defense in depth, and it works even if the pinned PocketBase version can't resolve relations inside `@request.body`). The UI also hides those controls on your own campsites.
- Hidden content still exists for its author and for admins. The author sees a "hidden by moderation" badge.
- Every rule change needs an entry in `docs/API.md`.

### 4.4 Querying for the map

- The map requests campsites **by viewport bounding box**, never "all campsites":
  `filter=(lat >= {s} && lat <= {n} && lng >= {w} && lng <= {e})` (or the equivalent on `location`).
- Request only the fields a marker needs (`fields=id,title,location`) and paginate (`perPage` capped, e.g. 200). Full details are fetched when a campsite is opened.
- Optional filters go in the same query: tags (`tags ~ 'TAG_ID'`, one clause per tag, AND-combined — `?=` doesn't work for this with the pinned PocketBase, see ADR 0011) and a tent capacity range (`tent_capacity >= min`, `tent_capacity <= max`; no upper bound when max is 10 = "10+").
- **Search by name** is the one query that ignores the viewport: `title ~ 'query'` plus the same filters, sorted by title, 50 results.
- Add an index on the coordinate columns in the migration.
- Clustering of markers is done client-side for now (§5.5).

### 4.5 Hooks (`pb_hooks/*.pb.js`) — allowed uses

- Force `author = auth.id` on create (defense in depth on top of rules).
- Reject a rating or comment on a campsite the requester authored.
- Reports: check that exactly one target is set, that it isn't the reporter's own content or a duplicate, and set `status = open`.
- Admin updates on `campsites`/`comments` can only change `hidden`. An admin never edits another user's content.
- Content sanitation / profanity or spam checks on comments.
- Custom read-only endpoints if a query can't be expressed via the REST API (e.g. "nearby campsites").
  Keep hooks small, one concern per file, and documented in `docs/API.md`.

### 4.6 Auth

- Email + password via PocketBase auth collection. OAuth2 providers (Google, GitHub…) can be enabled later with no frontend architecture change.
- Email verification is required before posting: `VERIFIED` in the create rules (§4.3). Registering sends the confirmation email (`request-verification`); the link opens PocketBase's own confirmation page (`PB_APP_URL/_/#/auth/confirm-verification/…`). The UI replaces create forms with "Confirm your email" + "Resend email" / "I've confirmed it" (auth-refresh) while `verified` is false. SMTP comes from env (`pb_hooks/settings.pb.js`); in dev, mailpit catches the mail (§7).
- Built-in rate limiting is on for auth and create endpoints (`backend/pb_settings.json`; off in dev).

---

## 5. Frontend (Rust + egui, web only)

### 5.1 Constraints to keep in mind

- egui is **immediate mode**: the UI is redrawn every frame from state. UI code must be **cheap and side-effect free** except for emitting actions.
- It renders to a **single `<canvas>`**: no DOM, weak SEO and accessibility, and mobile text input is limited. Accept this; don't fight it with DOM hacks unless an ADR says so.
- Wasm is **single-threaded** here: never block. No `std::thread`, no `std::time::Instant` (use `web-time` or `instant`), no blocking I/O.
- Only `wasm32-unknown-unknown` is a target. Don't add `#[cfg(not(target_arch = "wasm32"))]` desktop paths — except in unit tests of pure logic (§9).

### 5.2 Architecture pattern: unidirectional data flow

```
 UI (screens/widgets)          reads &AppState, returns Vec<Action>
        │ Action
        ▼
 Controller::handle(action)    mutates AppState synchronously and/or spawns API tasks
        │ spawn_local(async)
        ▼
 ApiClient (async)             HTTP to PocketBase
        │ Event (via channel)
        ▼
 App::update drains channel ──► state::apply(event) ──► ctx.request_repaint()
```

Rules:

- **UI functions never call the API directly.** They read `AppState` and push `Action`s. The one exception: egui text inputs need `&mut String`, so panels get `&mut AppState` and may edit **form input buffers** (and small view toggles like "confirm delete"). Anything else goes through an `Action`.
- **All API results come back as `Event`s** through a single `std::sync::mpsc` receiver drained at the start of every frame. `ehttp` callbacks send the event, then call `ctx.request_repaint()` (see `Controller::done`).
- **Exception — images:** `egui::Image::new(url)` fetches campsite photos itself, through the `egui_extras` loaders installed in `app.rs`, the same way walkers fetches tiles. It's read-only and cached, and it never touches `AppState`. UI code only builds the URL with `api::photo_url` (ADR 0010).
- **Browser APIs** that egui lacks (the file picker) live in `web/` and are called by the controller only. Their results come back as `Event`s, like API results.
- **State mutation from events is pure**: `fn apply(state: &mut AppState, event: Event) -> Vec<Action>`. The returned follow-up actions (e.g. "log in" after signup, "refresh markers" after a save) are handled by the controller in the same frame. It is unit-tested on the host.
- Remote data is modeled explicitly, e.g.:
  ```rust
  enum Remote<T> { NotAsked, Loading, Loaded(T), Failed(ApiError) }
  ```
  Every screen must render all four states (loading spinner, error with retry).
- Guard against stale responses: tag requests (e.g. a viewport generation counter) and drop events that don't match the latest request.

### 5.3 State shape (starting point)

See `frontend/src/state/mod.rs`: `session`, `panel` + `panel_collapsed`, `confirm_discard` (unsaved-changes prompt), `after_login` (panel to return to), `campsite_count`, `tags`, `viewport` + `markers` (with a generation counter), `filters` (global map filters), `search` (query buffer, submitted query, results + generation), `map_focus` (a position the map centers on next frame), `detail` (the open campsite: data, stats, my rating, paginated comments), one draft per form, `toasts`.
The walkers tile cache and map memory are not in `AppState`; they live in `map::MapView`, owned by the app.

- **Routing (not implemented yet — panel state is in memory only):** there is one page (the map). The "route" is the active side panel. Sync `panel` with the URL hash so links are shareable and back/forward work: `#/` (none), `#/login`, `#/register`, `#/profile`, `#/new`, `#/campsite/<id>`, `#/campsite/<id>/edit`, `#/moderation`. Read the hash on startup. Unknown hash → `#/`. An auth-only panel opened while logged out → login panel, then back to the requested panel after login.
- **Persistence:** only the auth session goes to `localStorage`, through eframe's storage (`App::save`, key `wwc_session`, saved every 2 s). The token is refreshed on startup. Never persist passwords.

### 5.4 API client (`api/`)

- One `ApiClient` struct: base URL + optional auth token. Base URL defaults to **same origin** (`/api/...`) — in dev Trunk proxies it, in prod a reverse proxy will.
- Typed methods per use case (full list in `docs/API.md`). Each takes a `Done<T>` callback: `Box<dyn FnOnce(Result<T, ApiError>) + Send>`.
- DTOs in `models.rs` with `serde` mirroring PocketBase JSON (snake_case field names, `id`, `created`, `updated`, `expand` for relations). Use `expand=author` to fetch display names in one call.
- `ApiError` maps PocketBase's error body (`status`, `message`, `data` per-field errors) so forms can show field-level validation messages.
- On `401` → clear session, route to login with a toast. Refresh the token on startup via `auth-refresh`.
- **Photos:** a save with new photos is one `multipart/form-data` request: a `@jsonPayload` part (the normal JSON body, with `photos-` for removals) plus `photos+` file parts. The body is built by a pure function in `api/client.rs`. `photo_url(origin, campsite, file, PhotoSize)` builds absolute thumb URLs (egui's loader only takes `http(s)://`). The origin comes from `AppState.origin`. The UI never loads original files.
- **No hand-built filter strings scattered in UI code.** Filters are built inside `api/`, only from numbers and from ids that pass `is_record_id` (alphanumeric only). The one exception is search text, which goes through `clean_search` (quotes, backslashes, backticks and control characters dropped, 100 chars max) before being quoted (ADR 0011).

### 5.5 Map (`map/`)

- Use `walkers` with an OSM tile source in dev. Always render **tile attribution** ("© OpenStreetMap contributors").
- The public OSM tile server has a strict usage policy: fine for dev, **not for production** — switching provider is a config change in `map/`, nothing else.
- Markers are painted in the `Map::show` closure (`map/view.rs`); clicking one emits `Action::OpenPanel(Panel::Campsite(id))`. Markers are colored by state: normal, your own, selected.
- **Clustering** (`map/cluster.rs`): below `map.cluster_max_zoom`, markers closer than `map.cluster_distance` points on screen are drawn as one circle with a count (radius `cluster_radius`…`cluster_radius_max`). Clicking a cluster centers on it and zooms in by `cluster_zoom_step`. The selected campsite is always drawn alone. The grouping only uses relative screen positions, so panning doesn't change it.
- `AppState.map_focus`: set by `Action::FocusCampsite` (a search result click). The map takes it on its next frame, centers there and zooms in to at least `map.focus_zoom`.
- Controls: the mouse wheel zooms around the pointer (no Ctrl needed, walkers `zoom_with_ctrl(false)`), dragging pans, pinch zooms on touch, double-click zooms in, and +/− buttons sit in the top-left corner. The wheel no longer pans, so a touchpad two-finger swipe zooms too. The cursor is a grab hand over the map, a closed hand while dragging, and a pointer over markers.
- When the viewport settles (debounce ~300 ms after pan/zoom stops), emit `Action::ViewportChanged(bbox)`, which triggers the bbox query (§4.4).
- "Add campsite" flow: the "New campsite" panel is open, the map stays interactive, and clicking the map places or moves a draft pin that fills the form's lat/lng. Opening a fresh form shows an info toast: "Tip: click on the map to place your campsite." The panel shows the coordinates plus a "Use map center" button.
- Campsite form fields: title, description, tags (multi-select chips from the loaded tag list), tent capacity (a stepper or slider 1–10 that shows `10+` at max), photos (thumbnails with × to remove, and "Add photos (n/3)", which opens the browser file dialog; wrong type, size or count → error toast). Picked photos are previewed from a small JPEG made by the browser and uploaded on save.
- **Filters panel** (`Panel::Filters`): tag chips (all selected tags must match) and a tent capacity range (one two-handle slider, `widgets::range_slider`, 1–10+; the handles can't cross), plus Reset. Filters are global: they apply to the markers and to search, and stay on when the panel is closed. Each change is an `Action` that re-runs the bbox query and the current search.

### 5.6 Page layout and navigation

The design is **plain and flat**: solid colors, no gradients, no shadows, few borders, generous spacing. It is **one page**: the map always fills the window, and everything else is a **side panel** over it.

```
┌──────────────────────────────────────────────────────────────────────┐
│ ▲ WWC  1 284 campsites  [Search…][×] [Filters (2)]  [+ New][Log in]  │  ← top bar
├──────────────────────────────────────┬───────────────────────────────┤
│                                      │ ◀ Campsite: Lake Spot     [✕] │
│                                      │ ★★★★☆ 4.2 (12)  ·  tents: 10+ │
│             MAP (always              │ [safe water] [river] [flat]   │
│             visible and              │ photos …                      │
│             interactive)             │ description …                 │
│                                      │ comments …                    │
│   ●     ●        ●                   │                               │
│         ● (selected)                 │                     side panel│
└──────────────────────────────────────┴───────────────────────────────┘
```

**Top bar** (always visible, fixed height from the theme):

- Left: app name/logo. Clicking it closes the panel.
- Center: **total number of campsites** (`stats`, unfiltered), refreshed on startup and after a campsite is created or deleted. Then the **search field** (`layout.search_width`, `search_width_narrow` on narrow screens): Enter runs the search and opens the Search panel, × clears it. Then the **Filters** button, which shows the number of active filters and is filled (`tag_selected_*` colors) while any is on; it toggles the Filters panel.
- Right, logged out: **Log in**, **Sign up**. Logged in: **+ New campsite**, then the avatar/name menu (Profile, Moderation if admin, Log out).

**Photo viewer** (`ui/photo_viewer.rs`): the one exception to "everything is a side panel". Clicking a campsite photo opens an egui `Modal` over the whole page (inset by `layout.photo_viewer_margin`, backdrop `colors.photo_viewer_backdrop`). It is a carousel: the current photo (`1200x1200f` thumb), fitted to the space, with ◀ ▶ on the sides (wrapping around), "n / N" and × in the header, and a thumbnail row at the bottom with the current photo outlined. ← / → browse; Esc, × or a click on the backdrop close it. State is `CampsiteDetail::photo_open`, so it closes with the Campsite panel.

**Side panels:** only one is active at a time. They share one frame: a header with the title plus a collapse button (◀/▶) and a close button (✕), and a scrollable body.

| Panel          | Hash                   | Access     | Content                                                                                                                                                                                                                                                             |
| -------------- | ---------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Login          | `#/login`              | logged out | email, password, link to Register                                                                                                                                                                                                                                   |
| Register       | `#/register`           | logged out | display name, email, password + confirm, link to Login                                                                                                                                                                                                              |
| Profile        | `#/profile`            | logged in  | display name, avatar, change email (PocketBase email-change flow), change password (needs old password), log out, delete account (confirmation typed inside the panel, no browser dialogs)                                                                          |
| New campsite   | `#/new`                | logged in  | form from §5.5, draft pin on the map                                                                                                                                                                                                                                |
| Campsite       | `#/campsite/<id>`      | public     | photos (thumbnail strip; a click opens the photo viewer, see below), title, author, stats, tags, tent capacity, coordinates (copy button), description, rating widget (not on your own campsite), comments + comment box, Report buttons; Edit/Delete if it's yours |
| Edit campsite  | `#/campsite/<id>/edit` | owner      | same form as New campsite, prefilled                                                                                                                                                                                                                                |
| Report         | — (no route)           | logged in  | preview of the reported campsite or comment, reason, details, Send / Cancel (back to the campsite). §5.8                                                                                                                                                            |
| Moderation     | `#/moderation`         | admin      | §5.8                                                                                                                                                                                                                                                                |
| Search results | `#/search`             | public     | the submitted query, a note when filters are on, matching campsite names (50 max, "showing 50 of N"). A click centers the map on the campsite and opens it; the Campsite panel then shows "◀ Search results" to come back                                           |
| Filters        | `#/filters`            | public     | §5.5                                                                                                                                                                                                                                                                |

Behavior:

- **Collapse** (◀) shrinks the panel to a thin strip and keeps its state, including unsaved form drafts. **Close** (✕) clears the panel and resets the hash to `#/`. Closing a form with unsaved changes asks for confirmation inside the panel.
- Clicking a marker opens the Campsite panel, or switches to it if another panel is open. The selected marker is highlighted, and the map pans only if the marker is hidden behind the panel.
- **Wide screens** (≥ `layout.narrow_breakpoint`): the panel is docked to the **right** with `layout.side_panel_width` and the user can resize it within the min/max. **Narrow screens:** the panel covers the whole map area under the top bar, and the top bar compacts (the count stays, the buttons move into a ☰ menu).
- The panel slides open and closed, and slides to and from its collapsed strip, over `layout.panel_animation_ms` (`widgets::panel_frame`, egui `Panel::show_switched`). Use 0 to turn animation off. Dragging the resize edge all the way in (or double-clicking it) collapses the panel.
- Clickable widgets show a pointer cursor: every `Button` through `Visuals::interact_cursor` (set in `Theme::apply`), and custom widgets, radios and sliders set it themselves.
- Errors appear as toasts at the bottom-left of the map area, or inline in forms.

### 5.7 Theme (`frontend/assets/theme.json`)

**Every visual value comes from `theme.json`.** UI code must not contain hard-coded colors, font sizes, paddings, radii or panel sizes. Always read them from `Theme`.

- `ui/theme.rs` defines `Theme` (serde, `#[serde(deny_unknown_fields)]`) matching the JSON. It is loaded with `include_str!("../../assets/theme.json")`, so edits trigger a Trunk rebuild and hot reload in dev, and a broken theme is caught at build/test time rather than failing at runtime.
- At startup, `Theme::apply(&ctx)` maps it onto `egui::Style` / `Visuals`: panel fill, window fill, widget colors for inactive, hovered and active states, selection color, corner radii, stroke widths, spacing, text styles. Shadows are turned off for the flat look.
- Values egui doesn't cover (marker colors, star color, top bar and panel sizes, tag chip colors, photo thumbnail and viewer sizes and colors, loading spinner colors, sizes and speed…) are read from `&Theme`, which screens and widgets receive as a parameter.
- **Colors** are hex strings `#RRGGBB` or `#RRGGBBAA`. **Sizes** are logical points.
- **Fonts:** `typography.font_family` and `font_family_heading` name a font from `typography.fonts`, each pointing to a file in `assets/fonts/`. Font bytes are embedded at compile time through a small registry in `theme.rs`, so adding a font means adding the file and one registry line. `"default"` uses egui's built-in font.
- Only a light theme exists for now. The file is organized so a `"dark"` palette can be added later without changing its shape.
- A unit test parses `theme.json`. It checks that every color is valid hex and that `text`/`background`, `text`/`surface` and `on_primary`/`primary` reach a contrast ratio of at least 4.5:1.

### 5.8 Reporting and moderation UI

- A **"Report"** action in the Campsite panel (for the campsite and for each comment), for logged-in users who aren't the author. It opens the **Report panel** (`Panel::Report(ReportTarget)`) in the side panel: a preview of the target, the reason (radio list) and optional details (required for "other"). Sending it shows a toast and returns to the campsite. A second report on the same target shows the hook's "You already reported this." (no "Reported" state yet: that would need to fetch the user's reports).
- **For now reports are only read in the PocketBase dashboard** (`/_/` → `reports`). Reporting users is deferred.
- **Moderation panel** (`Panel::Moderation`, only reachable if `session.user.role == Admin`). Keep it basic:
  - A list of `open` reports, oldest first, showing the target preview, reason, details, reporter and date.
  - Actions per report: **Open target**, **Hide content** (sets `hidden = true` on the target, marks the report `resolved`), **Dismiss** (marks it `dismissed`).
  - An "Unhide" toggle when looking at hidden content.
  - No dashboards, stats or bulk actions for now. Everything else is done in the PocketBase dashboard.
- Hiding the panel in the UI is only for convenience. Access is enforced by the `ADMIN` rules.

### 5.9 UI conventions

- Panels live in `ui/panels/`, each a function `fn show(ui: &mut egui::Ui, state: &AppState, theme: &Theme, actions: &mut Vec<Action>)`. The shared panel frame (header, collapse, close) is one widget and panels don't redraw it.
- Reusable widgets in `ui/widgets/`, no knowledge of `AppState` — plain inputs and outputs.
- Test every panel at both wide and narrow widths.
- No hard-coded style values (§5.7). If you need a new value, add it to `theme.json` **and** `Theme`, with a sensible name.
- No browser dialogs (`alert`/`confirm`). Confirmations are shown inside the panel.
- Remote data that is loading uses the themed spinner in `ui/widgets/spinner.rs`, never `ui.spinner()`: `loading_block` for the main content of a panel (campsite, search results, first page of comments), `loading` (inline, with a label) for a section inside a panel, and `Spinner` with a color override on non-panel backgrounds such as the top bar (campsite count, markers being fetched). A spinner that comes and goes inside a row of widgets uses `Spinner::visible(false)` rather than not being drawn, so it keeps its space and the row doesn't shift (top bar: markers being fetched). A value being refreshed keeps showing its last loaded value rather than going back to a spinner (campsite count).
- Icon characters must exist in egui's default fonts, or they render as an empty box. Use the ones already in the UI: `×` (close, U+00D7, not `✕`), `◀` `▶` `★` `⛺`.
- Client-side validation mirrors PocketBase field constraints for UX, but the server remains authoritative.

### 5.10 SEO (`frontend/assets/seo.json`)

The UI is a canvas, so search engines and link-preview scrapers only see the static HTML (ADR 0012).

- **Every SEO value lives in `seo.json`**: site URL, name, language and locale, title (≤ 60 chars), description (70–160), robots, favicon, Open Graph / Twitter card image, schema.org data, the text of the crawlable page, sitemap paths and robots.txt disallows. Never put SEO tags straight into `index.html`.
- **`tools/seo-gen`** (host binary, workspace member) runs as a Trunk `post_build` hook, in `trunk serve` and `trunk build` alike. It validates `seo.json` (a bad value fails the build) and fills the markers of the staged `index.html`: `lang="seo:lang"`, `<!-- seo:head -->` (title, description, robots, canonical, `theme-color`, favicon, `og:*`, `twitter:*`, JSON-LD `WebSite` + `WebApplication`, page background/text colors) and `<!-- seo:body -->`. It also writes `robots.txt` and `sitemap.xml` and copies the images from `assets/seo/` to the site root. Colors come from `theme.json`.
- **`<main id="about">`** is real content that describes the app: a heading, an intro and the feature list. Crawlers and no-JS browsers read it, and the canvas covers it once the app runs. Keep it truthful to what the app shows, because hidden or keyword-stuffed text is penalized as cloaking.
- **Placeholder domain:** while `site_url` is `https://example.com`, the build prints a warning. Set the real domain before deploying.
- **Images:** `og-image.png` (1200×630, PNG because scrapers don't read SVG) is rendered from `og-image.svg`; the command is in the SVG.
- **At runtime** the app only updates the tab title (`seo.rs`, applied in `app.rs`): the campsite name or panel title, then `· site_name`.
- Only `/` is indexable: panels live in the URL hash, which search engines ignore. Per-campsite pages are future work (ADR 0012).

---

## 6. Cross-cutting concerns

| Concern         | Guideline                                                                                                                                                                                                                                                                                                                                     |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Errors**      | No `unwrap()`/`expect()` outside tests and `main.rs` startup. Errors surface to the user as toasts or inline form errors; log details with `log` (`eframe::WebLogger` sends it to the browser console). `console_error_panic_hook` is installed in `main.rs`. An invalid `theme.json` panics at startup, but `cargo test` catches that first. |
| **Security**    | Authorization only in PocketBase rules. Escape user input in filters. File fields restricted to image MIME types and size limits. Comments rendered as plain text (egui doesn't render HTML — keep it that way).                                                                                                                              |
| **Privacy**     | Never expose user emails publicly. Campsite coordinates are public by design — state this in the UI when posting.                                                                                                                                                                                                                             |
| **Performance** | Bbox queries + field selection + pagination. Thumbnails via PocketBase's `?thumb=WxH` on file URLs — declare thumb sizes on the field. Release builds use `opt-level = "z"` or `"s"`, `lto = true`, and `wasm-opt`.                                                                                                                           |
| **Config**      | Frontend: `frontend/src/config.rs`, baked in at build time — `WWC_API_URL` (PocketBase URL; unset = same origin, the docker-compose default). Tiles are still the public OSM server. Backend: env vars applied at boot by `pb_hooks/settings.pb.js` and `entrypoint.sh` (full list in `.env.example`).                                        |
| **Time**        | Store UTC (PocketBase does); format in local time in the UI.                                                                                                                                                                                                                                                                                  |

---

## 7. Development environment (docker compose)

Services:

| Service      | Purpose                                                                                                                                                    | Ports                      |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `pocketbase` | Built from `backend/Dockerfile`, pinned version, runs `serve --http=0.0.0.0:8090` with automigrate enabled (`PB_DEV=1`)                                    | `8090` (API + admin `/_/`) |
| `mailpit`    | Catches the emails PocketBase sends (confirmation links)                                                                                                   | `8025` (web UI)            |
| `frontend`   | Rust image with `wasm32-unknown-unknown` + `trunk`; runs `trunk serve --address 0.0.0.0` with hot reload; proxies `/api/` to `http://pocketbase:8090/api/` | `8080`                     |

Volumes:

- `pb_data` → named volume (DB + uploads).
- `./backend/pb_migrations` and `./backend/pb_hooks` → bind-mounted so schema/hook edits apply live and generated migrations land in the repo.
- `./frontend` → bind-mounted source; **cargo registry and `target/` in named volumes** so rebuilds stay fast and the host `target/` isn't polluted.

Conventions:

- `docker compose up` must be the only command needed to get a working app at `http://localhost:8080`.
- A superuser for dev is created from `.env` values (`PB_ADMIN_EMAIL`, `PB_ADMIN_PASSWORD`) via `pocketbase superuser upsert` in the entrypoint. Dev credentials only; never reuse elsewhere.
- Seed data (a few users including one `role = admin`, ~50 campsites spread on the map with varied tags and tent capacities, ratings, comments, a few open reports) via `backend/seed/`, runnable with one command (document it in the README).
- Frontend devs may also run `trunk serve` on the host against the dockerized PocketBase, setting `TRUNK_SERVE_PROXY_BACKEND` and `TRUNK_SERVE_PROXY_REWRITE` (see README).
- Seed and rule-test users are marked `verified` through the superuser API (no email round-trip).

### 7.1 Production (ADR 0012)

- **Frontend:** `WWC_API_URL=https://api.example.com scripts/build-frontend.sh` → `frontend/dist/` (release build, wasm-opt). Host it on any static host; the API URL is baked in, so one build per environment.
- **Backend:** the `backend/Dockerfile` image is self-contained: migrations and hooks baked in, non-root, checksum-verified PocketBase, `VOLUME /pb_data`. No `--automigrate`. `PB_ORIGINS` restricts CORS to the frontend origin. Published releases are pushed to `ghcr.io/<owner>/wwc-backend` by `.github/workflows/backend-image.yml` (ADR 0013).
- **Storage:** SQLite stays on a persistent volume at `/pb_data`; uploaded files go to S3 (`PB_S3_*`) and scheduled backups to the same bucket (or `PB_BACKUPS_S3_*`). Losing the volume = restore the latest backup from the dashboard.
- **Email:** SMTP from `PB_SMTP_*`; `PB_APP_URL` is the public PocketBase URL used in email links.
- **Instance settings as code:** `backend/pb_settings.json` (non-secret: rate limits, logs, batch, app name) → optional `PB_SETTINGS_FILE` (per-deployment overrides) → `pb_settings.dev.json` in dev → `PB_*` env vars (secrets, per-environment values). Applied on every boot by `pb_hooks/settings.pb.js`; the dashboard is for inspection. Upload limits are field options, so they change through a migration, not settings.
- TLS termination, domain and the host itself are left to the platform.

---

## 8. Coding standards

- Rust toolchain pinned in `rust-toolchain.toml`; edition 2021 or later.
- `cargo fmt` and `cargo clippy --target wasm32-unknown-unknown -- -D warnings` must pass.
- Small modules; one screen per file; functions over ~60 lines are a smell in UI code.
- Dependencies: prefer well-maintained crates already in the tree. Adding a new crate requires checking wasm compatibility and noting it in the PR description.
- Keep `egui`, `eframe`, `egui_extras`, and `walkers` versions **in lockstep** — walkers depends on a specific egui version. Upgrade them together.
- JS in `pb_hooks`/`pb_migrations`: plain ES5-ish JS as supported by PocketBase's goja runtime; no npm packages.

---

## 9. Testing strategy

| Level         | What                                                                                                                                                                                                                                                                                             | How                                                                                                          |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| Unit (Rust)   | `state::apply`, controller logic, filter building/escaping, DTO (de)serialization against recorded PocketBase JSON fixtures                                                                                                                                                                      | `cargo test` on the host (pure logic, no wasm needed)                                                        |
| Wasm          | Anything touching `web-sys` (currently only `main.rs`)                                                                                                                                                                                                                                           | `wasm-bindgen-test` (headless browser), only where needed                                                    |
| Backend rules | Every API rule: anon/user/owner/other-user/admin cases for list/view/create/update/delete. Must include: rating or commenting on your own campsite is rejected; a user can't set `role`/`hidden`/`status`; hidden content is invisible to others; non-admins can't list reports they didn't file | Script (e.g. shell + curl or a small Rust integration test) against the dockerized PocketBase with seed data |
| Manual smoke  | Map loads, markers appear, login, create campsite with photos (add, remove, 3/3 limit, bad type/size), browse them in the viewer (arrows, keys, thumbnails, Esc/backdrop close), rate, comment, narrow width                                                                                     | Checklist in the PR                                                                                          |

**A change to an API rule without a corresponding rule test is incomplete.**

---

## 10. Definition of done (for agents)

A task is done when:

1. `docker compose up` still yields a working app.
2. fmt, clippy (wasm target), and tests pass.
3. Schema changes are committed as migrations; rules documented in `docs/API.md`.
4. New screens handle `Loading`, `Failed`, and empty states, and work at narrow width.
5. No secrets committed; new env vars added to `.env.example`.
6. This document is updated if the change affects architecture.

---

## 11. Roadmap hints (not yet in scope)

Full-text search (beyond the title substring search), favorites, a richer admin interface (stats, bulk actions, tag management in-app), auto-hide after N reports, marker clustering server-side, offline tile caching, OAuth providers, a custom email-confirmation page in the app, i18n.

---

## 12. Decision records

Significant decisions are recorded in `docs/adr/NNNN-title.md` (Context → Decision → Consequences):

- 0001 — egui/wasm web-only frontend (accepted trade-off: canvas rendering, limited a11y/SEO).
- 0002 — PocketBase as backend; rules as the authorization layer; JS hooks over custom Go build.
- 0003 — `walkers` + OSM tiles for the map in dev.
- 0004 — Viewport bbox querying instead of loading all campsites.
- 0005 — Product rules: owner-only editing, no rating or commenting on your own campsite, exact locations public.
- 0006 — Moderation: user reports + admin review; `hidden` flag instead of deletion; admin = `users.role`.
- 0007 — Campsite properties: tags as an admin-managed collection; `tent_capacity` 1–10 where 10 = "10+".
- 0008 — UI: one page with the map, a top bar, and collapsible side panels; flat design; all styling in `theme.json`.
- 0009 — MVP simplifications: `ehttp`, eframe storage for the session, lat/lng number fields, bash rule tests.
- 0010 — Campsite photos: file field (3 max), multipart `@jsonPayload` saves, web-sys file picker with browser-made previews, `egui_extras` image loaders.
- 0011 — Search by name (top bar → Search panel), global map filters (tags, tent range), client-side screen-space marker clustering.
- 0012 — Production deployment: static frontend with a build-time API URL, PocketBase image with a persistent volume, S3 for files and backups, settings from env, email verification required to post.
- 0013 — Backend image built and pushed to GHCR on each published GitHub Release (semver tags, `latest` for non-prereleases).
- 0014 — Build-time SEO: `seo.json` rendered into the static page by a Trunk hook (`tools/seo-gen`); crawlable `#about` block; only `/` indexed.

---

## 13. Implementation status (MVP)

**Done:**

- Map with markers loaded by viewport.
- Campsite count in the top bar.
- Register (logs you in), log in, log out, a session that survives reloads.
- Profile: display name and password.
- Create, edit and delete a campsite: draft pin on the map, tags, tent capacity.
- Campsite panel: stats, star rating (create or update), paginated comments, deleting your own comments.
- Collapsible and closable side panel, with an unsaved-changes prompt.
- Toasts.
- Campsite photos: up to 3 per campsite, uploaded from the campsite form, shown in the Campsite panel (ADR 0010).
- All API rules plus the rule-test script, seed script and docker compose stack.
- Search by campsite name (top bar → Search panel), global map filters (tags, tent capacity range) and marker clustering (ADR 0011).

- Reports: the `reports` collection and a Report panel for campsites and comments (§5.8). Admins read them in the dashboard.
- Email verification (required to post) with mailpit in dev, and a production setup: static frontend build, PocketBase image, S3 files and backups, SMTP (ADR 0012).
- Backend image published to GHCR on each GitHub Release (ADR 0013).

**Schema ready, UI not built yet:** the `hidden` flag and the admin `role` (admins can already hide content through the API).

**Deferred (roadmap order):**

1. Moderation panel (§5.8), with `resolved_by` and reporting users.
2. URL hash routing (§5.3).
3. Avatar upload (reuse the photo picker and multipart client).
4. Email change and account deletion.
5. Custom fonts.
6. Narrow-screen polish.
7. Photo extras: client-side resize before upload, protected files for hidden campsites.
