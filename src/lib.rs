//! `target-match` — identify which catalogued sky object a telescope frame captured.
//!
//! Given a **pointing** (right ascension / declination) and a **field of view**
//! (derived from optics, or supplied directly), `target-match` ranks a
//! caller-supplied set of catalogue objects by angular separation and returns
//! those that fall on the frame. Matching is done **by sky position only, never
//! by name** — a designation may ride along on a result for display, but it
//! never influences the match (capture software writes object names
//! inconsistently; coordinates are authoritative).
//!
//! The crate is **catalog-agnostic**: it owns no catalogue data and performs no
//! I/O. A consumer brings its own objects — from a database, a file, a SIMBAD
//! resolver, or a hand-built list — by implementing the [`matcher`] crate's
//! `SkyObject` trait, and `target-match` does the geometry.
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
//! # Status
//!
//! Extraction scaffold carved out of the `nightwatch-astro/alm` targeting
//! pipeline. The public API is being specified under `specs/` (SpecKit); the
//! module bodies are documented stubs pending that spec.

pub mod angle;
pub mod matcher;
pub mod optics;
