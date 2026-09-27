# 0012 — Production deployment

Status: accepted (2026-09-27)

## Context
Everything so far was dev-only: a compose stack with a Trunk dev server proxying `/api` to PocketBase, and
migrations/hooks bind-mounted into a bare PocketBase image. Production needs a static frontend bundle, a
self-contained backend image with persistent storage, uploaded files on S3, backups, and SMTP for the
confirmation emails (ARCHITECTURE §4.6 planned "verification required before posting, once SMTP exists").

## Decision
- **Split origins.** The frontend is a static bundle (`scripts/build-frontend.sh` → `trunk build --release`)
  hosted on any static host; PocketBase runs separately. The PocketBase URL is baked in at build time
  (`WWC_API_URL`, `frontend/src/config.rs`; unset = same origin, so dev is unchanged). CORS is restricted with
  `serve --origins=$PB_ORIGINS`. Auth uses the `Authorization` header, not cookies, so CORS needs no credentials.
  Rejected: serving the bundle from PocketBase (`pb_public`) or a reverse proxy on one origin — fewer moving
  parts at runtime, but it couples frontend releases to backend images and needs a proxy we'd have to operate.
- **One backend image** for dev and prod: pinned PocketBase verified against the release `checksums.txt`,
  migrations and hooks copied in (dev bind-mounts over them), non-root user, `VOLUME /pb_data`, healthcheck.
  `--automigrate` only with `PB_DEV=1`: production runs committed migrations only.
- **Storage.** SQLite stays on the `/pb_data` volume (PocketBase can't put its DB on S3). Uploaded files go
  to S3. Scheduled backups (a zip of `pb_data`) go to the same bucket by default: files live under
  `<collectionId>/<recordId>/`, backups at the bucket root, so they don't collide. `PB_BACKUPS_S3_*` can point
  them at a separate bucket. Disaster recovery = new volume + restore the latest backup from the dashboard.
- **Settings as code.** PocketBase keeps its settings in a table, not in flags. `pb_hooks/settings.pb.js`
  rebuilds them on every boot, in layers: the committed `backend/pb_settings.json` (non-secret: rate limits,
  logs, batch API, app name; PocketBase's own settings shape, deep-merged, unknown keys fail the boot), an
  optional mounted `PB_SETTINGS_FILE` per deployment, `pb_settings.dev.json` in dev, then `PB_*` env vars for
  secrets and per-environment values (S3, backups, SMTP, URLs; full list in `.env.example`).
  Upload limits are not settings but field options (`photos.maxSize`): they stay in migrations, mirrored by
  the frontend's `MAX_PHOTO_BYTES`, so the schema remains fully described by `pb_migrations/`.
- **Email verification required to post.** A migration prefixes the create rules of campsites, ratings,
  comments and reports with `@request.auth.verified = true`. Register sends the confirmation email; the link
  opens PocketBase's own confirmation page (`PB_APP_URL` = public PocketBase URL). The UI shows a
  "Confirm your email" prompt with resend / refresh instead of the create forms. Dev uses mailpit.

## Consequences
- One frontend build per environment (the API URL is compiled in).
- Existing dev `pb_data` volumes created by the old root image need a one-time `chown` (README).
- Accounts created before this change are unverified: they must confirm (Resend email) or an admin sets
  `verified` in the dashboard before they can post again.
- Settings changed in the dashboard are overwritten on the next boot for the keys covered by the settings
  files or env vars. Change them in the repo.
- No CI publishing yet; images are built where they're deployed.
