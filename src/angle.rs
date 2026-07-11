//! Angle and equatorial-coordinate primitives.
//!
//! Planned surface (pending the SpecKit spec under `specs/`):
//!
//! - `Angle` — a unit-aware angle with constructors/accessors for degrees,
//!   radians, arcminutes, arcseconds, and hours, plus normalization helpers.
//! - `Equatorial { ra, dec }` — an ICRS/J2000 sky position, RA `[0, 360)`,
//!   Dec `[-90, 90]`.
//! - Sexagesimal parsing and formatting in **both** directions: `HH:MM:SS(.s)` /
//!   `HH MM SS` for right ascension and `±DD:MM:SS` for declination ⇄ decimal
//!   degrees. (The parse follows `±(|D| + M/60 + S/3600)`, RA × 15; the crate
//!   also provides the reverse formatter that the upstream `alm` code lacks.)
//! - `separation(a, b)` — great-circle (haversine) angular separation, `[0, 180]°`.
//!
//! Pure `std`; no dependencies.

// TODO(spec): implement per the target-match specification under `specs/`.
