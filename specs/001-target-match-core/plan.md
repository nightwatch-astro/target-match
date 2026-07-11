# Implementation Plan: target-match Core Engine

**Branch**: `001-target-match-core` | **Date**: 2026-07-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-target-match-core/spec.md`

## Summary

Implement a pure-Rust, catalogue-agnostic library that identifies which catalogued sky
object a telescope frame captured, from a pointing and a field of view, matching by sky
position only. The engine is three cooperating modules — `angle` (epoch-aware coordinates,
sexagesimal parse/format, haversine separation, precession), `optics` (plate scale / FOV
geometry and radius policy), and `matcher` (the `SkyObject` input trait, membership shapes,
query modes, and both a stateless `rank` and a prebuilt dec-band `Matcher`) — over a shared
`error` type. No I/O, no catalogue data, no network.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.74`, `std`.

**Primary Dependencies**: `thiserror` (typed public `Error`); `serde` (optional, off by
default) on public data types; `anyhow` (dev/examples only). Dev: `proptest`.

**Storage**: N/A — the crate holds no state beyond a caller-built `Matcher` index and does
no I/O.

**Testing**: `cargo test` — unit tests per module, property tests via `proptest`, known-value
tests (M31/M110/M42; ASI2600 on 800 mm), and stateless-vs-prebuilt equivalence.

**Target Platform**: any Rust target (pure computation, MSVC-safe); CI on Linux/Windows/macOS.

**Project Type**: single library crate.

**Performance Goals**: stateless `rank` is a bounded linear scan (fine for ~10^4 objects);
the prebuilt `Matcher` uses a declination-band pre-filter so a bounded query touches only the
band, enabling batch identification of many frames against one catalogue.

**Constraints**: planning-grade accuracy (~1 arcminute), NOT pointing-grade; determinism is a
contract; zero I/O; every public item documented; `clippy -D warnings` clean.

**Scale/Scope**: catalogues on the order of 10^3–10^4 objects; queries per session on the
order of 10^2–10^3 frames.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution (`.specify/memory/constitution.md`) is still the unfilled template,
so there are no ratified numbered principles to gate against. The governing principles for
this crate are captured in `AGENTS.md` and are treated as the working gate:

- **Coordinates only, never names** — matching MUST NOT read a designation. ✅ enforced by the
  `SkyObject` trait shape (position only) and a dedicated test (SC-010).
- **Pure & dependency-light** — no I/O, no catalogue, minimal deps. ✅ deps limited to
  `thiserror` + optional `serde`.
- **Determinism** — reproducible ordering. ✅ FR-M8, SC-008.
- **Planning-grade boundary** — no pointing-grade astrometry. ✅ FR-X4; apparent place out of
  scope.

No violations. (A follow-up may ratify these as a formal constitution via
`/speckit-constitution`; noted in the handover, not blocking.)

## Project Structure

### Documentation (this feature)

```text
specs/001-target-match-core/
├── plan.md              # This file
├── research.md          # Phase 0 — technical decisions & rationale
├── data-model.md        # Phase 1 — entities/types
├── quickstart.md        # Phase 1 — runnable validation guide
├── contracts/
│   └── public-api.md    # Phase 1 — the public API surface (library contract)
├── checklists/
│   └── requirements.md  # spec quality checklist
└── tasks.md             # Phase 2 — created by /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── lib.rs        # crate docs + module decls + curated re-exports
├── error.rs      # Error enum (thiserror) + Result alias
├── angle.rs      # Angle, Epoch, Equatorial, parse/format, separation, precession
├── optics.rs     # Optics, Field, RadiusPolicy, constants
└── matcher.rs    # SkyObject, Membership, Query, Constraint, Match, Offset, rank, Matcher
tests/
├── known_values.rs     # M31/M110/M42, ASI2600 optics — SC-001/005
├── properties.rs       # proptest: separation, round-trip, precession, equivalence
└── coordinates_only.rs # SC-010: mislabelled object still matches by position
examples/
└── identify.rs         # end-to-end usage (may use anyhow)
```

**Structure Decision**: single crate, modules by concern. `matcher` depends on `angle` +
`optics` + `error`; `optics` depends on `angle` + `error`; `angle` depends on `error`. The
crate root re-exports the common surface (`Angle`, `Equatorial`, `Epoch`, `Field`, `Optics`,
`RadiusPolicy`, `SkyObject`, `Membership`, `Query`, `Constraint`, `Match`, `rank`, `Matcher`,
`Error`, `Result`).

## Phase 0 — Research

See [research.md](./research.md): precession model choice, gnomonic rectangle membership,
epoch representation (Julian-year `f64` vs a date dependency), pixel-scale constant, dec-band
pre-filter correctness, and offset/position-angle conventions.

## Phase 1 — Design & Contracts

- [data-model.md](./data-model.md) — every entity, its fields, invariants, and validation.
- [contracts/public-api.md](./contracts/public-api.md) — the exported API surface (the
  library's contract) with signatures and behavioural guarantees.
- [quickstart.md](./quickstart.md) — runnable validation scenarios mapping to the success
  criteria.

## Phase 2 — Tasks

Created by `/speckit-tasks` into `tasks.md`: TDD-ordered, dependency-aware tasks grouped by
module and user story, each tied to FRs/SCs.

## Complexity Tracking

No constitution violations to justify. The one genuine complexity is the **precession**
implementation (rotation-matrix IAU 1976); it is isolated in `angle`, validated against
reference values, and required by US4 (JNow support). Everything else is elementary
spherical trigonometry.
