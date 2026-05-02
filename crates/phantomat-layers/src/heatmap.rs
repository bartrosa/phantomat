//! Heatmap layer: compute accumulation into a 2D bin grid (fixed-point `u32` weights), flatten to
//! plain storage for sampling, then full-screen colormap. When `compute_aggregation` is disabled,
//! bins are filled on the CPU (same semantics) and only the present pass runs.

use std::mem;
use std::sync::Mutex;

use bytemuck::{Pod, Zeroable};
use phantomat_core::reference::heatmap_2d_weighted_wgpu_semantics;
use phantomat_core::ColorRamp;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, ColorTargetState, ColorWrites, CommandEncoder,
    ComputePipeline, Device, MultisampleState, Queue, RenderPipeline,
    ShaderModuleDescriptor, ShaderSource, StoreOp, TextureFormat, VertexState,
};

use phantomat_renderer::Renderable;

use crate::layer::Layer;

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FlattenParams {
    n_bins: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PresentParams {
    bins: [u32; 2],
    max_count: f32,
    _pad: u32,
    color_low: [f32; 4],
    color_high: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Xyw {
    xy: [f32; 2],
    weight: f32,
    _pad: f32,
}

struct HeatmapGpuState {
    target_format: TextureFormat,
    bins: (u32, u32),
    compute_accum: ComputePipeline,
    compute_flatten: ComputePipeline,
    present_pipeline: RenderPipeline,
    accum_bgl: BindGroupLayout,
    accum_bind_group: BindGroup,
    flatten_bind_group: BindGroup,
    present_bind_group: BindGroup,
    bins_atomic: Buffer,
    bins_plain: Buffer,
    points_buffer: Buffer,
    uniform_hist: Buffer,
    uniform_flatten: Buffer,
    uniform_present: Buffer,
    points_capacity: u32,
}

impl HeatmapGpuState {
    fn new(device: &Device, format: TextureFormat, bins: (u32, u32)) -> Self {
        let (bx, by) = (bins.0.max(1), bins.1.max(1));
        let n_bins = bx * by;
        let bin_bytes = u64::from(n_bins) * 4u64;
        let n_initial = 256u32;

        let shader_acc = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("heatmap_accumulate"),
            source: ShaderSource::Wgsl(include_str!("shaders/heatmap_accumulate.wgsl").into()),
        });
        let shader_flat = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("heatmap_flatten"),
            source: ShaderSource::Wgsl(include_str!("shaders/heatmap_flatten.wgsl").into()),
        });
        let shader_present = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("heatmap_present"),
            source: ShaderSource::Wgsl(include_str!("shaders/heatmap_present.wgsl").into()),
        });

        let accum_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("heatmap_accum_bgl"),
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

        let flatten_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("heatmap_flatten_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
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

        let present_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("heatmap_present_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let accum_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("heatmap_accum_pl"),
            bind_group_layouts: &[&accum_bgl],
            push_constant_ranges: &[],
        });
        let flatten_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("heatmap_flatten_pl"),
            bind_group_layouts: &[&flatten_bgl],
            push_constant_ranges: &[],
        });
        let present_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("heatmap_present_pl"),
            bind_group_layouts: &[&present_bgl],
            push_constant_ranges: &[],
        });

        let compute_accum = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("heatmap_compute_accum"),
            layout: Some(&accum_pl),
            module: &shader_acc,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let compute_flatten = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("heatmap_compute_flatten"),
            layout: Some(&flatten_pl),
            module: &shader_flat,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let present_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("heatmap_present"),
            layout: Some(&present_pl),
            vertex: VertexState {
                module: &shader_present,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_present,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bins_atomic = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_bins_atomic"),
            size: bin_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bins_plain = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_bins_plain"),
            size: bin_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let points_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_xyw"),
            size: u64::from(n_initial) * mem::size_of::<Xyw>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_hist = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_hist_uniform"),
            size: mem::size_of::<HistParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_flatten = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_flatten_uniform"),
            size: mem::size_of::<FlattenParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_present = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_present_uniform"),
            size: mem::size_of::<PresentParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let accum_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("heatmap_accum_bg"),
            layout: &accum_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: points_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bins_atomic.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_hist.as_entire_binding(),
                },
            ],
        });

        let flatten_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("heatmap_flatten_bg"),
            layout: &flatten_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bins_atomic.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bins_plain.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_flatten.as_entire_binding(),
                },
            ],
        });

        let present_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("heatmap_present_bg"),
            layout: &present_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bins_plain.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_present.as_entire_binding(),
                },
            ],
        });

        Self {
            target_format: format,
            bins: (bx, by),
            compute_accum,
            compute_flatten,
            present_pipeline,
            accum_bgl,
            accum_bind_group,
            flatten_bind_group,
            present_bind_group,
            bins_atomic,
            bins_plain,
            points_buffer,
            uniform_hist,
            uniform_flatten,
            uniform_present,
            points_capacity: n_initial,
        }
    }

    fn ensure_points_capacity(&mut self, device: &Device, needed: u32) {
        if needed <= self.points_capacity {
            return;
        }
        let new_cap = needed.next_power_of_two().max(1);
        self.points_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heatmap_xyw"),
            size: u64::from(new_cap) * mem::size_of::<Xyw>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.points_capacity = new_cap;
        self.accum_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("heatmap_accum_bg"),
            layout: &self.accum_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.points_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.bins_atomic.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_hist.as_entire_binding(),
                },
            ],
        });
    }
}

/// Binned density surface with a two-stop colormap.
pub struct HeatmapLayer {
    pub positions: Vec<[f32; 2]>,
    pub weights: Vec<f32>,
    pub bins: (u32, u32),
    /// Data domain in **f32** (default `(-1,1)` for X and Y).
    pub data_range: ((f32, f32), (f32, f32)),
    pub color_ramp: ColorRamp,
    pub canvas_px: (u32, u32),
    clear_before_draw: bool,
    gpu: Mutex<Option<HeatmapGpuState>>,
}

impl HeatmapLayer {
    #[must_use]
    pub fn new(
        positions: Vec<[f32; 2]>,
        weights: Vec<f32>,
        bins: (u32, u32),
        color_ramp: ColorRamp,
        canvas_px: (u32, u32),
    ) -> Self {
        assert_eq!(
            positions.len(),
            weights.len(),
            "positions and weights length mismatch"
        );
        Self {
            positions,
            weights,
            bins,
            data_range: ((-1.0, 1.0), (-1.0, 1.0)),
            color_ramp,
            canvas_px,
            clear_before_draw: true,
            gpu: Mutex::new(None),
        }
    }

    pub fn set_clear_before_draw(&mut self, clear: bool) {
        self.clear_before_draw = clear;
    }

    #[must_use]
    pub fn canvas_px(&self) -> (u32, u32) {
        self.canvas_px
    }

    fn build_xyw(&self) -> Vec<Xyw> {
        let n = self.positions.len();
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(Xyw {
                xy: self.positions[i],
                weight: self.weights[i],
                _pad: 0.0,
            });
        }
        v
    }

    fn cpu_oracle_grid(&self, bx: usize, by: usize) -> (Vec<Vec<u32>>, f32) {
        let ((rx0, rx1), (ry0, ry1)) = self.data_range;
        let xs: Vec<f64> = self.positions.iter().map(|p| f64::from(p[0])).collect();
        let ys: Vec<f64> = self.positions.iter().map(|p| f64::from(p[1])).collect();
        let grid = heatmap_2d_weighted_wgpu_semantics(
            &xs,
            &ys,
            &self.weights,
            bx,
            by,
            ((rx0 as f64, rx1 as f64), (ry0 as f64, ry1 as f64)),
        );
        let max_v = grid.iter().flatten().copied().max().unwrap_or(0);
        (grid, max_v as f32)
    }

    #[cfg(not(feature = "compute_aggregation"))]
    fn flatten_grid(grid: &[Vec<u32>]) -> Vec<u32> {
        let mut out = Vec::new();
        for row in grid {
            out.extend_from_slice(row);
        }
        out
    }
}

impl Renderable for HeatmapLayer {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &wgpu::TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    ) {
        let bx = self.bins.0.max(1);
        let by = self.bins.1.max(1);
        let bx_u = bx as usize;
        let by_u = by as usize;
        let n_bins = bx * by;
        let bin_bytes = u64::from(n_bins) * 4u64;

        let load_op = if self.clear_before_draw {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };

        let n = self.positions.len();
        if n == 0 {
            if self.clear_before_draw {
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("heatmap_clear_only"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                drop(pass);
            }
            return;
        }

        let low = self.color_ramp.sample_rgba(0.0);
        let high = self.color_ramp.sample_rgba(1.0);
        let (oracle_grid, max_count) = self.cpu_oracle_grid(bx_u, by_u);
        #[cfg(feature = "compute_aggregation")]
        let _ = oracle_grid;
        #[cfg(not(feature = "compute_aggregation"))]
        let flat_cpu = Self::flatten_grid(&oracle_grid);

        let mut gpu_slot = self.gpu.lock().expect("heatmap gpu mutex");
        let needs_new = match gpu_slot.as_ref() {
            None => true,
            Some(g) => g.target_format != format || g.bins != (bx, by),
        };
        if needs_new {
            *gpu_slot = Some(HeatmapGpuState::new(device, format, (bx, by)));
        }
        let gpu = gpu_slot.as_mut().expect("heatmap gpu");

        let prepresent = PresentParams {
            bins: [bx, by],
            max_count,
            _pad: 0,
            color_low: low,
            color_high: high,
        };
        queue.write_buffer(&gpu.uniform_present, 0, bytemuck::bytes_of(&prepresent));

        #[cfg(feature = "compute_aggregation")]
        {
            gpu.ensure_points_capacity(device, n as u32);
            let xyw = self.build_xyw();
            queue.write_buffer(&gpu.points_buffer, 0, bytemuck::cast_slice(&xyw));

            let ((rx0, rx1), (ry0, ry1)) = self.data_range;
            let hp = HistParams {
                range_x: [rx0, rx1],
                range_y: [ry0, ry1],
                bins_x: bx,
                bins_y: by,
                n_inputs: n as u32,
                _pad: 0,
            };
            queue.write_buffer(&gpu.uniform_hist, 0, bytemuck::bytes_of(&hp));

            let fp = FlattenParams {
                n_bins,
                _pad: [0; 3],
            };
            queue.write_buffer(&gpu.uniform_flatten, 0, bytemuck::bytes_of(&fp));

            let zeros = vec![0u8; bin_bytes as usize];
            queue.write_buffer(&gpu.bins_atomic, 0, &zeros);
            queue.write_buffer(&gpu.bins_plain, 0, &zeros);

            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("heatmap_accum"),
                    timestamp_writes: None,
                });
                c.set_pipeline(&gpu.compute_accum);
                c.set_bind_group(0, &gpu.accum_bind_group, &[]);
                let groups = (n as u32).div_ceil(64);
                c.dispatch_workgroups(groups, 1, 1);
            }
            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("heatmap_flatten"),
                    timestamp_writes: None,
                });
                c.set_pipeline(&gpu.compute_flatten);
                c.set_bind_group(0, &gpu.flatten_bind_group, &[]);
                let groups = n_bins.div_ceil(64);
                c.dispatch_workgroups(groups, 1, 1);
            }
        }

        #[cfg(not(feature = "compute_aggregation"))]
        {
            let bytes: &[u8] = bytemuck::cast_slice(&flat_cpu);
            debug_assert_eq!(bytes.len() as u64, bin_bytes);
            queue.write_buffer(&gpu.bins_plain, 0, bytes);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heatmap_present_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&gpu.present_pipeline);
            pass.set_bind_group(0, &gpu.present_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

impl Layer for HeatmapLayer {}
