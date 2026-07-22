// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Exercises the optional `serde` feature end-to-end (FR-X3).
//! Only compiled/run with `--features serde`.
#![cfg(feature = "serde")]

use skymath::{gnomonic_unproject, Angle, Epoch, Equatorial, GnomonicPoint};
use target_match::{
    CoverageBand, FootprintProvenance, ImageParity, Membership, Optics, RadiusPolicy,
    RotationSearch, SkyEllipse, SkyFootprint,
};

fn footprint() -> SkyFootprint {
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
    SkyFootprint::new(
        centre,
        corners,
        Angle::from_degrees(15.0),
        ImageParity::Direct,
        FootprintProvenance::new("serde-footprint").unwrap(),
    )
    .unwrap()
}

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

    let footprint = footprint();
    let back_footprint: SkyFootprint =
        serde_json::from_str(&serde_json::to_string(&footprint).unwrap()).unwrap();
    assert_eq!(footprint, back_footprint);

    let band = CoverageBand::new(0.5, 0.9).unwrap();
    let back_band: CoverageBand =
        serde_json::from_str(&serde_json::to_string(&band).unwrap()).unwrap();
    assert_eq!(band, back_band);

    let search = RotationSearch::new(
        Angle::from_degrees(-20.0),
        Angle::from_degrees(20.0),
        Angle::from_degrees(1.0),
        Angle::from_degrees(0.01),
    )
    .unwrap();
    let back_search: RotationSearch =
        serde_json::from_str(&serde_json::to_string(&search).unwrap()).unwrap();
    assert_eq!(search, back_search);

    let ellipse = SkyEllipse::new(
        footprint.centre(),
        Angle::from_degrees(0.1),
        Angle::from_degrees(0.05),
        Angle::from_degrees(30.0),
        64,
    )
    .unwrap();
    let back_ellipse: SkyEllipse =
        serde_json::from_str(&serde_json::to_string(&ellipse).unwrap()).unwrap();
    assert_eq!(ellipse, back_ellipse);
}

#[test]
fn invariant_bearing_types_reject_malformed_json() {
    assert!(serde_json::from_str::<FootprintProvenance>(r#"" ""#).is_err());

    let mut footprint_json = serde_json::to_value(footprint()).unwrap();
    footprint_json["corners"]
        .as_array_mut()
        .expect("corners serialize as an array")
        .truncate(2);
    assert!(serde_json::from_value::<SkyFootprint>(footprint_json).is_err());

    let mut band_json = serde_json::to_value(CoverageBand::new(0.2, 0.8).unwrap()).unwrap();
    band_json["minimum"] = serde_json::json!(0.9);
    assert!(serde_json::from_value::<CoverageBand>(band_json).is_err());

    let search = RotationSearch::new(
        Angle::from_degrees(-10.0),
        Angle::from_degrees(10.0),
        Angle::from_degrees(1.0),
        Angle::from_degrees(0.1),
    )
    .unwrap();
    let mut search_json = serde_json::to_value(search).unwrap();
    search_json["sample_step"] = serde_json::to_value(Angle::from_degrees(0.0)).unwrap();
    assert!(serde_json::from_value::<RotationSearch>(search_json).is_err());

    let ellipse = SkyEllipse::new(
        footprint().centre(),
        Angle::from_degrees(0.1),
        Angle::from_degrees(0.05),
        Angle::from_degrees(30.0),
        64,
    )
    .unwrap();
    let mut ellipse_json = serde_json::to_value(ellipse).unwrap();
    ellipse_json["segments"] = serde_json::json!(0);
    assert!(serde_json::from_value::<SkyEllipse>(ellipse_json).is_err());
}
