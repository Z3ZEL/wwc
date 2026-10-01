//! WWC frontend: egui app compiled to WebAssembly (see docs/ARCHITECTURE.md §5).
//! The app shell and the map view only build for wasm32; everything else also builds on the
//! host so `cargo test` runs without a browser.

pub mod actions;
pub mod api;
pub mod config;
pub mod consent;
pub mod controller;
pub mod documents;
pub mod map;
pub mod media;
pub mod seo;
pub mod state;
pub mod ui;

#[cfg(target_arch = "wasm32")]
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod web;
