//! Property-based tests of the matcher invariants (SC-007, SC-008).
//!
//! The coordinate-math properties this suite once carried (separation
//! symmetry, sexagesimal and precession round-trips) moved to `skymath`,
//! which owns that geometry; only matcher behaviour is tested here.

use proptest::prelude::*;
use skymath::{Angle, Equatorial};
use target_match::{rank, Constraint, Matcher, SkyObject};

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
