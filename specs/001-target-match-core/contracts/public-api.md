# Public API Contract: target-match

The library's product is this surface. Signatures are the contract; behaviour notes are
binding. All fallible calls return `target_match::Result<T>` (`Error` from `crate::error`).

## Crate root re-exports

```rust
pub use angle::{Angle, Epoch, Equatorial, separation, precess};
pub use optics::{Optics, Field, RadiusPolicy, ARCSEC_PER_RADIAN, ARCSEC_PER_DEGREE};
pub use matcher::{SkyObject, Membership, Query, Constraint, Offset, Match, rank, Matcher};
pub use error::{Error, Result};
```

## angle

```rust
impl Angle {
    pub fn from_radians(f64) -> Angle;       pub fn radians(&self) -> f64;
    pub fn from_degrees(f64) -> Angle;       pub fn degrees(&self) -> f64;
    pub fn from_arcminutes(f64) -> Angle;    pub fn arcminutes(&self) -> f64;
    pub fn from_arcseconds(f64) -> Angle;    pub fn arcseconds(&self) -> f64;
    pub fn from_hours(f64) -> Angle;         pub fn hours(&self) -> f64;
    pub fn normalized_0_360(&self) -> Angle; pub fn normalized_pm_180(&self) -> Angle;
}

pub enum Epoch { J2000, OfDate(f64) /* Julian year */ }
impl Epoch { pub fn julian_centuries_from_j2000(&self) -> f64; }

impl Equatorial {
    pub fn new(ra: Angle, dec: Angle, epoch: Epoch) -> Result<Equatorial>; // validates domain
    pub fn j2000(ra: Angle, dec: Angle) -> Result<Equatorial>;
    pub fn parse(ra: &str, dec: &str, epoch: Epoch) -> Result<Equatorial>;
    pub fn parse_j2000(ra: &str, dec: &str) -> Result<Equatorial>;
    pub fn ra(&self) -> Angle;  pub fn dec(&self) -> Angle;  pub fn epoch(&self) -> Epoch;
    pub fn to_degrees(&self) -> (f64, f64);
    pub fn ra_to_sexagesimal(&self, decimals: usize) -> String;  // "HH:MM:SS.sss"
    pub fn dec_to_sexagesimal(&self, decimals: usize) -> String; // "+DD:MM:SS.sss"
}

pub fn separation(a: Equatorial, b: Equatorial) -> Angle;      // haversine, [0,180]°, symmetric
pub fn precess(pos: Equatorial, to: Epoch) -> Result<Equatorial>; // IAU1976; needs a date if OfDate
```

**Guarantees**: `parse` accepts colon- or space-separated sexagesimal, fractional seconds, and
signs; a bare decimal RA is degrees; sexagesimal RA is hours (×15). `separation` is symmetric
and in [0,180]°. `precess` between `J2000` and `OfDate(y)` is accurate to ≤ 1″ over 1900–2100;
`precess` of a `J2000→J2000` (or same epoch) is identity. Round-trip parse↔format ≤ 1 mas.

## optics

```rust
pub struct Optics { pub focal_mm: f64, pub pixel_um: (f64,f64), pub binning: (u32,u32), pub pixels: (u32,u32) }

impl Field {
    pub fn from_optics(o: Optics) -> Result<Field>;
    pub fn from_pixel_scale(scale_arcsec_px: (f64,f64), pixels: (u32,u32)) -> Result<Field>;
    pub fn from_fov(width: Angle, height: Angle) -> Result<Field>;
    pub fn width(&self) -> Angle;  pub fn height(&self) -> Angle;  pub fn diagonal(&self) -> Angle;
    pub fn pixel_scale(&self) -> Option<(f64,f64)>;
    pub fn radius(&self, policy: RadiusPolicy) -> Angle;
}

pub enum RadiusPolicy { Circumscribed, Inscribed, Multiplier(f64), Explicit(Angle) }
pub const ARCSEC_PER_RADIAN: f64;  pub const ARCSEC_PER_DEGREE: f64;
pub const DEFAULT_FALLBACK_RADIUS: Angle; // 5°
```

**Guarantees**: `from_optics` is binning-aware (effective pixel = pixel×binning per axis) and
axis-independent; rejects non-positive/non-finite inputs with `Error::InvalidOptics`.
`radius(Circumscribed)` = ½ diagonal; `Inscribed` = ½ min(width,height). ASI2600 on 800 mm →
pixel scale ≈ 0.969″/px, field ≈ 1.68°×1.12° (± 0.1%).

## matcher

```rust
pub trait SkyObject { fn position(&self) -> Equatorial; }

pub enum Membership {
    Circular { radius: Angle },
    Rectangle { fov: (Angle, Angle) },
    Rotated { fov: (Angle, Angle), position_angle: Angle },
}
pub enum Query { AllWithinField, NearestOne, NearestN { n: usize, max_radius: Option<Angle> }, IsFramed }
pub struct Constraint { pub membership: Membership, pub query: Query }

pub struct Offset { pub sky: (Angle,Angle), pub frame: Option<(Angle,Angle)>, pub pixels: Option<(f64,f64)> }
pub struct Match<'a, T> { pub object: &'a T, pub separation: Angle, pub in_frame: bool, pub offset: Offset, pub position_angle: Angle }

pub fn rank<T: SkyObject>(pointing: Equatorial, objects: &[T], c: Constraint) -> Result<Vec<Match<'_, T>>>;

pub struct Matcher<T> { /* dec-sorted */ }
impl<T: SkyObject> Matcher<T> {
    pub fn from_objects(objects: Vec<T>) -> Matcher<T>;
    pub fn query(&self, pointing: Equatorial, c: Constraint) -> Result<Vec<Match<'_, T>>>;
    pub fn objects(&self) -> &[T];
    pub fn is_framed(&self, pointing: Equatorial, object: &T, m: Membership) -> Result<Match<'_, T>>;
}
```

**Guarantees**:
- Matching reads `position()` only — never a name (FR-X5). A pointing not in J2000 is precessed
  to J2000 first; JNow without a date → `Error::MissingObservationDate`.
- `Membership::Rectangle`/`Rotated` test true tangent-plane membership; the circumscribed circle
  pre-filters. `AllWithinField` returns every in-frame object ranked ascending by separation;
  `NearestOne` the nearest; `NearestN` the N nearest (honouring `max_radius`).
- Ranking is deterministic (separation, then input index). `Matcher::query` returns results
  identical to `rank` for the same objects/pointing/constraint (dec-band is optimization only).
- `Match::in_frame` reflects the active membership shape; `offset.frame`/`offset.pixels` are
  `Some` only when orientation (rectangle/rotated) and plate scale are known; `position_angle`
  is degrees East of North.
