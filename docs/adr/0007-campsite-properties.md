# 0007 — Campsite properties

Status: accepted (2026-09-25)

## Context
The property list will change often.

## Decision
Yes/no properties are rows in an admin-managed `tags` collection (safe water, river, flat…). `tent_capacity` is an integer 1–10 where 10 means "10+".

## Consequences
New tags need no migration. The frontend never hard-codes the tag list.
