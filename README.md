# WWC — Campsite sharing map

An interactive map of campsites. Anyone can browse; registered users can add campsites and rate and comment on other people's.

- Frontend: Rust + egui, compiled to WebAssembly (`frontend/`)
- Backend: PocketBase (`backend/`)
- Architecture and conventions: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) · API: [`docs/API.md`](docs/API.md)

## Run (development)

```sh
cp .env.example .env          # dev-only superuser credentials + SMTP to mailpit
docker compose up --build     # first build takes a few minutes
backend/seed/seed.sh          # optional: 3 users + 50 campsites (needs curl, jq)
```

- App: http://localhost:8080
- PocketBase admin: http://localhost:8090/_/ (credentials from `.env`)
- Emails (account confirmation links): http://localhost:8025 (mailpit)
- Seeded users: `alice@example.com` (admin), `bob@example.com`, `carol@example.com`, all with password `password123` (already confirmed)

Posting, rating, commenting and reporting need a confirmed email: after registering, open the link from mailpit, then click "I've confirmed it".

Upgrading an existing dev volume: the PocketBase image now runs as a non-root user, so a `pb_data` volume created before needs this once:
`docker compose run --rm --user root --entrypoint chown pocketbase -R pocketbase:pocketbase /pb_data`

Frontend edits hot-reload (Trunk watches `frontend/src` and `frontend/assets`). Schema changes go in `backend/pb_migrations/`, and PocketBase applies them on restart: `docker compose restart pocketbase`.

### Frontend on the host (faster rebuilds)

Keep PocketBase running in Docker and run Trunk locally (`rustup` picks up the toolchain from `rust-toolchain.toml`; install Trunk with `cargo install trunk`):

```sh
docker compose up -d pocketbase
cd frontend
TRUNK_SERVE_PROXY_BACKEND=http://localhost:8090/api/ TRUNK_SERVE_PROXY_REWRITE=/api/ trunk serve
```

## Production

The frontend is a static site; the backend is one PocketBase container with a persistent volume. Uploaded photos and backups go to S3. See ADR [0012](docs/adr/0012-production-deployment.md).

### Frontend: static build

```sh
WWC_API_URL=https://api.example.com scripts/build-frontend.sh   # → frontend/dist/
```

- `WWC_API_URL` is the public PocketBase URL. It's compiled into the wasm, so you need one build per environment. Set `PUBLIC_URL=/sub/path/` if the site isn't served from `/`.
- No local Rust/Trunk? Build in the dev image: `docker compose run --rm -e WWC_API_URL=https://api.example.com frontend /app/scripts/build-frontend.sh`
- Upload `frontend/dist/` to any static host (S3 + CDN, Netlify, Cloudflare Pages, nginx…). Serve `.wasm` as `application/wasm`. The JS and wasm file names are hashed, so they can be cached forever; serve `index.html` with `Cache-Control: no-cache`.
- Set `PB_ORIGINS` on the backend to the frontend's origin, or every API call is blocked by CORS.

**Building on a host or in CI** (Render, Netlify, Cloudflare Pages, GitHub Actions…): build environments don't ship Trunk or the pinned toolchain, so install them in the same step. On any Linux x86_64 machine with `curl`, `tar` and `cc`:

```sh
sh scripts/install-frontend-tools.sh && sh scripts/build-frontend.sh
```

Set the `WWC_API_URL` env var on the host, and use `frontend/dist` as the publish directory. The install script is idempotent: it installs rustup (in `~/.cargo`), the toolchain from `rust-toolchain.toml`, and the pinned Trunk release, skipping what's already there. If the host preinstalls rustup in a read-only location (Render does), it uses that rustup but puts toolchains and tools in `~/.rustup` and `~/.cargo`.

### Backend: PocketBase image

```sh
cp .env.example .env.prod     # then fill in the "Production" block
docker build -t wwc-pocketbase backend
docker run -d --name wwc-pocketbase --restart unless-stopped \
  -p 8090:8090 --env-file .env.prod -v wwc_pb_data:/pb_data wwc-pocketbase
```

Put a TLS-terminating proxy or load balancer in front of port 8090. The image runs the committed migrations on start, and `/api/health` is its healthcheck.

**Released images:** publishing a GitHub Release tagged `vX.Y.Z` builds the image for `linux/amd64` and `linux/arm64/v8` and pushes it to `ghcr.io/z3zel/wwc-backend` (tags `X.Y.Z`, `X.Y`, `latest`; prereleases get only `X.Y.Z-…`). The tag must be full semver: `v1.0.0-rc.1` works, `1.0-rc.1` gets no version tag. The `sha256-…` entries in the GHCR package page are build attestations, not images to pull. See ADR [0013](docs/adr/0013-backend-image-publishing.md). You can also start it by hand from the Actions tab ("Release").

**Frontend deploy:** a full (non-pre) release then calls the Render deploy hook, which rebuilds the frontend from the branch Render tracks. Store the hook URL in the `DEPLOY_HOOK` repository secret (Settings → Secrets and variables → Actions). See ADR [0015](docs/adr/0015-frontend-deploy-on-release.md).

```sh
gh release create v0.1.0 --generate-notes      # → ghcr.io/z3zel/wwc-backend:0.1.0
docker pull ghcr.io/z3zel/wwc-backend:0.1.0     # use it instead of `docker build` above
```

Hosts that deploy an existing image (e.g. Render's "Existing Image") can use this URL. The package is private after the first publish: make it public in its GitHub package settings, or give the host a registry credential (a token with `read:packages`).

- **Persistent storage:** mount a volume on `/pb_data`. It holds the SQLite database (plus local files if S3 is off). Don't run more than one container on the same volume.
- **CORS:** set `PB_ORIGINS` to the frontend origin (e.g. `https://wwc.example.com`).
- **First superuser:** set `PB_ADMIN_EMAIL` and `PB_ADMIN_PASSWORD` for the first boot, then remove them. The dashboard is at `https://<api host>/_/`.

**General PocketBase settings** (rate limits, logs retention, batch API, app name…) live in [`backend/pb_settings.json`](backend/pb_settings.json), in the same shape as PocketBase's settings (`GET /api/settings`). It's baked into the image and applied on every boot by `backend/pb_hooks/settings.pb.js`. For one deployment, mount your own JSON file and point `PB_SETTINGS_FILE` at it: it's merged on top of the committed file, so it only needs the keys you change. An unknown key stops the boot. Dev layers `backend/pb_settings.dev.json` on top (rate limits off for the rule tests). Keep secrets out of these files and use the env vars below.

The env vars below are applied last. Only sections whose variables are set are touched; the full list is in `.env.example`.

| Variables                                                                                                                                                              | What                                                                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `PB_S3_ENABLED`, `PB_S3_BUCKET`, `PB_S3_REGION`, `PB_S3_ENDPOINT`, `PB_S3_ACCESS_KEY`, `PB_S3_SECRET`, `PB_S3_FORCE_PATH_STYLE`                                        | Uploaded photos on S3 (or any S3-compatible storage)                                                         |
| `PB_BACKUPS_CRON` (default `0 3 * * *`), `PB_BACKUPS_MAX_KEEP` (default 7), optional `PB_BACKUPS_S3_*`                                                                 | Scheduled backups. They go to the `PB_S3_*` bucket (at its root) unless `PB_BACKUPS_S3_*` names another one. |
| `PB_SMTP_ENABLED`, `PB_SMTP_HOST`, `PB_SMTP_PORT`, `PB_SMTP_USERNAME`, `PB_SMTP_PASSWORD`, `PB_SMTP_TLS`, `PB_SMTP_AUTH_METHOD`, `PB_SENDER_NAME`, `PB_SENDER_ADDRESS` | Outgoing mail: account confirmation, password reset                                                          |
| `PB_APP_URL`, `PB_APP_NAME`                                                                                                                                            | Public PocketBase URL (used in email links) and the name shown in emails                                     |
| `PB_TRUSTED_PROXY_HEADERS`                                                                                                                                             | e.g. `X-Forwarded-For` behind a proxy, so client IPs and rate limits are right                               |

Check the S3 and SMTP settings from the dashboard (Settings → Files storage / Backups / Mail settings, "Test connection"). Settings covered by the files or env vars are reset on the next restart, so change them in the repo, not the dashboard.

**Upload size** isn't a PocketBase setting. It's the `maxSize` of the `photos` field, which is part of the schema. To change it, write a migration that updates the field, and update `MAX_PHOTO_BYTES` in `frontend/src/state/forms.rs`. The frontend checks the size before uploading, and the migration's comment points at the frontend constant. PocketBase raises its request body limit to match file fields on its own.

**Restoring:** with a new or empty volume, start the container, log in to the dashboard, open Settings → Backups, pick the latest backup from the bucket and click Restore. PocketBase restarts with that data.

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
