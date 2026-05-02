//! Core types and algorithms for Phantomat.

pub mod color;
pub mod scale;

pub use color::{interpolate_oklch, Rgb};
pub use scale::{LinearScale, LogScale, Scale, ScaleError};
