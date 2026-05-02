//! Marker trait for drawable visualization layers.

use phantomat_renderer::Renderable;

/// A [`Renderable`] produced by this crate (scatter, future charts, …).
pub trait Layer: Renderable {}
