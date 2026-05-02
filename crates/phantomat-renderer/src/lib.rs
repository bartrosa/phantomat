//! Headless [**wgpu**](https://wgpu.rs/) rendering and PNG export for Phantomat.

pub mod error;
pub mod headless;
pub mod scene;

pub use error::RendererError;
pub use headless::HeadlessRenderer;
pub use scene::Renderable;
pub use scene::{ClearScene, Scene, TriangleScene};
