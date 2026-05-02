//! PyO3 extension: headless [`ScatterLayer`] rendering via [`phantomat_renderer::HeadlessRenderer`].

use ndarray::ArrayView2;
use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use phantomat_layers::ScatterLayer;
use phantomat_renderer::{ClearScene, HeadlessRenderer, Renderable, Scene as RendererScene};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};

/// Renders multiple [`ScatterLayer`]s in order (first clears to black by default).
struct StackedScatters<'a> {
    layers: &'a [ScatterLayer],
}

impl Renderable for StackedScatters<'_> {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
    ) {
        for layer in self.layers {
            layer.render(encoder, view, device, queue, format);
        }
    }
}

fn renderer_err(e: phantomat_renderer::RendererError) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

#[pyclass(name = "Scene")]
pub struct PyScene {
    renderer: HeadlessRenderer,
    width: u32,
    height: u32,
    layers: Vec<ScatterLayer>,
}

#[pymethods]
impl PyScene {
    #[new]
    fn new(width: u32, height: u32) -> PyResult<Self> {
        let renderer = HeadlessRenderer::new(width, height).map_err(renderer_err)?;
        Ok(Self {
            renderer,
            width,
            height,
            layers: Vec::new(),
        })
    }

    fn add_scatter(
        &mut self,
        positions: PyReadonlyArray2<f32>,
        colors: PyReadonlyArray2<f32>,
        sizes: PyReadonlyArray1<f32>,
    ) -> PyResult<()> {
        let pos = positions.as_array();
        let col = colors.as_array();
        let siz = sizes.as_array();

        let pos_shape = pos.shape();
        if pos_shape.len() != 2 || pos_shape[1] != 2 {
            return Err(PyValueError::new_err(
                "positions must be a 2D array with shape (N, 2)",
            ));
        }
        let n = pos_shape[0];
        let col_shape = col.shape();
        if col_shape != [n, 4] {
            return Err(PyValueError::new_err(
                "colors must have shape (N, 4) matching positions",
            ));
        }
        if siz.shape() != [n] {
            return Err(PyValueError::new_err(
                "sizes must be a 1D array of shape (N,) matching positions",
            ));
        }

        let positions_vec = array2_points(pos);
        let colors_vec = array2_rgba(col);
        let sizes_vec = array1_sizes(siz);

        let mut layer = ScatterLayer::new(
            positions_vec,
            colors_vec,
            sizes_vec,
            (self.width, self.height),
        );
        if !self.layers.is_empty() {
            layer.set_clear_before_draw(false);
        }
        self.layers.push(layer);
        Ok(())
    }

    fn render_to_png(&self, py: Python<'_>) -> PyResult<Vec<u8>> {
        py.allow_threads(|| {
            let png = if self.layers.is_empty() {
                let clear = RendererScene::Clear(ClearScene {
                    color: [0.0, 0.0, 0.0, 1.0],
                });
                self.renderer.render_to_png(&clear)
            } else {
                let stack = StackedScatters {
                    layers: self.layers.as_slice(),
                };
                self.renderer.render_to_png(&stack)
            };
            png.map_err(renderer_err)
        })
    }
}

fn array2_points(view: ArrayView2<f32>) -> Vec<[f32; 2]> {
    let n = view.nrows();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push([view[[i, 0]], view[[i, 1]]]);
    }
    out
}

fn array2_rgba(view: ArrayView2<f32>) -> Vec<[f32; 4]> {
    let n = view.nrows();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push([view[[i, 0]], view[[i, 1]], view[[i, 2]], view[[i, 3]]]);
    }
    out
}

fn array1_sizes(view: ndarray::ArrayView1<f32>) -> Vec<f32> {
    view.iter().copied().collect()
}

/// Python package `phantomat._native` (see `python/pyproject.toml` / maturin `module-name`).
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyScene>()?;
    Ok(())
}
