//! WASM bindings: composes [`phantomat_layers::ScatterLayer`] on a canvas via **wgpu**
//! (WebGPU / WebGL). Desktop [`phantomat_renderer::Scene`] (clear/triangle enum) is not embedded;
//! this crate drives [`phantomat_renderer::Renderable`] layers only.

#[cfg(target_arch = "wasm32")]
include!("wasm_lib.rs");

#[cfg(not(target_arch = "wasm32"))]
/// Host `cargo check` stub — build with `--target wasm32-unknown-unknown` or use `wasm-pack`.
pub fn phantomat_wasm_requires_wasm32_target() {}
