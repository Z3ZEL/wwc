# 0002 — PocketBase as the backend

Status: accepted (2026-09-25)

## Context
We need auth, a REST API, file storage and an admin UI with minimal code.

## Decision
PocketBase (pinned version). Collection API rules are the authorization layer. JS migrations and hooks; no custom Go build.

## Consequences
Every permission lives in `pb_migrations` and is covered by `backend/tests/rules.sh`. PocketBase upgrades are deliberate because it is pre-1.0.
