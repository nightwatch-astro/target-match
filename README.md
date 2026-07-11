# target-match

Rust library that identifies which catalogued sky objects a telescope frame
covers, given a pointing (right ascension / declination) and a field of view.

Matching is by angular position; object names are never used as search keys (a
designation can be carried on a result for display). The crate holds no
catalogue data and performs no I/O: callers supply objects by implementing the
one-method `SkyObject` trait, and the library computes the geometry — angular
separation, in-frame membership (circular or rectangular, with optional camera
rotation), tangent-plane offsets, and deterministic ranking. Coordinates accept
decimal degrees or sexagesimal (`HH:MM:SS` / `±DD:MM:SS`). The field of view
can be computed from optics (focal length, pixel size, binning, sensor
dimensions), from a pixel scale, or supplied directly. JNow ↔ J2000 precession
is included.

API documentation, generated from the source on every release:
[docs.rs/target-match](https://docs.rs/target-match).

## Usage

```toml
[dependencies]
target-match = "0.1"
```

```rust
use target_match::{rank, Angle, Constraint, Equatorial, Field, Optics, RadiusPolicy, SkyObject};

// Your catalogue type — target-match owns no catalogue data, and never reads the name.
struct Target { name: &'static str, ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg)).unwrap()
    }
}

let catalog = [
    Target { name: "M 31", ra_deg: 10.6847, dec_deg: 41.2688 },
    Target { name: "M 33", ra_deg: 23.4621, dec_deg: 30.6599 },
];

// Where the scope pointed (decimal degrees or sexagesimal)...
let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09").unwrap();

// ...and how much sky the frame covers (from optics, a pixel scale, or a direct FOV).
let field = Field::from_optics(Optics {
    focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
}).unwrap();

// Which catalogued object the frame captured, nearest first.
let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
assert_eq!(hits[0].object.name, "M 31");
```

For a batch of frames against one catalogue, build the index once with
`Matcher::from_objects(..)` and call `.query(pointing, constraint)` repeatedly.
See [`examples/identify.rs`](examples/identify.rs) for a runnable end-to-end
demo.

## Features

- `serde` *(off by default)* — derives `Serialize`/`Deserialize` on the public
  coordinate and match types.

## Development

Requires a stable Rust toolchain (pinned via `rust-toolchain.toml`) and, optionally,
[`just`](https://github.com/casey/just).

```sh
just verify   # fmt-check + clippy (-D warnings) + tests
just test
just doc
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
