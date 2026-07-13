# Guide

A task-oriented walkthrough of `target-match`. Every snippet below uses the
same fixture: a pointing at Messier 31 (`00:42:44.3 +41:16:09`) and a frame
from an 800 mm scope with a 3.76 µm, 6248×4176 sensor. It is the same fixture
used in the [crate-level doctest](https://docs.rs/target-match/latest/target_match/)
and in
[`examples/identify.rs`](https://github.com/nightwatch-astro/target-match/blob/main/examples/identify.rs)
— run the full program with `cargo run --example identify`.

## Install

```toml
[dependencies]
target-match = "0.3"
```

## 1. Implement `SkyObject` for your catalogue type

`target-match` holds no catalogue data. Give it a type — from a database, a
file, a name resolver, or a hand-built list — that implements
[`SkyObject`](https://docs.rs/target-match/latest/target_match/trait.SkyObject.html):
one method returning a J2000
[`skymath::Equatorial`](https://docs.rs/skymath/latest/skymath/struct.Equatorial.html)
position.

```rust
use skymath::{Angle, Equatorial};
use target_match::SkyObject;

struct Target {
    name: &'static str,
    ra_deg: f64,
    dec_deg: f64,
}

impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg))
            .unwrap()
    }
}

let catalog = [
    Target { name: "M 31", ra_deg: 10.6847, dec_deg: 41.2688 },
    Target { name: "M 33", ra_deg: 23.4621, dec_deg: 30.6599 },
];
```

The trait exposes position only — matching never reads `name` or any other
field. The struct is free to carry a name for display, as above.

## 2. Describe the frame

A [`Field`](https://docs.rs/target-match/latest/target_match/struct.Field.html)
is the angular extent of a frame. Build one from
[`Optics`](https://docs.rs/target-match/latest/target_match/struct.Optics.html)
(focal length, pixel size, binning, sensor dimensions):

```rust
use target_match::{Field, Optics};

let field = Field::from_optics(Optics {
    focal_mm: 800.0,
    pixel_um: (3.76, 3.76),
    binning: (1, 1),
    pixels: (6248, 4176),
})
.unwrap();
```

`Field` has two other constructors for when you already know the plate scale
or the field of view directly:
[`Field::from_pixel_scale`](https://docs.rs/target-match/latest/target_match/struct.Field.html#method.from_pixel_scale)
and
[`Field::from_fov`](https://docs.rs/target-match/latest/target_match/struct.Field.html#method.from_fov).

All three constructors are fallible: non-positive or non-finite inputs (a zero
focal length, a negative pixel size) return
[`Error::InvalidOptics`](https://docs.rs/target-match/latest/target_match/enum.Error.html),
aliased through
[`Result`](https://docs.rs/target-match/latest/target_match/type.Result.html).
The snippets here `.unwrap()` because the values are known-good; handle the
`Result` in production. Matching itself is infallible once a `Field` exists.

## 3. Rank the catalogue against a pointing

Parse the pointing (decimal degrees or sexagesimal), build a
[`Constraint`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html)
from the field, and call
[`rank`](https://docs.rs/target-match/latest/target_match/fn.rank.html):

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};

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

let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
let field = Field::from_optics(Optics {
    focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
}).unwrap();

let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed));
assert_eq!(hits[0].object.name, "M 31");
```

The pointing is precessed to J2000 before matching, so it can be given at any
epoch. `Constraint::within` derives a circular search radius from the field
under a
[`RadiusPolicy`](https://docs.rs/target-match/latest/target_match/enum.RadiusPolicy.html):

- `Circumscribed` (used in this example) — half the diagonal, the circle that
  bounds the whole frame. Never misses an in-frame object, but can report corner
  objects that fall outside the sensor rectangle.
- `Inscribed` — half the shorter side. The opposite trade: no false positives,
  but misses objects near the corners.
- `Multiplier(m)` — the circumscribed radius scaled by `m`.
- `Explicit(angle)` — a fixed radius, ignoring the field extent.

For an exact answer with no circular approximation, use
[`Constraint::frame`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.frame)
instead of `within` (see §5).

Results are a `Vec<`[`Match`](https://docs.rs/target-match/latest/target_match/struct.Match.html)`>`,
ascending by separation. Each `Match` carries the borrowed object, its great-circle
separation, whether it is inside the frame (`in_frame`), an
[`Offset`](https://docs.rs/target-match/latest/target_match/struct.Offset.html)
relative to the frame centre, and a position angle.

## 4. One object, no ranking

To evaluate a single object without filtering or sorting a catalogue, use
[`is_framed`](https://docs.rs/target-match/latest/target_match/fn.is_framed.html)
with an explicit
[`Membership`](https://docs.rs/target-match/latest/target_match/enum.Membership.html)
shape:

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{is_framed, Membership, SkyObject};

struct Target { ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg)).unwrap()
    }
}

let m31 = Target { ra_deg: 10.6847, dec_deg: 41.2688 };
let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();

let m = is_framed(pointing, &m31, Membership::Circular { radius: Angle::from_degrees(1.0) });
assert!(m.in_frame);
```

## 5. Rectangular frames and camera rotation

`Constraint::within` always uses circular membership. For the frame's actual
rectangle, use
[`Constraint::frame`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.frame)
(axis-aligned) or
[`Constraint::frame_rotated`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.frame_rotated)
(rotated by a camera position angle, degrees East of North). Rectangular
membership fills in `Offset::frame` (and `Offset::pixels`, when the field
carries a plate scale):

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, Optics, SkyObject};

struct Target { ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg)).unwrap()
    }
}

let catalog = [Target { ra_deg: 10.6847, dec_deg: 41.2688 }];
let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
let field = Field::from_optics(Optics {
    focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
}).unwrap();

let hits = rank(pointing, &catalog, Constraint::frame(&field));
assert!(hits[0].offset.frame.is_some());
assert!(hits[0].offset.pixels.is_some());
```

The camera position angle is measured in degrees East of North. To see the
convention, place an object due north of the pointing and read `Offset::frame`
`(x, y)` under two rotations. Axis-aligned, it sits on the `+y` (North) axis;
rotated 90° East of North, the same object moves onto the `-x` axis:

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, SkyObject};

struct Target { ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg)).unwrap()
    }
}

let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
let field = Field::from_fov(Angle::from_degrees(2.0), Angle::from_degrees(2.0)).unwrap();
// ~0.3° due north of the pointing.
let catalog = [Target { ra_deg: 10.6847, dec_deg: 41.2688 + 0.3 }];

let (ax, ay) = rank(pointing, &catalog, Constraint::frame(&field))[0].offset.frame.unwrap();
assert!(ay.degrees() > 0.25 && ax.degrees().abs() < 0.05, "on the +y axis");

let rotated = Constraint::frame_rotated(&field, Angle::from_degrees(90.0));
let (rx, ry) = rank(pointing, &catalog, rotated)[0].offset.frame.unwrap();
assert!(rx.degrees() < -0.25 && ry.degrees().abs() < 0.05, "rotated onto the -x axis");
```

## 6. Nearest-one and nearest-N

Chain a query mode onto any `Constraint`:
[`nearest_one`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.nearest_one)
for the single closest match,
[`nearest_n`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.nearest_n)
for the closest `n` regardless of frame membership, or
[`nearest_n_within`](https://docs.rs/target-match/latest/target_match/struct.Constraint.html#method.nearest_n_within)
to additionally bound them by a radius:

```rust
use skymath::Angle;
use target_match::{Constraint, Query};

let radius = Angle::from_degrees(1.0);
let c = Constraint::circular(radius).nearest_n_within(3, radius);
assert_eq!(c.query, Query::NearestN { n: 3, max_radius: Some(radius) });
```

## 7. Repeated queries against one catalogue

Ranking a large catalogue with
[`rank`](https://docs.rs/target-match/latest/target_match/fn.rank.html) rescans
the whole slice every call. For many pointings against the same catalogue,
build a
[`Matcher`](https://docs.rs/target-match/latest/target_match/struct.Matcher.html)
once — it produces results identical to `rank`, faster:

```rust
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{Constraint, Field, Matcher, Optics, RadiusPolicy, SkyObject};

struct Target { name: &'static str, ra_deg: f64, dec_deg: f64 }
impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra_deg), Angle::from_degrees(self.dec_deg)).unwrap()
    }
}

let matcher = Matcher::from_objects(vec![
    Target { name: "M 31", ra_deg: 10.6847, dec_deg: 41.2688 },
    Target { name: "M 33", ra_deg: 23.4621, dec_deg: 30.6599 },
]);

let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
let field = Field::from_optics(Optics {
    focal_mm: 800.0, pixel_um: (3.76, 3.76), binning: (1, 1), pixels: (6248, 4176),
}).unwrap();

let hits = matcher.query(pointing, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
assert_eq!(hits[0].object.name, "M 31");
```

## Full example

[`examples/identify.rs`](https://github.com/nightwatch-astro/target-match/blob/main/examples/identify.rs)
runs this same fixture end-to-end, printing the frame size, every catalogued
object on it, and the best target:

```sh
cargo run --example identify
```
