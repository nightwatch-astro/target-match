//! Identify which catalogued sky objects a telescope frame covers, given a
//! pointing (right ascension / declination) and a field of view (derived from
//! optics, or supplied directly).
//!
//! A caller-supplied set of catalogue objects is ranked by angular separation
//! from the pointing; objects inside the frame's membership shape are flagged.
//! Matching uses sky position only — a designation may ride along on a result
//! for display, but it never influences the match.
//!
//! The crate holds no catalogue data and performs no I/O. A consumer supplies
//! its own objects — from a database, a file, a name resolver, or a hand-built
//! list — by implementing the [`matcher`] module's `SkyObject` trait.
//!
//! # Modules
//!
//! - [`angle`] — angle and equatorial-coordinate primitives: decimal ⇄
//!   sexagesimal (`HH:MM:SS` / `±DD:MM:SS`) parsing and formatting, and
//!   great-circle (haversine) angular separation.
//! - [`optics`] — plate scale and field-of-view geometry from focal length,
//!   pixel size (x/y), binning (x/y), and sensor dimensions — or a directly
//!   supplied pixel scale / field of view — plus the search-radius policies.
//! - [`matcher`] — the `SkyObject` input trait, match constraints (radius,
//!   rectangular field of view, nearest-N), and deterministic ranking.
//!
//! # Example
//!
//! ```
//! use target_match::{Angle, Constraint, Equatorial, Field, Optics, RadiusPolicy, SkyObject, rank};
//!
//! struct Target { name: &'static str, ra: f64, dec: f64 }
//! impl SkyObject for Target {
//!     fn position(&self) -> Equatorial {
//!         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
//!     }
//! }
//!
//! let catalog = [
//!     Target { name: "M 31",  ra: 10.6847, dec: 41.2688 },
//!     Target { name: "M 33",  ra: 23.4621, dec: 30.6599 },
//! ];
//! let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();
//! let field = Field::from_optics(Optics {
//!     focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
//! }).unwrap();
//!
//! let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
//! assert_eq!(hits[0].object.name, "M 31");
//! ```

pub mod angle;
pub mod error;
pub mod matcher;
pub mod optics;

pub use angle::{precess, separation, Angle, Epoch, Equatorial};
pub use error::{Error, Result};
pub use matcher::{
    is_framed, rank, Constraint, Match, Matcher, Membership, Offset, Query, SkyObject,
};
pub use optics::{
    Field, Optics, RadiusPolicy, ARCSEC_PER_DEGREE, ARCSEC_PER_RADIAN, DEFAULT_FALLBACK_RADIUS,
};
