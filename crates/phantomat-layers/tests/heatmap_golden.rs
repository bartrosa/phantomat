//! Golden heatmap PNGs (DSSIM). Baselines are per-OS under `tests/golden/<subdir>/`.

use std::path::{Path, PathBuf};

use bytemuck::cast_slice;
use dssim::Dssim;
use phantomat_core::ColorRamp;
use phantomat_layers::HeatmapLayer;
use phantomat_renderer::HeadlessRenderer;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use rgb::RGBA;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden_subdir() -> &'static str {
    if let Ok(s) = std::env::var("PHANTOMAT_TEST_BACKEND") {
        return match s.as_str() {
            "linux-lavapipe" | "linux" => "linux-lavapipe",
            "macos-metal" | "macos" => "macos-metal",
            "windows-warp" | "windows" => "windows-warp",
            other => panic!("unknown PHANTOMAT_TEST_BACKEND={other}"),
        };
    }
    #[cfg(target_os = "linux")]
    {
        "linux-lavapipe"
    }
    #[cfg(target_os = "macos")]
    {
        "macos-metal"
    }
    #[cfg(target_os = "windows")]
    {
        "windows-warp"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "unknown"
    }
}

fn golden_path(name: &str) -> PathBuf {
    manifest_dir()
        .join("tests/golden")
        .join(golden_subdir())
        .join(format!("heatmap_{name}.png"))
}

#[derive(Clone, Copy, Debug)]
enum Tolerance {
    Dssim(f64),
}

fn assert_image_matches(actual: &[u8], golden_path: &Path, tol: Tolerance) {
    if std::env::var_os("PHANTOMAT_UPDATE_GOLDENS").is_some() {
        if let Some(parent) = golden_path.parent() {
            std::fs::create_dir_all(parent).expect("create golden dirs");
        }
        std::fs::write(golden_path, actual).expect("write golden PNG");
        eprintln!("updated golden {}", golden_path.display());
        return;
    }

    assert!(
        golden_path.is_file(),
        "missing golden {} — set PHANTOMAT_UPDATE_GOLDENS=1 to create on this machine",
        golden_path.display()
    );

    let expected_bytes = std::fs::read(golden_path).expect("read golden");
    match tol {
        Tolerance::Dssim(max_dssim) => {
            let d = dssim_difference(actual, &expected_bytes);
            assert!(
                d <= max_dssim,
                "DSSIM {d:.6} exceeds {max_dssim} (golden {})",
                golden_path.display()
            );
        }
    }
}

fn dssim_difference(a: &[u8], b: &[u8]) -> f64 {
    let a = image::load_from_memory(a)
        .expect("decode actual PNG")
        .into_rgba8();
    let e = image::load_from_memory(b)
        .expect("decode golden PNG")
        .into_rgba8();
    assert_eq!(a.dimensions(), e.dimensions());
    let w = a.width() as usize;
    let h = a.height() as usize;
    let attr = Dssim::new();
    let i1 = attr
        .create_image_rgba(cast_slice::<u8, RGBA<u8>>(a.as_raw()), w, h)
        .expect("dssim actual");
    let i2 = attr
        .create_image_rgba(cast_slice::<u8, RGBA<u8>>(e.as_raw()), w, h)
        .expect("dssim golden");
    let (val, _) = attr.compare(&i1, &i2);
    f64::from(val)
}

fn render_heatmap(layer: &HeatmapLayer) -> Vec<u8> {
    let r = HeadlessRenderer::new(256, 256).expect("headless");
    r.render_to_png(layer).expect("render")
}

/// Single 1×1 bin, one point at the origin — full frame should map to max color.
#[test]
fn heatmap_single_bin() {
    let layer = HeatmapLayer::new(
        vec![[0.0, 0.0]],
        vec![1.0],
        (1, 1),
        ColorRamp::blue_yellow(),
        (256, 256),
    );
    let png = render_heatmap(&layer);
    assert_image_matches(
        &png,
        &golden_path("single_bin"),
        Tolerance::Dssim(0.02),
    );
}

/// Points spread uniformly in the domain.
#[test]
fn heatmap_uniform_distribution() {
    let mut rng = StdRng::seed_from_u64(0xC0FFEE_AABBu64);
    let n = 500;
    let mut positions = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    for _ in 0..n {
        positions.push([
            rng.gen_range(-0.95_f32..0.95),
            rng.gen_range(-0.95_f32..0.95),
        ]);
        weights.push(1.0);
    }
    let layer = HeatmapLayer::new(
        positions,
        weights,
        (24, 24),
        ColorRamp::blue_yellow(),
        (256, 256),
    );
    let png = render_heatmap(&layer);
    assert_image_matches(
        &png,
        &golden_path("uniform_distribution"),
        Tolerance::Dssim(0.02),
    );
}

/// Gaussian-ish cluster (Box–Muller simplified with sum of uniforms).
#[test]
fn heatmap_gaussian_cluster() {
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEFu64);
    let n = 800;
    let mut positions = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    for _ in 0..n {
        let u1 = rng.gen_range(0.001_f32..1.0);
        let u2 = rng.gen_range(0.0..1.0);
        let z0 = (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos();
        let z1 = (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).sin();
        positions.push([z0 * 0.25, z1 * 0.25]);
        weights.push(1.0);
    }
    let layer = HeatmapLayer::new(
        positions,
        weights,
        (32, 32),
        ColorRamp::blue_yellow(),
        (256, 256),
    );
    let png = render_heatmap(&layer);
    assert_image_matches(
        &png,
        &golden_path("gaussian_cluster"),
        Tolerance::Dssim(0.02),
    );
}

/// Two separated clusters.
#[test]
fn heatmap_two_clusters() {
    let mut rng = StdRng::seed_from_u64(0xBADC0DEu64);
    let n = 400;
    let mut positions = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    for _ in 0..n / 2 {
        positions.push([
            rng.gen_range(-0.85_f32..-0.35),
            rng.gen_range(-0.25_f32..0.25),
        ]);
        weights.push(1.0);
    }
    for _ in 0..n / 2 {
        positions.push([
            rng.gen_range(0.35_f32..0.85),
            rng.gen_range(-0.25_f32..0.25),
        ]);
        weights.push(1.0);
    }
    let layer = HeatmapLayer::new(
        positions,
        weights,
        (28, 20),
        ColorRamp::blue_yellow(),
        (256, 256),
    );
    let png = render_heatmap(&layer);
    assert_image_matches(
        &png,
        &golden_path("two_clusters"),
        Tolerance::Dssim(0.02),
    );
}
