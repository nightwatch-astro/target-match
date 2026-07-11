//! SC-010 / FR-X5: matching is decided by coordinates, never by a name.

use target_match::{rank, Angle, Constraint, Equatorial, SkyObject};

struct Named {
    name: &'static str,
    ra: f64,
    dec: f64,
}
impl SkyObject for Named {
    fn position(&self) -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
    }
}

#[test]
fn a_mislabelled_name_never_matches() {
    // One entry is literally called "M 31" but parked on the far side of the sky;
    // the true M31 position is held by a differently-named entry.
    let catalog = vec![
        Named {
            name: "M 31",
            ra: 200.0,
            dec: -40.0,
        },
        Named {
            name: "unlabelled galaxy",
            ra: 10.6847,
            dec: 41.2688,
        },
    ];
    let pointing =
        Equatorial::j2000(Angle::from_degrees(10.6847), Angle::from_degrees(41.2688)).unwrap();

    let hits = rank(
        pointing,
        &catalog,
        Constraint::circular(Angle::from_degrees(2.0)).all(),
    );

    assert_eq!(hits.len(), 1, "only the coordinate match is returned");
    assert_eq!(hits[0].object.name, "unlabelled galaxy");
    assert!(
        !hits.iter().any(|h| h.object.name == "M 31"),
        "the far 'M 31'-named entry must never match by name"
    );
}
