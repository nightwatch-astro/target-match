# Handover — target-match v0.1 (feature 001-target-match-core)

**Date**: 2026-07-11 · **Repo**: [github.com/nightwatch-astro/target-match](https://github.com/nightwatch-astro/target-match) (public)
· **Branch**: `001-target-match-core` → merged to `main`

## What this is

A pure-Rust, catalogue-agnostic library that answers **"which catalogued sky object did this
frame capture?"** from a telescope pointing + field of view, matching **by sky position only,
never by name**. Extracted from the `nightwatch-astro/alm` targeting pipeline; the sibling
`simbad-resolver` (name→identity + catalogue) is the *upstream* producer of objects — this crate
is the downstream matcher and does **no I/O and owns no catalogue data**.

## Status: complete and green

- `just verify` — **fmt-check + clippy `-D warnings` + tests all pass**; `cargo doc` warning-free;
  every public item documented.
- **43 tests + 1 doctest** across 6 suites (unit, known-values, proptest properties,
  coordinates-only, serde feature, doctest). All 11 success criteria (SC-001…SC-011) are covered.
- CI (GitHub Actions) runs fmt·clippy·test·doc on Linux/Windows/macOS.

## The crate

Single crate, three modules (+ `error`), re-exported from the root:

- **`angle`** — `Angle` (deg/rad/arcmin/arcsec/hours), `Epoch` (J2000 / OfDate(Julian year)),
  `Equatorial` (decimal + sexagesimal parse *and* format), `separation` (haversine),
  `precess` (IAU-1976, JNow↔J2000).
- **`optics`** — `Optics`/`Field` (from optics [binning-aware, x/y], from pixel scale, or direct
  FOV), `RadiusPolicy` (circumscribed/inscribed/multiplier/explicit), exposed constants.
- **`matcher`** — `SkyObject` trait (position only), `Membership` (circular / rectangle /
  rotated), `Query` (all-within / nearest-one / nearest-N), `Constraint`, `Match`/`Offset`,
  stateless `rank`, prebuilt dec-band `Matcher`, `is_framed`.

```sh
cargo run --example identify   # end-to-end demo (identifies M31, lists M110 on-frame)
just verify                    # the full local gate
cargo doc --no-deps --open     # API docs
```

## SpecKit artifacts

Under `specs/001-target-match-core/`: `spec.md`, `plan.md`, `research.md`, `data-model.md`,
`contracts/public-api.md`, `quickstart.md`, `tasks.md` (all 33 tasks done),
`checklists/requirements.md`.

## Autonomous decisions & deviations (full log: `docs/DECISIONS.md`)

Highlights you may want to review:

1. **Matching & precession are infallible** — because `Epoch::OfDate` always carries its
   Julian year, "JNow without a date" is unrepresentable. So `rank`/`Matcher::query`/`precess`/
   `is_framed` return values (not `Result`), and the `Error` enum is `ParseCoord` / `OutOfRange`
   / `InvalidOptics` only. This is a **minor, strictly-safer deviation** from the spec's literal
   FR-M11 wording (which named `MissingObservationDate`/`EpochMismatch`). Spec text kept as-is;
   contract doc + DECISIONS reconciled.
2. **Public repo created** per your earlier explicit choice (sibling `fits-header` is private).
   Flip with `gh repo edit nightwatch-astro/target-match --visibility private` if desired.
3. **Exact `ARCSEC_PER_RADIAN`** (206264.806…) replaces alm's rounded `206.265`.
4. **Robust tangent-offset geometry** (separation+PA → East/North; circle pre-filters the
   rectangle) — planning-grade exact, never divides by cos(separation).
5. **Binning/FOV model**: `pixels` = the (binned) image dimensions you supply, `pixel_um` =
   physical unbinned pixel size, `binning` = factor → ×2 binning ⇒ ×2 scale ⇒ ×2 field for a
   fixed pixel count.
6. **SpecKit ceremony streamlined** for a solo autonomous run (ran specify→plan→tasks→implement→
   verify; treated the interactive clarify gate as satisfied by our grilling; did not run the
   optional taskstoissues / multi-agent review-panel steps).

## Input I'd like from you

- **Confirm public visibility** is intended (decision #2).
- **The infallible-matching refinement** (decision #1) — confirm you're happy dropping the two
  epoch error variants, or ask me to reinstate them.
- **License**: Apache-2.0-only was chosen; say if you want dual `MIT OR Apache-2.0` before any
  crates.io publish.
- **Ratify a constitution?** `.specify/memory/constitution.md` is still the template; principles
  live in `AGENTS.md`. Run `/speckit-constitution` if you want them formalised.

## Deferred / future work (recorded, not built)

- Extended-object angular **size** with fit/overlap/coverage tests (objects are points in v1).
- Full **apparent-place** astrometry (nutation/aberration/proper motion) — out of scope by design.
- Promote `angle` to a shared **`astro-angle`** crate when `fits-header`/`xisf-header` want the
  sexagesimal logic.
- Optional `framing` feature on `simbad-resolver` for a one-call "resolve + rank" convenience
  (depends on target-match; heavy→light).
- `crates.io` publish (add `LICENSE-MIT` if dual-licensing; `release-please` is wired).

## Next steps for a maintainer

1. Review the merged `main` (or the PR) and the decisions above.
2. Wire `simbad-resolver` to feed `SkyObject`s into `rank`/`Matcher` (one trivial trait impl).
3. When ready, `cargo publish` (or let `release-please` cut the first tagged release).
