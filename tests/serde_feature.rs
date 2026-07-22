// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Exercises the optional `serde` feature end-to-end (FR-X3).
//! Only compiled/run with `--features serde`.
#![cfg(feature = "serde")]

use skymath::{gnomonic_unproject, Angle, Epoch, Equatorial, GnomonicPoint};
use target_match::{
    FootprintProvenance, ImageParity, Membership, Optics, RadiusPolicy, SkyFootprint,
};

#[test]
fn public_types_round_trip_through_json() {
    // Coordinate types come from skymath (the feature forwards to skymath/serde).
    let pos = Equatorial::at_epoch(
        Angle::from_degrees(10.6847),
        Angle::from_degrees(41.2688),
        Epoch::OfDate(2026.5),
    )
    .unwrap();
    let json = serde_json::to_string(&pos).unwrap();
    let back: Equatorial = serde_json::from_str(&json).unwrap();
    assert_eq!(pos, back);

    let optics = Optics {
        focal_mm: 800.0,
        pixel_um: (3.76, 3.76),
        binning: (1, 1),
        pixels: (6248, 4176),
    };
    let back_o: Optics = serde_json::from_str(&serde_json::to_string(&optics).unwrap()).unwrap();
    assert_eq!(optics, back_o);

    let policy = RadiusPolicy::Multiplier(1.5);
    let back_p: RadiusPolicy =
        serde_json::from_str(&serde_json::to_string(&policy).unwrap()).unwrap();
    assert_eq!(policy, back_p);

    let m = Membership::Rotated {
        fov: (Angle::from_degrees(1.6), Angle::from_degrees(1.1)),
        position_angle: Angle::from_degrees(30.0),
    };
    let back_m: Membership = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(m, back_m);

    let centre = Equatorial::j2000(Angle::from_degrees(10.0), Angle::from_degrees(20.0)).unwrap();
    let corners = [
        (-0.01, -0.005),
        (0.01, -0.005),
        (0.01, 0.005),
        (-0.01, 0.005),
    ]
    .into_iter()
    .map(|(east, north)| gnomonic_unproject(centre, GnomonicPoint { east, north }).unwrap())
    .collect();
    let footprint = SkyFootprint::new(
        centre,
        corners,
        Angle::from_degrees(15.0),
        ImageParity::Direct,
        FootprintProvenance::new("serde-footprint").unwrap(),
    )
    .unwrap();
    let back_footprint: SkyFootprint =
        serde_json::from_str(&serde_json::to_string(&footprint).unwrap()).unwrap();
    assert_eq!(footprint, back_footprint);
}
