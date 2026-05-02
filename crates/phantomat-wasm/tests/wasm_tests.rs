#[cfg(target_arch = "wasm32")]
use phantomat_core::{LinearScale, Scale};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test_configure!(run_in_browser);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn webgpu_or_webgl_adapter() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await;
    assert!(adapter.is_some(), "expected a WebGPU or WebGL adapter");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
fn scale_works_in_browser() {
    let s = LinearScale::new((0.0, 10.0), (0.0, 100.0));
    let y = s.apply(5.0);
    assert!((y - 50.0).abs() < 1e-9, "apply(5) = {y}, expected 50");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn wasm_pack_browser_tests_only_on_wasm32() {}
