# Changelog

## [0.2.0](https://github.com/nightwatch-astro/target-match/compare/v0.1.0...v0.2.0) (2026-07-12)


### ⚠ BREAKING CHANGES

* target_match::{Angle, Epoch, Equatorial, precess, separation} are gone from the root - use the skymath crate (re-exported as target_match::skymath). Equatorial::parse_j2000 now takes ParseMode; Equatorial::new is at_epoch. Error::{ParseCoord, OutOfRange} removed.

### Features

* adopt skymath for coordinates, angles, and spherical geometry ([#3](https://github.com/nightwatch-astro/target-match/issues/3)) ([0b9f8f1](https://github.com/nightwatch-astro/target-match/commit/0b9f8f1f962521778323fecf32c50cd1581b9b65))
