# Quickstart & Validation Guide: target-match

Runnable scenarios that prove the feature works. Each maps to success criteria in
[spec.md](./spec.md). Run from the repo root.

## Prerequisites

- Stable Rust toolchain (pinned by `rust-toolchain.toml`).
- `just` (optional) for the aggregate gate.

## Build & gate

```sh
just verify        # fmt-check + clippy -D warnings + tests   (or:)
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

Expected: all green, zero warnings, every public item documented (SC-011).

## Scenario 1 — Identify the captured target (US1 / SC-001)

```rust
use target_match::{Equatorial, Angle, Optics, Field, RadiusPolicy, Constraint, SkyObject, rank};

struct Cat { name: &'static str, ra: f64, dec: f64 }
impl SkyObject for Cat {
    fn position(&self) -> Equatorial { Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap() }
}

let catalog = [
    Cat { name: "M 31",  ra: 10.6847, dec: 41.2688 },
    Cat { name: "M 110", ra: 10.0921, dec: 41.6853 },
    Cat { name: "M 33",  ra: 23.4621, dec: 30.6599 },
];
let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();
let field = Field::from_optics(Optics { focal_mm: 800.0, pixel_um: (3.76,3.76), binning: (1,1), pixels: (6248,4176) }).unwrap();
let c = Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one();

let hits = rank(pointing, &catalog, c).unwrap();
assert_eq!(hits[0].object.name, "M 31");        // nearest is M31
assert!(hits[0].separation.degrees() < 1e-3);
```

**Expected**: M31 is rank 1; M33 (~14.7° away) is excluded.

## Scenario 2 — Coordinates parse decimal & sexagesimal; FOV from optics (US2 / SC-003, SC-005)

```rust
let a = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();
let b = Equatorial::j2000(Angle::from_degrees(10.6846), Angle::from_degrees(41.2692)).unwrap();
assert!(target_match::separation(a, b).arcseconds() < 1.0);      // agree

let f = Field::from_optics(Optics { focal_mm: 800.0, pixel_um: (3.76,3.76), binning:(1,1), pixels:(6248,4176) }).unwrap();
let (sx, _sy) = f.pixel_scale().unwrap();
assert!((sx - 0.969).abs() < 0.005);                             // ~0.969"/px
assert!((f.width().degrees() - 1.683).abs() < 0.01);             // ~1.68°
```

## Scenario 3 — Rectangular membership with rotation (US3 / SC-006)

Place an object just outside the axis-aligned rectangle but inside the circumscribed circle;
assert `in_frame == false` for `Rectangle` and that a 90° rotation flips edge cases as expected.

## Scenario 4 — JNow via precession (US4 / SC-004)

```rust
use target_match::{Epoch, precess};
let j2000 = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();
let now  = precess(j2000, Epoch::OfDate(2026.5)).unwrap();       // to epoch-of-date
// Matching a JNow pointing precesses it back to J2000 and finds the same object as Scenario 1.
```

Missing date → error:

```rust
let jnow = Equatorial::new(a.ra(), a.dec(), Epoch::OfDate(2026.5)).unwrap();
// a pointing with OfDate carries its date, so it precesses; but a query built to require a date
// and given none returns Error::MissingObservationDate.
```

## Scenario 5 — Query modes (US5)

`AllWithinField` over a frame covering M31 + M110 returns both ranked nearest-first;
`NearestN{n:1}` returns only M31; `is_framed(pointing, &m110, membership)` returns membership +
geometry.

## Scenario 6 — Prebuilt matcher equivalence (US6 / SC-007)

```rust
use target_match::Matcher;
let m = Matcher::from_objects(catalog.into_iter().collect());
let via_index = m.query(pointing, c).unwrap();
// via_index is identical (objects, order, geometry) to rank(pointing, &catalog, c).
```

## Coordinates-only guarantee (SC-010)

A catalogue entry named "M 31" placed at the wrong coordinates is never returned for a pointing
at M31 — only the coordinate-nearest entry is. Proven in `tests/coordinates_only.rs`.
