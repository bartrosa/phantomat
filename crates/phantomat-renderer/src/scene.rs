//! Minimal scenes: solid clear and a single colored triangle in NDC.

use wgpu::util::DeviceExt;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};

/// Renders into the given `view` (single mip, 2D).
pub trait Renderable {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    );
}

/// Full-screen clear to a single premultiplied-ish float color (linear in render pass).
pub struct ClearScene {
    pub color: [f32; 4],
}

/// Triangle in normalized device coordinates (clip-space XY, CCW, Z = 0).
pub struct TriangleScene {
    pub positions: [[f32; 2]; 3],
    pub color: [f32; 4],
}

/// Dispatch wrapper for golden tests.
pub enum Scene {
    Clear(ClearScene),
    Triangle(TriangleScene),
}

impl Renderable for ClearScene {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        _device: &Device,
        _queue: &Queue,
        _format: TextureFormat,
    ) {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.color[0] as f64,
                        g: self.color[1] as f64,
                        b: self.color[2] as f64,
                        a: self.color[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        drop(pass);
    }
}

impl Renderable for TriangleScene {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    ) {
        render_triangle(self, encoder, view, device, queue, format);
    }
}

impl Renderable for Scene {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    ) {
        match self {
            Scene::Clear(s) => s.render(encoder, view, device, queue, format),
            Scene::Triangle(s) => s.render(encoder, view, device, queue, format),
        }
    }
}

fn render_triangle(
    scene: &TriangleScene,
    encoder: &mut CommandEncoder,
    view: &TextureView,
    device: &Device,
    queue: &Queue,
    format: TextureFormat,
) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("triangle_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/triangle.wgsl").into()),
    });

    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("triangle_uniform_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("triangle_pipeline_layout"),
        bind_group_layouts: &[&uniform_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("triangle_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Uni {
        color: [f32; 4],
    }

    let u = Uni { color: scene.color };

    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("triangle_uniform"),
        contents: bytemuck::bytes_of(&u),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("triangle_bind_group"),
        layout: &uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });

    let vertices: [[f32; 2]; 3] = scene.positions;
    let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("triangle_vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Clear to black, then draw (stable golden vs. undefined load).
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("triangle_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.draw(0..3, 0..1);
    }

    // Silence unused `queue` until we need uploads mid-frame (uniform already initialized buffer).
    let _ = queue;
}
