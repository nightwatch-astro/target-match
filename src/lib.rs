#![doc = include_str!("../README.md")]

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//!
//! # Modules
//!
//! - [`optics`] — plate scale and field-of-view geometry from focal length,
//!   pixel size (x/y), binning (x/y), and sensor dimensions — or a directly
//!   supplied pixel scale / field of view — plus the search-radius policies.
//! - [`matcher`] — the `SkyObject` input trait, match constraints (radius,
//!   rectangular field of view, nearest-N), and deterministic ranking.
//! - [`guide`] — a task-oriented walkthrough, from implementing `SkyObject`
//!   through repeated queries with `Matcher`.

pub mod error;
pub mod matcher;
pub mod optics;

/// A task-oriented walkthrough of `target-match`, from implementing
/// [`SkyObject`] through repeated queries with [`Matcher`]. Source:
/// [`docs/guide.md`](https://github.com/nightwatch-astro/target-match/blob/main/docs/guide.md).
#[doc = include_str!("../docs/guide.md")]
pub mod guide {}

/// The astronomy-math crate whose types appear in this crate's public API,
/// re-exported so consumers can use the exact version target-match was built
/// against.
pub use skymath;

// `#[doc(inline)]` puts each item's canonical rustdoc page at the crate root
// (`target_match::Foo`), matching how these re-exports are used. Without it,
// because the source modules are public, rustdoc keeps the page under the
// submodule path (`target_match/matcher/…`) and crate-root links 404.
#[doc(inline)]
pub use error::{Error, Result};
#[doc(inline)]
pub use matcher::{
    is_framed, rank, Constraint, Match, Matcher, Membership, Offset, Query, SkyObject,
};
#[doc(inline)]
pub use optics::{
    Field, Optics, RadiusPolicy, ARCSEC_PER_DEGREE, ARCSEC_PER_RADIAN, DEFAULT_FALLBACK_RADIUS,
};
