//! Property-based tests of the geometric invariants
//! (SC-002, SC-003, SC-004, SC-007, SC-008).

use proptest::prelude::*;
use target_match::{
    precess, rank, separation, Angle, Constraint, Epoch, Equatorial, Matcher, SkyObject,
};

#[derive(Clone)]
struct Obj {
    id: usize,
    ra: f64,
    dec: f64,
}
impl SkyObject for Obj {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
    }
}

fn eq(ra: f64, dec: f64) -> Equatorial {
    Equatorial::j2000(Angle::from_degrees(ra), Angle::from_degrees(dec)).unwrap()
}

proptest! {
    /// SC-002: separation is symmetric and bounded to [0, 180]°.
    #[test]
    fn separation_symmetric_and_bounded(
        ra1 in 0.0..360.0f64, dec1 in -90.0..90.0f64,
        ra2 in 0.0..360.0f64, dec2 in -90.0..90.0f64,
    ) {
        let (a, b) = (eq(ra1, dec1), eq(ra2, dec2));
        let ab = separation(a, b).degrees();
        let ba = separation(b, a).degrees();
        prop_assert!((ab - ba).abs() < 1e-9);
        prop_assert!((-1e-9..=180.0 + 1e-9).contains(&ab));
    }

    /// SC-003: sexagesimal format → parse round-trips to sub-milliarcsecond.
    #[test]
    fn sexagesimal_round_trip(ra in 0.0..359.999f64, dec in -89.999..89.999f64) {
        let p = eq(ra, dec);
        let q = Equatorial::parse_j2000(&p.ra_to_sexagesimal(5), &p.dec_to_sexagesimal(5)).unwrap();
        prop_assert!(separation(p, q).arcseconds() < 1e-2, "ra={ra} dec={dec}");
    }

    /// SC-004: precession to epoch-of-date and back is the identity (inverse consistency).
    #[test]
    fn precession_round_trip(ra in 0.0..360.0f64, dec in -89.0..89.0f64, year in 1900.0..2100.0f64) {
        let p = eq(ra, dec);
        let back = precess(precess(p, Epoch::OfDate(year)), Epoch::J2000);
        prop_assert!(separation(p, back).arcseconds() < 1e-3, "ra={ra} dec={dec} year={year}");
    }

    /// SC-007 / SC-008: the prebuilt Matcher returns identical results to the stateless
    /// scan for the same objects, pointing, and constraint.
    #[test]
    fn matcher_equals_rank(
        seed in prop::collection::vec((0.0..360.0f64, -90.0..90.0f64), 1..40),
        pra in 0.0..360.0f64, pdec in -90.0..90.0f64, radius in 0.1..40.0f64,
    ) {
        let cat: Vec<Obj> = seed.iter().enumerate().map(|(i, &(ra, dec))| Obj { id: i, ra, dec }).collect();
        let pointing = eq(pra, pdec);

        for c in [
            Constraint::circular(Angle::from_degrees(radius)).all(),
            Constraint::circular(Angle::from_degrees(radius)).nearest_n(5),
            Constraint::circular(Angle::from_degrees(radius)).nearest_one(),
        ] {
            let via_rank: Vec<usize> = rank(pointing, &cat, c).iter().map(|m| m.object.id).collect();
            let matcher = Matcher::from_objects(cat.clone());
            let via_index: Vec<usize> = matcher.query(pointing, c).iter().map(|m| m.object.id).collect();
            prop_assert_eq!(via_rank, via_index);
        }
    }
}
