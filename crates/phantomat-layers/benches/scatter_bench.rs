use criterion::black_box;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use phantomat_layers::ScatterLayer;
use phantomat_renderer::HeadlessRenderer;

const W: u32 = 512;
const H: u32 = 512;

fn make_layer(n: usize) -> ScatterLayer {
    let mut positions = Vec::with_capacity(n);
    let mut colors = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / n.max(1) as f32;
        positions.push([t * 1.6 - 0.8, (i as f32 * 0.13).sin() * 0.5]);
        colors.push([0.2 + 0.8 * t, 0.3, 0.6, 1.0]);
        sizes.push(4.0 + 20.0 * t);
    }
    ScatterLayer::new(positions, colors, sizes, (W, H))
}

fn bench_cpu_setup(c: &mut Criterion) {
    let mut g = c.benchmark_group("scatter_cpu_allocate");
    // Instance stride is 48 B → ~5.5 M instances fit under typical `max_buffer_size` ≈ 256 MiB.
    for n in [1_000usize, 10_000, 100_000, 1_000_000, 10_000_000] {
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let layer = make_layer(n);
                black_box(layer);
            });
        });
    }
    g.finish();
}

/// Build layer and upload + draw a single frame (first call also creates GPU cache).
fn bench_full_frame(c: &mut Criterion) {
    let r = HeadlessRenderer::new(W, H).expect("headless");
    let mut g = c.benchmark_group("scatter_gpu_one_frame");
    // Larger counts exceed default instance buffer limits on Metal/Vulkan; include 5 M as “large”.
    for n in [1_000usize, 10_000, 100_000, 1_000_000, 5_000_000] {
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let layer = make_layer(n);
            let _ = r.render_to_png(&layer).expect("warmup render");
            b.iter(|| {
                r.render_to_png(&layer).expect("render");
            });
        });
    }
    g.finish();
}

criterion_group!(benches, bench_cpu_setup, bench_full_frame);
criterion_main!(benches);
