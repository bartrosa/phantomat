//! Golden scatter PNGs (DSSIM). Baselines are per-OS under `tests/golden/<subdir>/`.

use std::path::{Path, PathBuf};

use bytemuck::cast_slice;
use dssim::Dssim;
use phantomat_core::{interpolate_oklch, Rgb};
use phantomat_layers::ScatterLayer;
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
        .join(format!("{name}.png"))
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

fn render_scatter(layer: &ScatterLayer) -> Vec<u8> {
    let r = HeadlessRenderer::new(256, 256).expect("headless");
    r.render_to_png(layer).expect("render")
}

#[test]
fn single_red_point() {
    let layer = ScatterLayer::new(
        vec![[0.0, 0.0]],
        vec![[1.0, 0.0, 0.0, 1.0]],
        vec![32.0],
        (256, 256),
    );
    let png = render_scatter(&layer);
    assert_image_matches(
        &png,
        &golden_path("single_red_point"),
        Tolerance::Dssim(0.01),
    );
}

#[test]
fn ten_points_grid() {
    let v0 = Rgb::new(0.267_004, 0.004_874, 0.329_415);
    let v1 = Rgb::new(0.993_248, 0.906_157, 0.143_936);
    let mut positions = Vec::with_capacity(10);
    let mut colors = Vec::with_capacity(10);
    let sizes = vec![18.0; 10];
    for row in 0..2 {
        for col in 0..5 {
            let x = -0.75 + (col as f32 / 4.0) * 1.5;
            let y = 0.35 - (row as f32) * 0.7;
            positions.push([x, y]);
            let t = (positions.len() - 1) as f32 / 9.0;
            let c = interpolate_oklch(v0, v1, t);
            colors.push([c.red, c.green, c.blue, 1.0]);
        }
    }
    let layer = ScatterLayer::new(positions, colors, sizes, (256, 256));
    let png = render_scatter(&layer);
    assert_image_matches(
        &png,
        &golden_path("ten_points_grid"),
        Tolerance::Dssim(0.01),
    );
}

#[test]
fn hundred_points_random_seeded() {
    let mut rng = StdRng::seed_from_u64(0x0123_4567_89AB_CDEF);
    let n = 100;
    let mut positions = Vec::with_capacity(n);
    let mut colors = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n);
    for _ in 0..n {
        positions.push([
            rng.gen_range(-0.92_f32..0.92),
            rng.gen_range(-0.92_f32..0.92),
        ]);
        colors.push([
            rng.gen_range(0.15..1.0),
            rng.gen_range(0.15..1.0),
            rng.gen_range(0.15..1.0),
            1.0,
        ]);
        sizes.push(rng.gen_range(4.0_f32..24.0));
    }
    let layer = ScatterLayer::new(positions, colors, sizes, (256, 256));
    let png = render_scatter(&layer);
    assert_image_matches(
        &png,
        &golden_path("100_points_random_seeded"),
        Tolerance::Dssim(0.01),
    );
}

#[test]
fn gradient_line() {
    let n = 50;
    let mut positions = Vec::with_capacity(n);
    let mut colors = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1).max(1) as f32;
        let x = -0.9 + t * 1.8;
        let y = -0.9 + t * 1.8;
        positions.push([x, y]);
        colors.push([0.1, 0.6 + 0.4 * t, 0.85, 1.0]);
        let size = 32.0 + (4.0 - 32.0) * t;
        sizes.push(size);
    }
    let layer = ScatterLayer::new(positions, colors, sizes, (256, 256));
    let png = render_scatter(&layer);
    assert_image_matches(&png, &golden_path("gradient_line"), Tolerance::Dssim(0.01));
}
