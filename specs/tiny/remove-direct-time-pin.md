# TinySpec: Remove Direct Time Pin

**Branch**: `fix/remove-direct-time-pin`
**Date**: 2026-07-22
**Status**: implemented
**Complexity**: dependency compatibility repair

## What

Remove target-match's unused direct dependency on `time` so downstream
applications can select a mutually compatible `time` release. Keep the
committed lockfile compatible with the crate's Rust 1.74 minimum.

## Context

| File | Role |
|---|---|
| `Cargo.toml` | Declares target-match's direct dependency contract |
| `Cargo.lock` | Pins repository verification to MSRV-compatible releases |
| `specs/tiny/remove-direct-time-pin.md` | Records the compatibility repair and proof |

## Requirements

1. `target-match` has no direct `time` dependency because its source does not use the crate.
2. `skymath` remains pinned to `0.6.0` and resolves `time` `0.3.41` in the committed lockfile.
3. Locked all-feature checking and testing pass on Rust 1.74.
4. A pre-publish consumer fixture combines the packaged crate with SQLx 0.9.
5. The fixture resolves exactly one `time` version and passes `cargo check`.
6. Formatting, linting, tests, documentation, packaging, and publish dry-run pass.

## Plan

1. Remove the unused dependency through Cargo and retain the compatible lock.
2. Verify the locked dependency graph and Rust 1.74 behavior.
3. Run repository quality, package, and publish gates.
4. Check the packaged crate with SQLx 0.9 in an isolated consumer fixture.

## Tasks

- [x] Remove the direct `time` dependency through Cargo.
- [x] Confirm the locked `skymath` and `time` versions.
- [x] Run locked all-feature check and test on Rust 1.74.
- [x] Run all repository and publication gates.
- [x] Verify the packaged crate and SQLx 0.9 consumer fixture.

## Done When

- [x] All tasks are checked.
- [x] The package manifest does not constrain downstream `time` selection.
- [x] All verification and compatibility evidence passes.
