//! Plate scale and field-of-view geometry.
//!
//! A [`Field`] is the angular extent of a frame, built from full [`Optics`], from
//! a directly supplied pixel scale, or from a directly supplied field of view. It
//! is binning-aware (effective pixel = pixel size × binning, per axis) and
//! axis-independent (x and y are handled separately). A [`RadiusPolicy`] turns a
//! field into a search radius.

use crate::angle::Angle;
use crate::error::{Error, Result};

/// Exact number of arcseconds in one radian (supersedes the rounded `206.265`).
pub const ARCSEC_PER_RADIAN: f64 = crate::angle::ARCSEC_PER_RADIAN;
/// Arcseconds per degree.
pub const ARCSEC_PER_DEGREE: f64 = 3600.0;
/// Fallback search radius when a field of view cannot be derived (5°).
pub const DEFAULT_FALLBACK_RADIUS: Angle = Angle::from_radians(5.0 * core::f64::consts::PI / 180.0);

/// Full optical train: focal length, per-axis pixel size, per-axis binning, and
/// sensor pixel counts.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Optics {
    /// Focal length, millimetres.
    pub focal_mm: f64,
    /// Pixel size in micrometres, `(x, y)`.
    pub pixel_um: (f64, f64),
    /// Binning factor, `(x, y)` (1 = unbinned).
    pub binning: (u32, u32),
    /// Sensor pixel counts, `(naxis1, naxis2)`.
    pub pixels: (u32, u32),
}

/// How a search radius is derived from a [`Field`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RadiusPolicy {
    /// Half the diagonal — the circle that circumscribes the whole frame (default).
    Circumscribed,
    /// Half the shorter side — the circle inscribed within the frame.
    Inscribed,
    /// A multiplier applied to the circumscribed radius.
    Multiplier(f64),
    /// An explicit radius, ignoring the field extent.
    Explicit(Angle),
}

/// The angular extent of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Field {
    fov: (Angle, Angle),
    pixel_scale: Option<(f64, f64)>, // arcsec/px, per axis; None for a direct-FOV field
}

impl Field {
    /// Derive a field from full optics.
    ///
    /// Effective pixel size = pixel size × binning (per axis); pixel scale
    /// (arcsec/px) = effective pixel size (mm) / focal length (mm) × arcsec/radian;
    /// field extent = pixel scale × pixel count.
    ///
    /// # Errors
    /// [`Error::InvalidOptics`] if any input is non-positive or non-finite.
    pub fn from_optics(o: Optics) -> Result<Self> {
        let focal = finite_positive(o.focal_mm, "focal length")?;
        let (px, py) = (
            finite_positive(o.pixel_um.0, "pixel size x")?,
            finite_positive(o.pixel_um.1, "pixel size y")?,
        );
        let (bx, by) = (
            positive_count(o.binning.0, "binning x")?,
            positive_count(o.binning.1, "binning y")?,
        );
        let (nx, ny) = (
            positive_count(o.pixels.0, "naxis1")?,
            positive_count(o.pixels.1, "naxis2")?,
        );
        // arcsec/px = eff_pixel_um / 1000 (→ mm) / focal_mm × arcsec/radian
        let scale_x = (px * bx) / 1000.0 / focal * ARCSEC_PER_RADIAN;
        let scale_y = (py * by) / 1000.0 / focal * ARCSEC_PER_RADIAN;
        Ok(Self {
            fov: (
                Angle::from_arcseconds(scale_x * nx),
                Angle::from_arcseconds(scale_y * ny),
            ),
            pixel_scale: Some((scale_x, scale_y)),
        })
    }

    /// Build a field from a directly supplied pixel scale (arcsec/px, per axis)
    /// and sensor pixel counts.
    ///
    /// # Errors
    /// [`Error::InvalidOptics`] if any input is non-positive or non-finite.
    pub fn from_pixel_scale(scale_arcsec_px: (f64, f64), pixels: (u32, u32)) -> Result<Self> {
        let sx = finite_positive(scale_arcsec_px.0, "pixel scale x")?;
        let sy = finite_positive(scale_arcsec_px.1, "pixel scale y")?;
        let nx = positive_count(pixels.0, "naxis1")?;
        let ny = positive_count(pixels.1, "naxis2")?;
        Ok(Self {
            fov: (
                Angle::from_arcseconds(sx * nx),
                Angle::from_arcseconds(sy * ny),
            ),
            pixel_scale: Some((sx, sy)),
        })
    }

    /// Build a field directly from its angular width and height (no optics; pixel
    /// scale is unknown).
    ///
    /// # Errors
    /// [`Error::InvalidOptics`] if width or height is non-positive or non-finite.
    pub fn from_fov(width: Angle, height: Angle) -> Result<Self> {
        finite_positive(width.degrees(), "field width")?;
        finite_positive(height.degrees(), "field height")?;
        Ok(Self {
            fov: (width, height),
            pixel_scale: None,
        })
    }

    /// Field width (x extent).
    #[must_use]
    pub fn width(self) -> Angle {
        self.fov.0
    }
    /// Field height (y extent).
    #[must_use]
    pub fn height(self) -> Angle {
        self.fov.1
    }
    /// Diagonal field of view.
    #[must_use]
    pub fn diagonal(self) -> Angle {
        Angle::from_degrees(self.fov.0.degrees().hypot(self.fov.1.degrees()))
    }
    /// Per-axis pixel scale (arcsec/px), if this field was built with a scale.
    #[must_use]
    pub fn pixel_scale(self) -> Option<(f64, f64)> {
        self.pixel_scale
    }

    /// Compute a search radius from this field under `policy`.
    #[must_use]
    pub fn radius(self, policy: RadiusPolicy) -> Angle {
        match policy {
            RadiusPolicy::Circumscribed => Angle::from_degrees(self.diagonal().degrees() / 2.0),
            RadiusPolicy::Inscribed => {
                Angle::from_degrees(self.fov.0.degrees().min(self.fov.1.degrees()) / 2.0)
            }
            RadiusPolicy::Multiplier(m) => Angle::from_degrees(self.diagonal().degrees() / 2.0 * m),
            RadiusPolicy::Explicit(a) => a,
        }
    }
}

fn finite_positive(v: f64, what: &str) -> Result<f64> {
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(Error::InvalidOptics(format!(
            "{what} must be finite and > 0, got {v}"
        )))
    }
}

fn positive_count(v: u32, what: &str) -> Result<f64> {
    if v >= 1 {
        Ok(f64::from(v))
    } else {
        Err(Error::InvalidOptics(format!("{what} must be >= 1")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn asi2600_800mm() -> Optics {
        Optics {
            focal_mm: 800.0,
            pixel_um: (3.76, 3.76),
            binning: (1, 1),
            pixels: (6248, 4176),
        }
    }

    #[test]
    fn from_optics_matches_hand_calc() {
        let f = Field::from_optics(asi2600_800mm()).unwrap();
        let (sx, sy) = f.pixel_scale().unwrap();
        assert!(approx(sx, 0.9694, 1e-3), "scale {sx}");
        assert!(approx(sy, 0.9694, 1e-3));
        assert!(
            approx(f.width().degrees(), 1.683, 5e-3),
            "w {}",
            f.width().degrees()
        );
        assert!(
            approx(f.height().degrees(), 1.125, 5e-3),
            "h {}",
            f.height().degrees()
        );
        // radius = circumscribed = half diagonal ≈ 1.012°
        assert!(approx(
            f.radius(RadiusPolicy::Circumscribed).degrees(),
            1.012,
            5e-3
        ));
    }

    #[test]
    fn binning_doubles_scale_and_fov() {
        let mut o = asi2600_800mm();
        o.binning = (2, 2);
        let f = Field::from_optics(o).unwrap();
        let (sx, _) = f.pixel_scale().unwrap();
        assert!(approx(sx, 2.0 * 0.9694, 2e-3), "binned scale {sx}");
        // Holding the (binned) pixel count, ×2 binning ⇒ ×2 scale ⇒ ×2 field (SC-009).
        assert!(
            approx(f.width().degrees(), 2.0 * 1.683, 1e-2),
            "w {}",
            f.width().degrees()
        );
    }

    #[test]
    fn binning_fov_doubles() {
        let base = Field::from_optics(asi2600_800mm()).unwrap();
        let mut o = asi2600_800mm();
        o.binning = (2, 2);
        let binned = Field::from_optics(o).unwrap();
        assert!(approx(
            binned.width().degrees(),
            2.0 * base.width().degrees(),
            1e-6
        ));
    }

    #[test]
    fn from_fov_and_pixel_scale_paths() {
        let direct =
            Field::from_fov(Angle::from_degrees(1.683), Angle::from_degrees(1.125)).unwrap();
        assert!(direct.pixel_scale().is_none());
        assert!(approx(
            direct.radius(RadiusPolicy::Circumscribed).degrees(),
            1.012,
            5e-3
        ));

        let by_scale = Field::from_pixel_scale((0.9694, 0.9694), (6248, 4176)).unwrap();
        assert!(approx(by_scale.width().degrees(), 1.683, 5e-3));
        assert_eq!(by_scale.pixel_scale(), Some((0.9694, 0.9694)));
    }

    #[test]
    fn radius_policies() {
        let f = Field::from_fov(Angle::from_degrees(2.0), Angle::from_degrees(1.0)).unwrap();
        assert!(approx(
            f.radius(RadiusPolicy::Inscribed).degrees(),
            0.5,
            1e-9
        )); // half min side
        let circ = f.radius(RadiusPolicy::Circumscribed).degrees();
        assert!(approx(circ, (2.0_f64.hypot(1.0)) / 2.0, 1e-9));
        assert!(approx(
            f.radius(RadiusPolicy::Multiplier(2.0)).degrees(),
            circ * 2.0,
            1e-9
        ));
        assert!(approx(
            f.radius(RadiusPolicy::Explicit(Angle::from_degrees(3.0)))
                .degrees(),
            3.0,
            1e-9
        ));
    }

    #[test]
    fn rejects_bad_optics() {
        let mut o = asi2600_800mm();
        o.focal_mm = 0.0;
        assert!(matches!(
            Field::from_optics(o).unwrap_err(),
            Error::InvalidOptics(_)
        ));
        let mut o2 = asi2600_800mm();
        o2.pixel_um = (-1.0, 3.76);
        assert!(matches!(
            Field::from_optics(o2).unwrap_err(),
            Error::InvalidOptics(_)
        ));
        let mut o3 = asi2600_800mm();
        o3.pixels = (0, 4176);
        assert!(matches!(
            Field::from_optics(o3).unwrap_err(),
            Error::InvalidOptics(_)
        ));
        assert!(matches!(
            Field::from_fov(Angle::from_degrees(0.0), Angle::from_degrees(1.0)).unwrap_err(),
            Error::InvalidOptics(_)
        ));
    }

    #[test]
    fn fallback_radius_is_five_degrees() {
        assert!(approx(DEFAULT_FALLBACK_RADIUS.degrees(), 5.0, 1e-9));
    }
}
