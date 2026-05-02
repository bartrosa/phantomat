//! OKLCH interpolation tests and colormap snapshots.

use palette::{IntoColor, Oklch};
use phantomat_core::color::{interpolate_oklch, Rgb};
use proptest::prelude::*;

fn rgb_arr(c: Rgb) -> [f32; 3] {
    [c.red, c.green, c.blue]
}

/// Eleven stops: first knot, OKLCH midpoint of each of the first nine consecutive pairs,
/// last knot (matches “10 segment pairs” along sampled knots).
fn eleven_stop_piecewise(knots: &[Rgb; 11]) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(11);
    out.push(rgb_arr(knots[0]));
    for i in 0..9 {
        out.push(rgb_arr(interpolate_oklch(knots[i], knots[i + 1], 0.5)));
    }
    out.push(rgb_arr(knots[10]));
    out
}

/// Matplotlib ListedColormap samples (`viridis`, indices spaced ~linearly in 0..255).
const VIRIDIS_KNOTS: [Rgb; 11] = [
    Rgb::new(0.267004, 0.004874, 0.329415),
    Rgb::new(0.282290, 0.145912, 0.461510),
    Rgb::new(0.253935, 0.265254, 0.529983),
    Rgb::new(0.203063, 0.379716, 0.553925),
    Rgb::new(0.163625, 0.471133, 0.558148),
    Rgb::new(0.127568, 0.566949, 0.550556),
    Rgb::new(0.134692, 0.658636, 0.517649),
    Rgb::new(0.266941, 0.748751, 0.440573),
    Rgb::new(0.477504, 0.821444, 0.318195),
    Rgb::new(0.730889, 0.871916, 0.156029),
    Rgb::new(0.993248, 0.906157, 0.143936),
];

const MAGMA_KNOTS: [Rgb; 11] = [
    Rgb::new(0.001462, 0.000466, 0.013866),
    Rgb::new(0.083446, 0.056225, 0.220755),
    Rgb::new(0.245543, 0.059352, 0.448436),
    Rgb::new(0.402548, 0.105420, 0.503386),
    Rgb::new(0.600868, 0.177743, 0.500394),
    Rgb::new(0.729216, 0.219437, 0.471279),
    Rgb::new(0.874176, 0.291859, 0.406205),
    Rgb::new(0.971582, 0.454210, 0.361030),
    Rgb::new(0.993834, 0.609644, 0.418613),
    Rgb::new(0.997019, 0.762398, 0.528821),
    Rgb::new(0.987053, 0.991438, 0.749504),
];

const PLASMA_KNOTS: [Rgb; 11] = [
    Rgb::new(0.050383, 0.029803, 0.527975),
    Rgb::new(0.261183, 0.013308, 0.617911),
    Rgb::new(0.411580, 0.000577, 0.657730),
    Rgb::new(0.568201, 0.055778, 0.639477),
    Rgb::new(0.692840, 0.165141, 0.564522),
    Rgb::new(0.798216, 0.280197, 0.469538),
    Rgb::new(0.875376, 0.383347, 0.389976),
    Rgb::new(0.944844, 0.507658, 0.302433),
    Rgb::new(0.987332, 0.646633, 0.214648),
    Rgb::new(0.993851, 0.771720, 0.152855),
    Rgb::new(0.940015, 0.975158, 0.131326),
];

#[test]
fn oklch_t0_t1_exact_endpoints() {
    let a = Rgb::new(0.2, 0.4, 0.6);
    let b = Rgb::new(0.9, 0.1, 0.3);
    assert_eq!(interpolate_oklch(a, b, 0.0), a);
    assert_eq!(interpolate_oklch(a, b, 1.0), b);
    assert_eq!(interpolate_oklch(a, b, -0.5), a);
    assert_eq!(interpolate_oklch(a, b, 1.5), b);
}

#[test]
fn oklch_t_half_lightness_between_in_oklch() {
    let a = Rgb::new(0.15, 0.25, 0.9);
    let b = Rgb::new(0.85, 0.2, 0.15);
    let m = interpolate_oklch(a, b, 0.5);
    let a_o: Oklch = a.into_color();
    let b_o: Oklch = b.into_color();
    let m_o: Oklch = m.into_color();
    for (ac, bc, mc) in [(a_o.l, b_o.l, m_o.l), (a_o.chroma, b_o.chroma, m_o.chroma)] {
        let lo = ac.min(bc);
        let hi = ac.max(bc);
        assert!(
            mc >= lo - 1e-2 && mc <= hi + 1e-2,
            "OKLCH component not between endpoints: {mc} not in [{lo}, {hi}]"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn oklch_interpolation_finite_and_in_unit_cube(
        r0 in 0.0f32..1.0, g0 in 0.0f32..1.0, b0 in 0.0f32..1.0,
        r1 in 0.0f32..1.0, g1 in 0.0f32..1.0, b1 in 0.0f32..1.0,
        t in 0.0f32..1.0
    ) {
        let a = Rgb::new(r0, g0, b0);
        let b = Rgb::new(r1, g1, b1);
        let c = interpolate_oklch(a, b, t);
        let [cr, cg, cb] = rgb_arr(c);
        let band = -1e-3..=1.0f32 + 1e-3;
        prop_assert!(cr.is_finite() && cg.is_finite() && cb.is_finite());
        prop_assert!(band.contains(&cr));
        prop_assert!(band.contains(&cg));
        prop_assert!(band.contains(&cb));
    }
}

#[test]
fn snapshot_viridis_11_stops() {
    insta::assert_yaml_snapshot!("viridis_11_stops", eleven_stop_piecewise(&VIRIDIS_KNOTS));
}

#[test]
fn snapshot_magma_11_stops() {
    insta::assert_yaml_snapshot!("magma_11_stops", eleven_stop_piecewise(&MAGMA_KNOTS));
}

#[test]
fn snapshot_plasma_11_stops() {
    insta::assert_yaml_snapshot!("plasma_11_stops", eleven_stop_piecewise(&PLASMA_KNOTS));
}
