# 0006 — Moderation model

Status: accepted (2026-09-25)

## Context
Users must be able to report content, and admins need to act without destroying data.

## Decision
User reports + admin review. Admins set `hidden` instead of deleting. Admin = `users.role = "admin"`, set in the dashboard only.

## Consequences
The `hidden` and `role` fields exist. The `reports` collection and the Report panel exist (campsites and comments);
admins read reports in the PocketBase dashboard until the moderation panel is built. Reporting users and
`resolved_by` come with that panel.
