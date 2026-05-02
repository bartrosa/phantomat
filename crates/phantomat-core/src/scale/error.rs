/// Errors returned when constructing scales with invalid parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScaleError {
    /// Logarithmic mapping requires strictly positive domain endpoints.
    #[error("log scale domain must be strictly positive")]
    NonPositiveDomain,

    /// Base must be finite, positive, and not 1.
    #[error("log scale base must be finite, positive, and not 1")]
    InvalidLogBase,
}
