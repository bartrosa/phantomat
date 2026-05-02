//! Color interpolation (OKLCH).

use palette::{FromColor, IntoColor, Mix, Oklch, Srgb};

/// Standard **sRGB** color in linear-light encoding used by [`palette::Srgb`].
pub type Rgb = Srgb<f32>;

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
