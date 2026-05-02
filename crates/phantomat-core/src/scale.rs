//! Continuous scales: domain → range mapping.

mod error;
mod linear;
mod log;

pub use error::ScaleError;
pub use linear::LinearScale;
pub use log::LogScale;

/// Maps values from a **domain** interval into a **range** interval.
pub trait Scale {
    /// Maps `value` (in domain space) into range space (unclamped).
    fn apply(&self, value: f64) -> f64;

    /// Inverse of [`apply`](Scale::apply): maps a range value back to domain space.
    fn invert(&self, value: f64) -> f64;

    /// Data domain `(min, max)` (order may match or invert mapping direction).
    fn domain(&self) -> (f64, f64);

    /// Output range `(min, max)`.
    fn range(&self) -> (f64, f64);
}
