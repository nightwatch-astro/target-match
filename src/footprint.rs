// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Captured sky-footprint comparison and union geometry.
//!
//! A [`SkyFootprint`] carries an ordered solved boundary, its centre, solved
//! sky position angle, image parity, and caller-owned provenance. Pair
//! comparison projects both boundaries onto the spherical midpoint's gnomonic
//! plane. [`FootprintUnion`] keeps one tangent-plane anchor for its lifetime so
//! disconnected components, holes, point containment, and object coverage all
//! use the same geometry.
//!
//! All areas are measured in squared dimensionless gnomonic coordinates. The
//! polygon implementation is private; callers receive measurements and typed
//! evidence rather than planar geometry types.

use core::cmp::Ordering;

use geo::coordinate_position::{CoordPos, CoordinatePosition};
use geo::line_intersection::line_intersection;
use geo::orient::Direction;
use geo::{
    Area, BooleanOps, BoundingRect, Coord, Line, LineString, MultiPolygon, Orient, Point, Polygon,
    Rotate,
};
use skymath::{
    gnomonic_project, gnomonic_unproject, separation, transport_position_angle, Angle, Equatorial,
    GnomonicPoint,
};

use crate::error::{Error, Result};

const MAX_ROTATION_SAMPLES: usize = 1_000_000;
const MAX_ELLIPSE_SEGMENTS: usize = 65_536;
const FULL_COVERAGE_EPSILON: f64 = 1e-10;

/// Whether a solved image transform preserves or mirrors handedness.
///
/// Parity is reported separately from sky-axis rotation. It never changes the
/// footprint's polygon or rotation residual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ImageParity {
    /// The image transform preserves handedness.
    Direct,
    /// The image transform mirrors handedness.
    Mirrored,
}

/// Opaque caller-supplied identity for the evidence behind a footprint.
///
/// The value can be a stable image, panel, WCS-solution, or evidence digest.
/// `target-match` compares and returns it but does not interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String"))]
pub struct FootprintProvenance(String);

impl FootprintProvenance {
    /// Construct a non-empty provenance identity.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidFootprint`] when `value` is empty or only
    /// whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(Error::InvalidFootprint {
                provenance: value,
                reason: "provenance must not be empty".into(),
            });
        }
        Ok(Self(value))
    }

    /// Return the caller-supplied identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "serde")]
impl TryFrom<String> for FootprintProvenance {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

/// An ordered solved image boundary on the sky.
///
/// Corners may describe any simple polygon. The boundary must stay within the
/// gnomonic horizon of `centre`, contain `centre`, and use the same coordinate
/// epoch as `centre`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UncheckedSkyFootprint"))]
pub struct SkyFootprint {
    centre: Equatorial,
    corners: Vec<Equatorial>,
    sky_position_angle: Angle,
    parity: ImageParity,
    provenance: FootprintProvenance,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UncheckedSkyFootprint {
    centre: Equatorial,
    corners: Vec<Equatorial>,
    sky_position_angle: Angle,
    parity: ImageParity,
    provenance: FootprintProvenance,
}

#[cfg(feature = "serde")]
impl TryFrom<UncheckedSkyFootprint> for SkyFootprint {
    type Error = Error;

    fn try_from(value: UncheckedSkyFootprint) -> Result<Self> {
        Self::new(
            value.centre,
            value.corners,
            value.sky_position_angle,
            value.parity,
            value.provenance,
        )
    }
}

impl SkyFootprint {
    /// Construct and validate a solved footprint.
    ///
    /// A repeated final corner equal to the first is accepted and removed.
    ///
    /// # Errors
    ///
    /// Returns a typed footprint, epoch, or projection error for an invalid
    /// boundary.
    pub fn new(
        centre: Equatorial,
        mut corners: Vec<Equatorial>,
        sky_position_angle: Angle,
        parity: ImageParity,
        provenance: FootprintProvenance,
    ) -> Result<Self> {
        if corners.first() == corners.last() && corners.len() > 1 {
            corners.pop();
        }
        if corners.len() < 3 {
            return Err(invalid_footprint(
                &provenance,
                "boundary must contain at least three distinct corners",
            ));
        }
        if !sky_position_angle.degrees().is_finite() {
            return Err(invalid_footprint(
                &provenance,
                "sky position angle must be finite",
            ));
        }
        for corner in &corners {
            ensure_epoch(centre, *corner)?;
        }

        let footprint = Self {
            centre,
            corners,
            sky_position_angle: sky_position_angle.normalized_0_360(),
            parity,
            provenance,
        };
        let polygon = project_polygon(centre, &footprint)?;
        if polygon.coordinate_position(&Coord { x: 0.0, y: 0.0 }) == CoordPos::Outside {
            return Err(invalid_footprint(
                &footprint.provenance,
                "boundary does not contain its centre",
            ));
        }
        Ok(footprint)
    }

    /// Solved footprint centre.
    #[must_use]
    pub fn centre(&self) -> Equatorial {
        self.centre
    }

    /// Ordered sky boundary without a repeated closing corner.
    #[must_use]
    pub fn corners(&self) -> &[Equatorial] {
        &self.corners
    }

    /// Solved sky position angle, East of North and normalized to `[0, 360)`°.
    #[must_use]
    pub fn sky_position_angle(&self) -> Angle {
        self.sky_position_angle
    }

    /// Solved image parity.
    #[must_use]
    pub fn parity(&self) -> ImageParity {
        self.parity
    }

    /// Evidence identity supplied by the caller.
    #[must_use]
    pub fn provenance(&self) -> &FootprintProvenance {
        &self.provenance
    }

    /// Longest great-circle separation between any two boundary corners.
    #[must_use]
    pub fn diagonal(&self) -> Angle {
        let mut maximum = 0.0_f64;
        for (index, left) in self.corners.iter().enumerate() {
            for right in &self.corners[index + 1..] {
                maximum = maximum.max(separation(*left, *right).radians());
            }
        }
        Angle::from_radians(maximum)
    }
}

/// Measured relationship between two captured footprints.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FootprintComparison {
    /// Spherical-midpoint anchor of the common gnomonic plane.
    pub anchor: Equatorial,
    /// Left footprint area in the common plane.
    pub left_area: f64,
    /// Right footprint area in the common plane.
    pub right_area: f64,
    /// Intersection area in the common plane.
    pub intersection_area: f64,
    /// `intersection_area / min(left_area, right_area)`.
    pub normalized_coverage: f64,
    /// Great-circle separation between footprint centres.
    pub centre_separation: Angle,
    /// Smaller footprint's longest great-circle corner-to-corner diagonal.
    pub smaller_diagonal: Angle,
    /// Centre separation divided by `smaller_diagonal`.
    pub normalized_centre_separation: f64,
    /// Right solved sky axis relative to left after transport to `anchor`.
    ///
    /// The result is normalized modulo 180° into `[-90, 90)`.
    pub residual_sky_rotation: Angle,
    /// Whether the independently supplied image parities match.
    pub parity_match: bool,
}

/// Compare two footprints on their deterministic common plane.
///
/// # Errors
///
/// Returns a typed error for mismatched epochs, antipodal centres, projection
/// failure, or invalid projected geometry.
pub fn compare_footprints(
    left: &SkyFootprint,
    right: &SkyFootprint,
) -> Result<FootprintComparison> {
    PairGeometry::new(left, right)?.comparison(left, right)
}

/// Inclusive normalized-coverage band supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UncheckedCoverageBand"))]
pub struct CoverageBand {
    minimum: f64,
    maximum: f64,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UncheckedCoverageBand {
    minimum: f64,
    maximum: f64,
}

#[cfg(feature = "serde")]
impl TryFrom<UncheckedCoverageBand> for CoverageBand {
    type Error = Error;

    fn try_from(value: UncheckedCoverageBand) -> Result<Self> {
        Self::new(value.minimum, value.maximum)
    }
}

impl CoverageBand {
    /// Construct an inclusive coverage band within `[0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCoverageBand`] for non-finite, out-of-range, or
    /// reversed bounds.
    pub fn new(minimum: f64, maximum: f64) -> Result<Self> {
        if !minimum.is_finite()
            || !maximum.is_finite()
            || minimum < 0.0
            || maximum > 1.0
            || minimum > maximum
        {
            return Err(Error::InvalidCoverageBand(format!(
                "expected 0 <= minimum <= maximum <= 1, got {minimum}..={maximum}"
            )));
        }
        Ok(Self { minimum, maximum })
    }

    /// Inclusive lower bound.
    #[must_use]
    pub fn minimum(self) -> f64 {
        self.minimum
    }

    /// Inclusive upper bound.
    #[must_use]
    pub fn maximum(self) -> f64 {
        self.maximum
    }

    fn contains(self, coverage: f64) -> bool {
        (self.minimum..=self.maximum).contains(&coverage)
    }
}

/// Caller-controlled numerical search domain for coverage rotation intervals.
///
/// `sample_step` determines the narrowest interval the grid can discover.
/// Each detected transition is bisected to `tolerance`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UncheckedRotationSearch"))]
pub struct RotationSearch {
    minimum: Angle,
    maximum: Angle,
    sample_step: Angle,
    tolerance: Angle,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UncheckedRotationSearch {
    minimum: Angle,
    maximum: Angle,
    sample_step: Angle,
    tolerance: Angle,
}

#[cfg(feature = "serde")]
impl TryFrom<UncheckedRotationSearch> for RotationSearch {
    type Error = Error;

    fn try_from(value: UncheckedRotationSearch) -> Result<Self> {
        Self::new(
            value.minimum,
            value.maximum,
            value.sample_step,
            value.tolerance,
        )
    }
}

impl RotationSearch {
    /// Construct a finite increasing search domain and positive resolutions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRotationSearch`] when the domain or resolutions
    /// are invalid or require more than one million samples.
    pub fn new(
        minimum: Angle,
        maximum: Angle,
        sample_step: Angle,
        tolerance: Angle,
    ) -> Result<Self> {
        let values = [
            minimum.degrees(),
            maximum.degrees(),
            sample_step.degrees(),
            tolerance.degrees(),
        ];
        if values.iter().any(|value| !value.is_finite())
            || values[0] >= values[1]
            || values[2] <= 0.0
            || values[3] <= 0.0
            || values[3] > values[2]
        {
            return Err(Error::InvalidRotationSearch(format!(
                "expected finite min < max and 0 < tolerance <= sample step, got {}..{} step {} tolerance {} degrees",
                values[0], values[1], values[2], values[3]
            )));
        }
        checked_rotation_sample_count(values[0], values[1], values[2])?;
        Ok(Self {
            minimum,
            maximum,
            sample_step,
            tolerance,
        })
    }

    /// Inclusive search-domain start.
    #[must_use]
    pub fn minimum(self) -> Angle {
        self.minimum
    }

    /// Inclusive search-domain end.
    #[must_use]
    pub fn maximum(self) -> Angle {
        self.maximum
    }

    /// Grid spacing used to discover intervals.
    #[must_use]
    pub fn sample_step(self) -> Angle {
        self.sample_step
    }

    /// Maximum bisection width for a discovered transition.
    #[must_use]
    pub fn tolerance(self) -> Angle {
        self.tolerance
    }
}

/// Closed residual-rotation interval whose coverage lies inside a band.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RotationInterval {
    /// Inclusive interval start in the caller's rotation domain.
    pub start: Angle,
    /// Inclusive interval end in the caller's rotation domain.
    pub end: Angle,
}

/// Measure normalized coverage at a caller-supplied residual sky rotation.
///
/// The right footprint rotates about its own projected centre from its observed
/// transported residual to `residual`. Parity remains unchanged and is not
/// folded into this value.
pub fn coverage_at_residual_rotation(
    left: &SkyFootprint,
    right: &SkyFootprint,
    residual: Angle,
) -> Result<f64> {
    if !residual.degrees().is_finite() {
        return Err(Error::InvalidRotationSearch(
            "residual rotation must be finite".into(),
        ));
    }
    PairGeometry::new(left, right)?.coverage_at(residual.degrees())
}

/// Find closed residual-rotation intervals whose coverage lies in `band`.
///
/// The result can contain multiple disjoint intervals. Discovery resolution is
/// controlled by [`RotationSearch::sample_step`]; transition endpoints are
/// refined to [`RotationSearch::tolerance`].
pub fn coverage_rotation_intervals(
    left: &SkyFootprint,
    right: &SkyFootprint,
    band: CoverageBand,
    search: RotationSearch,
) -> Result<Vec<RotationInterval>> {
    let pair = PairGeometry::new(left, right)?;
    let minimum = search.minimum.degrees();
    let maximum = search.maximum.degrees();
    let step = search.sample_step.degrees();
    let tolerance = search.tolerance.degrees();

    let sample_count = checked_rotation_sample_count(minimum, maximum, step)?;
    let mut samples = Vec::with_capacity(sample_count);
    samples.push(minimum);
    for index in 1..sample_count - 1 {
        let angle = minimum + step * index as f64;
        if angle > *samples.last().expect("minimum sample exists") && angle < maximum {
            samples.push(angle);
        }
    }
    samples.push(maximum);

    let mut intervals = Vec::new();
    let mut previous_angle = samples[0];
    let mut previous_inside = band.contains(pair.coverage_at(previous_angle)?);
    let mut interval_start = previous_inside.then_some(previous_angle);

    for current_angle in samples.into_iter().skip(1) {
        let current_inside = band.contains(pair.coverage_at(current_angle)?);
        if previous_inside != current_inside {
            let transition = bisect_transition(
                &pair,
                band,
                previous_angle,
                current_angle,
                previous_inside,
                tolerance,
            )?;
            if current_inside {
                interval_start = Some(transition);
            } else if let Some(start) = interval_start.take() {
                intervals.push(RotationInterval {
                    start: Angle::from_degrees(start),
                    end: Angle::from_degrees(transition),
                });
            }
        }
        previous_angle = current_angle;
        previous_inside = current_inside;
    }

    if previous_inside {
        intervals.push(RotationInterval {
            start: Angle::from_degrees(interval_start.unwrap_or(minimum)),
            end: Angle::from_degrees(maximum),
        });
    }
    Ok(intervals)
}

/// Position of a point relative to a footprint union or one of its members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Containment {
    /// The point is outside the captured area.
    Outside,
    /// The point lies on a captured boundary and counts as covered.
    Boundary,
    /// The point lies inside the captured area.
    Interior,
}

impl Containment {
    /// Whether the point counts as covered under the inclusive-boundary rule.
    #[must_use]
    pub fn is_covered(self) -> bool {
        self != Self::Outside
    }
}

/// Point-containment evidence for one input panel.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PanelContainmentEvidence {
    /// Caller-supplied panel or image evidence identity.
    pub provenance: FootprintProvenance,
    /// Position relative to this panel's footprint.
    pub containment: Containment,
}

/// Point-containment result for the captured union.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointContainmentEvidence {
    /// Position relative to the complete captured union.
    pub containment: Containment,
    /// Stable indices of union components that contain or bound the point.
    pub component_indices: Vec<usize>,
    /// Per-panel evidence, sorted by provenance.
    pub panels: Vec<PanelContainmentEvidence>,
}

/// A tangent-plane ellipse on the sky.
///
/// The semi-axis angles are converted to gnomonic radii around `centre`. The
/// sampled boundary is then projected to a union's persisted anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UncheckedSkyEllipse"))]
pub struct SkyEllipse {
    centre: Equatorial,
    semi_major: Angle,
    semi_minor: Angle,
    sky_position_angle: Angle,
    segments: usize,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UncheckedSkyEllipse {
    centre: Equatorial,
    semi_major: Angle,
    semi_minor: Angle,
    sky_position_angle: Angle,
    segments: usize,
}

#[cfg(feature = "serde")]
impl TryFrom<UncheckedSkyEllipse> for SkyEllipse {
    type Error = Error;

    fn try_from(value: UncheckedSkyEllipse) -> Result<Self> {
        Self::new(
            value.centre,
            value.semi_major,
            value.semi_minor,
            value.sky_position_angle,
            value.segments,
        )
    }
}

impl SkyEllipse {
    /// Construct a sampled sky ellipse.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEllipse`] unless both semi-axes are finite,
    /// positive, below 90°, ordered major then minor, and sampled with 8 through
    /// 65,536 segments.
    pub fn new(
        centre: Equatorial,
        semi_major: Angle,
        semi_minor: Angle,
        sky_position_angle: Angle,
        segments: usize,
    ) -> Result<Self> {
        let major = semi_major.radians();
        let minor = semi_minor.radians();
        if !major.is_finite()
            || !minor.is_finite()
            || !sky_position_angle.degrees().is_finite()
            || major <= 0.0
            || minor <= 0.0
            || major < minor
            || major >= core::f64::consts::FRAC_PI_2
            || !(8..=MAX_ELLIPSE_SEGMENTS).contains(&segments)
        {
            return Err(Error::InvalidEllipse(format!(
                "expected 0 < semi_minor <= semi_major < 90 degrees and 8..={MAX_ELLIPSE_SEGMENTS} segments"
            )));
        }
        Ok(Self {
            centre,
            semi_major,
            semi_minor,
            sky_position_angle: sky_position_angle.normalized_0_360(),
            segments,
        })
    }

    /// Ellipse centre.
    #[must_use]
    pub fn centre(self) -> Equatorial {
        self.centre
    }

    /// Major semi-axis.
    #[must_use]
    pub fn semi_major(self) -> Angle {
        self.semi_major
    }

    /// Minor semi-axis.
    #[must_use]
    pub fn semi_minor(self) -> Angle {
        self.semi_minor
    }

    /// Major-axis position angle East of North.
    #[must_use]
    pub fn sky_position_angle(self) -> Angle {
        self.sky_position_angle
    }

    /// Number of boundary segments used for intersection.
    #[must_use]
    pub fn segments(self) -> usize {
        self.segments
    }
}

/// Caller-supplied object geometry to measure against a captured union.
#[derive(Debug, Clone, Copy)]
pub enum ObjectShape<'a> {
    /// Point-like or unknown-extent object.
    Point(Equatorial),
    /// Ordered object boundary supplied as a validated footprint.
    Footprint(&'a SkyFootprint),
    /// Sampled extended-object ellipse.
    Ellipse(&'a SkyEllipse),
}

/// Area evidence for one disconnected captured-union component.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentCoverageEvidence {
    /// Stable component index.
    pub component_index: usize,
    /// Intersection area with the object geometry.
    pub intersection_area: f64,
}

/// Area evidence for one input panel.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PanelCoverageEvidence {
    /// Caller-supplied panel or image evidence identity.
    pub provenance: FootprintProvenance,
    /// Intersection area with the object geometry.
    pub intersection_area: f64,
}

/// Captured-union coverage state for an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CoverageState {
    /// The object has zero captured area, or a point is outside the union.
    None,
    /// Some but not all object area is captured.
    Partial,
    /// All object area is captured, or a point is inside/on the boundary.
    Full,
}

/// Measured object coverage and its component/panel evidence.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectCoverageEvidence {
    /// Overall object coverage classification.
    pub state: CoverageState,
    /// Captured area divided by object area; absent for point objects.
    pub covered_fraction: Option<f64>,
    /// Point evidence; present only for [`ObjectShape::Point`].
    pub point: Option<PointContainmentEvidence>,
    /// Positive-area intersections by stable union component.
    pub components: Vec<ComponentCoverageEvidence>,
    /// Positive-area intersections by input panel, sorted by provenance.
    pub panels: Vec<PanelCoverageEvidence>,
}

/// Hole-aware union of captured footprints on one persisted gnomonic plane.
#[derive(Debug, Clone)]
pub struct FootprintUnion {
    anchor: Equatorial,
    geometry: MultiPolygon<f64>,
    panels: Vec<ProjectedPanel>,
}

impl FootprintUnion {
    /// Build a union while preserving disconnected components and interior holes.
    ///
    /// Input order does not affect the anchor, component ordering, or panel
    /// evidence ordering. Provenance identities must be unique.
    ///
    /// # Errors
    ///
    /// Returns a typed empty-set, duplicate-provenance, epoch, antipodal,
    /// projection, or invalid-geometry error.
    pub fn new(footprints: &[SkyFootprint]) -> Result<Self> {
        if footprints.is_empty() {
            return Err(Error::EmptyFootprintSet);
        }

        let mut ordered: Vec<&SkyFootprint> = footprints.iter().collect();
        ordered.sort_by(|left, right| left.provenance.cmp(&right.provenance));
        for pair in ordered.windows(2) {
            if pair[0].provenance == pair[1].provenance {
                return Err(Error::DuplicateFootprintProvenance(
                    pair[0].provenance.0.clone(),
                ));
            }
        }

        let centres: Vec<Equatorial> = ordered.iter().map(|footprint| footprint.centre).collect();
        let anchor = common_anchor(&centres)?;
        let mut panels = Vec::with_capacity(ordered.len());
        let mut union: Option<MultiPolygon<f64>> = None;

        for footprint in ordered {
            let polygon = project_polygon(anchor, footprint)?;
            union = Some(match union {
                None => MultiPolygon(vec![polygon.clone()]),
                Some(current) => current.union(&MultiPolygon(vec![polygon.clone()])),
            });
            panels.push(ProjectedPanel {
                provenance: footprint.provenance.clone(),
                polygon,
            });
        }

        let mut components = union.expect("non-empty input creates a union").0;
        components.sort_by(compare_polygons);
        Ok(Self {
            anchor,
            geometry: MultiPolygon(components),
            panels,
        })
    }

    /// Persisted common-plane anchor used by all union operations.
    #[must_use]
    pub fn anchor(&self) -> Equatorial {
        self.anchor
    }

    /// Total captured area in squared gnomonic coordinates.
    #[must_use]
    pub fn area(&self) -> f64 {
        self.geometry.unsigned_area()
    }

    /// Number of disconnected captured components.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.geometry.0.len()
    }

    /// Total number of interior holes across all components.
    #[must_use]
    pub fn hole_count(&self) -> usize {
        self.geometry
            .0
            .iter()
            .map(|polygon| polygon.interiors().len())
            .sum()
    }

    /// Measure inclusive-boundary point containment with component/panel evidence.
    pub fn contains_point(&self, point: Equatorial) -> Result<PointContainmentEvidence> {
        let projected = project_position(self.anchor, point, "object point")?;
        let coordinate = Coord {
            x: projected.east,
            y: projected.north,
        };
        let union_containment = containment(self.geometry.coordinate_position(&coordinate));

        let component_indices = self
            .geometry
            .0
            .iter()
            .enumerate()
            .filter_map(|(index, polygon)| {
                let position = containment(polygon.coordinate_position(&coordinate));
                position.is_covered().then_some(index)
            })
            .collect();
        let panels = self
            .panels
            .iter()
            .map(|panel| PanelContainmentEvidence {
                provenance: panel.provenance.clone(),
                containment: containment(panel.polygon.coordinate_position(&coordinate)),
            })
            .collect();

        Ok(PointContainmentEvidence {
            containment: union_containment,
            component_indices,
            panels,
        })
    }

    /// Measure point, footprint, or ellipse coverage against the captured union.
    pub fn measure_object(&self, object: ObjectShape<'_>) -> Result<ObjectCoverageEvidence> {
        match object {
            ObjectShape::Point(point) => {
                let evidence = self.contains_point(point)?;
                let state = if evidence.containment.is_covered() {
                    CoverageState::Full
                } else {
                    CoverageState::None
                };
                Ok(ObjectCoverageEvidence {
                    state,
                    covered_fraction: None,
                    point: Some(evidence),
                    components: Vec::new(),
                    panels: Vec::new(),
                })
            }
            ObjectShape::Footprint(footprint) => {
                let polygon = project_polygon(self.anchor, footprint)?;
                self.measure_polygon(&polygon)
            }
            ObjectShape::Ellipse(ellipse) => {
                let polygon = project_ellipse(self.anchor, ellipse)?;
                self.measure_polygon(&polygon)
            }
        }
    }

    fn measure_polygon(&self, object: &Polygon<f64>) -> Result<ObjectCoverageEvidence> {
        let object_area = object.unsigned_area();
        if !object_area.is_finite() || object_area <= 0.0 {
            return Err(Error::InvalidObjectGeometry(
                "projected object area must be finite and positive".into(),
            ));
        }
        let object_multi = MultiPolygon(vec![object.clone()]);
        let intersection_area = self.geometry.intersection(&object_multi).unsigned_area();
        let covered_fraction = (intersection_area / object_area).clamp(0.0, 1.0);
        let state = if intersection_area <= 0.0 {
            CoverageState::None
        } else if covered_fraction >= 1.0 - FULL_COVERAGE_EPSILON {
            CoverageState::Full
        } else {
            CoverageState::Partial
        };

        let components = self
            .geometry
            .0
            .iter()
            .enumerate()
            .filter_map(|(component_index, component)| {
                let area = component.intersection(object).unsigned_area();
                (area > 0.0).then_some(ComponentCoverageEvidence {
                    component_index,
                    intersection_area: area,
                })
            })
            .collect();
        let panels = self
            .panels
            .iter()
            .filter_map(|panel| {
                let area = panel.polygon.intersection(object).unsigned_area();
                (area > 0.0).then_some(PanelCoverageEvidence {
                    provenance: panel.provenance.clone(),
                    intersection_area: area,
                })
            })
            .collect();

        Ok(ObjectCoverageEvidence {
            state,
            covered_fraction: Some(covered_fraction),
            point: None,
            components,
            panels,
        })
    }
}

#[derive(Debug, Clone)]
struct ProjectedPanel {
    provenance: FootprintProvenance,
    polygon: Polygon<f64>,
}

struct PairGeometry {
    anchor: Equatorial,
    left: Polygon<f64>,
    right: Polygon<f64>,
    right_centre: Point<f64>,
    left_area: f64,
    right_area: f64,
    observed_residual_degrees: f64,
}

impl PairGeometry {
    fn new(left: &SkyFootprint, right: &SkyFootprint) -> Result<Self> {
        let anchor = common_anchor(&[left.centre, right.centre])?;
        let left_polygon = project_polygon(anchor, left)?;
        let right_polygon = project_polygon(anchor, right)?;
        let right_centre = project_position(anchor, right.centre, right.provenance.as_str())?;
        let left_axis = transport_position_angle(left.centre, anchor, left.sky_position_angle)
            .ok_or(Error::AntipodalGeometry)?;
        let right_axis = transport_position_angle(right.centre, anchor, right.sky_position_angle)
            .ok_or(Error::AntipodalGeometry)?;

        Ok(Self {
            anchor,
            left_area: left_polygon.unsigned_area(),
            right_area: right_polygon.unsigned_area(),
            left: left_polygon,
            right: right_polygon,
            right_centre: Point::new(right_centre.east, right_centre.north),
            observed_residual_degrees: axis_residual_degrees(
                left_axis.degrees(),
                right_axis.degrees(),
            ),
        })
    }

    fn comparison(&self, left: &SkyFootprint, right: &SkyFootprint) -> Result<FootprintComparison> {
        let smaller_area = self.left_area.min(self.right_area);
        let intersection_area = self.left.intersection(&self.right).unsigned_area();
        let smaller_diagonal = if left.diagonal().radians() <= right.diagonal().radians() {
            left.diagonal()
        } else {
            right.diagonal()
        };
        if smaller_area <= 0.0 || smaller_diagonal.radians() <= 0.0 {
            return Err(Error::InvalidObjectGeometry(
                "footprint area and diagonal must be positive".into(),
            ));
        }
        let centre_separation = separation(left.centre, right.centre);
        Ok(FootprintComparison {
            anchor: self.anchor,
            left_area: self.left_area,
            right_area: self.right_area,
            intersection_area,
            normalized_coverage: (intersection_area / smaller_area).clamp(0.0, 1.0),
            centre_separation,
            smaller_diagonal,
            normalized_centre_separation: centre_separation.radians() / smaller_diagonal.radians(),
            residual_sky_rotation: Angle::from_degrees(self.observed_residual_degrees),
            parity_match: left.parity == right.parity,
        })
    }

    fn coverage_at(&self, residual_degrees: f64) -> Result<f64> {
        if !residual_degrees.is_finite() {
            return Err(Error::InvalidRotationSearch(
                "residual rotation must be finite".into(),
            ));
        }
        let delta = residual_degrees - self.observed_residual_degrees;
        // Sky position angle increases clockwise in an East/North plane; geo's
        // positive rotation is counter-clockwise.
        let rotated = self.right.rotate_around_point(-delta, self.right_centre);
        let intersection_area = self.left.intersection(&rotated).unsigned_area();
        Ok((intersection_area / self.left_area.min(self.right_area)).clamp(0.0, 1.0))
    }
}

fn bisect_transition(
    pair: &PairGeometry,
    band: CoverageBand,
    mut lower: f64,
    mut upper: f64,
    lower_inside: bool,
    tolerance: f64,
) -> Result<f64> {
    while upper - lower > tolerance {
        let middle = lower + (upper - lower) / 2.0;
        if band.contains(pair.coverage_at(middle)?) == lower_inside {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Ok(if lower_inside { lower } else { upper })
}

fn checked_rotation_sample_count(minimum: f64, maximum: f64, step: f64) -> Result<usize> {
    let span = maximum - minimum;
    let intervals = span / step;
    let maximum_intervals = (MAX_ROTATION_SAMPLES - 1) as f64;
    let required_intervals = intervals.ceil().max(1.0);
    if !span.is_finite() || !intervals.is_finite() || required_intervals > maximum_intervals {
        return Err(Error::InvalidRotationSearch(format!(
            "search exceeds the maximum of {MAX_ROTATION_SAMPLES} samples"
        )));
    }
    Ok(required_intervals as usize + 1)
}

fn common_anchor(positions: &[Equatorial]) -> Result<Equatorial> {
    let first = *positions.first().ok_or(Error::EmptyFootprintSet)?;
    for position in &positions[1..] {
        ensure_epoch(first, *position)?;
    }

    let mut ordered = positions.to_vec();
    ordered.sort_by(|left, right| {
        left.ra()
            .degrees()
            .total_cmp(&right.ra().degrees())
            .then_with(|| left.dec().degrees().total_cmp(&right.dec().degrees()))
    });
    let sum = ordered.iter().fold([0.0_f64; 3], |mut sum, position| {
        let (ra, dec) = (position.ra().radians(), position.dec().radians());
        let vector = [dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin()];
        sum[0] += vector[0];
        sum[1] += vector[1];
        sum[2] += vector[2];
        sum
    });
    let norm = sum[0].hypot(sum[1]).hypot(sum[2]);
    if !norm.is_finite() || norm <= 1e-12 {
        return Err(Error::AntipodalGeometry);
    }
    let vector = [sum[0] / norm, sum[1] / norm, sum[2] / norm];
    let mut right_ascension = vector[1].atan2(vector[0]).to_degrees().rem_euclid(360.0);
    if right_ascension >= 360.0 {
        right_ascension = 0.0;
    }
    Equatorial::at_epoch(
        Angle::from_degrees(right_ascension),
        Angle::from_radians(vector[2].clamp(-1.0, 1.0).asin()),
        first.epoch(),
    )
    .map_err(|error| Error::InvalidObjectGeometry(error.to_string()))
}

fn project_polygon(anchor: Equatorial, footprint: &SkyFootprint) -> Result<Polygon<f64>> {
    ensure_epoch(anchor, footprint.centre)?;
    let coordinates: Result<Vec<Coord<f64>>> = footprint
        .corners
        .iter()
        .map(|corner| {
            let point = project_position(anchor, *corner, footprint.provenance.as_str())?;
            Ok(Coord {
                x: point.east,
                y: point.north,
            })
        })
        .collect();
    polygon_from_coordinates(coordinates?, &footprint.provenance)
}

fn polygon_from_coordinates(
    coordinates: Vec<Coord<f64>>,
    provenance: &FootprintProvenance,
) -> Result<Polygon<f64>> {
    if coordinates.len() < 3 {
        return Err(invalid_footprint(
            provenance,
            "projected boundary must contain at least three corners",
        ));
    }
    for index in 0..coordinates.len() {
        if coordinates[index] == coordinates[(index + 1) % coordinates.len()] {
            return Err(invalid_footprint(
                provenance,
                "boundary contains a zero-length edge",
            ));
        }
    }
    for left_index in 0..coordinates.len() {
        let left = Line::new(
            coordinates[left_index],
            coordinates[(left_index + 1) % coordinates.len()],
        );
        for right_index in left_index + 1..coordinates.len() {
            let adjacent = right_index == left_index + 1
                || (left_index == 0 && right_index == coordinates.len() - 1);
            if adjacent {
                continue;
            }
            let right = Line::new(
                coordinates[right_index],
                coordinates[(right_index + 1) % coordinates.len()],
            );
            if line_intersection(left, right).is_some() {
                return Err(invalid_footprint(provenance, "boundary self-intersects"));
            }
        }
    }

    let mut closed = coordinates;
    closed.push(closed[0]);
    let polygon = Polygon::new(LineString::new(closed), Vec::new()).orient(Direction::Default);
    let area = polygon.unsigned_area();
    if !area.is_finite() || area <= 0.0 {
        return Err(invalid_footprint(
            provenance,
            "projected boundary area must be finite and positive",
        ));
    }
    Ok(polygon)
}

fn project_ellipse(anchor: Equatorial, ellipse: &SkyEllipse) -> Result<Polygon<f64>> {
    ensure_epoch(anchor, ellipse.centre)?;
    let major = ellipse.semi_major.radians().tan();
    let minor = ellipse.semi_minor.radians().tan();
    let position_angle = ellipse.sky_position_angle.radians();
    let (sin_pa, cos_pa) = position_angle.sin_cos();
    let mut coordinates = Vec::with_capacity(ellipse.segments);

    for index in 0..ellipse.segments {
        let phase = core::f64::consts::TAU * index as f64 / ellipse.segments as f64;
        let (sin_phase, cos_phase) = phase.sin_cos();
        let east = major * cos_phase * sin_pa + minor * sin_phase * cos_pa;
        let north = major * cos_phase * cos_pa - minor * sin_phase * sin_pa;
        let sky = gnomonic_unproject(ellipse.centre, GnomonicPoint { east, north })
            .ok_or_else(|| Error::ProjectionFailed("object ellipse".into()))?;
        let projected = project_position(anchor, sky, "object ellipse")?;
        coordinates.push(Coord {
            x: projected.east,
            y: projected.north,
        });
    }
    polygon_from_coordinates(coordinates, &FootprintProvenance("object ellipse".into()))
        .map_err(|error| Error::InvalidObjectGeometry(error.to_string()))
}

fn project_position(
    anchor: Equatorial,
    position: Equatorial,
    context: &str,
) -> Result<GnomonicPoint> {
    ensure_epoch(anchor, position)?;
    gnomonic_project(anchor, position).ok_or_else(|| Error::ProjectionFailed(context.to_owned()))
}

fn ensure_epoch(left: Equatorial, right: Equatorial) -> Result<()> {
    if left.epoch() == right.epoch() {
        Ok(())
    } else {
        Err(Error::FootprintEpochMismatch)
    }
}

fn axis_residual_degrees(left: f64, right: f64) -> f64 {
    (right - left + 90.0).rem_euclid(180.0) - 90.0
}

fn containment(position: CoordPos) -> Containment {
    match position {
        CoordPos::Outside => Containment::Outside,
        CoordPos::OnBoundary => Containment::Boundary,
        CoordPos::Inside => Containment::Interior,
    }
}

fn compare_polygons(left: &Polygon<f64>, right: &Polygon<f64>) -> Ordering {
    let left_rect = left
        .bounding_rect()
        .expect("validated polygon has a bounding rectangle");
    let right_rect = right
        .bounding_rect()
        .expect("validated polygon has a bounding rectangle");
    [
        left_rect.min().x.total_cmp(&right_rect.min().x),
        left_rect.min().y.total_cmp(&right_rect.min().y),
        left_rect.max().x.total_cmp(&right_rect.max().x),
        left_rect.max().y.total_cmp(&right_rect.max().y),
        left.unsigned_area().total_cmp(&right.unsigned_area()),
    ]
    .into_iter()
    .find(|ordering| *ordering != Ordering::Equal)
    .unwrap_or(Ordering::Equal)
}

fn invalid_footprint(provenance: &FootprintProvenance, reason: &str) -> Error {
    Error::InvalidFootprint {
        provenance: provenance.0.clone(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_residual_is_modulo_180() {
        assert_eq!(axis_residual_degrees(10.0, 190.0), 0.0);
        assert_eq!(axis_residual_degrees(350.0, 10.0), 20.0);
        assert_eq!(axis_residual_degrees(10.0, 100.0), -90.0);
    }
}
