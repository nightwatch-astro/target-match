# target-match

[![CI](https://github.com/nightwatch-astro/target-match/actions/workflows/ci.yml/badge.svg)](https://github.com/nightwatch-astro/target-match/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/target-match.svg)](https://crates.io/crates/target-match)
[![docs.rs](https://img.shields.io/docsrs/target-match)](https://docs.rs/target-match)

Rust library that identifies which catalogued sky objects a telescope frame
covers, given a pointing (right ascension / declination) and a field of view.

Matching is by angular position; object names are never used as search keys (a
designation can be carried on a result for display). The crate holds no
catalogue data and performs no I/O: callers supply objects by implementing the
one-method [`SkyObject`](https://docs.rs/target-match/latest/target_match/trait.SkyObject.html)
trait, and the library computes the geometry — angular separation, in-frame
[`Membership`](https://docs.rs/target-match/latest/target_match/enum.Membership.html)
(a fast circular radius that approximates the frame, or the exact rectangle,
with optional camera rotation), tangent-plane
[`Offset`](https://docs.rs/target-match/latest/target_match/struct.Offset.html)s,
and deterministic ranking.

The field of view can be computed from
[`Optics`](https://docs.rs/target-match/latest/target_match/struct.Optics.html)
(focal length, pixel size, binning, sensor dimensions), from a pixel scale, or
supplied directly via
[`Field`](https://docs.rs/target-match/latest/target_match/struct.Field.html).
Each `Field` constructor is fallible: non-positive or non-finite inputs return
[`Error::InvalidOptics`](https://docs.rs/target-match/latest/target_match/enum.Error.html)
(the snippets below `.unwrap()` on known-good values). Matching itself is
infallible once inputs are valid. Pointings at any epoch are precessed to J2000
before matching.

Coordinate and angle types come from the
[`skymath`](https://github.com/nightwatch-astro/skymath) crate and appear
directly in this crate's API (`skymath::Equatorial`, `skymath::Angle` — with
decimal and strict/lenient sexagesimal parsing); `skymath` is re-exported as
`target_match::skymath` so a version-matched copy is always available.

API documentation, generated from the source on every release:
[docs.rs/target-match](https://docs.rs/target-match). For a task-oriented
walkthrough, see the
[`guide`](https://docs.rs/target-match/latest/target_match/guide/index.html) module.

## Usage

```toml
[dependencies]
target-match = "0.3"
```

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};

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
let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();

// ...and how much sky the frame covers (from optics, a pixel scale, or a direct FOV).
let field = Field::from_optics(Optics {
    focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
}).unwrap();

// Nearest catalogued object within the frame's search radius. `within` uses a
// circular approximation (here the circumscribed circle, half the diagonal); for
// the exact sensor rectangle use `Constraint::frame(&field)`.
let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
assert_eq!(hits[0].object.name, "M 31");
```

Each hit is a
[`Match`](https://docs.rs/target-match/latest/target_match/struct.Match.html)
carrying the borrowed object, its separation, in-frame flag, offset, and
position angle. The
[`Query`](https://docs.rs/target-match/latest/target_match/enum.Query.html)
mode on the constraint selects all-within-field, nearest-one, or nearest-N.
To test a single object without ranking a catalogue, call
[`is_framed`](https://docs.rs/target-match/latest/target_match/fn.is_framed.html).

For a batch of frames against one catalogue, build the index once with
[`Matcher::from_objects`](https://docs.rs/target-match/latest/target_match/struct.Matcher.html#method.from_objects)`(..)`
and call
[`.query(pointing, constraint)`](https://docs.rs/target-match/latest/target_match/struct.Matcher.html#method.query)
repeatedly. See
[`examples/identify.rs`](https://github.com/nightwatch-astro/target-match/blob/main/examples/identify.rs)
for a runnable end-to-end demo, and the
[`guide`](https://docs.rs/target-match/latest/target_match/guide/index.html)
module for a task-oriented walkthrough.

## Features

- `serde` *(off by default)* — derives `Serialize`/`Deserialize` on the public
  match types, and forwards to `skymath/serde` for the coordinate types they
  embed.

## Development

Requires a stable Rust toolchain (pinned via `rust-toolchain.toml`) and, optionally,
[`just`](https://github.com/casey/just).

```sh
just verify   # fmt-check + clippy (-D warnings) + tests
just test
just doc
```

## License

Licensed under the
[Apache License, Version 2.0](https://github.com/nightwatch-astro/target-match/blob/main/LICENSE).
