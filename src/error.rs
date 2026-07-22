// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Error type for `target-match`.
//!
//! Every fallible entry point returns [`Result`]. Coordinate parsing and domain
//! checks happen in [`skymath`] when the consumer builds an
//! [`skymath::Equatorial`]. This crate validates optics, captured boundaries,
//! common-plane projections, and caller-supplied geometry search parameters.

use thiserror::Error;

/// Everything that can go wrong constructing `target-match` values.
///
/// # Example
///
/// ```
/// use target_match::{Error, Field, Optics};
///
/// let err = Field::from_optics(Optics {
///     focal_mm: 0.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
/// })
/// .unwrap_err();
/// assert!(matches!(err, Error::InvalidOptics(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// Optics or field inputs were non-positive, non-finite, or insufficient to
    /// derive a field of view.
    #[error("invalid optics: {0}")]
    InvalidOptics(String),
    /// An ordered captured boundary is degenerate, self-intersecting, or does
    /// not contain its declared centre.
    #[error("invalid footprint {provenance:?}: {reason}")]
    InvalidFootprint {
        /// Caller-supplied evidence identity.
        provenance: String,
        /// Validation failure.
        reason: String,
    },
    /// Compared geometry uses different coordinate epochs.
    #[error("footprint coordinates must use one epoch")]
    FootprintEpochMismatch,
    /// A common tangent anchor cannot be selected for antipodal geometry.
    #[error("antipodal geometry has no unique common tangent plane")]
    AntipodalGeometry,
    /// A point is at or beyond the selected gnomonic projection horizon.
    #[error("gnomonic projection failed for {0}")]
    ProjectionFailed(String),
    /// A footprint union requires at least one input.
    #[error("footprint union requires at least one footprint")]
    EmptyFootprintSet,
    /// Per-panel evidence requires unique provenance identities.
    #[error("duplicate footprint provenance: {0}")]
    DuplicateFootprintProvenance(String),
    /// A normalized coverage band is non-finite, reversed, or outside `[0, 1]`.
    #[error("invalid coverage band: {0}")]
    InvalidCoverageBand(String),
    /// A residual-rotation search domain or resolution is invalid.
    #[error("invalid rotation search: {0}")]
    InvalidRotationSearch(String),
    /// An ellipse has invalid axes, orientation, or sampling resolution.
    #[error("invalid ellipse: {0}")]
    InvalidEllipse(String),
    /// Caller-supplied object geometry cannot produce a positive planar area.
    #[error("invalid object geometry: {0}")]
    InvalidObjectGeometry(String),
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
        assert_eq!(
            Error::AntipodalGeometry.to_string(),
            "antipodal geometry has no unique common tangent plane"
        );
    }
}
