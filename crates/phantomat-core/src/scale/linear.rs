use super::Scale;

/// Linear map from `domain` to `range` (both closed intervals in ℝ).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain: (f64, f64),
    range: (f64, f64),
}

impl LinearScale {
    /// Builds a linear scale.
    ///
    /// # Panics
    ///
    /// Panics if `domain.0 == domain.1` (zero-length domain).
    #[must_use]
    pub fn new(domain: (f64, f64), range: (f64, f64)) -> Self {
        assert!(
            domain.0 != domain.1,
            "LinearScale::new: domain endpoints must differ"
        );
        Self { domain, range }
    }

    /// Like [`apply`](Scale::apply), but clamps `value` to the domain interval first.
    #[must_use]
    pub fn apply_clamped(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        let lo = d0.min(d1);
        let hi = d0.max(d1);
        let v = value.clamp(lo, hi);
        self.apply(v)
    }

    #[inline]
    fn t(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        (value - d0) / (d1 - d0)
    }

    #[inline]
    fn range_at_t(&self, t: f64) -> f64 {
        let (r0, r1) = self.range;
        r0 + t * (r1 - r0)
    }

    #[inline]
    fn t_from_range_value(&self, value: f64) -> f64 {
        let (r0, r1) = self.range;
        (value - r0) / (r1 - r0)
    }

    #[inline]
    fn domain_at_t(&self, t: f64) -> f64 {
        let (d0, d1) = self.domain;
        d0 + t * (d1 - d0)
    }
}

impl Scale for LinearScale {
    fn apply(&self, value: f64) -> f64 {
        self.range_at_t(self.t(value))
    }

    fn invert(&self, value: f64) -> f64 {
        self.domain_at_t(self.t_from_range_value(value))
    }

    fn domain(&self) -> (f64, f64) {
        self.domain
    }

    fn range(&self) -> (f64, f64) {
        self.range
    }
}
