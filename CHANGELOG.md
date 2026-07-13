# Changelog

## [0.3.1](https://github.com/nightwatch-astro/target-match/compare/v0.3.0...v0.3.1) (2026-07-13)


### Bug Fixes

* crate-root docs.rs links now resolve ([#8](https://github.com/nightwatch-astro/target-match/issues/8)) ([e1bcf69](https://github.com/nightwatch-astro/target-match/commit/e1bcf6935a76833c81838be7b049585a3f51727f))

## [0.3.0](https://github.com/nightwatch-astro/target-match/compare/v0.2.0...v0.3.0) (2026-07-12)


### ⚠ BREAKING CHANGES

* public API types now come from skymath 0.3; consumers on skymath 0.1 must bump their own skymath dependency to match.

### Features

* move to skymath 0.3 shared types ([#6](https://github.com/nightwatch-astro/target-match/issues/6)) ([e8179cc](https://github.com/nightwatch-astro/target-match/commit/e8179cc4f2346545140ea29386c82bf62c69d7b5))

## [0.2.0](https://github.com/nightwatch-astro/target-match/compare/v0.1.0...v0.2.0) (2026-07-12)


### ⚠ BREAKING CHANGES

* target_match::{Angle, Epoch, Equatorial, precess, separation} are gone from the root - use the skymath crate (re-exported as target_match::skymath). Equatorial::parse_j2000 now takes ParseMode; Equatorial::new is at_epoch. Error::{ParseCoord, OutOfRange} removed.

### Features

* adopt skymath for coordinates, angles, and spherical geometry ([#3](https://github.com/nightwatch-astro/target-match/issues/3)) ([0b9f8f1](https://github.com/nightwatch-astro/target-match/commit/0b9f8f1f962521778323fecf32c50cd1581b9b65))
