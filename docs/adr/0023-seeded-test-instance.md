# 0023 — Test instances seed the dev data on every boot

Status: proposed (2026-10-25).

## Context
A test backend runs on Render as a free web service built from `backend/Dockerfile`. It needs data to try
the app against, and the dev seed (`backend/seed/seed.sh`: 3 users with alice as admin, 50 campsites,
ratings, comments, reports) already makes it and is safe to re-run.

Render's free web services have no persistent disk: the filesystem is reset on every deploy, restart and
spin-down (15 minutes without traffic). `/pb_data` starts empty each time. Render's pre-deploy command is
for paid services only, and runs on a separate instance that can't reach the service's disk.

The seed runs through the public API, with one login per user. The production rate limits
(`pb_settings.json`, `*:auth`: 2 requests every 3 s) reject most of those logins.

## Decision
- **The entrypoint seeds when `WWC_SEED=1`.** A background step waits for `/api/health`, then runs
  `/seed/seed.sh` against `127.0.0.1:8090`. It runs on every boot; with a persistent disk, the seed reuses
  the users and skips the campsites. PocketBase still replaces the shell (`exec`) and gets the stop signal.
  A failed seed is logged and leaves the server running.
- **The seed's tools only go into test images.** `bash`, `curl` and `jq` are installed when the build arg
  `WWC_SEED=1` is set. Render turns a service's env vars into build args, so the one variable does both
  there; elsewhere it's `--build-arg WWC_SEED=1`. The release workflow and docker compose don't set it, so
  their images are unchanged apart from the copied script.
- **Rate limits off through the existing settings layers:** `PB_SETTINGS_FILE=/pb_settings.dev.json`. It
  doesn't turn on `--automigrate` the way `PB_DEV=1` would.
- **The seed fails loudly when a user can't log in**, instead of going on with empty tokens and reporting
  success.
- Rejected: a JS hook or migration that writes the records directly. It needs no extra tools, but it
  would duplicate the seed and skip the API rules the seed goes through. A pre-seeded `pb_data` snapshot
  in the image: a binary file to keep in sync with the migrations.

## Consequences
- On the free plan, the demo data is new and random after each spin-down (new ids), and anything added by
  hand is lost.
- The seeded passwords are in the README, and alice is an app admin: anyone with the test URL can log in
  as her. A test instance must never hold real data. Its rate limits are off.
- The superuser env vars stay set on test instances, unlike production's "first boot only".
