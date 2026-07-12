//! Exercises the optional `serde` feature end-to-end (FR-X3).
//! Only compiled/run with `--features serde`.
#![cfg(feature = "serde")]

use skymath::{Angle, Epoch, Equatorial};
use target_match::{Membership, Optics, RadiusPolicy};

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
}
