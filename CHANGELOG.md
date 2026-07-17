# Changelog

## [0.4.0](https://github.com/nightwatch-astro/target-match/compare/v0.3.3...v0.4.0) (2026-07-17)


### ⚠ BREAKING CHANGES

* relicense from Apache-2.0 to MPL-2.0 ([#14](https://github.com/nightwatch-astro/target-match/issues/14))

### Bug Fixes

* store CLA signatures on unprotected branch, allowlist owner ([#17](https://github.com/nightwatch-astro/target-match/issues/17)) ([00f79dc](https://github.com/nightwatch-astro/target-match/commit/00f79dc1e9343acc7be5d11ee68dd1ec9967394e))
* use GitHub App token for CLA bot instead of PAT ([#16](https://github.com/nightwatch-astro/target-match/issues/16)) ([c90de4e](https://github.com/nightwatch-astro/target-match/commit/c90de4e02f1f7a1eef45bec22dbeb0e88ce75539))


### Miscellaneous Chores

* relicense from Apache-2.0 to MPL-2.0 ([#14](https://github.com/nightwatch-astro/target-match/issues/14)) ([3ac3064](https://github.com/nightwatch-astro/target-match/commit/3ac3064d863ade76f27d2da8dc10e041fbea2724))

## [0.3.3](https://github.com/nightwatch-astro/target-match/compare/v0.3.2...v0.3.3) (2026-07-13)


### Documentation

* add status badges ([#12](https://github.com/nightwatch-astro/target-match/issues/12)) ([85d7d1e](https://github.com/nightwatch-astro/target-match/commit/85d7d1ec565b4ccdf62ddd06108f712c74e04a92))

## [0.3.2](https://github.com/nightwatch-astro/target-match/compare/v0.3.1...v0.3.2) (2026-07-13)


### Documentation

* render README and guide on docs.rs, add per-method examples ([#10](https://github.com/nightwatch-astro/target-match/issues/10)) ([f71eadc](https://github.com/nightwatch-astro/target-match/commit/f71eadcb6bf3793ee0edb4ab81183294416bd0f9))

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
