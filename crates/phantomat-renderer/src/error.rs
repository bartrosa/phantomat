/// Errors from adapter/device setup and PNG export.
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("no suitable GPU adapter found")]
    NoAdapter,

    #[error("failed to request wgpu device: {0}")]
    DeviceRequest(String),

    #[error("texture readback failed: {0}")]
    BufferMap(String),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("PNG encode failed: {0}")]
    ImageEncode(#[from] image::ImageError),

    #[error("invalid scene parameters: {0}")]
    Scene(String),
}
