//! Scatter plot layer: instanced quads with SDF disk anti-aliasing.
//!
//! **Blending:** the render pipeline uses `wgpu::BlendState::ALPHA_BLENDING` (source-over
//! with non-premultiplied fragment output: rgb from the material, alpha = `color.a * coverage`).

use std::mem;
use std::sync::{Arc, Mutex};

use arrow::record_batch::RecordBatch;
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, ColorTargetState, ColorWrites, CommandEncoder, Device,
    IndexFormat, MultisampleState, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, StoreOp, TextureFormat, TextureView,
};

use phantomat_renderer::Renderable;

use crate::arrow_schema::{self, ArrowSchemaError};
use crate::layer::Layer;

/// Column-major raw pointers into wasm linear memory (see [`ScatterLayer::from_raw_f32_columns`]).
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct WasmRawCols {
    n: usize,
    x: *const f32,
    y: *const f32,
    r: *const f32,
    g: *const f32,
    b: *const f32,
    a: *const f32,
    s: *const f32,
}

/// Owns a single contiguous block in **wasm linear memory** that was allocated via
/// `wasm-bindgen`'s `__wbindgen_malloc` (which routes through Rust's global allocator).
/// On drop, the block is freed with the matching [`Layout`]. This is the lifetime hook
/// for the column pointers stored in [`WasmRawCols`].
#[cfg(target_arch = "wasm32")]
struct WasmBlock {
    base: *mut u8,
    size: usize,
    align: usize,
}

#[cfg(target_arch = "wasm32")]
impl Drop for WasmBlock {
    fn drop(&mut self) {
        if !self.base.is_null() && self.size > 0 {
            // Safety: matches the `Layout` used by `__wbindgen_malloc(size, align)` —
            // both go through `alloc::alloc::{alloc, dealloc}` in wasm-bindgen 0.2.
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(self.size, self.align);
                std::alloc::dealloc(self.base, layout);
            }
        }
    }
}

/// Per-point instance data. Layout matches WGSL `min` sizes and 16-byte rules for the uniform
/// block is separate; instance stride is 48 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ScatterInstance {
    position: [f32; 2],
    _pad0: [f32; 2],
    color: [f32; 4],
    size_px: f32,
    _pad1: [f32; 3],
}

impl ScatterInstance {
    fn new(position: [f32; 2], color: [f32; 4], size_px: f32) -> Self {
        Self {
            position,
            _pad0: [0.0; 2],
            color,
            size_px,
            _pad1: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CanvasUniform {
    size_px: [f32; 2],
    _pad: [f32; 2],
}

struct ScatterGpuState {
    target_format: TextureFormat,
    pipeline: RenderPipeline,
    _bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    uniform_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    instance_capacity: u32,
    last_canvas: (u32, u32),
}

impl ScatterGpuState {
    fn new(
        device: &Device,
        format: TextureFormat,
        initial_instances: u32,
        canvas: (u32, u32),
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scatter_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scatter.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scatter_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scatter_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scatter_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: 48,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 32,
                                shader_location: 3,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let canvas_u = CanvasUniform {
            size_px: Vec2::new(canvas.0 as f32, canvas.1 as f32).to_array(),
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scatter_canvas_uniform"),
            contents: bytemuck::bytes_of(&canvas_u),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let quad: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scatter_quad_vertices"),
            contents: bytemuck::cast_slice(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scatter_quad_indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let cap = initial_instances.max(1);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scatter_instances"),
            size: u64::from(cap) * mem::size_of::<ScatterInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scatter_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            target_format: format,
            pipeline,
            _bind_group_layout: bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_capacity: cap,
            last_canvas: canvas,
        }
    }

    fn ensure_instance_capacity(&mut self, device: &Device, needed: u32) {
        if needed <= self.instance_capacity {
            return;
        }
        let new_cap = needed.next_power_of_two().max(1);
        self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scatter_instances"),
            size: u64::from(new_cap) * mem::size_of::<ScatterInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_cap;
    }

    fn sync_canvas(&mut self, queue: &Queue, canvas: (u32, u32)) {
        if canvas == self.last_canvas {
            return;
        }
        self.last_canvas = canvas;
        let u = CanvasUniform {
            size_px: Vec2::new(canvas.0 as f32, canvas.1 as f32).to_array(),
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&u));
    }
}

/// Scatter marks in **NDC** (−1…1), colors in linear-ish sRGBA 0…1, sizes in **pixels** (diameter).
///
/// `canvas_px` must match the render target size so pixel-sized disks map correctly.
pub struct ScatterLayer {
    /// Owned path (from [`ScatterLayer::new`]). Empty when using Arrow or raw pointer storage.
    pub positions: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub sizes: Vec<f32>,
    pub canvas_px: (u32, u32),
    /// When `true` (default), the layer clears the render target to black before drawing.
    /// Set to `false` when compositing multiple layers so earlier draws stay visible.
    clear_before_draw: bool,
    gpu: Mutex<Option<ScatterGpuState>>,
    /// Zero-copy path: read instance columns from Arrow (`x`,`y`,`r`,`g`,`b`,`a`,`size` Float32).
    arrow_batch: Option<Arc<RecordBatch>>,
    /// WebAssembly: read columns from pointers into wasm linear memory.
    #[cfg(target_arch = "wasm32")]
    wasm_raw: Option<WasmRawCols>,
    /// Owns the wasm-malloc block backing [`Self::wasm_raw`] when this layer was built
    /// via [`Self::from_arrow_block`]. Dropping the layer frees the block.
    #[cfg(target_arch = "wasm32")]
    wasm_block: Option<WasmBlock>,
}

impl ScatterLayer {
    /// Builds a layer. **Panics** if `positions`, `colors`, and `sizes` length differ.
    #[must_use]
    pub fn new(
        positions: Vec<[f32; 2]>,
        colors: Vec<[f32; 4]>,
        sizes: Vec<f32>,
        canvas_px: (u32, u32),
    ) -> Self {
        let n = positions.len();
        assert_eq!(
            colors.len(),
            n,
            "colors length {} != positions length {}",
            colors.len(),
            n
        );
        assert_eq!(
            sizes.len(),
            n,
            "sizes length {} != positions length {}",
            sizes.len(),
            n
        );
        Self {
            positions,
            colors,
            sizes,
            canvas_px,
            clear_before_draw: true,
            gpu: Mutex::new(None),
            arrow_batch: None,
            #[cfg(target_arch = "wasm32")]
            wasm_raw: None,
            #[cfg(target_arch = "wasm32")]
            wasm_block: None,
        }
    }

    /// Builds from an Arrow [`RecordBatch`] with Float32 columns `x`, `y`, `r`, `g`, `b`, `a`, `size`.
    ///
    /// Retains the underlying Arrow buffers (no copy of column data into Rust `Vec`s).
    pub fn from_arrow(batch: &RecordBatch, canvas_px: (u32, u32)) -> Result<Self, ArrowSchemaError> {
        Self::from_arrow_arc(Arc::new(batch.clone()), canvas_px)
    }

    /// Same as [`ScatterLayer::from_arrow`] but takes an owned [`Arc`] (avoids cloning buffers).
    pub fn from_arrow_arc(batch: Arc<RecordBatch>, canvas_px: (u32, u32)) -> Result<Self, ArrowSchemaError> {
        arrow_schema::validate_scatter_batch(batch.as_ref())?;
        Ok(Self {
            positions: Vec::new(),
            colors: Vec::new(),
            sizes: Vec::new(),
            canvas_px,
            clear_before_draw: true,
            gpu: Mutex::new(None),
            arrow_batch: Some(batch),
            #[cfg(target_arch = "wasm32")]
            wasm_raw: None,
            #[cfg(target_arch = "wasm32")]
            wasm_block: None,
        })
    }

    /// Builds from seven aligned `f32` buffers (length `n`) in wasm linear memory.
    ///
    /// # Safety
    /// Each pointer must be valid for `n` elements until this layer is dropped or replaced via [`ScatterLayer::set_points`].
    #[cfg(target_arch = "wasm32")]
    pub unsafe fn from_raw_f32_columns(
        n: usize,
        x: *const f32,
        y: *const f32,
        r: *const f32,
        g: *const f32,
        b: *const f32,
        a: *const f32,
        size: *const f32,
        canvas_px: (u32, u32),
    ) -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            sizes: Vec::new(),
            canvas_px,
            clear_before_draw: true,
            gpu: Mutex::new(None),
            arrow_batch: None,
            wasm_raw: Some(WasmRawCols {
                n,
                x,
                y,
                r,
                g,
                b,
                a,
                s: size,
            }),
            wasm_block: None,
        }
    }

    /// WebAssembly: builds from a single contiguous **column-major** block in wasm linear
    /// memory holding seven Float32 columns of length `n` (layout: `x | y | r | g | b | a | size`).
    /// The layer **takes ownership of the block** and frees it via [`std::alloc::dealloc`] when
    /// dropped — pair this with `__wbindgen_malloc(total_bytes, align)` on the JS side so the
    /// allocation does not leak across repeated widget updates.
    ///
    /// # Safety
    /// * `base` must point to `total_bytes` valid bytes in wasm linear memory.
    /// * `total_bytes` must equal `n * 7 * size_of::<f32>()`.
    /// * `align` must match the alignment passed to `__wbindgen_malloc` (4 for `f32` columns).
    /// * The caller must transfer ownership exactly once: do **not** also pass the same block
    ///   to another `from_arrow_block` / `from_raw_f32_columns` call.
    #[cfg(target_arch = "wasm32")]
    pub unsafe fn from_arrow_block(
        base: *mut u8,
        total_bytes: usize,
        align: usize,
        n: usize,
        canvas_px: (u32, u32),
    ) -> Self {
        let base_f32 = base as *const f32;
        let cols = WasmRawCols {
            n,
            x: base_f32,
            y: base_f32.add(n),
            r: base_f32.add(2 * n),
            g: base_f32.add(3 * n),
            b: base_f32.add(4 * n),
            a: base_f32.add(5 * n),
            s: base_f32.add(6 * n),
        };
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            sizes: Vec::new(),
            canvas_px,
            clear_before_draw: true,
            gpu: Mutex::new(None),
            arrow_batch: None,
            wasm_raw: Some(cols),
            wasm_block: Some(WasmBlock {
                base,
                size: total_bytes,
                align,
            }),
        }
    }

    /// Debug-only: address of the `x` column values buffer when using Arrow storage (`None` otherwise).
    #[must_use]
    pub fn debug_values_buffer_addr_x(&self) -> Option<usize> {
        let batch = self.arrow_batch.as_ref()?;
        let arr = arrow_schema::col_f32(batch.as_ref(), "x").ok()?;
        Some(arr.values().as_ptr() as usize)
    }

    /// Canvas size in pixels (must match headless renderer dimensions).
    #[must_use]
    pub fn canvas_px(&self) -> (u32, u32) {
        self.canvas_px
    }

    /// Number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.row_count()
    }

    /// Returns `true` when there are no points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.row_count() == 0
    }

    #[must_use]
    fn row_count(&self) -> usize {
        if let Some(b) = &self.arrow_batch {
            return b.num_rows();
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(w) = self.wasm_raw {
            return w.n;
        }
        self.positions.len()
    }

    /// Replaces point data. **Panics** on length mismatch.
    pub fn set_points(&mut self, positions: Vec<[f32; 2]>, colors: Vec<[f32; 4]>, sizes: Vec<f32>) {
        let n = positions.len();
        assert_eq!(colors.len(), n);
        assert_eq!(sizes.len(), n);
        self.positions = positions;
        self.colors = colors;
        self.sizes = sizes;
        self.arrow_batch = None;
        #[cfg(target_arch = "wasm32")]
        {
            self.wasm_raw = None;
            // Drop the owned wasm-malloc block (if any) so its memory is released.
            self.wasm_block = None;
        }
    }

    /// Updates canvas resolution (e.g. after resize). Must match render target.
    pub fn set_canvas_px(&mut self, canvas_px: (u32, u32)) {
        self.canvas_px = canvas_px;
    }

    /// When `false`, this layer loads the existing attachment contents instead of clearing to black.
    pub fn set_clear_before_draw(&mut self, clear: bool) {
        self.clear_before_draw = clear;
    }

    fn build_instances(&self) -> Vec<ScatterInstance> {
        let n = self.row_count();
        let mut out = Vec::with_capacity(n);
        if let Some(batch) = &self.arrow_batch {
            let x = arrow_schema::col_f32(batch.as_ref(), "x").expect("validated batch");
            let y = arrow_schema::col_f32(batch.as_ref(), "y").expect("validated batch");
            let r = arrow_schema::col_f32(batch.as_ref(), "r").expect("validated batch");
            let g = arrow_schema::col_f32(batch.as_ref(), "g").expect("validated batch");
            let b = arrow_schema::col_f32(batch.as_ref(), "b").expect("validated batch");
            let a = arrow_schema::col_f32(batch.as_ref(), "a").expect("validated batch");
            let sz = arrow_schema::col_f32(batch.as_ref(), "size").expect("validated batch");
            for i in 0..n {
                out.push(ScatterInstance::new(
                    [x.value(i), y.value(i)],
                    [r.value(i), g.value(i), b.value(i), a.value(i)],
                    sz.value(i),
                ));
            }
            return out;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(w) = self.wasm_raw {
            unsafe {
                for i in 0..n {
                    out.push(ScatterInstance::new(
                        [*w.x.add(i), *w.y.add(i)],
                        [
                            *w.r.add(i),
                            *w.g.add(i),
                            *w.b.add(i),
                            *w.a.add(i),
                        ],
                        *w.s.add(i),
                    ));
                }
            }
            return out;
        }
        for i in 0..n {
            out.push(ScatterInstance::new(
                self.positions[i],
                self.colors[i],
                self.sizes[i],
            ));
        }
        out
    }
}

impl Renderable for ScatterLayer {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    ) {
        let count = self.row_count() as u32;
        let instances = self.build_instances();
        let canvas = self.canvas_px;

        let mut gpu_slot = self.gpu.lock().expect("scatter gpu mutex poisoned");
        let needs_new_gpu = match gpu_slot.as_ref() {
            None => true,
            Some(g) => g.target_format != format,
        };
        if needs_new_gpu {
            *gpu_slot = Some(ScatterGpuState::new(device, format, count.max(1), canvas));
        }
        let gpu = gpu_slot.as_mut().expect("scatter gpu");

        gpu.ensure_instance_capacity(device, count.max(1));
        gpu.sync_canvas(queue, canvas);

        let instance_bytes: &[u8] = bytemuck::cast_slice(&instances);
        queue.write_buffer(&gpu.instance_buffer, 0, instance_bytes);

        let load_op = if self.clear_before_draw {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };

        if count == 0 {
            if !self.clear_before_draw {
                return;
            }
            let pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("scatter_clear_only"),
                color_attachments: &[Some(RenderPassColorAttachment {
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
            return;
        }

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("scatter_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
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
        pass.set_pipeline(&gpu.pipeline);
        pass.set_bind_group(0, &gpu.bind_group, &[]);
        pass.set_vertex_buffer(0, gpu.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, gpu.instance_buffer.slice(..));
        pass.set_index_buffer(gpu.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..count);
    }
}

impl Layer for ScatterLayer {}
