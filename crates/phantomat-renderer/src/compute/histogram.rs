//! 2D histogram using atomic storage (`u32` counts).

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use wgpu::util::DeviceExt;

#[derive(Debug, Error)]
pub enum ComputeError {
    #[error("histogram compute aggregation disabled at compile time (feature compute_aggregation off)")]
    Disabled,
    #[error("invalid histogram arguments: {0}")]
    InvalidInput(String),
    #[error("wgpu pipeline / submission error: {0}")]
    Gpu(String),
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HistParams {
    range_x: [f32; 2],
    range_y: [f32; 2],
    bins_x: u32,
    bins_y: u32,
    n_inputs: u32,
    _pad: u32,
}

/// Read-back 2D counts as `grid[by][bx]` (same layout as [`phantomat_core::reference::histogram_2d_cpu`]).
#[cfg(feature = "compute_aggregation")]
pub fn histogram_2d_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    xs: &[f64],
    ys: &[f64],
    bins_x: usize,
    bins_y: usize,
    range: ((f64, f64), (f64, f64)),
) -> Result<Vec<Vec<u32>>, ComputeError> {
    if bins_x == 0 || bins_y == 0 {
        return Ok(Vec::new());
    }
    let n = xs.len().min(ys.len());
    if n == 0 {
        return Ok(vec![vec![0u32; bins_x]; bins_y]);
    }
    let ((xmin, xmax), (ymin, ymax)) = range;
    let wx = xmax - xmin;
    let wy = ymax - ymin;
    if !wx.is_finite() || !wy.is_finite() || wx <= 0.0 || wy <= 0.0 {
        return Err(ComputeError::InvalidInput("degenerate range".into()));
    }

    let params = HistParams {
        range_x: [xmin as f32, xmax as f32],
        range_y: [ymin as f32, ymax as f32],
        bins_x: bins_x as u32,
        bins_y: bins_y as u32,
        n_inputs: n as u32,
        _pad: 0,
    };

    let mut xy: Vec<[f32; 2]> = Vec::with_capacity(n);
    for i in 0..n {
        xy.push([xs[i] as f32, ys[i] as f32]);
    }
    let xy_bytes: &[u8] = bytemuck::cast_slice(&xy);
    let bin_count = bins_x * bins_y;
    let bin_bytes = (bin_count * 4) as u64;

    let xy_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("hist2d_xy"),
        contents: xy_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });

    let bins_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hist2d_bins"),
        size: bin_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(&bins_staging, 0, &vec![0u8; bin_bytes as usize]);

    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("hist2d_params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("histogram_2d"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("histogram_2d.wgsl"))),
    });

    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("hist2d_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hist2d_pl"),
        bind_group_layouts: &[&bind_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("hist2d_pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("hist2d_bg"),
        layout: &bind_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: xy_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: bins_staging.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    });

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hist2d_readback"),
        size: bin_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hist2d_enc"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("hist2d_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let groups = (n as u32).div_ceil(64);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&bins_staging, 0, &readback, 0, bin_bytes);
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .expect("map")
        .map_err(|e| ComputeError::Gpu(e.to_string()))?;
    let data = slice.get_mapped_range();
    let raw: &[u32] = bytemuck::cast_slice(&data);
    let mut grid = Vec::with_capacity(bins_y);
    for row in 0..bins_y {
        let start = row * bins_x;
        grid.push(raw[start..start + bins_x].to_vec());
    }
    drop(data);
    readback.unmap();

    Ok(grid)
}

#[cfg(not(feature = "compute_aggregation"))]
pub fn histogram_2d_gpu(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _xs: &[f64],
    _ys: &[f64],
    _bins_x: usize,
    _bins_y: usize,
    _range: ((f64, f64), (f64, f64)),
) -> Result<Vec<Vec<u32>>, ComputeError> {
    Err(ComputeError::Disabled)
}
