# Feature Specification: target-match Core Engine

**Feature Branch**: `001-target-match-core`

**Created**: 2026-07-11

**Status**: Draft

**Input**: User description: "target-match core engine — epoch-aware coordinates with sexagesimal + precession, field-of-view geometry, and coordinate-only catalogue matching."

## Overview

`target-match` is a pure-Rust, catalogue-agnostic library that answers one question:
**given where a telescope pointed and how much sky the frame covered, which catalogued
sky object(s) did the frame capture?** It matches **by sky position only, never by name**.
The consuming developer is the user; the product is the library's API contract and the
correctness of its geometry. The crate owns no catalogue data and performs no I/O —
candidates are supplied by the caller.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Identify the object a frame captured (Priority: P1)

An integrator has a light frame's pointing (RA/Dec) and its optics, plus a list of
catalogue objects. They want the single nearest catalogued object that falls within the
frame's field, so their app can label the frame's target automatically.

**Why this priority**: This is the headline capability and the reason the crate exists;
everything else supports or refines it. It alone delivers a usable MVP.

**Independent Test**: With a pointing at M31's coordinates, a representative sensor/scope,
and a small catalogue containing M31 and neighbours, confirm the nearest-object query
returns M31 first with a near-zero separation, and excludes objects outside the field.

**Acceptance Scenarios**:

1. **Given** a pointing at (RA 10.6847°, Dec 41.2688°), a catalogue containing M31 at
   those coordinates and M33 ~14.7° away, and a ~1° field, **When** the nearest-object
   query runs, **Then** M31 is returned with separation ≈ 0 and M33 is not returned.
2. **Given** a catalogue entry whose *name* is "M 31" but whose *coordinates* are on the
   far side of the sky, **When** any query runs at M31's pointing, **Then** that entry is
   NOT returned — only coordinate proximity decides membership.
3. **Given** a pointing with no catalogue object inside the field, **When** the
   nearest-object query runs, **Then** no match is returned (empty result, not an error).

---

### User Story 2 - Flexible coordinate & field-of-view input (Priority: P1)

An integrator's coordinates arrive as decimal degrees from one source and as sexagesimal
strings (`HH:MM:SS` / `±DD:MM:SS`) from another; their field of view is sometimes known
from full optics and sometimes only as an angular size. They want one consistent way to
express a pointing and a field regardless of source, and to render coordinates back as
sexagesimal for display.

**Why this priority**: Real capture metadata is heterogeneous; without flexible input the
P1 identification cannot be fed reliably. Parsing and FOV derivation are prerequisites.

**Independent Test**: Parse the same sky position from decimal degrees and from
`00:42:44.3 +41:16:09`, confirm they agree; derive a field from full optics and confirm
the pixel scale and FOV match hand-computed values; format a position back to sexagesimal
and confirm it round-trips.

**Acceptance Scenarios**:

1. **Given** RA `"00:42:44.3"` and Dec `"+41:16:09"`, **When** parsed, **Then** the result
   equals the decimal position (10.6846°, 41.2692°) within 1 milliarcsecond, and RA is
   multiplied by 15 (hours→degrees).
2. **Given** focal length 800 mm, pixel size 3.76 µm, binning 1×1, sensor 6248×4176,
   **When** the field is derived, **Then** the pixel scale is ≈ 0.969″/px and the field is
   ≈ 1.68° × 1.12°.
3. **Given** a field expressed directly as 1.68° × 1.12° (no optics), **When** it is used,
   **Then** membership and radius behave identically to the optics-derived field.
4. **Given** a decimal position, **When** formatted to sexagesimal and parsed back, **Then**
   the round-trip agrees within 1 milliarcsecond.

---

### User Story 3 - Precise rectangular in-frame membership, with and without rotation (Priority: P2)

An integrator wants to know not merely "within a search circle" but "actually inside the
rectangular frame that was captured" — including when the camera was rotated to a position
angle.

**Why this priority**: The circular radius is sufficient for a first identification, but
edge objects and framing questions need the true rectangle; rotation reflects real setups.

**Independent Test**: Place objects just inside and just outside a rectangular frame's
edges and corners, and repeat with the frame rotated by a known position angle; confirm
membership matches the geometry in every case.

**Acceptance Scenarios**:

1. **Given** an axis-aligned 1.68°×1.12° frame, **When** membership is tested, **Then** an
   object inside the rectangle is in-frame and one outside it (but inside the circumscribed
   circle) is not.
2. **Given** the same frame rotated 90° (position angle), **When** membership is tested,
   **Then** the in/out results swap for objects near the long/short edges consistent with
   the rotation.
3. **Given** any rectangular query, **When** it runs, **Then** it first excludes objects
   outside the circumscribed circle (pre-filter) and produces the same result as testing
   the rectangle directly.

---

### User Story 4 - JNow pointing support via precession (Priority: P2)

An integrator's mount reports pointing in epoch-of-date (JNow) coordinates, while the
catalogue is J2000. They want correct matches without normalizing coordinates themselves.

**Why this priority**: JNow mounts are common; matching a JNow pointing against a J2000
catalogue without conversion introduces tens-of-arcminutes error near the frame edges.

**Independent Test**: Take a J2000 position, express the same pointing as JNow for a given
date, run identification against a J2000 catalogue, and confirm the same object is matched
as when the pointing is supplied in J2000.

**Acceptance Scenarios**:

1. **Given** a pointing tagged JNow with an observation date, **When** a query runs, **Then**
   the pointing is precessed to J2000 before matching and the correct J2000 object is found.
2. **Given** a pointing tagged JNow but **without** an observation date, **When** a query
   runs, **Then** a typed error is returned explaining the missing date (matching cannot be
   done correctly).
3. **Given** a J2000-to-date precession of a known star, **When** compared to a reference
   ephemeris over 1900–2100, **Then** the result agrees within 1 arcsecond.

---

### User Story 5 - All-within-field, nearest-N, and is-framed queries (Priority: P2)

Beyond "the nearest one", an integrator wants: every catalogued object on the frame
(ranked), the top-N nearest candidates, and a direct yes/no for whether a specific known
object is in a given frame.

**Why this priority**: Different app surfaces need different shapes (a labelled list, a
short candidate menu, a verification check); these are refinements over the P1 nearest-one.

**Independent Test**: For a frame containing a close pair (e.g. M31 + M110), confirm
all-within-field returns both ranked nearest-first; nearest-N with N=1 returns only the
nearest; is-framed on a chosen object returns membership plus its geometry.

**Acceptance Scenarios**:

1. **Given** a frame covering M31 and M110, **When** all-within-field runs, **Then** both
   are returned ordered by ascending separation (M31 before M110).
2. **Given** nearest-N with N=3 against a catalogue of 4 objects, **When** it runs, **Then**
   exactly the 3 nearest by separation are returned in order, honouring any max-radius bound.
3. **Given** a specific object and a frame, **When** is-framed runs, **Then** it returns
   whether the object is inside the active membership shape plus its separation, offset, and
   position angle.

---

### User Story 6 - Prebuilt indexed matcher for batch identification (Priority: P3)

An integrator identifies the target for every sub in a night's session — hundreds or
thousands of frames — against one catalogue. They want to build the catalogue index once
and query it repeatedly without rescanning everything each time.

**Why this priority**: A performance/ergonomics refinement for the batch case; the stateless
scan already produces correct answers, so this is lowest priority.

**Independent Test**: Build a matcher from a catalogue, run many pointings through it, and
confirm each result is identical to the stateless scan for the same pointing and constraint.

**Acceptance Scenarios**:

1. **Given** a catalogue and a set of pointings, **When** queried through the prebuilt
   matcher, **Then** every result is identical (objects, order, geometry) to the stateless
   `rank` for the same inputs.
2. **Given** a pointing near a pole and one near the equator, **When** queried through the
   prebuilt matcher, **Then** the declination-band pre-filter never drops an object that the
   stateless scan would have matched.

---

### Edge Cases

- **Out-of-range coordinates**: RA outside [0, 360) or Dec outside [-90, 90] → typed error.
- **Malformed sexagesimal**: empty, non-numeric, negative minutes/seconds → typed error.
- **RA wrap-around**: pointing near RA 0°/360° with a catalogue object across the seam is
  matched by true angular separation (no false miss).
- **Poles**: pointing at/near Dec ±90° — separation and rectangle membership remain correct.
- **Insufficient optics**: missing/zero focal length or pixel size → no derived FOV → the
  configurable fixed fallback radius governs the circular query; rectangle modes that require
  a real FOV report a typed error.
- **Non-positive / non-finite optics** (0, negative, NaN, ∞) → typed error.
- **Empty catalogue** → empty result, not an error.
- **Zero / negative / non-finite radius** → empty result (nothing can be within it).
- **Ties**: two objects at identical separation → deterministic order by stable input index.
- **Epoch mismatch**: JNow pointing without a date, or a catalogue object not in J2000 →
  typed error rather than a silent wrong match.

## Requirements *(mandatory)*

### Functional Requirements

**Coordinates & angles (angle)**

- **FR-A1**: The system MUST represent an angle with construction and read-out in degrees,
  radians, arcminutes, arcseconds, and hours, and MUST normalize to a canonical range on
  request (e.g. RA into [0, 360)).
- **FR-A2**: The system MUST represent an equatorial sky position (RA, Dec) tagged with an
  epoch that is either J2000 or epoch-of-date carrying a concrete observation date.
- **FR-A3**: The system MUST parse right ascension from sexagesimal (`HH:MM:SS(.s)` or
  `HH MM SS`) and from decimal degrees, and declination from `±DD:MM:SS(.s)` / `±DD MM SS`
  and decimal degrees, using ±(|D| + M/60 + S/3600) with RA multiplied by 15.
- **FR-A4**: The system MUST format an equatorial position back to sexagesimal RA/Dec
  strings and to decimal degrees, round-tripping within 1 milliarcsecond for valid inputs.
- **FR-A5**: The system MUST compute great-circle angular separation between two positions
  using a numerically stable method, returning a value in [0, 180]°, symmetric in its
  arguments and accurate for very small separations.
- **FR-A6**: The system MUST precess an equatorial position between epoch-of-date and J2000
  using standard precession, accurate to ≤ 1 arcsecond over 1900–2100. Apparent-place
  corrections (nutation, aberration, proper motion) are explicitly out of scope.
- **FR-A7**: The system MUST validate coordinate domains (RA [0, 360), Dec [-90, 90]) and
  report out-of-range inputs as typed errors.

**Field of view (optics)**

- **FR-O1**: The system MUST derive pixel scale (arcsec/px) and field-of-view width, height,
  and diagonal from focal length, per-axis pixel size, per-axis binning, and per-axis sensor
  pixel counts.
- **FR-O2**: The system MUST accept a directly supplied pixel scale (per-axis) plus sensor
  pixel counts as an alternative way to define the field.
- **FR-O3**: The system MUST accept a directly supplied field of view (width° × height°) as
  an alternative way to define the field, with no optics required.
- **FR-O4**: Field derivation MUST be binning-aware (effective pixel size = pixel size ×
  binning, per axis) and axis-independent (x and y handled separately).
- **FR-O5**: The system MUST compute a search radius under a selectable policy: circumscribed
  (half the diagonal, the default), inscribed (half the shorter side), a multiplier of a base
  radius, or an explicit radius.
- **FR-O6**: When optics are insufficient to derive a field, the system MUST use a
  configurable fixed fallback radius (default 5°). The pixel-scale constant (206.265) and the
  arcseconds-per-degree constant (3600) MUST be exposed, not hidden.
- **FR-O7**: The system MUST reject non-positive or non-finite optics inputs with typed errors.

**Matching (matcher)**

- **FR-M1**: The system MUST accept caller objects through a trait that exposes a J2000 sky
  position only. The system MUST NOT read any name/designation to decide a match.
- **FR-M2**: The system MUST support membership modes: circular (pure angular distance ≤
  radius), axis-aligned rectangle (width×height), and rotated rectangle (width×height plus a
  camera position angle, degrees East of North). Rectangle modes MUST use the circumscribed
  circle as a pre-filter and produce the same result as testing the rectangle directly.
- **FR-M3**: The system MUST provide an all-within-field query returning every object inside
  the active membership shape, ranked ascending by angular separation.
- **FR-M4**: The system MUST provide a nearest-one query returning the single nearest in-frame
  object (or none).
- **FR-M5**: The system MUST provide a nearest-N query returning the N nearest objects by
  separation, optionally bounded by a maximum radius.
- **FR-M6**: The system MUST provide an is-framed query returning, for one object and a frame,
  its membership plus geometry.
- **FR-M7**: Each match MUST carry a reference to the caller's object, the angular separation,
  an in-frame flag for the active membership shape, an offset (sky-tangent Δ in degrees always;
  frame-aligned x/y and pixel offsets additionally when frame orientation and plate scale are
  known), and a position angle (degrees East of North) from frame centre to the object.
- **FR-M8**: Ranking MUST be deterministic — ascending by separation with ties broken by
  stable input order — so identical inputs yield identical output ordering across runs.
- **FR-M9**: The system MUST provide a stateless ranking function that takes a pointing, a
  slice of objects, and a constraint, performing a bounded linear scan.
- **FR-M10**: The system MUST provide a prebuilt matcher, constructed once from a set of
  objects, that answers repeated queries using a declination-band pre-filter, returning
  results identical to the stateless function for the same inputs (an optimization only, never
  a semantic change).
- **FR-M11**: A pointing not already in J2000 MUST be precessed to J2000 before matching; a
  pointing whose epoch cannot be reconciled (e.g. JNow without a date) MUST yield a typed error.

**Cross-cutting**

- **FR-X1**: The system MUST perform no I/O, hold no catalogue data, and make no network
  access; it operates purely over caller-supplied inputs.
- **FR-X2**: The system MUST expose a typed public error covering parse failures, out-of-range
  coordinates, invalid/insufficient optics, and epoch/date errors.
- **FR-X3**: The system MUST offer optional (de)serialization of its public data types, off by
  default so consumers who do not need it incur no cost.
- **FR-X4**: All computed positions and geometry MUST be planning-grade (≈ 1 arcminute), not
  pointing-grade; the system MUST NOT claim or attempt pointing-grade precision.
- **FR-X5**: A caller object or a result MAY carry a name/designation for display, but that
  value MUST NOT influence matching in any code path.

### Key Entities

- **Angle**: a single angular quantity, expressible/readable in degrees, radians, arcminutes,
  arcseconds, or hours.
- **Equatorial position**: an RA/Dec pair tagged with an **Epoch** (J2000 or epoch-of-date +
  observation date).
- **Field**: the angular extent of a frame — width, height, diagonal, and (when known) pixel
  scale — derived from optics, a pixel scale, or a direct FOV.
- **Radius policy**: how a search radius is chosen from a field (circumscribed, inscribed,
  multiplier, explicit) plus the fixed fallback.
- **Constraint**: the membership shape (circular / rectangle / rotated rectangle) combined with
  the query mode (all-within / nearest-one / nearest-N / is-framed).
- **SkyObject**: the caller-implemented input contract exposing a J2000 position (and only that).
- **Match**: a result binding a caller object to its computed geometry (separation, in-frame,
  offset, position angle).
- **Matcher**: a prebuilt, queryable index over a set of objects.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a representative set of frames each pointed at a known object with real
  optics, the nearest-object query returns that object as rank 1 in 100% of cases.
- **SC-002**: Angular separation agrees with an independent reference to ≤ 1 arcsecond across
  separations up to 180°, and to ≤ 1e-6° for separations under 2°.
- **SC-003**: Sexagesimal parsing accepts colon- and space-separated forms, fractional seconds,
  and signs; parse↔format round-trips within 1 milliarcsecond for representative inputs.
- **SC-004**: Precession between J2000 and epoch-of-date agrees with a reference ephemeris to
  ≤ 1 arcsecond over 1900–2100.
- **SC-005**: Pixel scale and field of view derived from optics match hand-computed values to
  ≤ 0.1% (e.g. 3.76 µm on 800 mm → ≈ 0.969″/px; 6248×4176 → ≈ 1.68° × 1.12°).
- **SC-006**: Rectangle membership classifies objects at frame edges and corners correctly for
  axis-aligned and rotated frames in 100% of a geometric test set.
- **SC-007**: For ≥ 1000 randomized cases, the prebuilt matcher returns results identical
  (objects, order, and geometry) to the stateless ranking for the same inputs.
- **SC-008**: Ranking is deterministic — identical inputs produce identical output order across
  repeated runs and across the stateless and prebuilt paths.
- **SC-009**: Field derivation is binning-aware — doubling binning doubles pixel scale and
  field extents within floating-point tolerance.
- **SC-010**: No public API performs I/O or references a catalogue, and matching never reads a
  name field — proven by a test in which a mislabelled object still matches purely by position.
- **SC-011**: The test suite passes, lint is clean under warnings-as-errors, documentation
  builds without warnings, and every public item is documented.

## Assumptions

- Catalogue objects are expressed in J2000/ICRS (SIMBAD and most FITS convention); the input
  trait treats supplied positions as J2000.
- Consumers own their catalogue and supply candidate objects; the crate never fetches or stores
  them.
- Planning-grade tolerance (≈ 1 arcminute) is acceptable; pointing-grade corrections are not
  required by any consumer of v1.
- Position angle follows the standard astronomical convention: degrees measured East of North.
- "JNow" means the mean equinox of date (precession only); nutation and apparent-place terms
  are excluded by design.

## Out of Scope

- Catalogue data, catalogue storage, and name resolution (SIMBAD/Sesame) — these belong in a
  sibling resolver crate, not here.
- Plate-solving (deriving coordinates from image pixels).
- Full apparent-place astrometry (nutation, annual aberration, light deflection, proper motion,
  parallax).
- Extended-object angular size with fit/overlap/coverage tests (objects are points in v1) —
  recorded as a planned future feature.
- Promoting the angle module into a shared standalone crate — a future refactor, not v1.
