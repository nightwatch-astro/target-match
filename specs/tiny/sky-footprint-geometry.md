# TinySpec: Sky Footprint Geometry

**Branch**: `feat/sky-footprint-geometry`
**Date**: 2026-07-22
**Status**: implemented
**Complexity**: bounded library feature

## What

Add generic captured-image footprint comparison and union geometry.
The API returns measurements and evidence without product thresholds or image processing.

## Context

| File | Role |
|---|---|
| `Cargo.toml` | Pins skymath 0.6.0 and private polygon operations |
| `src/footprint.rs` | Owns footprint, comparison, interval, union, and object geometry |
| `src/error.rs` | Exposes typed geometry failures |
| `src/lib.rs` | Re-exports the public API |
| `tests/footprint_geometry.rs` | Covers examples, failures, and properties |
| `README.md` | Describes the released API surface |

## Requirements

1. `SkyFootprint` carries centre, ordered corners, solved sky position angle, parity, and provenance.
2. Pair comparison uses one spherical-midpoint gnomonic plane.
3. Coverage equals intersection area divided by the smaller footprint area.
4. Centre separation is also reported relative to the smaller footprint diagonal.
5. Sky axes are transported to the common plane and compared modulo 180 degrees.
6. Parity remains separate from the rotation residual.
7. A caller supplies the inclusive coverage band and rotation search resolution.
8. Unions preserve holes and disconnected components without exposing dependency geometry types.
9. Point boundaries count as covered.
10. Point, footprint, and ellipse queries return union-component and panel evidence.
11. The module contains no session, catalogue, target-confirmation, or image-cropping policy.

## Plan

1. Pin skymath 0.6.0 and add an MSRV-compatible polygon-boolean dependency.
2. Implement validated footprint projection and pair measurements.
3. Implement residual-rotation coverage and interval search.
4. Implement persisted-anchor unions and object coverage evidence.
5. Verify geometry behavior, public docs, optional serde, and the repository gates.

## Tasks

- [x] Add dependencies through Cargo.
- [x] Implement typed errors and the footprint API.
- [x] Implement comparison and rotation measurements.
- [x] Implement hole-aware unions and object evidence.
- [x] Add integration and property tests.
- [x] Document the public surface.
- [x] Run format, clippy, tests, serde, docs, and diff checks.

## Done When

- [x] All tasks are checked.
- [x] Repository verification passes.
- [x] API documentation builds without warnings.
