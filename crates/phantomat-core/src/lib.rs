//! Core types and algorithms for Phantomat.

pub mod color;
pub mod reference;
pub mod scale;

pub use color::{
    heatmap_weight_contrib_u32, interpolate_oklch, ColorRamp, Rgb, HEATMAP_WEIGHT_SCALE,
};
pub use scale::{LinearScale, LogScale, Scale, ScaleError};
