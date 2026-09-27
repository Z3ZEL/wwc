# WWC — Campsite sharing map

Full-stack web app: an interactive map where anyone can browse campsites; logged-in users can post, rate, and comment.

**Read `docs/ARCHITECTURE.md` before making any change.** It is the source of truth for structure and conventions.

## Stack
- Frontend: Rust + egui/eframe → WebAssembly (web only), built with Trunk, map via `walkers` — `frontend/`
- Backend: PocketBase (pinned version), schema in `backend/pb_migrations/`, server logic in `backend/pb_hooks/`
- Dev: `docker compose up` → app at http://localhost:8080, PocketBase admin at http://localhost:8090/_/

## Non-negotiables
- Authorization lives in PocketBase API rules, never only in the frontend.
- Product rules: owner-only editing; no rating or commenting on your own campsite; exact locations are public; moderation = user reports + admin hides content (`hidden` flag, admins = `users.role = "admin"`).
- Campsite tags come from the `tags` collection — never hard-code them. `tent_capacity` is 1–10, and 10 is displayed as "10+".
- Schema changes = committed migrations. Rule changes = documented in `docs/API.md` + rule tests.
- UI code never does I/O: it emits `Action`s; async results come back as `Event`s (see ARCHITECTURE §5.2).
- No `unwrap()` in app code; wasm is single-threaded — never block.
- `cargo fmt` + `cargo clippy --target wasm32-unknown-unknown -- -D warnings` + `cargo test` must pass.
- Keep egui / eframe / egui_extras / walkers versions in lockstep.
- UI = one page: full-screen map + top bar (campsite count, auth buttons) + one collapsible side panel at a time (ARCHITECTURE §5.6). Flat design.
- Never hard-code colors, fonts, sizes or spacing: everything comes from `frontend/assets/theme.json` via `Theme` (§5.7).
- SEO text and tags come from `frontend/assets/seo.json`, rendered at build time by `tools/seo-gen` (§5.10); never hand-edit SEO tags in `index.html`.
- Architecture changes update `docs/ARCHITECTURE.md` and add an ADR in `docs/adr/`.
