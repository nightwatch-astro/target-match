// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Integration coverage for captured sky-footprint geometry.

use proptest::prelude::*;
use skymath::{gnomonic_unproject, separation, Angle, Epoch, Equatorial, GnomonicPoint};
use target_match::{
    compare_footprints, coverage_at_residual_rotation, coverage_rotation_intervals, Containment,
    CoverageBand, CoverageState, Error, FootprintProvenance, FootprintUnion, ImageParity,
    ObjectShape, RotationSearch, SkyEllipse, SkyFootprint,
};

fn eq(ra: f64, dec: f64) -> Equatorial {
    Equatorial::j2000(Angle::from_degrees(ra), Angle::from_degrees(dec)).unwrap()
}

fn sky(anchor: Equatorial, east: f64, north: f64) -> Equatorial {
    gnomonic_unproject(anchor, GnomonicPoint { east, north }).unwrap()
}

fn provenance(id: &str) -> FootprintProvenance {
    FootprintProvenance::new(id).unwrap()
}

fn local_rectangle(
    id: &str,
    centre: Equatorial,
    half_east: f64,
    half_north: f64,
    position_angle_degrees: f64,
    parity: ImageParity,
) -> SkyFootprint {
    let angle = position_angle_degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let corners = [
        (-half_east, -half_north),
        (half_east, -half_north),
        (half_east, half_north),
        (-half_east, half_north),
    ]
    .into_iter()
    .map(|(east, north)| sky(centre, east * cos + north * sin, -east * sin + north * cos))
    .collect();
    SkyFootprint::new(
        centre,
        corners,
        Angle::from_degrees(position_angle_degrees),
        parity,
        provenance(id),
    )
    .unwrap()
}

fn plane_rectangle(
    anchor: Equatorial,
    id: &str,
    centre: (f64, f64),
    half_size: (f64, f64),
) -> SkyFootprint {
    let (centre_east, centre_north) = centre;
    let (half_east, half_north) = half_size;
    let footprint_centre = sky(anchor, centre_east, centre_north);
    let corners = [
        (centre_east - half_east, centre_north - half_north),
        (centre_east + half_east, centre_north - half_north),
        (centre_east + half_east, centre_north + half_north),
        (centre_east - half_east, centre_north + half_north),
    ]
    .into_iter()
    .map(|(east, north)| sky(anchor, east, north))
    .collect();
    SkyFootprint::new(
        footprint_centre,
        corners,
        Angle::from_degrees(0.0),
        ImageParity::Direct,
        provenance(id),
    )
    .unwrap()
}

fn assert_close(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance,
        "expected {left} ~= {right} within {tolerance}"
    );
}

fn unit_vector(position: Equatorial) -> [f64; 3] {
    let (ra, dec) = (position.ra().radians(), position.dec().radians());
    [dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin()]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn spherical_polygon_area(corners: &[Equatorial]) -> f64 {
    let origin = unit_vector(corners[0]);
    corners[1..]
        .windows(2)
        .map(|pair| {
            let left = unit_vector(pair[0]);
            let right = unit_vector(pair[1]);
            let determinant = dot(origin, cross(left, right)).abs();
            2.0 * determinant.atan2(1.0 + dot(origin, left) + dot(left, right) + dot(right, origin))
        })
        .sum()
}

#[test]
fn comparison_measures_nested_rotated_edge_and_parity_cases() {
    let centre = eq(20.0, 30.0);
    let outer = local_rectangle("outer", centre, 0.02, 0.01, 0.0, ImageParity::Direct);
    let inner = local_rectangle("inner", centre, 0.01, 0.005, 0.0, ImageParity::Direct);
    let nested = compare_footprints(&outer, &inner).unwrap();
    assert_close(nested.normalized_coverage, 1.0, 1e-12);
    assert_close(nested.centre_separation.radians(), 0.0, 1e-14);
    assert_close(nested.normalized_centre_separation, 0.0, 1e-14);

    let rotated = local_rectangle("rotated", centre, 0.02, 0.01, 45.0, ImageParity::Direct);
    let rotated_comparison = compare_footprints(&outer, &rotated).unwrap();
    assert!((0.5..1.0).contains(&rotated_comparison.normalized_coverage));
    assert_close(
        rotated_comparison.residual_sky_rotation.degrees(),
        45.0,
        1e-10,
    );

    let mirrored = local_rectangle("mirrored", centre, 0.02, 0.01, 0.0, ImageParity::Mirrored);
    let parity = compare_footprints(&outer, &mirrored).unwrap();
    assert_close(parity.normalized_coverage, 1.0, 1e-12);
    assert!(!parity.parity_match);

    let anchor = eq(5.0, 0.0);
    let left = plane_rectangle(anchor, "left", (-0.01, 0.0), (0.01, 0.01));
    let right = plane_rectangle(anchor, "right", (0.01, 0.0), (0.01, 0.01));
    let touching = compare_footprints(&left, &right).unwrap();
    assert_close(touching.intersection_area, 0.0, 1e-18);
    assert_close(touching.normalized_coverage, 0.0, 1e-14);
}

#[test]
fn common_plane_handles_ra_wrap_and_pole_aliases() {
    let across_wrap_left = local_rectangle(
        "wrap-left",
        eq(359.9, 70.0),
        0.02,
        0.015,
        0.0,
        ImageParity::Direct,
    );
    let across_wrap_right = local_rectangle(
        "wrap-right",
        eq(0.1, 70.0),
        0.02,
        0.015,
        0.0,
        ImageParity::Direct,
    );
    let wrap = compare_footprints(&across_wrap_left, &across_wrap_right).unwrap();
    assert!(wrap.anchor.ra().degrees() < 1e-8 || wrap.anchor.ra().degrees() > 360.0 - 1e-8);
    assert!(wrap.normalized_coverage > 0.9);

    let north_alias_a = local_rectangle(
        "north-a",
        eq(0.0, 90.0),
        0.01,
        0.005,
        0.0,
        ImageParity::Direct,
    );
    let north_alias_b = local_rectangle(
        "north-b",
        eq(90.0, 90.0),
        0.01,
        0.005,
        90.0,
        ImageParity::Direct,
    );
    let pole = compare_footprints(&north_alias_a, &north_alias_b).unwrap();
    assert_close(pole.normalized_coverage, 1.0, 1e-10);
    assert_close(pole.residual_sky_rotation.degrees(), 0.0, 1e-10);
}

#[test]
fn common_plane_area_ratio_tracks_spherical_reference_across_the_field() {
    for (index, anchor) in [eq(359.8, 10.0), eq(120.0, 60.0), eq(45.0, 85.0)]
        .into_iter()
        .enumerate()
    {
        let outer = local_rectangle(
            &format!("outer-{index}"),
            anchor,
            3.0_f64.to_radians().tan(),
            2.0_f64.to_radians().tan(),
            23.0,
            ImageParity::Direct,
        );
        let inner = local_rectangle(
            &format!("inner-{index}"),
            anchor,
            1.8_f64.to_radians().tan(),
            1.2_f64.to_radians().tan(),
            23.0,
            ImageParity::Direct,
        );
        let comparison = compare_footprints(&outer, &inner).unwrap();
        let plane_ratio = comparison.right_area / comparison.left_area;
        let spherical_ratio =
            spherical_polygon_area(inner.corners()) / spherical_polygon_area(outer.corners());
        let relative_error = (plane_ratio - spherical_ratio).abs() / spherical_ratio;
        assert!(
            relative_error < 0.01,
            "common-plane ratio {plane_ratio} differs from spherical {spherical_ratio} by {relative_error}"
        );
    }
}

#[test]
fn invalid_boundaries_epochs_projection_and_antipodes_are_typed() {
    assert!(matches!(
        FootprintProvenance::new("  "),
        Err(Error::InvalidFootprint { .. })
    ));

    let centre = eq(0.0, 0.0);
    assert!(matches!(
        SkyFootprint::new(
            centre,
            vec![sky(centre, -0.01, 0.0), sky(centre, 0.01, 0.0)],
            Angle::from_degrees(0.0),
            ImageParity::Direct,
            provenance("short"),
        ),
        Err(Error::InvalidFootprint { .. })
    ));

    let bow_tie = vec![
        sky(centre, -0.01, -0.01),
        sky(centre, 0.01, 0.01),
        sky(centre, -0.01, 0.01),
        sky(centre, 0.01, -0.01),
    ];
    assert!(matches!(
        SkyFootprint::new(
            centre,
            bow_tie,
            Angle::from_degrees(0.0),
            ImageParity::Direct,
            provenance("bow-tie"),
        ),
        Err(Error::InvalidFootprint { .. })
    ));

    let horizon = vec![eq(100.0, 0.0), eq(110.0, 1.0), eq(110.0, -1.0)];
    assert!(matches!(
        SkyFootprint::new(
            centre,
            horizon,
            Angle::from_degrees(0.0),
            ImageParity::Direct,
            provenance("horizon"),
        ),
        Err(Error::ProjectionFailed(_))
    ));

    let dated = Equatorial::at_epoch(
        Angle::from_degrees(0.0),
        Angle::from_degrees(0.0),
        Epoch::OfDate(2026.0),
    )
    .unwrap();
    let dated_corners = [(-0.01, -0.01), (0.01, -0.01), (0.01, 0.01), (-0.01, 0.01)]
        .into_iter()
        .map(|(east, north)| sky(dated, east, north))
        .collect();
    let dated_footprint = SkyFootprint::new(
        dated,
        dated_corners,
        Angle::from_degrees(0.0),
        ImageParity::Direct,
        provenance("dated"),
    )
    .unwrap();
    let j2000 = local_rectangle("j2000", centre, 0.01, 0.01, 0.0, ImageParity::Direct);
    assert_eq!(
        compare_footprints(&j2000, &dated_footprint),
        Err(Error::FootprintEpochMismatch)
    );

    let opposite = local_rectangle(
        "opposite",
        eq(180.0, 0.0),
        0.01,
        0.01,
        0.0,
        ImageParity::Direct,
    );
    assert_eq!(
        compare_footprints(&j2000, &opposite),
        Err(Error::AntipodalGeometry)
    );
}

#[test]
fn rotation_coverage_reports_multiple_closed_intervals() {
    let centre = eq(40.0, -10.0);
    let left = local_rectangle("left", centre, 0.02, 0.01, 0.0, ImageParity::Direct);
    let right = local_rectangle("right", centre, 0.02, 0.01, 0.0, ImageParity::Direct);

    assert_close(
        coverage_at_residual_rotation(&left, &right, Angle::from_degrees(0.0)).unwrap(),
        1.0,
        1e-12,
    );
    assert_close(
        coverage_at_residual_rotation(&left, &right, Angle::from_degrees(90.0)).unwrap(),
        0.5,
        1e-9,
    );

    let band = CoverageBand::new(0.95, 1.0).unwrap();
    let intervals = coverage_rotation_intervals(
        &left,
        &right,
        band,
        RotationSearch::new(
            Angle::from_degrees(-180.0),
            Angle::from_degrees(180.0),
            Angle::from_degrees(2.0),
            Angle::from_degrees(0.001),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(intervals.len(), 3);
    assert!(intervals[0].start.degrees() <= -180.0);
    assert!(intervals[0].end.degrees() < -170.0);
    assert!(intervals[1].start.degrees() < 0.0 && intervals[1].end.degrees() > 0.0);
    assert!(intervals[2].start.degrees() > 170.0);
    assert!(intervals[2].end.degrees() >= 180.0);
    for interval in &intervals {
        for endpoint in [interval.start, interval.end] {
            let coverage = coverage_at_residual_rotation(&left, &right, endpoint).unwrap();
            assert!(
                (band.minimum()..=band.maximum()).contains(&coverage),
                "closed interval endpoint {} has out-of-band coverage {coverage}",
                endpoint.degrees()
            );
        }
    }

    assert!(CoverageBand::new(0.8, 0.2).is_err());
    assert!(RotationSearch::new(
        Angle::from_degrees(10.0),
        Angle::from_degrees(-10.0),
        Angle::from_degrees(1.0),
        Angle::from_degrees(0.1),
    )
    .is_err());
    assert!(RotationSearch::new(
        Angle::from_degrees(0.0),
        Angle::from_degrees(1e300),
        Angle::from_degrees(1e-300),
        Angle::from_degrees(1e-300),
    )
    .is_err());
    assert!(RotationSearch::new(
        Angle::from_degrees(-1e308),
        Angle::from_degrees(1e308),
        Angle::from_degrees(1e308),
        Angle::from_degrees(1e308),
    )
    .is_err());
}

fn ring_and_island(anchor: Equatorial) -> Vec<SkyFootprint> {
    vec![
        plane_rectangle(anchor, "left", (-0.015, 0.0), (0.005, 0.02)),
        plane_rectangle(anchor, "right", (0.015, 0.0), (0.005, 0.02)),
        plane_rectangle(anchor, "top", (0.0, 0.015), (0.02, 0.005)),
        plane_rectangle(anchor, "bottom", (0.0, -0.015), (0.02, 0.005)),
        plane_rectangle(anchor, "island", (0.05, 0.0), (0.005, 0.005)),
    ]
}

#[test]
fn union_preserves_holes_disconnected_components_and_ordered_evidence() {
    let anchor = eq(120.0, 45.0);
    let footprints = ring_and_island(anchor);
    let union = FootprintUnion::new(&footprints).unwrap();
    assert_eq!(union.component_count(), 2);
    assert_eq!(union.hole_count(), 1);

    let gap = union.contains_point(anchor).unwrap();
    assert_eq!(gap.containment, Containment::Outside);
    assert!(gap.component_indices.is_empty());

    let captured = union.contains_point(sky(anchor, -0.015, 0.0)).unwrap();
    assert!(captured.containment.is_covered());
    assert_eq!(captured.component_indices.len(), 1);
    assert_eq!(
        captured
            .panels
            .iter()
            .filter(|evidence| evidence.containment.is_covered())
            .map(|evidence| evidence.provenance.as_str())
            .collect::<Vec<_>>(),
        vec!["left"]
    );

    let island = union.contains_point(sky(anchor, 0.05, 0.0)).unwrap();
    assert!(island.containment.is_covered());
    assert_ne!(captured.component_indices, island.component_indices);

    let mut reversed = footprints.clone();
    reversed.reverse();
    let reordered = FootprintUnion::new(&reversed).unwrap();
    assert_close(
        union.anchor().ra().degrees(),
        reordered.anchor().ra().degrees(),
        1e-14,
    );
    assert_close(
        union.anchor().dec().degrees(),
        reordered.anchor().dec().degrees(),
        1e-14,
    );
    assert_close(union.area(), reordered.area(), 1e-15);
    assert_eq!(union.component_count(), reordered.component_count());
    assert_eq!(union.hole_count(), reordered.hole_count());
    assert_eq!(
        captured,
        reordered.contains_point(sky(anchor, -0.015, 0.0)).unwrap()
    );

    let duplicate = vec![footprints[0].clone(), footprints[0].clone()];
    assert!(matches!(
        FootprintUnion::new(&duplicate),
        Err(Error::DuplicateFootprintProvenance(_))
    ));
    assert!(matches!(
        FootprintUnion::new(&[]),
        Err(Error::EmptyFootprintSet)
    ));
}

#[test]
fn point_boundaries_and_extended_object_coverage_are_explicit() {
    let anchor = eq(200.0, -30.0);
    let panel = plane_rectangle(anchor, "panel", (0.0, 0.0), (0.02, 0.01));
    let union = FootprintUnion::new(core::slice::from_ref(&panel)).unwrap();

    let boundary = union.contains_point(panel.corners()[1]).unwrap();
    assert_eq!(boundary.containment, Containment::Boundary);
    assert!(boundary.containment.is_covered());

    let full = SkyEllipse::new(
        anchor,
        Angle::from_radians(0.004),
        Angle::from_radians(0.002),
        Angle::from_degrees(25.0),
        128,
    )
    .unwrap();
    let full_evidence = union.measure_object(ObjectShape::Ellipse(&full)).unwrap();
    assert_eq!(full_evidence.state, CoverageState::Full);
    assert_close(full_evidence.covered_fraction.unwrap(), 1.0, 1e-10);
    assert_eq!(full_evidence.components.len(), 1);
    assert_eq!(full_evidence.panels.len(), 1);

    let partial = SkyEllipse::new(
        sky(anchor, 0.02, 0.0),
        Angle::from_radians(0.006),
        Angle::from_radians(0.003),
        Angle::from_degrees(0.0),
        128,
    )
    .unwrap();
    let partial_evidence = union
        .measure_object(ObjectShape::Ellipse(&partial))
        .unwrap();
    assert_eq!(partial_evidence.state, CoverageState::Partial);
    assert!((0.0..1.0).contains(&partial_evidence.covered_fraction.unwrap()));

    let absent = SkyEllipse::new(
        sky(anchor, 0.05, 0.0),
        Angle::from_radians(0.004),
        Angle::from_radians(0.002),
        Angle::from_degrees(0.0),
        64,
    )
    .unwrap();
    let absent_evidence = union.measure_object(ObjectShape::Ellipse(&absent)).unwrap();
    assert_eq!(absent_evidence.state, CoverageState::None);
    assert_close(absent_evidence.covered_fraction.unwrap(), 0.0, 1e-14);

    let object_footprint = plane_rectangle(anchor, "object", (0.0, 0.0), (0.005, 0.005));
    let footprint_evidence = union
        .measure_object(ObjectShape::Footprint(&object_footprint))
        .unwrap();
    assert_eq!(footprint_evidence.state, CoverageState::Full);

    let point = union.measure_object(ObjectShape::Point(anchor)).unwrap();
    assert_eq!(point.state, CoverageState::Full);
    assert!(point.covered_fraction.is_none());
    assert!(point.point.is_some());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn comparison_is_symmetric_and_repeatable(
        ra in 0.0_f64..360.0,
        dec in -80.0_f64..80.0,
        offset_east in -0.004_f64..0.004,
        offset_north in -0.004_f64..0.004,
        half_east in 0.005_f64..0.02,
        half_north in 0.005_f64..0.02,
        left_pa in -360.0_f64..360.0,
        right_pa in -360.0_f64..360.0,
    ) {
        let anchor = eq(ra, dec);
        let left = local_rectangle(
            "left",
            anchor,
            half_east,
            half_north,
            left_pa,
            ImageParity::Direct,
        );
        let right = local_rectangle(
            "right",
            sky(anchor, offset_east, offset_north),
            half_east,
            half_north,
            right_pa,
            ImageParity::Direct,
        );

        let forward = compare_footprints(&left, &right).unwrap();
        let repeated = compare_footprints(&left, &right).unwrap();
        let reverse = compare_footprints(&right, &left).unwrap();

        prop_assert_eq!(forward, repeated);
        prop_assert!((0.0..=1.0).contains(&forward.normalized_coverage));
        prop_assert!((forward.normalized_coverage - reverse.normalized_coverage).abs() < 1e-10);
        prop_assert!((forward.centre_separation.radians() - reverse.centre_separation.radians()).abs() < 1e-14);
        prop_assert!(separation(forward.anchor, reverse.anchor).arcseconds() < 1e-7);
    }
}
