# 0009 — MVP simplifications

Status: accepted (2026-09-25)

## Context
The first version should stay small.

## Decision
`ehttp` callbacks + an mpsc channel (no async runtime). eframe storage for the session. Two number fields for coordinates. Bash + curl rule tests. Panel state in memory (no URL routing yet). Panels may edit form input buffers directly.

## Consequences
Fewer moving parts. Revisit when routing, photos or heavier API use arrive.
