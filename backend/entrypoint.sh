#!/bin/sh
set -e

PB="pocketbase --dir=/pb_data --migrationsDir=/pb_migrations --hooksDir=/pb_hooks"

# Apply migrations first so the superuser upsert runs against an up-to-date schema.
$PB migrate up

if [ -n "$PB_ADMIN_EMAIL" ] && [ -n "$PB_ADMIN_PASSWORD" ]; then
  $PB superuser upsert "$PB_ADMIN_EMAIL" "$PB_ADMIN_PASSWORD"
fi

exec $PB serve --http=0.0.0.0:8090 --automigrate
