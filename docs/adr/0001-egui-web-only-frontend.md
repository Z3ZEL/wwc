# 0001 — egui/wasm web-only frontend

Status: accepted (2026-09-25)

## Context
We want one language (Rust) for the frontend and a simple immediate-mode UI.

## Decision
Rust + egui/eframe compiled to `wasm32-unknown-unknown`, built with Trunk. No native target.

## Consequences
Canvas rendering: weak SEO and accessibility, limited mobile text input. Accepted for now. Pure logic is kept free of eframe so it can be unit-tested on the host.
