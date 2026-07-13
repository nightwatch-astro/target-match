//! The matching engine: input trait, constraints, ranking, and a prebuilt index.
//!
//! A consumer's catalogue type implements [`SkyObject`] (position only — never a
//! name). A [`Constraint`] combines a [`Membership`] shape (circular, rectangle,
//! or rotated rectangle) with a [`Query`] mode. [`rank`] scans a slice; [`Matcher`]
//! builds a declination-sorted index once and answers repeated queries, returning
//! results identical to [`rank`].
//!
//! # Geometry
//!
//! Matching precesses the pointing to J2000 (via [`skymath::precess`]), then works
//! in the local tangent frame about the pointing: an object's offset is decomposed
//! from its great-circle separation and position angle (East of North, both from
//! `skymath`) into East/North components, which are rotated into the camera frame
//! for rectangular membership. The circumscribed circle pre-filters both rectangle
//! tests, so the tangent decomposition is only evaluated for objects near the
//! frame — never on the far side of the sky.

use core::cmp::Ordering;

use skymath::{position_angle, precess, separation, Angle, Epoch, Equatorial};

use crate::optics::{Field, RadiusPolicy};

/// A catalogue object that can be matched by sky position.
///
/// The trait exposes **only** a J2000 position — matching never reads a name or
/// designation. A caller's own type keeps its identity; a [`Match`] borrows it.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial};
/// use target_match::SkyObject;
///
/// struct Target {
///     name: &'static str,
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let m31 = Target { name: "M 31", ra: 10.6847, dec: 41.2688 };
/// assert!((m31.position().ra().degrees() - 10.6847).abs() < 1e-9);
/// ```
pub trait SkyObject {
    /// The object's J2000 equatorial position.
    fn position(&self) -> Equatorial;
}

/// The shape that decides whether an object is "in frame".
///
/// Usually built for you by a [`Constraint`] constructor; pass one directly to
/// [`is_framed`] to test a single object.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{is_framed, Membership, SkyObject};
///
/// struct Target {
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let m31 = Target { ra: 10.6847, dec: 41.2688 };
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
///
/// let shape = Membership::Circular { radius: Angle::from_degrees(1.0) };
/// assert!(is_framed(pointing, &m31, shape).in_frame);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Membership {
    /// Pure angular distance: in frame iff separation ≤ `radius`.
    Circular {
        /// Search radius.
        radius: Angle,
    },
    /// Axis-aligned rectangle of the given width×height field of view.
    Rectangle {
        /// `(width, height)` field of view.
        fov: (Angle, Angle),
    },
    /// Rectangle rotated by a camera position angle (degrees East of North).
    Rotated {
        /// `(width, height)` field of view.
        fov: (Angle, Angle),
        /// Camera position angle, East of North.
        position_angle: Angle,
    },
}

/// What to return from a match.
///
/// Set on a [`Constraint`] via [`Constraint::all`], [`Constraint::nearest_one`],
/// [`Constraint::nearest_n`], or [`Constraint::nearest_n_within`].
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{rank, Constraint, SkyObject};
///
/// struct Target {
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let catalog = [Target { ra: 10.6847, dec: 41.2688 }];
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
///
/// // `nearest_n(1)` sets the query to `Query::NearestN { n: 1, .. }`.
/// let hits = rank(pointing, &catalog, Constraint::circular(Angle::from_degrees(1.0)).nearest_n(1));
/// assert_eq!(hits.len(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Query {
    /// Every object inside the membership shape, ranked nearest-first.
    AllWithinField,
    /// The single nearest in-frame object.
    NearestOne,
    /// The `n` nearest objects by separation, optionally bounded by a radius.
    NearestN {
        /// Maximum number of results.
        n: usize,
        /// Optional maximum separation; unbounded when `None`.
        max_radius: Option<Angle>,
    },
}

/// A [`Membership`] shape combined with a [`Query`] mode (and the plate scale,
/// when known, so pixel offsets can be reported).
///
/// Build one with a shape constructor ([`within`](Constraint::within),
/// [`circular`](Constraint::circular), [`frame`](Constraint::frame),
/// [`frame_rotated`](Constraint::frame_rotated)), then optionally switch the
/// query mode ([`all`](Constraint::all), [`nearest_one`](Constraint::nearest_one),
/// [`nearest_n`](Constraint::nearest_n),
/// [`nearest_n_within`](Constraint::nearest_n_within)). Pass the result to
/// [`rank`], [`Matcher::query`], or [`is_framed`].
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};
///
/// struct Target {
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let catalog = [Target { ra: 10.6847, dec: 41.2688 }];
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
/// let field = Field::from_optics(Optics {
///     focal_mm: 800.0,
///     pixel_um: (3.76, 3.76),
///     binning: (1, 1),
///     pixels: (6248, 4176),
/// })
/// .unwrap();
///
/// let c = Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one();
/// assert_eq!(rank(pointing, &catalog, c).len(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Constraint {
    /// The in-frame shape.
    pub membership: Membership,
    /// The query mode.
    pub query: Query,
    /// Per-axis plate scale (arcsec/px), used only to fill pixel offsets.
    pub pixel_scale: Option<(f64, f64)>,
}

impl Constraint {
    /// Circular membership with a radius derived from a field under `policy`.
    ///
    /// # Example
    ///
    /// ```
    /// use target_match::{Constraint, Field, Optics, RadiusPolicy};
    ///
    /// let field = Field::from_optics(Optics {
    ///     focal_mm: 800.0,
    ///     pixel_um: (3.76, 3.76),
    ///     binning: (1, 1),
    ///     pixels: (6248, 4176),
    /// })
    /// .unwrap();
    /// let c = Constraint::within(&field, RadiusPolicy::Circumscribed);
    /// assert!(c.pixel_scale.is_some(), "the field's plate scale carries through");
    /// ```
    #[must_use]
    pub fn within(field: &Field, policy: RadiusPolicy) -> Self {
        Self {
            membership: Membership::Circular {
                radius: field.radius(policy),
            },
            query: Query::AllWithinField,
            pixel_scale: field.pixel_scale(),
        }
    }
    /// Circular membership with an explicit radius (no plate scale).
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::Constraint;
    ///
    /// let c = Constraint::circular(Angle::from_degrees(2.0));
    /// assert!(c.pixel_scale.is_none(), "no `Field`, so no plate scale");
    /// ```
    #[must_use]
    pub fn circular(radius: Angle) -> Self {
        Self {
            membership: Membership::Circular { radius },
            query: Query::AllWithinField,
            pixel_scale: None,
        }
    }
    /// Axis-aligned rectangular membership from a field's width×height.
    ///
    /// # Example
    ///
    /// ```
    /// use target_match::{Constraint, Field, Membership, Optics};
    ///
    /// let field = Field::from_optics(Optics {
    ///     focal_mm: 800.0,
    ///     pixel_um: (3.76, 3.76),
    ///     binning: (1, 1),
    ///     pixels: (6248, 4176),
    /// })
    /// .unwrap();
    /// let c = Constraint::frame(&field);
    /// assert!(matches!(c.membership, Membership::Rectangle { .. }));
    /// ```
    #[must_use]
    pub fn frame(field: &Field) -> Self {
        Self {
            membership: Membership::Rectangle {
                fov: (field.width(), field.height()),
            },
            query: Query::AllWithinField,
            pixel_scale: field.pixel_scale(),
        }
    }
    /// Rotated rectangular membership from a field and a camera position angle.
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::{Constraint, Field, Membership, Optics};
    ///
    /// let field = Field::from_optics(Optics {
    ///     focal_mm: 800.0,
    ///     pixel_um: (3.76, 3.76),
    ///     binning: (1, 1),
    ///     pixels: (6248, 4176),
    /// })
    /// .unwrap();
    /// let c = Constraint::frame_rotated(&field, Angle::from_degrees(15.0));
    /// assert!(matches!(c.membership, Membership::Rotated { .. }));
    /// ```
    #[must_use]
    pub fn frame_rotated(field: &Field, position_angle: Angle) -> Self {
        Self {
            membership: Membership::Rotated {
                fov: (field.width(), field.height()),
                position_angle,
            },
            query: Query::AllWithinField,
            pixel_scale: field.pixel_scale(),
        }
    }
    /// Set the query to all-within-field.
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::{Constraint, Query};
    ///
    /// let c = Constraint::circular(Angle::from_degrees(1.0)).nearest_one().all();
    /// assert_eq!(c.query, Query::AllWithinField);
    /// ```
    #[must_use]
    pub fn all(mut self) -> Self {
        self.query = Query::AllWithinField;
        self
    }
    /// Set the query to nearest-one.
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::{Constraint, Query};
    ///
    /// let c = Constraint::circular(Angle::from_degrees(1.0)).nearest_one();
    /// assert_eq!(c.query, Query::NearestOne);
    /// ```
    #[must_use]
    pub fn nearest_one(mut self) -> Self {
        self.query = Query::NearestOne;
        self
    }
    /// Set the query to the `n` nearest (unbounded).
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::{Constraint, Query};
    ///
    /// let c = Constraint::circular(Angle::from_degrees(1.0)).nearest_n(3);
    /// assert_eq!(c.query, Query::NearestN { n: 3, max_radius: None });
    /// ```
    #[must_use]
    pub fn nearest_n(mut self, n: usize) -> Self {
        self.query = Query::NearestN {
            n,
            max_radius: None,
        };
        self
    }
    /// Set the query to the `n` nearest within `max_radius`.
    ///
    /// # Example
    ///
    /// ```
    /// use skymath::Angle;
    /// use target_match::{Constraint, Query};
    ///
    /// let radius = Angle::from_degrees(1.0);
    /// let c = Constraint::circular(radius).nearest_n_within(3, radius);
    /// assert_eq!(c.query, Query::NearestN { n: 3, max_radius: Some(radius) });
    /// ```
    #[must_use]
    pub fn nearest_n_within(mut self, n: usize, max_radius: Angle) -> Self {
        self.query = Query::NearestN {
            n,
            max_radius: Some(max_radius),
        };
        self
    }
}

/// The offset of a matched object relative to the frame centre.
///
/// Carried on every [`Match`]. `frame` and `pixels` are only populated for
/// rectangular [`Membership`] (see [`Constraint::frame`],
/// [`Constraint::frame_rotated`]) — circular membership has no frame axes.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{rank, Constraint, Field, Optics, SkyObject};
///
/// struct Target {
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let catalog = [Target { ra: 10.6847, dec: 41.2688 }];
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
/// let field = Field::from_optics(Optics {
///     focal_mm: 800.0,
///     pixel_um: (3.76, 3.76),
///     binning: (1, 1),
///     pixels: (6248, 4176),
/// })
/// .unwrap();
///
/// let hits = rank(pointing, &catalog, Constraint::frame(&field));
/// let offset = hits[0].offset;
/// assert!(offset.frame.is_some(), "rectangular membership reports a frame-aligned offset");
/// assert!(offset.pixels.is_some(), "plate scale from `Optics` fills the pixel offset");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Offset {
    /// Sky-tangent offset `(East, North)` — always present.
    pub sky: (Angle, Angle),
    /// Frame-aligned offset `(x, y)`, present for rectangular membership.
    pub frame: Option<(Angle, Angle)>,
    /// Frame-aligned offset in pixels `(x, y)`, present when a plate scale is known.
    pub pixels: Option<(f64, f64)>,
}

/// A ranked match: a borrowed catalogue object plus its computed geometry.
///
/// Returned by [`rank`], [`Matcher::query`], and [`is_framed`].
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};
///
/// struct Target {
///     name: &'static str,
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let catalog = [Target { name: "M 31", ra: 10.6847, dec: 41.2688 }];
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
/// let field = Field::from_optics(Optics {
///     focal_mm: 800.0,
///     pixel_um: (3.76, 3.76),
///     binning: (1, 1),
///     pixels: (6248, 4176),
/// })
/// .unwrap();
///
/// let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed));
/// let m = &hits[0];
/// assert_eq!(m.object.name, "M 31");
/// assert!(m.in_frame);
/// assert!(m.separation.arcseconds() < 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match<'a, T> {
    /// The matched object, borrowed from the caller's slice or the [`Matcher`].
    pub object: &'a T,
    /// Great-circle separation from the frame centre.
    pub separation: Angle,
    /// Whether the object is inside the active membership shape.
    pub in_frame: bool,
    /// The object's offset relative to the frame centre.
    pub offset: Offset,
    /// Position angle from frame centre to the object (degrees East of North).
    pub position_angle: Angle,
}

/// Tangent-frame `(East, North)` offset in radians, decomposed from an
/// already-computed separation and position angle (the polar decomposition
/// `skymath::tangent_offset` performs, reusing this evaluation's sep/PA
/// instead of recomputing them).
fn tangent_components(sep: Angle, pa: Angle) -> (f64, f64) {
    let (s, r) = (pa.radians(), sep.radians());
    (r * s.sin(), r * s.cos())
}

fn rotate(east: f64, north: f64, pa_rad: f64) -> (f64, f64) {
    let (s, c) = pa_rad.sin_cos();
    (east * c - north * s, east * s + north * c)
}

/// The circumscribed-circle radius (radians) that bounds a membership shape.
fn bound_radius(m: Membership) -> f64 {
    match m {
        Membership::Circular { radius } => radius.radians(),
        Membership::Rectangle { fov } | Membership::Rotated { fov, .. } => {
            (fov.0.radians() / 2.0).hypot(fov.1.radians() / 2.0)
        }
    }
}

fn contains(sep: Angle, east: f64, north: f64, m: Membership) -> bool {
    match m {
        Membership::Circular { radius } => {
            let r = radius.radians();
            r.is_finite() && r >= 0.0 && sep.radians() <= r
        }
        Membership::Rectangle { fov } => within_rect(sep, east, north, fov, 0.0),
        Membership::Rotated {
            fov,
            position_angle,
        } => within_rect(sep, east, north, fov, position_angle.radians()),
    }
}

fn within_rect(sep: Angle, east: f64, north: f64, fov: (Angle, Angle), pa: f64) -> bool {
    let (hx, hy) = (fov.0.radians() / 2.0, fov.1.radians() / 2.0);
    let circum = hx.hypot(hy);
    // Circumscribed pre-filter (also rejects NaN separations).
    if sep.radians() > circum || sep.radians().is_nan() {
        return false;
    }
    let (x, y) = rotate(east, north, pa);
    x.abs() <= hx && y.abs() <= hy
}

fn build_offset(east: f64, north: f64, m: Membership, scale: Option<(f64, f64)>) -> Offset {
    let sky = (Angle::from_radians(east), Angle::from_radians(north));
    match m {
        Membership::Circular { .. } => Offset {
            sky,
            frame: None,
            pixels: None,
        },
        Membership::Rectangle { .. } | Membership::Rotated { .. } => {
            let pa = match m {
                Membership::Rotated { position_angle, .. } => position_angle.radians(),
                _ => 0.0,
            };
            let (x, y) = rotate(east, north, pa);
            let frame = Some((Angle::from_radians(x), Angle::from_radians(y)));
            let pixels = scale.map(|(sx, sy)| {
                (
                    Angle::from_radians(x).arcseconds() / sx,
                    Angle::from_radians(y).arcseconds() / sy,
                )
            });
            Offset { sky, frame, pixels }
        }
    }
}

fn evaluate<'a, T: SkyObject>(
    pointing: Equatorial,
    obj: &'a T,
    m: Membership,
    scale: Option<(f64, f64)>,
) -> Match<'a, T> {
    let pos = obj.position();
    let sep = separation(pointing, pos);
    let pa = position_angle(pointing, pos);
    let (east, north) = tangent_components(sep, pa);
    Match {
        object: obj,
        separation: sep,
        in_frame: contains(sep, east, north, m),
        offset: build_offset(east, north, m, scale),
        position_angle: pa,
    }
}

/// Whether `obj` should be kept for `query` given its evaluated match.
fn keep<T>(m: &Match<'_, T>, query: Query) -> bool {
    match query {
        Query::AllWithinField | Query::NearestOne => m.in_frame,
        Query::NearestN { max_radius, .. } => {
            max_radius.map_or(true, |r| m.separation.radians() <= r.radians())
        }
    }
}

/// Shared core: evaluate candidates, filter, rank, and truncate per the query.
fn rank_candidates<'a, T: SkyObject, I>(
    pointing: Equatorial,
    candidates: I,
    c: &Constraint,
) -> Vec<Match<'a, T>>
where
    I: Iterator<Item = (usize, &'a T)>,
{
    let mut scored: Vec<(usize, Match<'a, T>)> = candidates
        .map(|(i, o)| (i, evaluate(pointing, o, c.membership, c.pixel_scale)))
        .filter(|(_, m)| keep(m, c.query))
        .collect();
    scored.sort_by(|a, b| {
        a.1.separation
            .radians()
            .partial_cmp(&b.1.separation.radians())
            .unwrap_or(Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let mut out: Vec<Match<'a, T>> = scored.into_iter().map(|(_, m)| m).collect();
    match c.query {
        Query::NearestOne => out.truncate(1),
        Query::NearestN { n, .. } => out.truncate(n),
        Query::AllWithinField => {}
    }
    out
}

/// Rank a slice of objects against a pointing under a constraint (stateless scan).
///
/// The pointing is precessed to J2000 first. Results are ascending by separation
/// with ties broken by input order. For repeated queries against one catalogue,
/// build a [`Matcher`] instead — it returns identical results faster.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};
///
/// struct Target {
///     name: &'static str,
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let catalog = [
///     Target { name: "M 31", ra: 10.6847, dec: 41.2688 },
///     Target { name: "M 33", ra: 23.4621, dec: 30.6599 },
/// ];
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
/// let field = Field::from_optics(Optics {
///     focal_mm: 800.0,
///     pixel_um: (3.76, 3.76),
///     binning: (1, 1),
///     pixels: (6248, 4176),
/// })
/// .unwrap();
///
/// let hits = rank(pointing, &catalog, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
/// assert_eq!(hits[0].object.name, "M 31");
/// ```
#[must_use]
pub fn rank<T: SkyObject>(pointing: Equatorial, objects: &[T], c: Constraint) -> Vec<Match<'_, T>> {
    let p = precess(pointing, Epoch::J2000);
    rank_candidates(p, objects.iter().enumerate(), &c)
}

/// Evaluate a single object against a frame, returning its membership + geometry.
///
/// The pointing is precessed to J2000 first. Unlike [`rank`], no filtering or
/// ranking is applied — the returned [`Match`] always describes `object`.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{is_framed, Membership, SkyObject};
///
/// struct Target {
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let m31 = Target { ra: 10.6847, dec: 41.2688 };
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
///
/// let m = is_framed(pointing, &m31, Membership::Circular { radius: Angle::from_degrees(1.0) });
/// assert!(m.in_frame);
/// ```
#[must_use]
pub fn is_framed<T: SkyObject>(
    pointing: Equatorial,
    object: &T,
    membership: Membership,
) -> Match<'_, T> {
    let p = precess(pointing, Epoch::J2000);
    evaluate(p, object, membership, None)
}

/// A prebuilt, declination-sorted index for repeated queries against one catalogue.
///
/// Produces results identical to [`rank`] for the same objects, pointing, and
/// constraint — the index is a performance optimization only. Build it once, then
/// [`query`](Matcher::query) many pointings.
///
/// # Example
///
/// ```
/// use skymath::{Angle, Equatorial, ParseMode};
/// use target_match::{Constraint, Field, Matcher, Membership, Optics, RadiusPolicy, SkyObject};
///
/// struct Target {
///     name: &'static str,
///     ra: f64,
///     dec: f64,
/// }
/// impl SkyObject for Target {
///     fn position(&self) -> Equatorial {
///         Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
///     }
/// }
///
/// let matcher = Matcher::from_objects(vec![
///     Target { name: "M 31", ra: 10.6847, dec: 41.2688 },
///     Target { name: "M 33", ra: 23.4621, dec: 30.6599 },
/// ]);
/// assert_eq!(matcher.objects().len(), 2);
///
/// let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict).unwrap();
/// let field = Field::from_optics(Optics {
///     focal_mm: 800.0,
///     pixel_um: (3.76, 3.76),
///     binning: (1, 1),
///     pixels: (6248, 4176),
/// })
/// .unwrap();
///
/// let hits = matcher.query(pointing, Constraint::within(&field, RadiusPolicy::Circumscribed).nearest_one());
/// assert_eq!(hits[0].object.name, "M 31");
///
/// let m = matcher.is_framed(
///     pointing,
///     &matcher.objects()[0],
///     Membership::Circular { radius: Angle::from_degrees(1.0) },
/// );
/// assert!(m.in_frame);
/// ```
pub struct Matcher<T> {
    storage: Vec<T>,
    /// `(declination_degrees, original_index)` sorted by declination.
    sorted: Vec<(f64, usize)>,
}

impl<T: SkyObject> Matcher<T> {
    /// Build an index from a set of objects (original order is preserved for
    /// tie-breaking and [`objects`](Matcher::objects)).
    #[must_use]
    pub fn from_objects(objects: Vec<T>) -> Self {
        let mut sorted: Vec<(f64, usize)> = objects
            .iter()
            .enumerate()
            .map(|(i, o)| (o.position().dec().degrees(), i))
            .collect();
        sorted.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        Self {
            storage: objects,
            sorted,
        }
    }

    /// The stored objects, in their original insertion order.
    #[must_use]
    pub fn objects(&self) -> &[T] {
        &self.storage
    }

    /// Query the index for a pointing under a constraint.
    #[must_use]
    pub fn query(&self, pointing: Equatorial, c: Constraint) -> Vec<Match<'_, T>> {
        let p = precess(pointing, Epoch::J2000);
        let r = match c.query {
            Query::NearestN { max_radius, .. } => max_radius.map_or(f64::INFINITY, |a| a.radians()),
            _ => bound_radius(c.membership),
        };
        let idxs = self.band(p.dec().degrees(), r);
        rank_candidates(p, idxs.into_iter().map(|i| (i, &self.storage[i])), &c)
    }

    /// Evaluate a single stored-or-external object against a frame (see [`is_framed`]).
    #[must_use]
    pub fn is_framed<'a>(
        &self,
        pointing: Equatorial,
        object: &'a T,
        m: Membership,
    ) -> Match<'a, T> {
        is_framed(pointing, object, m)
    }

    /// Original indices whose declination lies within `r_rad` of `dec0_deg`.
    fn band(&self, dec0_deg: f64, r_rad: f64) -> Vec<usize> {
        if r_rad.is_infinite() && r_rad > 0.0 {
            return self.sorted.iter().map(|&(_, i)| i).collect();
        }
        if !r_rad.is_finite() || r_rad < 0.0 {
            return Vec::new();
        }
        let r_deg = r_rad.to_degrees();
        let (lo, hi) = (dec0_deg - r_deg, dec0_deg + r_deg);
        let start = self.sorted.partition_point(|&(d, _)| d < lo);
        let end = self.sorted.partition_point(|&(d, _)| d <= hi);
        self.sorted[start..end].iter().map(|&(_, i)| i).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Obj {
        name: &'static str,
        ra: f64,
        dec: f64,
    }
    impl SkyObject for Obj {
        fn position(&self) -> Equatorial {
            Equatorial::j2000(Angle::from_degrees(self.ra), Angle::from_degrees(self.dec)).unwrap()
        }
    }

    fn catalog() -> Vec<Obj> {
        vec![
            Obj {
                name: "M 31",
                ra: 10.6847,
                dec: 41.2688,
            },
            Obj {
                name: "M 110",
                ra: 10.0921,
                dec: 41.6853,
            },
            Obj {
                name: "M 33",
                ra: 23.4621,
                dec: 30.6599,
            },
            Obj {
                name: "M 42",
                ra: 83.8221,
                dec: -5.3911,
            },
        ]
    }

    fn m31() -> Equatorial {
        Equatorial::j2000(Angle::from_degrees(10.6847), Angle::from_degrees(41.2688)).unwrap()
    }

    #[test]
    fn nearest_one_circular() {
        let cat = catalog();
        let c = Constraint::circular(Angle::from_degrees(2.0)).nearest_one();
        let hits = rank(m31(), &cat, c);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].object.name, "M 31");
        assert!(hits[0].separation.arcseconds() < 1.0);
    }

    #[test]
    fn all_within_field_ranked() {
        let cat = catalog();
        // ~1° radius keeps M31 (self) + M110 (~0.62°); excludes M33/M42.
        let c = Constraint::circular(Angle::from_degrees(1.0)).all();
        let hits = rank(m31(), &cat, c);
        assert_eq!(
            hits.len(),
            2,
            "{hits:?}",
            hits = hits.iter().map(|h| h.object.name).collect::<Vec<_>>()
        );
        assert_eq!(hits[0].object.name, "M 31");
        assert_eq!(hits[1].object.name, "M 110");
        assert!(hits[0].separation.radians() <= hits[1].separation.radians());
    }

    #[test]
    fn nearest_n_bounds_and_counts() {
        let cat = catalog();
        let c = Constraint::circular(Angle::from_degrees(1.0)).nearest_n(3);
        let hits = rank(m31(), &cat, c);
        assert_eq!(hits.len(), 3, "top-3 by separation regardless of frame");
        assert_eq!(hits[0].object.name, "M 31");
        // Bounded nearest-N respects the radius.
        let c2 = Constraint::circular(Angle::from_degrees(1.0))
            .nearest_n_within(3, Angle::from_degrees(1.0));
        assert_eq!(rank(m31(), &cat, c2).len(), 2, "only M31 + M110 within 1°");
    }

    #[test]
    fn coordinates_only_never_name() {
        // A far object literally named "M 31" must NOT match near M31's pointing.
        let cat = vec![
            Obj {
                name: "M 31",
                ra: 200.0,
                dec: -40.0,
            },
            Obj {
                name: "Some Galaxy",
                ra: 10.6847,
                dec: 41.2688,
            },
        ];
        let c = Constraint::circular(Angle::from_degrees(2.0)).nearest_one();
        let hits = rank(m31(), &cat, c);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].object.name, "Some Galaxy");
    }

    #[test]
    fn rectangle_excludes_circle_only_corner() {
        // An object just outside the rectangle but inside the circumscribed circle.
        // Field 2°×1°: half-width 1°, half-height 0.5°. Put an object 0.9° North
        // (in frame) vs 0.9° along the diagonal (out of the axis-aligned rect).
        let field = Field::from_fov(Angle::from_degrees(2.0), Angle::from_degrees(1.0)).unwrap();
        let north_obj = Obj {
            name: "N",
            ra: 10.6847,
            dec: 41.2688 + 0.4,
        }; // within 0.5° height
        let high_obj = Obj {
            name: "H",
            ra: 10.6847,
            dec: 41.2688 + 0.9,
        }; // beyond height, inside circle
        let cat = vec![north_obj, high_obj];
        let c = Constraint::frame(&field).all();
        let hits = rank(m31(), &cat, c);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].object.name, "N");
    }

    #[test]
    fn matcher_matches_rank_exactly() {
        let cat = catalog();
        let c = Constraint::circular(Angle::from_degrees(5.0)).all();
        let via_rank: Vec<_> = rank(m31(), &cat, c).iter().map(|m| m.object.name).collect();
        let matcher = Matcher::from_objects(cat.clone());
        let via_index: Vec<_> = matcher
            .query(m31(), c)
            .iter()
            .map(|m| m.object.name)
            .collect();
        assert_eq!(via_rank, via_index);
        assert_eq!(matcher.objects().len(), 4);
    }

    #[test]
    fn is_framed_reports_geometry() {
        let m110 = catalog()[1].clone();
        let m = is_framed(
            m31(),
            &m110,
            Membership::Circular {
                radius: Angle::from_degrees(1.0),
            },
        );
        assert!(m.in_frame);
        assert!((0.4..0.9).contains(&m.separation.degrees()));
    }

    #[test]
    fn empty_catalog_and_zero_radius() {
        let cat = catalog();
        assert!(rank(
            m31(),
            &[] as &[Obj],
            Constraint::circular(Angle::from_degrees(1.0))
        )
        .is_empty());
        let c = Constraint::circular(Angle::from_degrees(-1.0)).all();
        assert!(
            rank(m31(), &cat, c).is_empty(),
            "negative radius matches nothing"
        );
    }
}
