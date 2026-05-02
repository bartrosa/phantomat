use super::{Scale, ScaleError};

/// Logarithmic map: domain and range are linear in log-base space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogScale {
    domain: (f64, f64),
    range: (f64, f64),
    base: f64,
}

impl LogScale {
    /// Builds a log scale with base `base` (e.g. 10.0, std::f64::consts::E).
    ///
    /// Returns [`ScaleError::NonPositiveDomain`] if either domain endpoint is ≤ 0 or non-finite,
    /// or if domain endpoints coincide.
    pub fn new(domain: (f64, f64), range: (f64, f64), base: f64) -> Result<Self, ScaleError> {
        if !base.is_finite() || base <= 0.0 || (base - 1.0).abs() < f64::EPSILON {
            return Err(ScaleError::InvalidLogBase);
        }
        if !domain.0.is_finite()
            || !domain.1.is_finite()
            || domain.0 <= 0.0
            || domain.1 <= 0.0
            || domain.0 == domain.1
        {
            return Err(ScaleError::NonPositiveDomain);
        }
        Ok(Self {
            domain,
            range,
            base,
        })
    }

    #[inline]
    fn log_b(&self, x: f64) -> f64 {
        x.ln() / self.base.ln()
    }

    #[inline]
    fn pow_b(&self, y: f64) -> f64 {
        self.base.powf(y)
    }
}

impl Scale for LogScale {
    fn apply(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let t = (self.log_b(value) - self.log_b(d0)) / (self.log_b(d1) - self.log_b(d0));
        r0 + t * (r1 - r0)
    }

    fn invert(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let t = (value - r0) / (r1 - r0);
        let log_d0 = self.log_b(d0);
        let log_d1 = self.log_b(d1);
        self.pow_b(log_d0 + t * (log_d1 - log_d0))
    }

    fn domain(&self) -> (f64, f64) {
        self.domain
    }

    fn range(&self) -> (f64, f64) {
        self.range
    }
}
