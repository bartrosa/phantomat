//! GPU visualization layers (scatter, …) built on [`phantomat_renderer`].

mod layer;
pub mod scatter;

pub use layer::Layer;
pub use scatter::ScatterLayer;
