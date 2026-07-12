//! Error type for `target-match`.
//!
//! Every fallible entry point returns [`Result`]. The only construction this
//! crate itself validates is optics/field geometry — coordinate parsing and
//! domain checks happen in [`skymath`] when the consumer builds an
//! [`skymath::Equatorial`], and surface as [`skymath::Error`]. Matching and
//! precession are infallible once inputs are valid values.

use thiserror::Error;

/// Everything that can go wrong constructing `target-match` values.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// Optics or field inputs were non-positive, non-finite, or insufficient to
    /// derive a field of view.
    #[error("invalid optics: {0}")]
    InvalidOptics(String),
}

/// Convenience alias for `Result<T, `[`Error`](enum@Error)`>`.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages() {
        assert_eq!(
            Error::InvalidOptics("focal <= 0".into()).to_string(),
            "invalid optics: focal <= 0"
        );
    }
}
