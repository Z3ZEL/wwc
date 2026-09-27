# WWC — Campsite sharing map

An interactive map of campsites. Anyone can browse; registered users can add campsites and rate and comment on other people's.

- Frontend: Rust + egui, compiled to WebAssembly (`frontend/`)
- Backend: PocketBase (`backend/`)
- Architecture and conventions: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) · API: [`docs/API.md`](docs/API.md)

## Run (development)

```sh
cp .env.example .env          # dev-only superuser credentials
docker compose up --build     # first build takes a few minutes
backend/seed/seed.sh          # optional: 3 users + 50 campsites (needs curl, jq)
```

- App: http://localhost:8080
- PocketBase admin: http://localhost:8090/_/ (credentials from `.env`)
- Seeded users: `alice@example.com` (admin), `bob@example.com`, `carol@example.com`, all with password `password123`

Frontend edits hot-reload (Trunk watches `frontend/src` and `frontend/assets`). Schema changes go in `backend/pb_migrations/`, and PocketBase applies them on restart: `docker compose restart pocketbase`.

### Frontend on the host (faster rebuilds)

Keep PocketBase running in Docker and run Trunk locally (`rustup` picks up the toolchain from `rust-toolchain.toml`; install Trunk with `cargo install trunk`):

```sh
docker compose up -d pocketbase
cd frontend
TRUNK_SERVE_PROXY_BACKEND=http://localhost:8090/api/ TRUNK_SERVE_PROXY_REWRITE=/api/ trunk serve
```

## Checks

```sh
cargo fmt --check
cargo clippy -p frontend --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p frontend --lib --tests -- -D warnings
cargo test -p frontend --lib
backend/tests/rules.sh        # API permission tests, against the running PocketBase
```

CI runs the same checks on every pull request and every push to `master` ([`.github/workflows/quality-gate.yml`](.github/workflows/quality-gate.yml)).
PRs that touch `backend/pb_migrations/` also get an automatic "database structure change" warning comment ([`.github/workflows/db-migration-check.yml`](.github/workflows/db-migration-check.yml)).

## Tuning the look

All colors, fonts, sizes and spacing live in [`frontend/assets/theme.json`](frontend/assets/theme.json). `cargo test` checks that the file parses and that text stays readable (contrast ≥ 4.5:1).
