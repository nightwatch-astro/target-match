# Autonomous Decisions, Ambiguities & Input Needed

Running log kept during the autonomous build of `target-match`. Every non-obvious
decision made without a live confirmation is recorded here, plus ambiguities resolved and
anything that still needs the maintainer's input.

Legend: **[DECISION]** made autonomously · **[AMBIGUITY]** resolved with rationale ·
**[INPUT-NEEDED]** awaiting maintainer.

## Project setup & process

- **[DECISION] Public GitHub repo created now.** `github.com/nightwatch-astro/target-match`
  was created **public** (your earlier explicit choice) and the scaffold pushed, so I can
  push throughout. The sibling `fits-header` is private; if you'd rather this be private too,
  it flips trivially: `gh repo edit nightwatch-astro/target-match --visibility private`.
- **[DECISION] SpecKit ceremony streamlined for a solo autonomous run.** The repo's
  `50-speckit-workflow.md` describes a ~19-step flow (clarify/checklist/analyze gates,
  taskstoissues, agent-assign orchestration, multiple review/QA/security subagents). Under
  your "take to completion" mandate I run the generative + verification core
  (specify → plan → tasks → implement → test/verify) and treat the interactive clarify gate
  as satisfied by our extensive grilling. Streamlined steps are noted in the handover.
- **[DECISION] Local directory stays `astro-target-id`.** The crate and repo are
  `target-match`; the working folder name is cosmetic and left as-is to avoid churn.
- **[DECISION] License Apache-2.0 (single).** Your choice; matches `fits-header`. Revisit to
  dual `MIT OR Apache-2.0` before a crates.io publish if broader compatibility is wanted.

## Design defaults taken during grilling (recorded for the record)

- **[DECISION] Offset reporting.** Sky-tangent Δ in **degrees** is always present; frame-aligned
  x/y and **pixel** offsets are additional and only populated when frame orientation and plate
  scale are known.
- **[DECISION] `nearest-N`** = top-N by separation, optionally bounded by a max radius.
- **[DECISION] `is-framed`** = one object + a frame → membership + full geometry.
- **[DECISION] Position angle convention** = degrees **East of North** (standard astronomical).
- **[DECISION] Precession is hand-rolled** (IAU 1976/2006 / Meeus rotation-matrix), no external
  astronomy dependency; validated against reference values.
- **[DECISION] `serde` optional, off by default** even though runtime deps are now allowed —
  forcing serde on every consumer is the one thing library authors still avoid. One flag flips
  it to default-on if you prefer.
- **[DECISION] `anyhow` is dev/examples-only**, never in the public API (it would erase the
  error type callers match on). Public fallible APIs return `target_match::Error` (`thiserror`).

## Ambiguities resolved

- **[AMBIGUITY] "JNow"** interpreted as **mean equinox of date** (precession only). Apparent
  place (nutation/aberration/proper motion) is explicitly excluded — planning-grade doesn't
  need it and it would break the pure/light identity.
- **[AMBIGUITY] Catalogue object epoch.** Objects supplied via `SkyObject` are assumed **J2000**
  (SIMBAD / most FITS convention). If a consumer holds non-J2000 catalogue coordinates, they
  normalize before supplying them (or a future trait extension can carry an epoch).

## Implementation-phase decisions

- **[DECISION] Matching & precession are infallible; two error variants dropped.** Because
  `Epoch::OfDate` always carries its Julian year, "JNow without a date" is unrepresentable.
  So `precess`, `rank`, `Matcher::query`, and `is_framed` return values (not `Result`), and
  the `Error` enum is just `ParseCoord` / `OutOfRange` / `InvalidOptics` — the spec's
  `MissingObservationDate` / `EpochMismatch` are removed as unreachable. This strengthens the
  spec's intent (no silent epoch mis-match) via the type system rather than a runtime check.
  A **minor, strictly-safer deviation** from the literal spec wording (FR-M11); flagged here
  and in the implementation commit. The spec artifacts still describe the original wording;
  the contract doc will be reconciled in the polish pass.
- **[DECISION] `is_framed` is a free function** (plus a thin `Matcher` method) rather than a
  `Query::IsFramed` variant — a single-object query needs no ranking, so a dedicated entry
  point is cleaner than threading one object through `rank`.
- **[DECISION] `Constraint` carries `pixel_scale: Option<(f64,f64)>`.** Pixel offsets need the
  plate scale, which lives on `Field`, not on the raw `Membership` fov. The `within`/`frame`/
  `frame_rotated` builders capture it from the `Field`; manual construction leaves it `None`
  (so pixel offsets are simply absent), matching FR-M7 ("when plate scale is known").
- **[DECISION] Robust tangent-offset geometry.** Offsets/membership use a polar decomposition
  of great-circle separation + position angle into East/North (never dividing by cos of the
  separation), with the circumscribed circle pre-filtering rectangle tests. Exact to planning
  grade for the small fields involved; robust for far objects returned by unbounded nearest-N.
- **[DECISION] Exact `ARCSEC_PER_RADIAN`** (206264.806…) replaces alm's rounded `206.265`.
- **[AMBIGUITY] Binning/FOV model.** `pixels` = the (binned) image dimensions you supply,
  `pixel_um` = physical unbinned pixel size, `binning` = factor. Effective pixel = pixel×binning,
  so ×2 binning ⇒ ×2 scale ⇒ ×2 field for a fixed pixel count (matches SC-009). Halve the pixel
  count for a fixed sensor to keep the field constant.

## Input needed from you

- _(none — all prior items ratified below; further items will be added here if they arise)_

## Ratified 2026-07-11

All open decisions from the v0.1 handover were reviewed and confirmed by the maintainer:

- **Repo visibility** — stays **public**.
- **Infallible matching API** — kept. `MissingObservationDate` / `EpochMismatch` stay dropped;
  the deviation from FR-M11's literal wording is now **ratified**, not merely flagged.
  Tradeoffs reviewed: compile-time unrepresentability beats the runtime check the spec's
  wording asked for; the sole cost is a breaking signature change *if* matching ever becomes
  fallible, acceptable at v0.x. `spec.md` wording intentionally left as the historical record.
- **License** — stays **Apache-2.0 only**; no dual MIT before a crates.io publish.
- **Constitution** — left unratified; principles remain in `AGENTS.md`.

## 0.2 — adopt skymath (2026-07-12)

- **[DECIDED earlier, executed now] Hard cut to `skymath`** (ratified during the skymath
  grilling): the `angle` module (Angle/Epoch/Equatorial, sexagesimal, separation,
  precession) is deleted; `skymath` types appear directly in public signatures. No
  compatibility re-exports of individual types; the `skymath` crate itself is re-exported
  (`target_match::skymath`) for version alignment.
- **[DECISION] API deltas for consumers**: `Equatorial::parse_j2000(ra, dec)` →
  `skymath::Equatorial::parse_j2000(ra, dec, ParseMode)` (strict/lenient is now explicit);
  `Equatorial::new(ra, dec, epoch)` → `at_epoch`; sexagesimal formatting via
  `SexaStyle`. `target_match::Error` shrinks to `InvalidOptics` — parse/domain errors
  surface as `skymath::Error` at construction.
- **[DECISION] Test ownership follows code ownership**: the separation/sexagesimal/
  precession property tests moved to skymath with the code; this crate keeps
  matcher/optics coverage (`matcher_equals_rank`, membership geometry, known values).
- **[RESOLVED] Interim git dependency**: skymath v0.1.0 published to crates.io
  (2026-07-12); the pin is now `skymath = "0.1"`. (skymath's first publish was local
  with a token — crates.io Trusted Publishing config can only be added to an existing
  crate; add the config in both crates' settings so future releases publish via OIDC.)
