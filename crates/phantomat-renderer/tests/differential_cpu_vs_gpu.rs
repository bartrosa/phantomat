//! CPU oracle vs GPU atomic histogram (native wgpu only).

use std::time::Instant;

use phantomat_core::reference::histogram_2d_cpu_wgpu_semantics;
use phantomat_renderer::compute::histogram_2d_gpu;
use phantomat_renderer::headless::HeadlessRenderer;
use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn smoke_cpu_gpu_identical_tiny() {
    let r = HeadlessRenderer::new(8, 8).expect("headless");
    let xs = [0.0f64, 0.5, -0.5];
    let ys = [0.0f64, 0.5, -0.5];
    let range = ((-1.0, 1.0), (-1.0, 1.0));
    let cpu = histogram_2d_cpu_wgpu_semantics(&xs, &ys, 2, 2, range);
    let gpu = histogram_2d_gpu(r.device(), r.queue(), &xs, &ys, 2, 2, range).expect("gpu");
    assert_eq!(cpu, gpu);
}

#[test]
fn cpu_vs_gpu_histogram_timing_smoke() {
    let r = HeadlessRenderer::new(8, 8).expect("headless");
    let n = 50_000usize;
    let range = ((-1.0f64, 1.0f64), (-1.0f64, 1.0f64));
    let xs: Vec<f64> = (0..n).map(|i| (i as f64 * 0.00003).sin()).collect();
    let ys: Vec<f64> = (0..n).map(|i| (i as f64 * 0.00007).cos()).collect();
    let bins_x = 32usize;
    let bins_y = 32usize;

    let t0 = Instant::now();
    let cpu = histogram_2d_cpu_wgpu_semantics(&xs, &ys, bins_x, bins_y, range);
    let dt_cpu = t0.elapsed();

    let t1 = Instant::now();
    let gpu = histogram_2d_gpu(
        r.device(),
        r.queue(),
        &xs,
        &ys,
        bins_x,
        bins_y,
        range,
    )
    .expect("gpu histogram");
    let dt_gpu = t1.elapsed();

    assert_eq!(cpu, gpu);
    let ratio = dt_gpu.as_secs_f64() / dt_cpu.as_secs_f64().max(1e-9);
    eprintln!(
        "histogram_2d n={n} bins={bins_x}x{bins_y}: CPU {dt_cpu:?}, GPU {dt_gpu:?}, GPU/CPU={ratio:.3}x"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn histogram_2d_cpu_gpu_parity(
        seed: u64,
        n in 100usize..10_000,
        bins_x in 4usize..64,
        bins_y in 4usize..64,
    ) {
        let r = HeadlessRenderer::new(8, 8).expect("headless");
        let range = ((-1.0f64, 1.0f64), (-1.0f64, 1.0f64));
        let mut rng = StdRng::seed_from_u64(seed);
        let xs: Vec<f64> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let ys: Vec<f64> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let cpu = histogram_2d_cpu_wgpu_semantics(&xs, &ys, bins_x, bins_y, range);
        let gpu = match histogram_2d_gpu(
            r.device(),
            r.queue(),
            &xs,
            &ys,
            bins_x,
            bins_y,
            range,
        ) {
            Ok(g) => g,
            Err(e) => {
                // Some CI adapters may not support this path; skip instead of fail.
                eprintln!("skip GPU histogram: {e}");
                return Ok(());
            }
        };
        prop_assert_eq!(cpu, gpu);
    }
}
