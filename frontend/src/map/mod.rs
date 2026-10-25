//! The full-screen map (ARCHITECTURE §5.5). `view` draws it with walkers and only builds
//! for wasm; `basemap` (tile sources, attribution) and `cluster` are pure and unit-tested on
//! the host.

pub mod basemap;
pub mod cluster;
#[cfg(target_arch = "wasm32")]
mod view;

pub use basemap::{Basemap, Provider};
#[cfg(target_arch = "wasm32")]
pub use view::MapView;
