//! Golden PNG comparison (DSSIM / optional pixel-eq) with per-backend baselines.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bytemuck::cast_slice;
use dssim::Dssim;
use phantomat_renderer::{ClearScene, HeadlessRenderer, Scene, TriangleScene};
use rgb::RGBA;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Subdirectory under `tests/golden/`, e.g. `macos-metal`.
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

fn golden_dir() -> PathBuf {
    manifest_dir().join("tests/golden").join(golden_subdir())
}

fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{name}.png"))
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
enum Tolerance {
    PixelExact,
    /// Minimum SSIM derived from DSSIM: `ssim ≈ 1 / (dssim + 1)`.
    Ssim(f64),
    /// Maximum DSSIM from [`dssim::Dssim::compare`] (0 = identical).
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
        "missing golden {} — run `cargo run -p xtask -- update-goldens --reason \"…\"` on this platform",
        golden_path.display()
    );

    let expected_bytes = std::fs::read(golden_path).expect("read golden");
    match tol {
        Tolerance::PixelExact => assert_eq!(
            actual,
            expected_bytes,
            "pixel mismatch vs {}",
            golden_path.display()
        ),
        Tolerance::Ssim(min_ssim) => {
            let dmax = (1.0 / min_ssim) - 1.0;
            let d = dssim_difference(actual, &expected_bytes);
            assert!(
                d <= dmax,
                "SSIM gate failed (approx): dssim={d:.6} (need SSIM ≥ {min_ssim}; golden {})",
                golden_path.display()
            );
        }
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
    assert_eq!(
        a.dimensions(),
        e.dimensions(),
        "resolution mismatch: actual {:?} vs golden {:?}",
        a.dimensions(),
        e.dimensions()
    );
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

fn render_png(scene: &Scene, w: u32, h: u32) -> Vec<u8> {
    let r = HeadlessRenderer::new(w, h).expect("headless renderer");
    r.render_to_png(scene).expect("render")
}

#[test]
fn clear_red_512x512() {
    let scene = Scene::Clear(ClearScene {
        color: [1.0, 0.0, 0.0, 1.0],
    });
    let t0 = Instant::now();
    let png = render_png(&scene, 512, 512);
    eprintln!(
        "clear_red_512x512: render+png {} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    assert_image_matches(
        &png,
        &golden_path("clear_red_512x512"),
        Tolerance::Dssim(0.08),
    );
}

#[test]
fn triangle_centered() {
    let scene = Scene::Triangle(TriangleScene {
        positions: [[0.0, 0.45], [-0.45, -0.35], [0.45, -0.35]],
        color: [0.0, 0.85, 0.2, 1.0],
    });
    let t0 = Instant::now();
    let png = render_png(&scene, 512, 512);
    eprintln!(
        "triangle_centered: render+png {} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    assert_image_matches(
        &png,
        &golden_path("triangle_centered"),
        Tolerance::Dssim(0.12),
    );
}
