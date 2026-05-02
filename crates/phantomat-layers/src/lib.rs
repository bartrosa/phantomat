//! GPU visualization layers (scatter, …) built on [`phantomat_renderer`].

mod arrow_schema;
mod layer;
pub mod scatter;

pub use arrow_schema::ArrowSchemaError;
pub use layer::Layer;
pub use scatter::ScatterLayer;
