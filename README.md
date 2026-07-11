# target-match

A pure-Rust library that identifies **which catalogued sky object a telescope frame
captured** — from where the scope pointed and how much sky the frame covers.

- **Coordinates, never names** — matching is done purely by sky position. A frame's
  `OBJECT` string (written inconsistently by capture software) is never a search key;
  a designation may ride along on a result for display only.
- **Catalog-agnostic** — the crate owns no catalogue data and does no I/O. You bring
  your own objects (a database, a file, a SIMBAD resolver, a hand-built list) by
  implementing one small trait; `target-match` does the geometry.
- **Pure Rust, dependency-light** — no runtime dependencies (just `std`), MSVC-safe
  and `no_std`-friendly at heart. Optional off-by-default `serde`.
- **Flexible inputs** — pointing and distances in decimal degrees *or* sexagesimal
  (`HH:MM:SS` / `±DD:MM:SS`); field of view from optics (focal length, pixel size
  x/y, binning x/y, sensor pixels), from a pixel scale, or given directly.

## Status

Early scaffold, extracted from the [`nightwatch-astro/alm`](https://github.com/nightwatch-astro/alm)
targeting pipeline. The `angle`, `optics`, and `matcher` modules are documented stubs;
the public API is being specified under [`specs/`](specs/) (SpecKit) and implemented as
follow-up work.

## Planned API

```rust
use target_match::angle::Equatorial;
use target_match::optics::{Optics, RadiusPolicy};
use target_match::matcher::{rank, SkyObject};

// Your catalogue type — target-match owns no catalogue data.
struct Target { id: u32, name: String, ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn ra_deg(&self) -> f64 { self.ra_deg }
    fn dec_deg(&self) -> f64 { self.dec_deg }
}

// Where the scope pointed (decimal degrees or sexagesimal)...
let pointing = Equatorial::from_degrees(10.6847, 41.2688);
// let pointing = Equatorial::parse("00 42 44", "+41 16 09")?;

// ...and how much sky the frame covers (from optics, a pixel scale, or a direct FOV).
let field = Optics { focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176) };

// Which catalogued objects fall on the frame, nearest first.
let hits = rank(pointing, &catalog, field.radius(RadiusPolicy::Circumscribed));
```

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
