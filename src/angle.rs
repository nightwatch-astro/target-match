//! Angle and equatorial-coordinate primitives.
//!
//! - [`Angle`] — a unit-aware angle (degrees / radians / arcminutes / arcseconds
//!   / hours) stored internally in radians.
//! - [`Epoch`] — J2000 or epoch-of-date (a Julian year).
//! - [`Equatorial`] — an RA/Dec sky position tagged with an epoch, with
//!   sexagesimal and decimal parsing and formatting.
//! - [`separation`] — great-circle (haversine) angular separation.
//! - [`precess`] — IAU 1976 precession between J2000 and epoch-of-date.
//!
//! Matching treats supplied positions as **coordinates only** — no name is ever
//! read (see the crate-level docs).

use core::f64::consts::PI;
use core::ops::{Add, Mul, Neg, Sub};

use crate::error::{Error, Result};

const DEG_PER_RAD: f64 = 180.0 / PI;
const RAD_PER_DEG: f64 = PI / 180.0;
/// Exact number of arcseconds in one radian (supersedes the rounded `206.265`).
pub(crate) const ARCSEC_PER_RADIAN: f64 = 206_264.806_247_096_36;

// ── Angle ──────────────────────────────────────────────────────────────────────

/// A unit-aware angle, stored internally in radians.
///
/// Construction and read-out are available in degrees, radians, arcminutes,
/// arcseconds, and hours (1 hour = 15°). Normalization is explicit — an `Angle`
/// holds whatever finite value it was given until you ask for a normalized form.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Angle {
    radians: f64,
}

impl Angle {
    /// Construct from radians.
    #[must_use]
    pub const fn from_radians(radians: f64) -> Self {
        Self { radians }
    }
    /// Construct from decimal degrees.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Self {
        Self {
            radians: degrees * RAD_PER_DEG,
        }
    }
    /// Construct from arcminutes (1/60 degree).
    #[must_use]
    pub fn from_arcminutes(arcmin: f64) -> Self {
        Self::from_degrees(arcmin / 60.0)
    }
    /// Construct from arcseconds (1/3600 degree).
    #[must_use]
    pub fn from_arcseconds(arcsec: f64) -> Self {
        Self::from_degrees(arcsec / 3600.0)
    }
    /// Construct from hours of right ascension (1 hour = 15°).
    #[must_use]
    pub fn from_hours(hours: f64) -> Self {
        Self::from_degrees(hours * 15.0)
    }

    /// Value in radians.
    #[must_use]
    pub const fn radians(self) -> f64 {
        self.radians
    }
    /// Value in decimal degrees.
    #[must_use]
    pub fn degrees(self) -> f64 {
        self.radians * DEG_PER_RAD
    }
    /// Value in arcminutes.
    #[must_use]
    pub fn arcminutes(self) -> f64 {
        self.degrees() * 60.0
    }
    /// Value in arcseconds.
    #[must_use]
    pub fn arcseconds(self) -> f64 {
        self.degrees() * 3600.0
    }
    /// Value in hours (degrees / 15).
    #[must_use]
    pub fn hours(self) -> f64 {
        self.degrees() / 15.0
    }

    /// Return an equivalent angle wrapped into `[0, 360)` degrees.
    #[must_use]
    pub fn normalized_0_360(self) -> Self {
        let mut d = self.degrees() % 360.0;
        if d < 0.0 {
            d += 360.0;
        }
        Self::from_degrees(d)
    }
    /// Return an equivalent angle wrapped into `(-180, 180]` degrees.
    #[must_use]
    pub fn normalized_pm_180(self) -> Self {
        let mut d = self.normalized_0_360().degrees();
        if d > 180.0 {
            d -= 360.0;
        }
        Self::from_degrees(d)
    }
}

impl Add for Angle {
    type Output = Angle;
    fn add(self, rhs: Angle) -> Angle {
        Angle::from_radians(self.radians + rhs.radians)
    }
}
impl Sub for Angle {
    type Output = Angle;
    fn sub(self, rhs: Angle) -> Angle {
        Angle::from_radians(self.radians - rhs.radians)
    }
}
impl Neg for Angle {
    type Output = Angle;
    fn neg(self) -> Angle {
        Angle::from_radians(-self.radians)
    }
}
impl Mul<f64> for Angle {
    type Output = Angle;
    fn mul(self, rhs: f64) -> Angle {
        Angle::from_radians(self.radians * rhs)
    }
}

// ── Epoch ──────────────────────────────────────────────────────────────────────

/// The reference epoch of a sky position.
///
/// `OfDate` carries a Julian year (e.g. `2026.5`) — the observation instant to
/// day precision, which is far finer than precession needs. Because the year is
/// always present, a "JNow without a date" state is unrepresentable and
/// precession is always well-defined.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Epoch {
    /// The J2000.0 standard epoch (≈ ICRS for planning-grade work).
    J2000,
    /// Epoch of date, as a Julian year.
    OfDate(f64),
}

impl Epoch {
    /// Julian centuries of this epoch measured from J2000 (`J2000` → 0).
    #[must_use]
    pub fn julian_centuries_from_j2000(self) -> f64 {
        match self {
            Epoch::J2000 => 0.0,
            Epoch::OfDate(year) => (year - 2000.0) / 100.0,
        }
    }
}

// ── Equatorial ─────────────────────────────────────────────────────────────────

/// An equatorial sky position (right ascension, declination) tagged with an
/// [`Epoch`]. After construction, `ra ∈ [0, 360)`° and `dec ∈ [-90, 90]`°.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Equatorial {
    ra: Angle,
    dec: Angle,
    epoch: Epoch,
}

impl Equatorial {
    /// Construct from RA/Dec angles at an epoch, validating domains.
    ///
    /// # Errors
    /// [`Error::OutOfRange`] if RA ∉ [0, 360), Dec ∉ [-90, 90], or the epoch
    /// year is non-finite.
    pub fn new(ra: Angle, dec: Angle, epoch: Epoch) -> Result<Self> {
        let ra_deg = ra.degrees();
        let dec_deg = dec.degrees();
        if !ra_deg.is_finite() || !(0.0..360.0).contains(&ra_deg) {
            return Err(Error::OutOfRange {
                what: "right ascension",
                value: ra_deg,
            });
        }
        if !dec_deg.is_finite() || !(-90.0..=90.0).contains(&dec_deg) {
            return Err(Error::OutOfRange {
                what: "declination",
                value: dec_deg,
            });
        }
        if let Epoch::OfDate(year) = epoch {
            if !year.is_finite() {
                return Err(Error::OutOfRange {
                    what: "epoch year",
                    value: year,
                });
            }
        }
        Ok(Self { ra, dec, epoch })
    }

    /// Construct a J2000 position from RA/Dec angles.
    ///
    /// # Errors
    /// See [`Equatorial::new`].
    pub fn j2000(ra: Angle, dec: Angle) -> Result<Self> {
        Self::new(ra, dec, Epoch::J2000)
    }

    /// Parse RA and Dec strings (sexagesimal or decimal) at an epoch.
    ///
    /// RA accepts `HH:MM:SS(.s)`, `HH MM SS`, or a bare decimal in **degrees**
    /// (sexagesimal RA is hours ×15). Dec accepts `±DD:MM:SS(.s)`, `±DD MM SS`,
    /// or decimal degrees.
    ///
    /// # Errors
    /// [`Error::ParseCoord`] on malformed input; [`Error::OutOfRange`] on domain.
    pub fn parse(ra: &str, dec: &str, epoch: Epoch) -> Result<Self> {
        let ra_deg = parse_ra_degrees(ra)?;
        let dec_deg = parse_dec_degrees(dec)?;
        Self::new(
            Angle::from_degrees(ra_deg),
            Angle::from_degrees(dec_deg),
            epoch,
        )
    }

    /// Parse a J2000 position from RA/Dec strings.
    ///
    /// # Errors
    /// See [`Equatorial::parse`].
    pub fn parse_j2000(ra: &str, dec: &str) -> Result<Self> {
        Self::parse(ra, dec, Epoch::J2000)
    }

    /// Right ascension.
    #[must_use]
    pub fn ra(self) -> Angle {
        self.ra
    }
    /// Declination.
    #[must_use]
    pub fn dec(self) -> Angle {
        self.dec
    }
    /// Reference epoch.
    #[must_use]
    pub fn epoch(self) -> Epoch {
        self.epoch
    }
    /// `(ra_degrees, dec_degrees)`.
    #[must_use]
    pub fn to_degrees(self) -> (f64, f64) {
        (self.ra.degrees(), self.dec.degrees())
    }

    /// Format RA as `HH:MM:SS.sss` with the given number of fractional-second digits.
    #[must_use]
    pub fn ra_to_sexagesimal(self, decimals: usize) -> String {
        format_sexagesimal(self.ra.hours(), 24.0, false, decimals)
    }
    /// Format Dec as `±DD:MM:SS.sss` with the given number of fractional-second digits.
    #[must_use]
    pub fn dec_to_sexagesimal(self, decimals: usize) -> String {
        format_sexagesimal(self.dec.degrees(), 90.0, true, decimals)
    }

    /// Unit direction vector `(x, y, z)` on the celestial sphere.
    pub(crate) fn to_unit_vector(self) -> [f64; 3] {
        let (a, d) = (self.ra.radians(), self.dec.radians());
        [d.cos() * a.cos(), d.cos() * a.sin(), d.sin()]
    }

    /// Build a position from a unit vector at the given epoch (RA normalized to
    /// `[0, 360)`).
    pub(crate) fn from_unit_vector(v: [f64; 3], epoch: Epoch) -> Self {
        let ra = Angle::from_radians(v[1].atan2(v[0])).normalized_0_360();
        let dec = Angle::from_radians(v[2].atan2((v[0] * v[0] + v[1] * v[1]).sqrt()));
        Self { ra, dec, epoch }
    }
}

// ── Separation ─────────────────────────────────────────────────────────────────

/// Great-circle angular separation between two positions (haversine form).
///
/// The result is in `[0, 180]`°, symmetric in its arguments, and numerically
/// stable for the small separations that dominate frame matching. Epochs are not
/// reconciled here — this is a purely geometric operation on the given numbers.
#[must_use]
pub fn separation(a: Equatorial, b: Equatorial) -> Angle {
    let (ra1, dec1) = (a.ra.radians(), a.dec.radians());
    let (ra2, dec2) = (b.ra.radians(), b.dec.radians());
    let (dra, ddec) = (ra2 - ra1, dec2 - dec1);
    let sin_ddec = (ddec / 2.0).sin();
    let sin_dra = (dra / 2.0).sin();
    // hav(θ) = sin²(Δδ/2) + cos δ1 · cos δ2 · sin²(Δα/2)
    let h = sin_ddec.mul_add(sin_ddec, dec1.cos() * dec2.cos() * sin_dra * sin_dra);
    let central = 2.0 * h.sqrt().clamp(0.0, 1.0).asin();
    Angle::from_radians(central)
}

// ── Precession (IAU 1976, Meeus ch. 21) ──────────────────────────────────────────

/// Precess a position to another epoch using IAU 1976 precession.
///
/// Handles J2000 → epoch-of-date, epoch-of-date → J2000, and date → date (via
/// J2000). Accurate to ≤ ~1 arcsecond over several centuries — well inside
/// planning grade. Apparent-place terms (nutation, aberration, proper motion)
/// are out of scope. Precessing to the same epoch is the identity.
#[must_use]
pub fn precess(pos: Equatorial, to: Epoch) -> Equatorial {
    if pos.epoch == to {
        return pos;
    }
    // Reduce to J2000 first, then forward to the target.
    let at_j2000 = match pos.epoch {
        Epoch::J2000 => pos,
        Epoch::OfDate(year) => {
            let v = apply_matrix(&transpose(&precession_matrix(year)), pos.to_unit_vector());
            Equatorial::from_unit_vector(v, Epoch::J2000)
        }
    };
    match to {
        Epoch::J2000 => at_j2000,
        Epoch::OfDate(year) => {
            let v = apply_matrix(&precession_matrix(year), at_j2000.to_unit_vector());
            Equatorial::from_unit_vector(v, to)
        }
    }
}

/// IAU 1976 precession matrix taking a J2000 unit vector to epoch-of-`year`.
///
/// `P = R3(-z) · R2(θ) · R3(-ζ)` with the accumulated angles (T = 0 form, since
/// the reference epoch is always J2000).
fn precession_matrix(year: f64) -> [[f64; 3]; 3] {
    let t = (year - 2000.0) / 100.0; // Julian centuries from J2000
    let arcsec = |a: f64| a * (RAD_PER_DEG / 3600.0);
    let zeta = arcsec(2306.2181 * t + 0.301_88 * t * t + 0.017_998 * t * t * t);
    let z = arcsec(2306.2181 * t + 1.094_68 * t * t + 0.018_203 * t * t * t);
    let theta = arcsec(2004.3109 * t - 0.426_65 * t * t - 0.041_833 * t * t * t);
    mat_mul(&mat_mul(&rot_z(-z), &rot_y(theta)), &rot_z(-zeta))
}

fn rot_z(phi: f64) -> [[f64; 3]; 3] {
    let (s, c) = phi.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}
fn rot_y(phi: f64) -> [[f64; 3]; 3] {
    let (s, c) = phi.sin_cos();
    [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]]
}
fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}
fn transpose(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut t = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = m[j][i];
        }
    }
    t
}
fn apply_matrix(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// ── Sexagesimal parse / format ───────────────────────────────────────────────────

/// Parse right ascension into decimal degrees. Sexagesimal is hours (×15); a bare
/// decimal is degrees.
fn parse_ra_degrees(raw: &str) -> Result<f64> {
    if looks_sexagesimal(raw) {
        Ok(parse_sexagesimal(raw)? * 15.0)
    } else {
        parse_decimal(raw)
    }
}

/// Parse declination into decimal degrees (always degrees).
fn parse_dec_degrees(raw: &str) -> Result<f64> {
    if looks_sexagesimal(raw) {
        parse_sexagesimal(raw)
    } else {
        parse_decimal(raw)
    }
}

fn looks_sexagesimal(raw: &str) -> bool {
    let t = raw.trim();
    t.contains(':') || t.split_whitespace().count() > 1
}

fn parse_decimal(raw: &str) -> Result<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error::ParseCoord(format!("not a finite number: {raw:?}")))
}

/// Parse `±D:M:S(.s)` / `±D M S` into a signed decimal value (degrees for Dec,
/// hours for RA). Minutes/seconds must be non-negative; the sign comes from the
/// leading field.
fn parse_sexagesimal(raw: &str) -> Result<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::ParseCoord("empty coordinate".to_owned()));
    }
    let normalized = trimmed.replace(':', " ");
    let mut parts = normalized.split_whitespace();
    let deg_str = parts
        .next()
        .ok_or_else(|| Error::ParseCoord(format!("no leading field: {raw:?}")))?;
    let negative = deg_str.starts_with('-');
    let deg: f64 = deg_str
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
        .ok_or_else(|| Error::ParseCoord(format!("bad degrees/hours field: {raw:?}")))?;
    let min = next_field(&mut parts, raw, "minutes")?;
    let sec = next_field(&mut parts, raw, "seconds")?;
    if parts.next().is_some() {
        return Err(Error::ParseCoord(format!("too many fields: {raw:?}")));
    }
    if min < 0.0 || sec < 0.0 || min >= 60.0 || sec >= 60.0 {
        return Err(Error::ParseCoord(format!(
            "minutes/seconds out of range: {raw:?}"
        )));
    }
    let magnitude = deg.abs() + min / 60.0 + sec / 3600.0;
    Ok(if negative { -magnitude } else { magnitude })
}

fn next_field<'a>(parts: &mut impl Iterator<Item = &'a str>, raw: &str, what: &str) -> Result<f64> {
    match parts.next() {
        None => Ok(0.0),
        Some(s) => s
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| Error::ParseCoord(format!("bad {what} field: {raw:?}"))),
    }
}

/// Format a signed decimal value as sexagesimal `[-]AA:BB:CC.cc`. `wrap` is the
/// modulus for the leading field (24 for hours, 90 unused for dec since dec is
/// bounded). `signed` prints a leading `+`/`-`.
fn format_sexagesimal(value: f64, _wrap: f64, signed: bool, decimals: usize) -> String {
    let neg = value < 0.0;
    let mut v = value.abs();
    // Round at the requested precision to avoid 59.9999 rollovers.
    let sec_scale = 3600.0 * 10f64.powi(decimals as i32);
    let total_units = (v * sec_scale).round() / sec_scale;
    v = total_units;
    let a = v.trunc();
    let rem_min = (v - a) * 60.0;
    let b = rem_min.trunc();
    let c = (rem_min - b) * 60.0;
    let sign = if neg {
        "-"
    } else if signed {
        "+"
    } else {
        ""
    };
    let width = if decimals > 0 { decimals + 3 } else { 2 };
    format!(
        "{sign}{a:02}:{b:02.0}:{c:0width$.decimals$}",
        a = a as i64,
        b = b
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    // ── Angle ──
    #[test]
    fn angle_conversions() {
        let a = Angle::from_degrees(90.0);
        assert!(approx(a.radians(), PI / 2.0, 1e-12));
        assert!(approx(a.arcminutes(), 5400.0, 1e-6));
        assert!(approx(a.arcseconds(), 324_000.0, 1e-3));
        assert!(approx(Angle::from_hours(1.0).degrees(), 15.0, 1e-12));
        assert!(approx(Angle::from_arcminutes(60.0).degrees(), 1.0, 1e-12));
        assert!(approx(Angle::from_arcseconds(3600.0).degrees(), 1.0, 1e-12));
    }

    #[test]
    fn angle_normalization() {
        assert!(approx(
            Angle::from_degrees(370.0).normalized_0_360().degrees(),
            10.0,
            1e-9
        ));
        assert!(approx(
            Angle::from_degrees(-10.0).normalized_0_360().degrees(),
            350.0,
            1e-9
        ));
        assert!(approx(
            Angle::from_degrees(350.0).normalized_pm_180().degrees(),
            -10.0,
            1e-9
        ));
    }

    #[test]
    fn angle_ops() {
        let s = Angle::from_degrees(10.0) + Angle::from_degrees(5.0);
        assert!(approx(s.degrees(), 15.0, 1e-12));
        assert!(approx(
            (Angle::from_degrees(10.0) * 3.0).degrees(),
            30.0,
            1e-12
        ));
        assert!(approx((-Angle::from_degrees(10.0)).degrees(), -10.0, 1e-12));
    }

    // ── Epoch ──
    #[test]
    fn epoch_centuries() {
        assert!(approx(
            Epoch::J2000.julian_centuries_from_j2000(),
            0.0,
            1e-15
        ));
        assert!(approx(
            Epoch::OfDate(2100.0).julian_centuries_from_j2000(),
            1.0,
            1e-12
        ));
    }

    // ── Equatorial construction / validation ──
    #[test]
    fn equatorial_validates_domain() {
        assert!(Equatorial::j2000(Angle::from_degrees(10.0), Angle::from_degrees(41.0)).is_ok());
        assert!(matches!(
            Equatorial::j2000(Angle::from_degrees(360.0), Angle::from_degrees(0.0)),
            Err(Error::OutOfRange {
                what: "right ascension",
                ..
            })
        ));
        assert!(matches!(
            Equatorial::j2000(Angle::from_degrees(0.0), Angle::from_degrees(90.1)),
            Err(Error::OutOfRange {
                what: "declination",
                ..
            })
        ));
        assert!(matches!(
            Equatorial::new(
                Angle::from_degrees(0.0),
                Angle::from_degrees(0.0),
                Epoch::OfDate(f64::NAN)
            ),
            Err(Error::OutOfRange {
                what: "epoch year",
                ..
            })
        ));
    }

    // ── Parsing ──
    #[test]
    fn parse_sexagesimal_and_decimal_agree() {
        let a = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();
        assert!(approx(a.ra().degrees(), 10.6846, 1e-3));
        assert!(approx(a.dec().degrees(), 41.2692, 1e-3));
        let b = Equatorial::parse_j2000("10.6846", "41.2692").unwrap();
        assert!(separation(a, b).arcseconds() < 1.0);
    }

    #[test]
    fn parse_ra_is_hours_dec_is_degrees() {
        // RA "06:00:00" = 6h = 90°; Dec "06:00:00" = 6°.
        let p = Equatorial::parse_j2000("06:00:00", "06:00:00").unwrap();
        assert!(approx(p.ra().degrees(), 90.0, 1e-9));
        assert!(approx(p.dec().degrees(), 6.0, 1e-9));
    }

    #[test]
    fn parse_space_separated_and_negative_dec() {
        let p = Equatorial::parse_j2000("05 35 17", "-05 23 28").unwrap();
        assert!(approx(p.ra().degrees(), 83.821, 1e-2));
        assert!(approx(p.dec().degrees(), -5.391, 1e-2));
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(matches!(
            Equatorial::parse_j2000("", "0").unwrap_err(),
            Error::ParseCoord(_)
        ));
        assert!(matches!(
            Equatorial::parse_j2000("00:70:00", "0").unwrap_err(),
            Error::ParseCoord(_)
        ));
        assert!(matches!(
            Equatorial::parse_j2000("abc", "0").unwrap_err(),
            Error::ParseCoord(_)
        ));
    }

    // ── Formatting / round-trip ──
    #[test]
    fn sexagesimal_round_trip() {
        let p = Equatorial::parse_j2000("00:42:44.300", "+41:16:09.00").unwrap();
        let ra = p.ra_to_sexagesimal(3);
        let dec = p.dec_to_sexagesimal(2);
        let q = Equatorial::parse_j2000(&ra, &dec).unwrap();
        assert!(separation(p, q).arcseconds() < 1e-3, "ra={ra} dec={dec}");
    }

    #[test]
    fn dec_formats_with_sign() {
        let p = Equatorial::j2000(Angle::from_degrees(0.0), Angle::from_degrees(-5.5)).unwrap();
        assert!(p.dec_to_sexagesimal(0).starts_with("-05:"));
        let q = Equatorial::j2000(Angle::from_degrees(0.0), Angle::from_degrees(5.5)).unwrap();
        assert!(q.dec_to_sexagesimal(0).starts_with("+05:"));
    }

    // ── Separation ──
    #[test]
    fn separation_known_cases() {
        let m31 =
            Equatorial::j2000(Angle::from_degrees(10.6847), Angle::from_degrees(41.2688)).unwrap();
        assert!(separation(m31, m31).arcseconds() < 1e-6);
        let a = Equatorial::j2000(Angle::from_degrees(100.0), Angle::from_degrees(0.0)).unwrap();
        let b = Equatorial::j2000(Angle::from_degrees(101.0), Angle::from_degrees(0.0)).unwrap();
        assert!(approx(separation(a, b).degrees(), 1.0, 1e-9));
        // RA compression at dec 60°.
        let c = Equatorial::j2000(Angle::from_degrees(100.0), Angle::from_degrees(60.0)).unwrap();
        let d = Equatorial::j2000(Angle::from_degrees(101.0), Angle::from_degrees(60.0)).unwrap();
        assert!(approx(separation(c, d).degrees(), 0.5, 1e-3));
        // M31 ↔ M110 ≈ 0.62°.
        let m110 =
            Equatorial::j2000(Angle::from_degrees(10.0921), Angle::from_degrees(41.6853)).unwrap();
        assert!((0.4..0.9).contains(&separation(m31, m110).degrees()));
    }

    #[test]
    fn separation_symmetric_and_antipodal() {
        let a = Equatorial::j2000(Angle::from_degrees(0.0), Angle::from_degrees(0.0)).unwrap();
        let b = Equatorial::j2000(Angle::from_degrees(180.0), Angle::from_degrees(0.0)).unwrap();
        assert!(approx(separation(a, b).degrees(), 180.0, 1e-6));
        assert!(approx(
            separation(a, b).degrees(),
            separation(b, a).degrees(),
            1e-12
        ));
    }

    // ── Precession ──
    #[test]
    fn precession_identity_on_same_epoch() {
        let p = Equatorial::j2000(Angle::from_degrees(45.0), Angle::from_degrees(20.0)).unwrap();
        assert_eq!(precess(p, Epoch::J2000), p);
    }

    #[test]
    fn precession_round_trip() {
        let p = Equatorial::j2000(Angle::from_degrees(45.0), Angle::from_degrees(20.0)).unwrap();
        let to_date = precess(p, Epoch::OfDate(2050.0));
        assert_eq!(to_date.epoch(), Epoch::OfDate(2050.0));
        let back = precess(to_date, Epoch::J2000);
        assert!(separation(p, back).arcseconds() < 1e-6, "round-trip drift");
    }

    #[test]
    fn precession_rate_matches_iau() {
        // At (α=0, δ=0), precessing J2000 → +1 century shifts Dec by ≈ θ (2004.31").
        let p = Equatorial::j2000(Angle::from_degrees(0.0), Angle::from_degrees(0.0)).unwrap();
        let d = precess(p, Epoch::OfDate(2100.0));
        assert!(
            approx(d.dec().arcseconds(), 2004.31, 2.0),
            "dec shift {}",
            d.dec().arcseconds()
        );
        // The shift for 26 years is tens of arcminutes (non-negligible for JNow).
        let d26 = precess(p, Epoch::OfDate(2026.0));
        let shift = separation(p, d26).arcminutes();
        assert!((5.0..30.0).contains(&shift), "26yr shift {shift} arcmin");
    }
}
