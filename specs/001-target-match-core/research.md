# Phase 0 Research: target-match Core Engine

Technical decisions resolved before design. Format: Decision / Rationale / Alternatives.

## R1 — Precession model (JNow ↔ J2000)

**Decision**: Implement IAU 1976 precession (Meeus, *Astronomical Algorithms*, ch. 21) as a
rotation applied to the position's unit vector, using the accumulated precession angles
ζ_A, z_A, θ_A as polynomials in Julian centuries from J2000. Reduce equatorial → unit vector,
rotate, convert back.

**Rationale**: Standard, well-documented, pure arithmetic (no deps), and accurate to ≤ ~1
arcsecond over several centuries — far inside the crate's planning-grade (~1 arcmin) target.
Rotation-matrix form is numerically stable at the poles (unlike the direct spherical
formulae). Matches FR-A6 / SC-004.

**Alternatives considered**:
- *IAU 2006 (Capitaine) precession*: sub-milliarcsec, more coefficients — precision we do not
  need; rejected as overkill.
- *External astronomy crate* (e.g. `astro`, ERFA bindings): heavier, some unmaintained or C
  FFI (breaks MSVC-safe/pure identity); rejected.
- *Simple per-year linear RA/Dec drift*: cheap but loses accuracy and misbehaves near the
  poles; rejected.

## R2 — Epoch representation

**Decision**: `Epoch` is `J2000` or `OfDate(JulianYear)` where `JulianYear` is a plain `f64`
Julian (Besselian-free) epoch, e.g. `2026.53`. Precession uses `T = (year − 2000) / 100`
Julian centuries.

**Rationale**: Avoids a date/time dependency in a pure-math crate while fully supporting JNow.
Consumers convert their observation instant (e.g. FITS `DATE-OBS`) to a Julian year; the docs
show the trivial conversion and note that day-level precision in the epoch is far finer than
needed (precession moves ~arcsec/decade). Keeps the type `Copy` and `serde`-trivial.

**Alternatives considered**:
- *Depend on `time`/`hifitime` for a real datetime epoch*: heavier and unnecessary at
  planning grade; deferred (could be a future feature-gated convenience).
- *Store a raw Julian Date*: more precise than needed and less legible than a Julian year.

## R3 — Rectangle membership via gnomonic projection

**Decision**: For rectangle / rotated-rectangle membership, project the object into the
tangent (gnomonic) plane centred on the pointing to get standard coordinates (ξ east, η
north) in angle units; for the rotated case, rotate (ξ, η) by −position_angle; test
`|x| ≤ fov_x/2 && |y| ≤ fov_y/2`. A circumscribed-circle test (½ diagonal) runs first as a
cheap reject.

**Rationale**: A camera frame *is* a tangent-plane rectangle; gnomonic projection is the
correct, standard mapping for the small fields involved (≤ a few degrees), well within
planning grade. The circle pre-filter guarantees the rectangle test only runs on plausible
candidates and lets rectangle and circular modes share the separation computation.

**Alternatives considered**:
- *Spherical-polygon point-in-quadrilateral*: exact on the sphere but unnecessary for these
  small FOVs and much more code; rejected.
- *Plain ΔRA/ΔDec box (no cos δ, no projection)*: wrong away from the equator and under
  rotation; rejected.

## R4 — Pixel-scale constant

**Decision**: Use the exact `ARCSEC_PER_RADIAN = 206_264.806_247_096_36` and derive
`arcsec/px = pixel_size_µm / 1000 / focal_mm × ARCSEC_PER_RADIAN` (equivalently
`206.264806 × pixel_µm / focal_mm`). Expose `ARCSEC_PER_RADIAN` and `ARCSEC_PER_DEGREE = 3600`
as public constants.

**Rationale**: The upstream `alm` code used the rounded `206.265`; the exact value is free and
removes a ~1 ppm systematic. Exposing the constants satisfies FR-O6 ("not hidden") and lets
consumers audit the geometry.

**Alternatives considered**: keep `206.265` for parity — rejected (needless imprecision).

## R5 — Declination-band pre-filter (prebuilt Matcher)

**Decision**: `Matcher::from_objects` stores objects sorted by declination. A bounded query
with search radius `r` about a pointing at declination `δ₀` only scans objects with
`dec ∈ [δ₀ − r, δ₀ + r]` (found by binary search), then applies the full separation /
membership test.

**Rationale**: Angular separation ≥ |Δδ| always, so any true match has `|dec − δ₀| ≤ r`; the
band therefore never drops a real match — the index is a pure optimization (FR-M10, SC-007).
Declination has no wrap-around, so the band is a simple contiguous range. RA wrap is handled by
the exact separation test on the surviving candidates.

**Alternatives considered**:
- *k-d tree / HEALPix / spatial hash*: more scalable but far more code and dependency weight
  for catalogues of ~10^4; the dec band captures most of the win at trivial cost. Deferred.
- *No index (always linear)*: fine for one-off queries but wasteful for batch; hence the
  prebuilt matcher exists.

## R6 — Offset and position-angle conventions

**Decision**: Position angle is measured **degrees East of North** from frame centre to the
object: `PA = atan2( cos δ₂ · sin Δα , cos δ₁ · sin δ₂ − sin δ₁ · cos δ₂ · cos Δα )`,
normalized to [0, 360). The sky-tangent offset is `(ξ, η)` with ξ toward East
(`≈ Δα · cos δ`) and η toward North (`≈ Δδ`), reported in degrees. When frame orientation
(a rectangle/rotated membership) and plate scale are known, additionally report frame-aligned
offset = `(ξ, η)` rotated by −position_angle, in degrees and in pixels (÷ plate scale).

**Rationale**: Matches the standard astronomical PA convention and FR-M7. Frame-aligned/pixel
offsets are only meaningful when orientation + scale exist, so they are optional.

**Alternatives considered**: PA North-through-West or image x/y only — rejected as
non-standard / less useful.

## R7 — Determinism & tie-breaking

**Decision**: Rank ascending by separation (`f64` compared with a total order that treats
equal/NaN deterministically); break ties by original input index (stable). The prebuilt
matcher reconstructs the same order by carrying original indices through the dec-sorted store.

**Rationale**: FR-M8 / SC-008 require reproducibility across runs and across the stateless and
prebuilt paths. Input-index tie-breaking needs no name/id (consistent with coordinates-only).
