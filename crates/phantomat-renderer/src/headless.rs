//! Offscreen wgpu: render to texture, read back, encode PNG.

use std::io::Cursor;

use image::{ImageBuffer, ImageFormat, Rgba};
use wgpu::{Instance, PowerPreference, RequestAdapterOptions, TextureViewDescriptor};

use crate::error::RendererError;
use crate::scene::Renderable;

/// Headless wgpu context at a fixed output size and `Rgba8UnormSrgb` target.
pub struct HeadlessRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl HeadlessRenderer {
    /// Creates a device/queue and picks an adapter: **HighPerformance**, then **LowPower** fallback.
    pub fn new(width: u32, height: u32) -> Result<Self, RendererError> {
        if width == 0 || height == 0 {
            return Err(RendererError::Scene(
                "width and height must be non-zero".into(),
            ));
        }

        let instance = Instance::default();

        let mut adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }));

        if adapter.is_none() {
            adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                force_fallback_adapter: true,
                compatible_surface: None,
            }));
        }

        let adapter = adapter.ok_or(RendererError::NoAdapter)?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("phantomat_headless"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|e| RendererError::DeviceRequest(e.to_string()))?;

        Ok(Self {
            device,
            queue,
            size: (width, height),
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Renders `scene` to a PNG (sRGB) byte vector.
    pub fn render_to_png(&self, scene: &dyn Renderable) -> Result<Vec<u8>, RendererError> {
        let (width, height) = self.size;
        let format = self.format;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("phantomat_offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("phantomat_encoder"),
            });

        scene.render(&mut encoder, &view, &self.device, &self.queue, format);

        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bytes_per_row = width * 4;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

        let buffer_size = (padded_bytes_per_row as u64) * u64::from(height);

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("phantomat_readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("map_async callback")
            .map_err(|e| RendererError::BufferMap(e.to_string()))?;

        let padded = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = row as usize * padded_bytes_per_row as usize;
            let end = start + unpadded_bytes_per_row as usize;
            rgba.extend_from_slice(&padded[start..end]);
        }
        drop(padded);

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba)
            .ok_or_else(|| {
                RendererError::Scene("image dimensions mismatch after readback".into())
            })?;

        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)?;
        Ok(bytes)
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}
