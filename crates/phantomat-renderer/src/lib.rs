//! Headless [**wgpu**](https://wgpu.rs/) rendering and PNG export for Phantomat.

pub mod compute;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod scene;

pub use error::RendererError;
#[cfg(not(target_arch = "wasm32"))]
pub use headless::HeadlessRenderer;
pub use scene::Renderable;
pub use scene::{ClearScene, Scene, TriangleScene};
