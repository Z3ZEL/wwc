# 0008 — One-page UI and theme file

Status: accepted (2026-09-25)

## Context
The app should be simple and flat, and easy to restyle.

## Decision
One page: a full-screen map, a top bar, and one collapsible side panel at a time. All visual values come from `frontend/assets/theme.json`, embedded at build time.

## Consequences
No hard-coded styles in UI code. Tests check that the theme parses and that text contrast is ≥ 4.5:1.
