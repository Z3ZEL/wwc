#!/bin/sh
set -e

PB="pocketbase --dir=/pb_data --migrationsDir=/pb_migrations --hooksDir=/pb_hooks"

# Apply migrations first so the superuser upsert runs against an up-to-date schema.
$PB migrate up

# Dev: always. Production: set these for the first boot only (or keep a strong secret).
if [ -n "$PB_ADMIN_EMAIL" ] && [ -n "$PB_ADMIN_PASSWORD" ]; then
  $PB superuser upsert "$PB_ADMIN_EMAIL" "$PB_ADMIN_PASSWORD"
fi

set -- --http=0.0.0.0:8090
# Dashboard schema edits write migration files: dev only, production runs committed migrations.
[ "$PB_DEV" = "1" ] && set -- "$@" --automigrate
# CORS: the frontend's origin(s), comma-separated. Unset = any origin (dev).
[ -n "$PB_ORIGINS" ] && set -- "$@" --origins="$PB_ORIGINS"

# Test instances (ADR 0023): seed demo data once PocketBase answers. Runs on every boot because a
# disk-less host starts from an empty /pb_data; seed.sh skips what's already there.
if [ "$WWC_SEED" = "1" ]; then
  (
    until wget -qO- http://127.0.0.1:8090/api/health >/dev/null 2>&1; do sleep 1; done
    PB_URL=http://127.0.0.1:8090 /seed/seed.sh || echo "seed failed" >&2
  ) &
fi

exec $PB serve "$@"
