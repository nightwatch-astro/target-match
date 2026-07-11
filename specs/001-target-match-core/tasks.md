# Tasks: target-match Core Engine

**Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Branch**: `001-target-match-core`

Bottom-up module order (error → angle → optics → matcher → re-exports → integration), phased
by user story so each phase is an independently testable increment. `[P]` = parallelizable
(distinct files, no incomplete deps). File paths are repo-relative.

MVP = Phase 1–4 (US1 + US2): identify the captured target from decimal/sexagesimal coordinates
and optics, using the circular radius.

---

## Phase 1: Setup

- [x] T001 Add dependencies to `Cargo.toml`: `thiserror` (runtime), `serde` (optional, feature `serde`), `proptest` (dev). Update the `serde` feature to `["dep:serde"]`. Keep lints (`missing_docs=warn`, `unsafe_code=forbid`, `clippy::all=warn`).
- [x] T002 Replace the stub `src/lib.rs` module list with `error`, `angle`, `optics`, `matcher` and a curated re-export block (fill in as modules land). Keep crate-level docs.

## Phase 2: Foundational (blocking prerequisites)

- [x] T003 Implement `src/error.rs`: `Error` enum via `thiserror` (`ParseCoord`, `OutOfRange { what, value }`, `InvalidOptics`, `MissingObservationDate`, `EpochMismatch`) + `pub type Result<T>`. Unit test each `Display`. (FR-X2)
- [x] T004 Create `src/angle.rs` `Angle` type: private `radians: f64`; constructors `from_radians/degrees/arcminutes/arcseconds/hours`; accessors; `normalized_0_360`/`normalized_pm_180`; `Add/Sub/Neg/Mul<f64>`; derive `Debug,Clone,Copy,PartialEq` (+ optional serde). Unit tests for each conversion + normalization. (FR-A1)
- [x] T005 Add `Epoch` to `src/angle.rs`: `J2000 | OfDate(f64)`; `julian_centuries_from_j2000()`. Unit tests. (FR-A2)

## Phase 3: US1 — Identify the captured target (Priority: P1) 🎯 MVP

**Goal**: nearest catalogued object a frame captured, from pointing + optics, circular radius.
**Independent test**: pointing at M31 + small catalogue + ~1° field → M31 rank 1, M33 excluded.

- [x] T006 [US1] Add `Equatorial` to `src/angle.rs`: fields `ra,dec: Angle`, `epoch: Epoch`; `new` (validates RA∈[0,360), Dec∈[-90,90] → `OutOfRange`), `j2000`, accessors, `to_degrees`. Unit + range-error tests. (FR-A2, FR-A7)
- [x] T007 [US1] Implement `separation(a,b)->Angle` (haversine, clamp, [0,180]°) in `src/angle.rs`. Unit tests: self=0, symmetric, 1° equator, RA-compression at dec 60°, M31↔M110≈0.62°, antipodal=180°. (FR-A5, SC-002)
- [x] T008 [US1] Implement `src/optics.rs` `Optics` struct + `Field` with `from_optics` (binning-aware, exact `ARCSEC_PER_RADIAN`), `width/height/diagonal`, `pixel_scale`; consts `ARCSEC_PER_RADIAN`, `ARCSEC_PER_DEGREE`, `DEFAULT_FALLBACK_RADIUS`. Reject non-positive/non-finite → `InvalidOptics`. Unit test ASI2600/800mm ≈0.969"/px, 1.68°×1.12°. (FR-O1, FR-O4, FR-O6, FR-O7, SC-005)
- [x] T009 [US1] Add `RadiusPolicy` + `Field::radius(policy)` (Circumscribed=½diag default, Inscribed=½min, Multiplier, Explicit) in `src/optics.rs`. Unit tests each policy. (FR-O5)
- [x] T010 [US1] Create `src/matcher.rs`: `SkyObject` trait (`position()->Equatorial`), `Membership::Circular`, `Query` enum, `Constraint` (+ `within(field,policy)`, `.nearest_one()`, `.all()`), `Offset`, `Match<'a,T>`. (FR-M1, FR-M2)
- [x] T011 [US1] Implement `rank(pointing,&[T],constraint)` in `src/matcher.rs` for Circular membership + AllWithinField/NearestOne: separation filter, ascending sort, tie-break by input index; compute `separation`, `in_frame`, `offset.sky`, `position_angle`. Deterministic. Unit tests. (FR-M3, FR-M4, FR-M7, FR-M8)
- [x] T012 [US1] Wire crate re-exports for the US1 surface in `src/lib.rs`; add `tests/known_values.rs` M31/M110/M33 nearest-one + exclusion test. (SC-001)

## Phase 4: US2 — Flexible coordinate & field-of-view input (Priority: P1)

**Goal**: parse decimal & sexagesimal coordinates; FOV from optics, pixel scale, or directly.
**Independent test**: decimal ≡ sexagesimal position; FOV from optics matches hand calc; format round-trips.

- [x] T013 [P] [US2] Implement sexagesimal + decimal parsing in `src/angle.rs`: `Equatorial::parse`, `parse_j2000`, internal `parse_ra`/`parse_dec` (`±(|D|+M/60+S/3600)`, RA×15; colon/space; fractional seconds; bare decimal=degrees). Reject malformed → `ParseCoord`. Unit tests incl. edge cases. (FR-A3)
- [x] T014 [P] [US2] Implement `ra_to_sexagesimal`/`dec_to_sexagesimal(decimals)` in `src/angle.rs`; round-trippable ≤1 mas. Unit tests. (FR-A4, SC-003)
- [x] T015 [P] [US2] Add `Field::from_pixel_scale` and `Field::from_fov` to `src/optics.rs` (direct paths; `pixel_scale=None` for direct FOV). Unit tests: direct FOV behaves identically to optics-derived. (FR-O2, FR-O3)

## Phase 5: US3 — Rectangular in-frame membership, with & without rotation (Priority: P2)

**Goal**: true tangent-plane rectangle membership, axis-aligned and rotated by a position angle.
**Independent test**: objects at edges/corners classified correctly; 90° rotation flips edge cases.

- [x] T016 [US3] Add gnomonic tangent-plane projection helpers (`Equatorial`→(ξ east, η north) about a centre) in `src/angle.rs` or `src/matcher.rs`. Unit tests vs known offsets. (FR-M2)
- [x] T017 [US3] Implement `Membership::Rectangle` and `Membership::Rotated` in `src/matcher.rs`: circumscribed-circle pre-filter → project → rotate by −PA → `|x|≤fov_x/2 && |y|≤fov_y/2`. `Constraint::frame`/`frame_rotated`. (FR-M2)
- [x] T018 [US3] Populate `offset.frame` (frame-aligned x/y) and `offset.pixels` (÷ plate scale when known) and `position_angle` (deg E of N) in `Match` for rectangle modes. Unit tests. (FR-M7, R6)
- [x] T019 [US3] Add rectangle membership tests to `tests/known_values.rs`/unit: edge/corner in/out for axis-aligned and rotated frames. (SC-006)

## Phase 6: US4 — JNow pointing support via precession (Priority: P2)

**Goal**: precess JNow↔J2000 so mixed-epoch inputs match; error if a JNow pointing lacks a date.
**Independent test**: JNow-tagged pointing finds the same object as its J2000 form; missing date → error.

- [x] T020 [US4] Implement unit-vector ↔ `Equatorial` helpers + `precess(pos,to)` (IAU 1976 ζ_A,z_A,θ_A rotation matrix) in `src/angle.rs`. Identity on same epoch. (FR-A6)
- [x] T021 [US4] In `rank`/`Matcher::query`, precess a non-J2000 `pointing` to J2000 before matching; `OfDate` with no usable date is impossible by construction, but reconciliation failures return `MissingObservationDate`/`EpochMismatch`. Unit tests. (FR-M11)
- [x] T022 [US4] Precession accuracy test vs a reference position (e.g. a star's J2000→2025 shift) ≤1″; forward/back ≈ identity. (SC-004)

## Phase 7: US5 — All-within-field, nearest-N, is-framed queries (Priority: P2)

**Goal**: full query-mode set.
**Independent test**: frame with M31+M110 → all ranked; nearest-N bounds; is-framed returns geometry.

- [x] T023 [US5] Implement `Query::NearestN { n, max_radius }` and `.nearest_n(n)` in `src/matcher.rs` (top-N by separation honouring max_radius). Unit tests. (FR-M5)
- [x] T024 [US5] Implement `Matcher::is_framed(pointing,&object,membership)` (and a `rank`-side equivalent) returning membership + geometry for one object. Unit tests. (FR-M6)
- [x] T025 [US5] Tests: all-within-field ordering (M31 before M110); nearest-N counts/order/bounds. (US5 scenarios)

## Phase 8: US6 — Prebuilt indexed matcher (Priority: P3)

**Goal**: build once, query many; identical results to stateless `rank`.
**Independent test**: many pointings through the index match the stateless scan exactly.

- [x] T026 [US6] Implement `Matcher<T>::from_objects` (dec-sorted store + original indices) and `query` using the declination-band pre-filter, delegating to the shared match/rank logic. `objects()`. (FR-M10, R5)
- [x] T027 [US6] Equivalence + pole/equator band tests in `tests/properties.rs` (proptest ≥1000 cases: `Matcher::query` == `rank`). (SC-007, SC-008)

## Phase 9: Polish & cross-cutting

- [x] T028 [P] Finalize `src/lib.rs` re-exports for the full surface; ensure every public item has a doc comment (missing_docs clean). (SC-011)
- [x] T029 [P] `tests/properties.rs` proptests: separation symmetry & range; sexagesimal parse↔format round-trip; precession forward/back. (SC-002, SC-003, SC-004)
- [x] T030 [P] `tests/coordinates_only.rs`: a mislabelled-name object still matches by coordinates only. (FR-X5, SC-010)
- [x] T031 [P] `examples/identify.rs`: end-to-end usage per quickstart (may use `anyhow`). Update `README.md` planned-API block to the real API.
- [x] T032 [P] Add `serde` derives behind the feature to all public data types; confirm `--features serde` builds/tests. (FR-X3)
- [x] T033 Final gate: `just verify` green (fmt-check, clippy `-D warnings`, tests), `cargo doc --no-deps` warning-free, `cargo test --all-features`. Confirm SC-001..SC-011 covered. (SC-011)

---

## Dependencies

- Phase 1 → Phase 2 → Phase 3 (MVP core). Phases 4–8 each build on 2–3.
- US2 (T013–T015) is parallelizable within itself and independent of US3–US6.
- US3 (rectangle) is independent of US4 (precession) and US5 (query modes); US6 depends on the
  shared match logic being final (after US3/US5).
- Polish (Phase 9) after all module logic lands.

## Parallel opportunities

- T013/T014/T015 (US2) — distinct concerns in angle/optics.
- T028–T032 (polish) — distinct files.

## Story → success-criteria map

| Story | Tasks | Key SC |
|-------|-------|--------|
| US1 | T006–T012 | SC-001, SC-002, SC-005 |
| US2 | T013–T015 | SC-003, SC-005, SC-009 |
| US3 | T016–T019 | SC-006 |
| US4 | T020–T022 | SC-004 |
| US5 | T023–T025 | — |
| US6 | T026–T027 | SC-007, SC-008 |
| Polish | T028–T033 | SC-010, SC-011 |
