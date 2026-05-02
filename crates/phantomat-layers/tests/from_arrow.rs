//! Parity: [`ScatterLayer::from_arrow`] vs owned [`ScatterLayer::new`] (same golden PNG as `single_red_point`).

use std::sync::Arc;
use std::time::Instant;

use arrow::array::Float32Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use phantomat_layers::ScatterLayer;
use phantomat_renderer::HeadlessRenderer;

fn render_scatter(layer: &ScatterLayer) -> Vec<u8> {
    let r = HeadlessRenderer::new(256, 256).expect("headless");
    r.render_to_png(layer).expect("render")
}

fn render_repeat(r: &HeadlessRenderer, layer: &ScatterLayer, n: usize) {
    for _ in 0..n {
        let _ = r.render_to_png(layer).expect("render");
    }
}

fn single_red_point_vec() -> ScatterLayer {
    ScatterLayer::new(
        vec![[0.0, 0.0]],
        vec![[1.0, 0.0, 0.0, 1.0]],
        vec![32.0],
        (256, 256),
    )
}

fn single_red_point_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("x", DataType::Float32, false),
        Field::new("y", DataType::Float32, false),
        Field::new("r", DataType::Float32, false),
        Field::new("g", DataType::Float32, false),
        Field::new("b", DataType::Float32, false),
        Field::new("a", DataType::Float32, false),
        Field::new("size", DataType::Float32, false),
    ]));
    let x = Arc::new(Float32Array::from(vec![0.0f32])) as _;
    let y = Arc::new(Float32Array::from(vec![0.0f32])) as _;
    let r = Arc::new(Float32Array::from(vec![1.0f32])) as _;
    let g = Arc::new(Float32Array::from(vec![0.0f32])) as _;
    let b = Arc::new(Float32Array::from(vec![0.0f32])) as _;
    let a = Arc::new(Float32Array::from(vec![1.0f32])) as _;
    let s = Arc::new(Float32Array::from(vec![32.0f32])) as _;
    RecordBatch::try_new(schema, vec![x, y, r, g, b, a, s]).expect("batch")
}

#[test]
fn from_arrow_bitwise_matches_vec_single_red_point() {
    let vec_layer = single_red_point_vec();
    let batch = single_red_point_batch();
    let arrow_layer = ScatterLayer::from_arrow(&batch, (256, 256)).expect("from_arrow");
    let png_vec = render_scatter(&vec_layer);
    let png_ar = render_scatter(&arrow_layer);
    assert_eq!(png_vec, png_ar, "Arrow path must match Vec path (bit-exact PNG)");
}

#[test]
fn arrow_render_overhead_under_10_percent_smoke() {
    let vec_layer = single_red_point_vec();
    let batch = single_red_point_batch();
    let arrow_layer = ScatterLayer::from_arrow(&batch, (256, 256)).expect("from_arrow");
    let n = 30;
    let r_vec = HeadlessRenderer::new(256, 256).expect("headless");
    let r_ar = HeadlessRenderer::new(256, 256).expect("headless");
    let t0 = Instant::now();
    render_repeat(&r_vec, &vec_layer, n);
    let dt_vec = t0.elapsed();
    let t1 = Instant::now();
    render_repeat(&r_ar, &arrow_layer, n);
    let dt_ar = t1.elapsed();
    let ratio = dt_ar.as_secs_f64() / dt_vec.as_secs_f64();
    assert!(
        ratio < 1.1,
        "arrow render should be <10% slower than vec path, got {:.1}%",
        (ratio - 1.0) * 100.0
    );
}
