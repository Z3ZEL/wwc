//! The full-screen map (ARCHITECTURE §5.5). `view` draws it with walkers and only builds
//! for wasm; `cluster` is pure and unit-tested on the host.

pub mod cluster;
#[cfg(target_arch = "wasm32")]
mod view;

#[cfg(target_arch = "wasm32")]
pub use view::MapView;
