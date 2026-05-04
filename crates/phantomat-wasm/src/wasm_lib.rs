use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::{
    AdapterInfo, CommandEncoderDescriptor, InstanceDescriptor, RequestAdapterOptions, SurfaceError,
    TextureViewDescriptor,
};

use phantomat_layers::ScatterLayer as InnerScatter;
use phantomat_renderer::Renderable;

#[cfg(debug_assertions)]
#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

/// Browser [`ScatterLayer`]: copies [`Float32Array`] data into Rust vectors (see PR-8 for zero-copy).
#[wasm_bindgen]
pub struct ScatterLayer {
    inner: Option<InnerScatter>,
}

#[wasm_bindgen]
impl ScatterLayer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        positions: js_sys::Float32Array,
        colors: js_sys::Float32Array,
        sizes: js_sys::Float32Array,
    ) -> Result<ScatterLayer, JsValue> {
        let pos = positions.to_vec();
        if pos.len() % 2 != 0 {
            return Err(JsValue::from_str("positions length must be a multiple of 2"));
        }
        let n = pos.len() / 2;
        let positions: Vec<[f32; 2]> = pos
            .chunks_exact(2)
            .map(|c| {
                let s: &[f32] = c;
                [s[0], s[1]]
            })
            .collect();
        let col = colors.to_vec();
        if col.len() != n * 4 {
            return Err(JsValue::from_str(
                "colors length must be positions.len() * 4 (RGBA per point)",
            ));
        }
        let colors: Vec<[f32; 4]> = col
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();
        let sizes = sizes.to_vec();
        if sizes.len() != n {
            return Err(JsValue::from_str("sizes length must match number of points"));
        }
        Ok(Self {
            inner: Some(InnerScatter::new(positions, colors, sizes, (1, 1))),
        })
    }

    /// Seven column-major `Float32` buffers in wasm linear memory (`n` points each).
    ///
    /// # Safety
    /// Pointers must be valid for `n` floats until this layer is consumed by [`Scene::add_layer`].
    #[wasm_bindgen(js_name = fromArrowPtrs)]
    pub unsafe fn from_arrow_ptrs(
        n: u32,
        x_ptr: *const f32,
        y_ptr: *const f32,
        r_ptr: *const f32,
        g_ptr: *const f32,
        b_ptr: *const f32,
        a_ptr: *const f32,
        size_ptr: *const f32,
    ) -> Result<ScatterLayer, JsValue> {
        let n = usize::try_from(n).map_err(|_| JsValue::from_str("n too large"))?;
        if n > 0
            && (x_ptr.is_null()
                || y_ptr.is_null()
                || r_ptr.is_null()
                || g_ptr.is_null()
                || b_ptr.is_null()
                || a_ptr.is_null()
                || size_ptr.is_null())
        {
            return Err(JsValue::from_str("null column pointer"));
        }
        let inner = InnerScatter::from_raw_f32_columns(
            n,
            x_ptr,
            y_ptr,
            r_ptr,
            g_ptr,
            b_ptr,
            a_ptr,
            size_ptr,
            (1, 1),
        );
        Ok(Self {
            inner: Some(inner),
        })
    }

    /// Ergonomic path: seven separate JS [`Float32Array`]s (length `n` each) — may copy from JS heap into wasm.
    #[wasm_bindgen(js_name = fromArrowFloat32Arrays)]
    pub fn from_arrow_float32_arrays(
        x: js_sys::Float32Array,
        y: js_sys::Float32Array,
        r: js_sys::Float32Array,
        g: js_sys::Float32Array,
        b: js_sys::Float32Array,
        a: js_sys::Float32Array,
        size: js_sys::Float32Array,
    ) -> Result<ScatterLayer, JsValue> {
        let n = x.length() as usize;
        if y.length() as usize != n
            || r.length() as usize != n
            || g.length() as usize != n
            || b.length() as usize != n
            || a.length() as usize != n
            || size.length() as usize != n
        {
            return Err(JsValue::from_str("all Arrow columns must have the same length"));
        }
        let mut positions: Vec<[f32; 2]> = Vec::with_capacity(n);
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(n);
        let mut sizes: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            positions.push([x.get_index(i as u32), y.get_index(i as u32)]);
            colors.push([
                r.get_index(i as u32),
                g.get_index(i as u32),
                b.get_index(i as u32),
                a.get_index(i as u32),
            ]);
            sizes.push(size.get_index(i as u32));
        }
        Ok(Self {
            inner: Some(InnerScatter::new(positions, colors, sizes, (1, 1))),
        })
    }
}

/// Canvas-backed compositor: holds a [`wgpu::Surface`] and ordered scatter layers.
///
/// This type is separate from [`phantomat_renderer::Scene`] (the desktop clear/triangle enum);
/// it only drives [`Renderable`] draws from `phantomat-layers`.
#[wasm_bindgen]
pub struct Scene {
    /// Retained so the canvas stays valid for the underlying surface.
    #[allow(dead_code)]
    canvas: HtmlCanvasElement,
    /// Instance must outlive the [`wgpu::Surface`].
    #[allow(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
    /// Snapshot from [`wgpu::Adapter::get_info`] at device creation (browser WebGPU often reports empty fields).
    adapter_info: AdapterInfo,
    /// `"webgpu"` or `"webgl"` (for E2E / diagnostics).
    backend_name: String,
    layers: Vec<InnerScatter>,
}

enum BackendPref {
    All,
    WebGpuOnly,
    GlOnly,
}

fn instance_backends(pref: BackendPref) -> wgpu::Backends {
    match pref {
        BackendPref::All => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
        BackendPref::WebGpuOnly => wgpu::Backends::BROWSER_WEBGPU,
        BackendPref::GlOnly => wgpu::Backends::GL,
    }
}

fn adapter_backend_label(adapter: &wgpu::Adapter) -> String {
    match adapter.get_info().backend {
        wgpu::Backend::Gl => "webgl".to_string(),
        _ => "webgpu".to_string(),
    }
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn device_type_str(dt: wgpu::DeviceType) -> &'static str {
    match dt {
        wgpu::DeviceType::Cpu => "cpu",
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Other => "other",
    }
}

fn adapter_info_to_js(info: &AdapterInfo) -> JsValue {
    let obj = js_sys::Object::new();
    let backend = match info.backend {
        wgpu::Backend::Gl => "webgl2",
        wgpu::Backend::BrowserWebGpu => "browser-webgpu",
        _ => info.backend.to_str(),
    };
    let _ = js_sys::Reflect::set(&obj, &"wgpuBackend".into(), &backend.into());
    let _ = js_sys::Reflect::set(&obj, &"name".into(), &info.name.as_str().into());
    let _ = js_sys::Reflect::set(
        &obj,
        &"vendorId".into(),
        &format!("0x{:x}", info.vendor).into(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &"deviceId".into(),
        &format!("0x{:x}", info.device).into(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &"deviceType".into(),
        &device_type_str(info.device_type).into(),
    );
    let _ = js_sys::Reflect::set(&obj, &"driver".into(), &info.driver.as_str().into());
    let _ = js_sys::Reflect::set(
        &obj,
        &"driverInfo".into(),
        &info.driver_info.as_str().into(),
    );
    let is_fallback = matches!(info.device_type, wgpu::DeviceType::Cpu);
    let _ = js_sys::Reflect::set(&obj, &"isFallback".into(), &is_fallback.into());
    obj.into()
}

#[wasm_bindgen]
impl Scene {
    /// Async GPU init: picks WebGPU when available, otherwise WebGL.
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Scene, JsValue> {
        Self::new_with_backend_impl(canvas, BackendPref::All).await
    }

    /// Init with a hint: `webgpu` (WebGPU only), `webgl` (GL only), or `all` / `auto` (default order).
    #[wasm_bindgen(js_name = newWithBackend)]
    pub async fn new_with_backend(
        canvas: HtmlCanvasElement,
        backend_preference: &str,
    ) -> Result<Scene, JsValue> {
        let pref = match backend_preference {
            "webgpu" => BackendPref::WebGpuOnly,
            "webgl" => BackendPref::GlOnly,
            "all" | "auto" | "" => BackendPref::All,
            _ => {
                return Err(JsValue::from_str(
                    "backend_preference must be 'webgpu', 'webgl', or 'all' / 'auto'",
                ));
            }
        };
        Self::new_with_backend_impl(canvas, pref).await
    }

    async fn new_with_backend_impl(
        canvas: HtmlCanvasElement,
        pref: BackendPref,
    ) -> Result<Scene, JsValue> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: instance_backends(pref),
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("create_surface failed: {e}")))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| JsValue::from_str("no suitable GPU adapter (WebGPU/WebGL)"))?;

        let backend_name = adapter_backend_label(&adapter);
        let adapter_info = adapter.get_info();

        let limits = wgpu::Limits::downlevel_webgl2_defaults();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("phantomat_wasm"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("request_device failed: {e}")))?;

        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| JsValue::from_str("surface not compatible with adapter"))?;

        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first())
        {
            config.format = *f;
        }

        surface.configure(&device, &config);
        let format = config.format;

        Ok(Self {
            canvas,
            instance,
            surface,
            device,
            queue,
            config,
            format,
            adapter_info,
            backend_name,
            layers: Vec::new(),
        })
    }

    /// Active graphics backend for this scene (`webgpu` or `webgl`).
    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.backend_name.clone()
    }

    /// [`wgpu::Adapter::get_info`] snapshot; browser WebGPU may return empty strings (enrich from `navigator.gpu` in JS).
    #[wasm_bindgen(js_name = getAdapterInfo)]
    pub fn get_adapter_info(&self) -> JsValue {
        adapter_info_to_js(&self.adapter_info)
    }

    /// Removes all layers (e.g. before replacing IPC payload from Jupyter).
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Adds a scatter layer; canvas size from this scene is applied. Consumes the JS wrapper.
    pub fn add_layer(&mut self, mut layer: ScatterLayer) -> Result<(), JsValue> {
        let mut inner = layer
            .inner
            .take()
            .ok_or_else(|| JsValue::from_str("ScatterLayer already consumed"))?;
        inner.set_canvas_px((self.config.width, self.config.height));
        if !self.layers.is_empty() {
            inner.set_clear_before_draw(false);
        }
        self.layers.push(inner);
        Ok(())
    }

    /// Submits one frame: draws all layers in order, then presents.
    pub async fn render(&self) -> Result<(), JsValue> {
        self.render_timed(false).await.map(|_| ())
    }

    /// Same as [`Scene::render`] plus encode/submit/present timings (ms).
    #[wasm_bindgen(js_name = renderWithStats)]
    pub async fn render_with_stats(&self) -> Result<JsValue, JsValue> {
        self.render_timed(true).await
    }

    async fn render_timed(&self, with_stats: bool) -> Result<JsValue, JsValue> {
        let texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|e| JsValue::from_str(&format!("surface texture: {e:?}")))?
            }
            Err(e) => return Err(JsValue::from_str(&format!("surface texture: {e:?}"))),
        };

        let view = texture.texture.create_view(&TextureViewDescriptor::default());
        let t0 = if with_stats { now_ms() } else { 0.0 };
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("phantomat_wasm_frame"),
            });

        for layer in &self.layers {
            layer.render(
                &mut encoder,
                &view,
                &self.device,
                &self.queue,
                self.format,
            );
        }

        let cmd_buf = encoder.finish();
        let t1 = if with_stats { now_ms() } else { 0.0 };
        self.queue.submit(std::iter::once(cmd_buf));
        let t2 = if with_stats { now_ms() } else { 0.0 };
        texture.present();
        let t3 = if with_stats { now_ms() } else { 0.0 };

        if with_stats {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &obj,
                &"encodeMs".into(),
                &(t1 - t0).into(),
            );
            let _ = js_sys::Reflect::set(
                &obj,
                &"submitMs".into(),
                &(t2 - t1).into(),
            );
            let _ = js_sys::Reflect::set(
                &obj,
                &"presentMs".into(),
                &(t3 - t2).into(),
            );
            Ok(obj.into())
        } else {
            Ok(JsValue::UNDEFINED)
        }
    }

    /// Swapchain width in pixels.
    #[wasm_bindgen(getter)]
    pub fn canvas_width(&self) -> u32 {
        self.config.width
    }

    /// Swapchain height in pixels.
    #[wasm_bindgen(getter)]
    pub fn canvas_height(&self) -> u32 {
        self.config.height
    }
}
