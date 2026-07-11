# target-match

A pure-Rust library that identifies **which catalogued sky object a telescope frame
captured** — from where the scope pointed and how much sky the frame covers.

- **Coordinates, never names** — matching is done purely by sky position. A frame's
  `OBJECT` string (written inconsistently by capture software) is never a search key;
  a designation may ride along on a result for display only.
- **Catalog-agnostic** — the crate owns no catalogue data and does no I/O. You bring
  your own objects (a database, a file, a SIMBAD resolver, a hand-built list) by
  implementing one small trait; `target-match` does the geometry.
- **Pure Rust, dependency-light** — one small runtime dependency (`thiserror`), MSVC-safe.
  Optional off-by-default `serde`.
- **Flexible inputs** — pointing and distances in decimal degrees *or* sexagesimal
  (`HH:MM:SS` / `±DD:MM:SS`); field of view from optics (focal length, pixel size
  x/y, binning x/y, sensor pixels), from a pixel scale, or given directly.

## Status

Implemented and tested, extracted from the [`nightwatch-astro/alm`](https://github.com/nightwatch-astro/alm)
targeting pipeline. The `angle`, `optics`, and `matcher` modules are complete; the
specification lives under [`specs/001-target-match-core/`](specs/001-target-match-core/)
(SpecKit). See [`examples/identify.rs`](examples/identify.rs) for a runnable demo.

## Usage

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
`Matcher::from_objects(..)` and call `.query(pointing, constraint)` repeatedly. Rectangular
in-frame membership (with optional camera rotation) and JNow→J2000 precession are supported;
see the module docs.

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on the public coordinate
  and match types. Enable with `target-match = { version = "…", features = ["serde"] }`.

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
