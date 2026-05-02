//! GPU compute helpers (histogram aggregation, …).

mod histogram;

pub use histogram::{histogram_2d_gpu, ComputeError};
