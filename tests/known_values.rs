// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Known-value integration tests (SC-001, SC-005, SC-006).

use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};

struct T {
    name: &'static str,
    ra: f64,
    dec: f64,
}
impl SkyObject for T {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
    }
}

fn deg(d: f64) -> Angle {
    Angle::from_degrees(d)
}
fn eq(ra: f64, dec: f64) -> Equatorial {
    Equatorial::j2000(deg(ra), deg(dec)).unwrap()
}

fn andromeda_field() -> Vec<T> {
    vec![
        T {
            name: "M 31",
            ra: 10.6847,
            dec: 41.2688,
        },
        T {
            name: "M 110",
            ra: 10.0921,
            dec: 41.6853,
        },
        T {
            name: "M 33",
            ra: 23.4621,
            dec: 30.6599,
        },
        T {
            name: "M 42",
            ra: 83.8221,
            dec: -5.3911,
        },
    ]
}

#[test]
fn m31_identified_from_pointing_and_optics() {
    // SC-001: nearest catalogued object a frame captured.
    let catalog = andromeda_field();
    let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
    let field = Field::from_optics(Optics {
        focal_mm: 800.0,
        pixel_um: (3.76, 3.76),
        binning: (1, 1),
        pixels: (6248, 4176),
    })
    .unwrap();
    let hits = rank(
        pointing,
        &catalog,
        Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one(),
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].object.name, "M 31");
    assert!(hits[0].separation.arcseconds() < 60.0);
    assert!(hits[0].in_frame);
}

#[test]
fn asi2600_on_800mm_field_matches_hand_calc() {
    // SC-005: pixel scale ≈ 0.969"/px; field ≈ 1.68° × 1.12°.
    let field = Field::from_optics(Optics {
        focal_mm: 800.0,
        pixel_um: (3.76, 3.76),
        binning: (1, 1),
        pixels: (6248, 4176),
    })
    .unwrap();
    let (sx, sy) = field.pixel_scale().unwrap();
    assert!((sx - 0.9694).abs() < 1e-3, "scale x {sx}");
    assert!((sy - 0.9694).abs() < 1e-3, "scale y {sy}");
    assert!((field.width().degrees() - 1.683).abs() < 5e-3);
    assert!((field.height().degrees() - 1.125).abs() < 5e-3);
    assert!((field.radius(RadiusPolicy::Circumscribed).degrees() - 1.012).abs() < 5e-3);
}

#[test]
fn rectangle_membership_and_rotation_swap_edges() {
    // SC-006: a 2°×1° frame at (100°, 0°). Object 0.7° East is inside the wide axis;
    // object 0.7° North is outside the short axis. Rotating the frame 90° swaps them.
    let field = Field::from_fov(deg(2.0), deg(1.0)).unwrap();
    let center = eq(100.0, 0.0);
    let east = T {
        name: "E",
        ra: 100.7,
        dec: 0.0,
    };
    let north = T {
        name: "N",
        ra: 100.0,
        dec: 0.7,
    };
    let cat = vec![east, north];

    let aligned: Vec<_> = rank(center, &cat, Constraint::frame(&field).all())
        .iter()
        .map(|m| m.object.name)
        .collect();
    assert_eq!(
        aligned,
        vec!["E"],
        "axis-aligned: East object inside, North outside"
    );

    let rotated: Vec<_> = rank(
        center,
        &cat,
        Constraint::frame_rotated(&field, deg(90.0)).all(),
    )
    .iter()
    .map(|m| m.object.name)
    .collect();
    assert_eq!(
        rotated,
        vec!["N"],
        "rotated 90°: North object inside, East outside"
    );
}

#[test]
fn all_within_field_orders_close_pair() {
    // M31 + M110 both fall in a ~1° field; ordered nearest-first.
    let catalog = andromeda_field();
    let hits = rank(
        eq(10.6847, 41.2688),
        &catalog,
        Constraint::circular(deg(1.0)).all(),
    );
    let names: Vec<_> = hits.iter().map(|m| m.object.name).collect();
    assert_eq!(names, vec!["M 31", "M 110"]);
}
