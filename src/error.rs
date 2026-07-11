//! Error type for `target-match`.
//!
//! Every fallible entry point returns [`Result`]. Errors arise only at the
//! *construction* boundary — parsing coordinates, validating coordinate domains,
//! and validating optics. Matching and precession are infallible once inputs are
//! valid values (an [`crate::Equatorial`] always carries a fully-specified epoch,
//! so there is no "epoch could not be resolved" runtime failure).

use thiserror::Error;

/// Everything that can go wrong constructing `target-match` values.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A coordinate string could not be parsed (empty, non-numeric, malformed
    /// sexagesimal, or negative minutes/seconds).
    #[error("could not parse coordinate: {0}")]
    ParseCoord(String),

    /// A numeric coordinate or epoch was outside its valid domain
    /// (RA `[0, 360)`, Dec `[-90, 90]`, or a non-finite epoch year).
    #[error("{what} out of range: {value}")]
    OutOfRange {
        /// Which quantity was out of range (e.g. `"right ascension"`).
        what: &'static str,
        /// The offending value.
        value: f64,
    },

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
            Error::ParseCoord("x".into()).to_string(),
            "could not parse coordinate: x"
        );
        assert_eq!(
            Error::OutOfRange {
                what: "declination",
                value: 91.0
            }
            .to_string(),
            "declination out of range: 91"
        );
        assert_eq!(
            Error::InvalidOptics("focal <= 0".into()).to_string(),
            "invalid optics: focal <= 0"
        );
    }
}
