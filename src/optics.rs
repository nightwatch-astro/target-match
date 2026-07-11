//! Plate scale and field-of-view geometry.
//!
//! Planned surface (pending the SpecKit spec under `specs/`):
//!
//! - Derive pixel scale (arcsec/px) and field width / height / diagonal from
//!   focal length + pixel size (x/y) + binning (x/y) + sensor pixels
//!   (naxis1/naxis2), **or** from a directly supplied pixel scale, **or** from a
//!   directly supplied field of view (x° × y°).
//! - Search-radius policies: circumscribed (½ diagonal — the original `alm`
//!   behaviour), inscribed (½ the shorter side), an explicit multiplier, or an
//!   explicit radius.
//!
//! Binning-aware and axis-independent by design — closing the two gaps in the
//! original `alm` implementation, which ignored binning and assumed a single
//! square pixel.
//!
//! Depends only on [`crate::angle`].

// TODO(spec): implement per the target-match specification under `specs/`.
