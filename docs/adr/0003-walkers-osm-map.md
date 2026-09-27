# 0003 — walkers + OpenStreetMap tiles

Status: accepted (2026-09-25)

## Context
egui has no built-in map.

## Decision
Use the `walkers` crate with OSM raster tiles in dev.

## Consequences
egui, eframe and walkers are upgraded together. The public OSM tile server is not for production: switch the provider in `map/` before launch.
