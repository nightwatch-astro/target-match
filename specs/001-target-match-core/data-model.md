# Phase 1 Data Model: target-match Core Engine

All types are pure values. `#[derive(Debug, Clone, Copy, PartialEq)]` where sensible; `serde`
derives are added behind the off-by-default `serde` feature. Angles are stored in radians
internally.

## angle

### `Angle`
- **Field**: `radians: f64` (private).
- **Constructors**: `from_radians`, `from_degrees`, `from_arcminutes`, `from_arcseconds`,
  `from_hours` (×15° = ×π/12 rad).
- **Accessors**: `radians()`, `degrees()`, `arcminutes()`, `arcseconds()`, `hours()`.
- **Ops**: `normalized_0_360()` (RA into [0,360)); `normalized_pm_180()`; arithmetic via
  `Add`/`Sub`/`Neg`/`Mul<f64>`.
- **Invariants**: value may be any finite `f64`; normalization is explicit, not automatic.

### `Epoch`
- **Variants**: `J2000` | `OfDate(f64)` where the `f64` is a Julian year (e.g. `2026.53`).
- **Helper**: `julian_centuries_from_j2000() -> f64` = `(year − 2000) / 100`; `J2000` → 0.

### `Equatorial`
- **Fields**: `ra: Angle`, `dec: Angle`, `epoch: Epoch`.
- **Constructors**: `new(ra, dec, epoch)`; `j2000(ra, dec)`; `parse_j2000(ra_str, dec_str)`
  and `parse(ra_str, dec_str, epoch)` (accept sexagesimal or decimal).
- **Parsing** (`parse_ra`, `parse_dec`): RA accepts `HH:MM:SS(.s)`, `HH MM SS`, or decimal
  degrees (a bare number is treated as **degrees**, not hours — documented; sexagesimal RA is
  hours ×15); Dec accepts `±DD:MM:SS(.s)`, `±DD MM SS`, or decimal degrees. Rule
  `±(|D| + M/60 + S/3600)`. Rejects empty, non-numeric, negative minutes/seconds,
  out-of-range.
- **Formatting**: `ra_to_sexagesimal(precision)`, `dec_to_sexagesimal(precision)`,
  `to_degrees() -> (f64, f64)`.
- **Invariants**: after validation, `ra ∈ [0,360)`, `dec ∈ [−90,90]`.
- **Free functions**: `separation(a: Equatorial, b: Equatorial) -> Angle` (haversine; epoch
  is NOT auto-reconciled here — separation is geometric on the given numbers; matching handles
  precession); `precess(pos: Equatorial, to: Epoch) -> Equatorial`.

## optics

### `Optics`
- **Fields**: `focal_mm: f64`, `pixel_um: (f64, f64)` (x, y), `binning: (u32, u32)`,
  `pixels: (u32, u32)` (naxis1, naxis2).
- **Invariants**: all strictly positive & finite; else `Error::InvalidOptics`.

### `Field`
- **Fields**: `fov: (Angle, Angle)` (width x, height y); `pixel_scale: Option<(f64, f64)>`
  (arcsec/px per axis; `Some` when derived from optics or given, `None` for a direct-FOV
  field).
- **Constructors**: `from_optics(Optics) -> Result<Field>` (effective pixel = `pixel_um ×
  binning`; `scale = eff_pixel/1000/focal_mm × ARCSEC_PER_RADIAN`; `fov_axis = scale × pixels
  / 3600` deg); `from_pixel_scale(scale:(f64,f64), pixels:(u32,u32)) -> Result<Field>`;
  `from_fov(fx: Angle, fy: Angle) -> Result<Field>`.
- **Accessors**: `width()`, `height()`, `diagonal()` (Angle).
- **Radius**: `radius(policy: RadiusPolicy) -> Angle`.

### `RadiusPolicy`
- **Variants**: `Circumscribed` (½ diagonal, default), `Inscribed` (½ shorter side),
  `Multiplier(f64)` (× ½ diagonal), `Explicit(Angle)`.

### Constants
- `ARCSEC_PER_RADIAN: f64 = 206_264.806_247_096_36`
- `ARCSEC_PER_DEGREE: f64 = 3600.0`
- `DEFAULT_FALLBACK_RADIUS: Angle` = 5°.

## matcher

### `trait SkyObject`
- **Required**: `fn position(&self) -> Equatorial;` (interpreted as J2000).
- Deliberately exposes **no name/id** — matching reads position only (FR-M1 / FR-X5).

### `Membership`
- `Circular { radius: Angle }`
- `Rectangle { fov: (Angle, Angle) }` (axis-aligned)
- `Rotated { fov: (Angle, Angle), position_angle: Angle }` (deg E of N)

### `Query`
- `AllWithinField`
- `NearestOne`
- `NearestN { n: usize, max_radius: Option<Angle> }`
- `IsFramed` (with the specific object provided to the is-framed entry point)

### `Constraint`
- **Fields**: `membership: Membership`, `query: Query`.
- **Helpers**: `Constraint::within(field, policy)` → Circular from a `Field`;
  `Constraint::frame(field)` / `frame_rotated(field, pa)` → Rectangle/Rotated;
  fluent `.nearest_one()`, `.nearest_n(n)`, `.all()`.

### `Offset`
- **Fields**: `sky: (Angle, Angle)` (ξ east, η north, always present);
  `frame: Option<(Angle, Angle)>` (frame-aligned x/y, when orientation known);
  `pixels: Option<(f64, f64)>` (when orientation + plate scale known).

### `Match<'a, T>`
- **Fields**: `object: &'a T`, `separation: Angle`, `in_frame: bool`, `offset: Offset`,
  `position_angle: Angle`.

### Free function & index
- `rank<T: SkyObject>(pointing: Equatorial, objects: &[T], constraint: Constraint) ->
  Result<Vec<Match<'_, T>>>` — precess `pointing` to J2000 (error if JNow w/o date), then
  scan, filter by membership + query, and rank deterministically.
- `Matcher<T>`: `from_objects(Vec<T>) -> Matcher<T>` (dec-sorted with original indices);
  `query(pointing, constraint) -> Result<Vec<Match<'_, T>>>` (dec-band pre-filter then the
  same logic as `rank`); `objects() -> &[T]`.

## error

### `Error` (enum, `thiserror`)
- `ParseCoord(String)` — malformed sexagesimal/decimal.
- `OutOfRange { what: &'static str, value: f64 }` — RA/Dec domain.
- `InvalidOptics(String)` — non-positive/non-finite optics, or FOV required but absent.
- `MissingObservationDate` — JNow pointing without a date to precess from.
- `EpochMismatch` — an epoch that cannot be reconciled for matching.
- `type Result<T> = core::result::Result<T, Error>`.
