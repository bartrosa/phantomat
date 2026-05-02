//! CPU reference implementations (oracle algorithms).

pub mod histogram;

pub use histogram::{
    heatmap_2d_weighted_wgpu_semantics, histogram_1d_cpu, histogram_2d_cpu,
    histogram_2d_cpu_wgpu_semantics,
};
