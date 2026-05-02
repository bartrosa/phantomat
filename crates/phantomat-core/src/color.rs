//! Color interpolation (OKLCH).

use palette::{FromColor, IntoColor, Mix, Oklch, Srgb};

/// Standard **sRGB** color in linear-light encoding used by [`palette::Srgb`].
pub type Rgb = Srgb<f32>;

/// Fixed-point scale for heatmap weight accumulation (must match WGSL `heatmap_accumulate`).
pub const HEATMAP_WEIGHT_SCALE: f32 = 1000.0;

/// Two-stop colormap (low → high) for density / heatmap rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRamp {
    pub low: Rgb,
    pub high: Rgb,
}

impl ColorRamp {
    /// Dark blue → yellow (good contrast on black backgrounds).
    #[must_use]
    pub fn blue_yellow() -> Self {
        Self {
            low: Srgb::new(0.05, 0.08, 0.35),
            high: Srgb::new(0.95, 0.92, 0.15),
        }
    }

    /// Returns premultiplied-friendly linear sRGBA for `t ∈ [0, 1]`.
    #[must_use]
    pub fn sample_rgba(&self, t: f32) -> [f32; 4] {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        let c = interpolate_oklch(self.low, self.high, t);
        [c.red, c.green, c.blue, 1.0]
    }
}

/// Converts a non-negative weight to the `u32` contribution used in GPU/CPU heatmap bins.
#[must_use]
pub fn heatmap_weight_contrib_u32(weight: f32) -> u32 {
    if !weight.is_finite() || weight <= 0.0 {
        return 0;
    }
    let v = (f64::from(weight) * f64::from(HEATMAP_WEIGHT_SCALE)).round();
    if v <= 0.0 {
        return 0;
    }
    if v >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    v as u32
}

/// Interpolates between `a` and `b` in **OKLCH**, then converts back to sRGB.
///
/// `t` is clamped to `[0, 1]`. For `t == 0` and `t == 1`, returns `a` and `b` exactly
/// (including component-wise bit patterns).
#[must_use]
pub fn interpolate_oklch(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Exact endpoints (including bit patterns for t ∈ {0, 1} after clamp).
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let a_ok: Oklch = a.into_color();
    let b_ok: Oklch = b.into_color();
    let mixed = a_ok.mix(b_ok, t);
    Srgb::from_color(mixed)
}
