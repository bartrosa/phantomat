//! Integration tests for linear and logarithmic scales.

use phantomat_core::scale::{LinearScale, LogScale, Scale, ScaleError};
use proptest::prelude::*;

#[test]
fn linear_maps_midpoint_std() {
    let s = LinearScale::new((0.0, 10.0), (0.0, 100.0));
    assert!((s.apply(5.0) - 50.0).abs() < 1e-12);
}

#[test]
fn linear_reversed_range() {
    let s = LinearScale::new((0.0, 10.0), (100.0, 0.0));
    assert!((s.apply(5.0) - 50.0).abs() < 1e-12);
}

#[test]
fn linear_negative_domain_to_unit() {
    let s = LinearScale::new((-10.0, 10.0), (0.0, 1.0));
    assert!((s.apply(0.0) - 0.5).abs() < 1e-12);
}

#[test]
fn linear_arbitrary_interval() {
    let s = LinearScale::new((1.0, 3.0), (10.0, 20.0));
    assert!((s.apply(2.0) - 15.0).abs() < 1e-12);
}

#[test]
fn linear_fractional_domain() {
    let s = LinearScale::new((0.0, 1.0), (0.0, 4.0));
    assert!((s.apply(0.25) - 1.0).abs() < 1e-12);
}

#[test]
fn log_maps_powers_of_ten() {
    let s = LogScale::new((1.0, 1000.0), (0.0, 1.0), 10.0).unwrap();
    assert!((s.apply(1.0) - 0.0).abs() < 1e-9);
    assert!((s.apply(1000.0) - 1.0).abs() < 1e-9);
    assert!((s.apply(10.0) - (1.0_f64 / 3.0)).abs() < 1e-9);
}

#[test]
fn log_custom_range() {
    let s = LogScale::new((10.0, 1000.0), (0.0, 100.0), 10.0).unwrap();
    assert!((s.apply(100.0) - 50.0).abs() < 1e-9);
}

#[test]
fn log_invert_roundtrip_one() {
    let s = LogScale::new((2.0, 128.0), (-1.0, 2.0), 2.0).unwrap();
    let v = 16.0;
    assert!((s.invert(s.apply(v)) - v).abs() < 1e-9 * v.abs().max(1.0));
}

#[test]
fn log_natural_base_midpoint() {
    let e = std::f64::consts::E;
    let s = LogScale::new((1.0, e * e), (0.0, 1.0), e).unwrap();
    assert!((s.apply(e) - 0.5).abs() < 1e-9);
}

#[test]
fn log_intermediate_value() {
    let s = LogScale::new((1.0, 100.0), (0.0, 2.0), 10.0).unwrap();
    let y = s.apply(10.0);
    assert!((y - 1.0).abs() < 1e-9);
}

#[test]
fn log_new_rejects_non_positive_domain() {
    assert_eq!(
        LogScale::new((0.0, 10.0), (0.0, 1.0), 10.0).unwrap_err(),
        ScaleError::NonPositiveDomain
    );
    assert_eq!(
        LogScale::new((-1.0, 10.0), (0.0, 1.0), 10.0).unwrap_err(),
        ScaleError::NonPositiveDomain
    );
    assert_eq!(
        LogScale::new((5.0, 5.0), (0.0, 1.0), 10.0).unwrap_err(),
        ScaleError::NonPositiveDomain
    );
}

#[test]
fn log_new_rejects_invalid_base() {
    assert_eq!(
        LogScale::new((1.0, 10.0), (0.0, 1.0), 1.0).unwrap_err(),
        ScaleError::InvalidLogBase
    );
    assert_eq!(
        LogScale::new((1.0, 10.0), (0.0, 1.0), 0.0).unwrap_err(),
        ScaleError::InvalidLogBase
    );
    assert_eq!(
        LogScale::new((1.0, 10.0), (0.0, 1.0), -2.0).unwrap_err(),
        ScaleError::InvalidLogBase
    );
}

#[test]
#[should_panic(expected = "domain endpoints must differ")]
fn linear_new_panics_on_degenerate_domain() {
    let _ = LinearScale::new((1.0, 1.0), (0.0, 1.0));
}

#[test]
fn linear_apply_clamped() {
    let s = LinearScale::new((0.0, 1.0), (0.0, 10.0));
    assert!((s.apply_clamped(2.0) - 10.0).abs() < 1e-12);
    assert!((s.apply_clamped(-3.0) - 0.0).abs() < 1e-12);
}

#[test]
fn linear_domain_range_accessors() {
    let s = LinearScale::new((3.0, 7.0), (-1.0, 2.0));
    assert_eq!(s.domain(), (3.0, 7.0));
    assert_eq!(s.range(), (-1.0, 2.0));
}

#[test]
fn log_domain_range_accessors() {
    let s = LogScale::new((1.0, 9.0), (10.0, 20.0), 3.0).unwrap();
    assert_eq!(s.domain(), (1.0, 9.0));
    assert_eq!(s.range(), (10.0, 20.0));
}

#[test]
fn linear_invert_is_inverse_of_apply() {
    let s = LinearScale::new((-2.0, 8.0), (100.0, 200.0));
    for &v in &[-1.0, 0.0, 2.5, 7.99] {
        assert!((s.invert(s.apply(v)) - v).abs() < 1e-9);
    }
}

#[test]
fn log_apply_at_domain_endpoints() {
    let s = LogScale::new((1.0, 100.0), (0.0, 10.0), 10.0).unwrap();
    assert!((s.apply(1.0) - 0.0).abs() < 1e-9);
    assert!((s.apply(100.0) - 10.0).abs() < 1e-9);
}

#[test]
fn log_fails_on_nan_domain() {
    assert_eq!(
        LogScale::new((f64::NAN, 1.0), (0.0, 1.0), 10.0).unwrap_err(),
        ScaleError::NonPositiveDomain
    );
}

#[test]
fn log_fails_on_infinite_domain() {
    assert_eq!(
        LogScale::new((1.0, f64::INFINITY), (0.0, 1.0), 10.0).unwrap_err(),
        ScaleError::NonPositiveDomain
    );
}

#[test]
fn log_fails_on_non_finite_base() {
    assert_eq!(
        LogScale::new((1.0, 10.0), (0.0, 1.0), f64::NAN).unwrap_err(),
        ScaleError::InvalidLogBase
    );
}

#[test]
fn log_fails_on_zero_base() {
    assert_eq!(
        LogScale::new((1.0, 10.0), (0.0, 1.0), 0.0).unwrap_err(),
        ScaleError::InvalidLogBase
    );
}

proptest! {
    #[test]
    fn linear_apply_invert_roundtrip(
        d0 in -1e6f64..1e6,
        d1 in -1e6f64..1e6,
        r0 in -1e6f64..1e6,
        r1 in -1e6f64..1e6,
        v in -1e6f64..1e6
    ) {
        prop_assume!((d0 - d1).abs() > 1e-6);
        prop_assume!((r0 - r1).abs() > 1e-6);
        let s = LinearScale::new((d0, d1), (r0, r1));
        let mapped = s.apply(v);
        let back = s.invert(mapped);
        let eps = 1e-6 * v.abs().max(1.0);
        prop_assert!((back - v).abs() < eps, "v={v} back={back} diff={}", (back - v).abs());
    }

    #[test]
    fn log_apply_invert_roundtrip(
        d0 in 1e-9f64..1e4,
        d1 in 1e-9f64..1e4,
        r0 in -1e3f64..1e3,
        r1 in -1e3f64..1e3,
        t in 0.02f64..0.98f64
    ) {
        prop_assume!((d0 - d1).abs() > 1e-9 * d0.max(d1));
        prop_assume!((r0 - r1).abs() > 1e-9);
        let s = match LogScale::new((d0, d1), (r0, r1), 10.0) {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };
        let lo = d0.min(d1);
        let hi = d0.max(d1);
        let v = lo * (hi / lo).powf(t);
        prop_assume!(v.is_finite());
        let mapped = s.apply(v);
        let back = s.invert(mapped);
        let eps = 1e-5 * v.abs().max(1.0);
        prop_assert!((back - v).abs() < eps, "v={v} back={back} diff={}", (back - v).abs());
    }
}
