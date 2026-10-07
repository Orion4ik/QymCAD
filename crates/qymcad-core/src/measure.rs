//! Measurement in 3D: pure mathematics over already resolved geometry.
//!
//! Measuring used to be possible only inside a sketch, between two points on a plane. Distance between faces,
//! a gap between parts, an angle of convergence, the diameter of a hole — none of that could be measured in 3D
//! at all, although it is a tool that sits on a hotkey in every professional CAD.
//!
//! The mathematics lives here and knows nothing about picking or about the screen: the application resolves a
//! click into a [`MeasureItem`], and the numbers are computed here and checked by tests against known geometry.
//! Code like this used to compute its numbers straight inside the click handler, where the only way to check
//! them was by eye.

/// What is being measured: the geometric primitive a pick resolved into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MeasureItem {
    /// A vertex.
    Point([f64; 3]),
    /// A straight edge: a point on it, a direction that need not be a unit vector, and a length.
    Line { origin: [f64; 3], dir: [f64; 3], len: f64 },
    /// A circular edge, such as the rim of a hole or a fillet.
    Circle { center: [f64; 3], axis: [f64; 3], r: f64 },
    /// A planar face.
    Plane { origin: [f64; 3], normal: [f64; 3] },
    /// A cylindrical face: the wall of a hole or of a shaft.
    Cylinder { origin: [f64; 3], axis: [f64; 3], r: f64 },
}

/// The result of a measurement. There are several fields because different pairs make different things
/// meaningful: two points have a distance and its projections onto the axes, two planes have either a distance,
/// when parallel, or an angle. An empty field means the quantity is meaningless for this pair, not that it is
/// zero.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasureResult {
    /// The shortest distance, in mm.
    pub distance: Option<f64>,
    /// The angle between directions or normals, in degrees.
    pub angle_deg: Option<f64>,
    /// The distance broken down by axis. Only for a pair of points; for any other pair it misleads.
    pub delta: Option<[f64; 3]>,
    /// Edge length, radius or diameter: whatever is meaningful for a single selected element.
    pub value: Option<(&'static str, f64)>,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = len(a);
    if l < 1e-12 {
        [0.0, 0.0, 0.0]
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

/// The angle between directions in degrees, folded into [0, 90]: the sign of a direction is arbitrary for lines
/// and normals, since an edge could have been built the other way round, and reporting 179° instead of 1° is an
/// artefact of how the part was assembled rather than information.
fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (u, v) = (norm(a), norm(b));
    let c = dot(u, v).abs().min(1.0);
    c.acos().to_degrees()
}

/// The axis or direction of an element, where it has one.
fn direction(i: &MeasureItem) -> Option<[f64; 3]> {
    match *i {
        MeasureItem::Line { dir, .. } => Some(norm(dir)),
        MeasureItem::Circle { axis, .. } | MeasureItem::Cylinder { axis, .. } => Some(norm(axis)),
        MeasureItem::Plane { normal, .. } => Some(norm(normal)),
        MeasureItem::Point(_) => None,
    }
}

/// The distance from a point to a line given by an origin and a direction.
fn point_line(p: [f64; 3], o: [f64; 3], d: [f64; 3]) -> f64 {
    let u = norm(d);
    let w = sub(p, o);
    len(cross(w, u))
}

/// The shortest distance between two lines: the distance between them when parallel, and along the common
/// normal when skew.
fn line_line(o1: [f64; 3], d1: [f64; 3], o2: [f64; 3], d2: [f64; 3]) -> f64 {
    let (u, v) = (norm(d1), norm(d2));
    let n = cross(u, v);
    if len(n) < 1e-9 {
        return point_line(o2, o1, u); // parallel
    }
    (dot(sub(o2, o1), norm(n))).abs()
}

/// Measure a single element: the length of an edge, or the radius and diameter of a circle or cylinder.
///
/// A planar face and a vertex have no size of their own, so the result for them is empty. That is more honest
/// than showing zero, which reads as a measurement that came out at zero.
pub fn measure_one(a: &MeasureItem) -> MeasureResult {
    let mut r = MeasureResult::default();
    match *a {
        MeasureItem::Line { len: l, .. } => r.value = Some(("m3-length", l)),
        MeasureItem::Circle { r: rad, .. } | MeasureItem::Cylinder { r: rad, .. } => {
            r.value = Some(("Ø", 2.0 * rad));
            r.distance = None;
        }
        _ => {}
    }
    r
}

/// Measure a pair: a distance, an angle, or both, according to what is meaningful for that pair.
///
/// One rule holds throughout: when the elements are parallel, or one of them is a point, the distance is
/// measured; when they converge at an angle, the distance between them is not constant, and the angle is shown
/// instead of a meaningless number. Reporting a distance between non-parallel planes would be a lie, since it
/// depends on where exactly it is measured.
pub fn measure_pair(a: &MeasureItem, b: &MeasureItem) -> MeasureResult {
    use MeasureItem::*;
    let mut out = MeasureResult::default();
    if let (Some(da), Some(db)) = (direction(a), direction(b)) {
        out.angle_deg = Some(angle_between(da, db));
    }
    let parallel = out.angle_deg.is_none_or(|ang| ang < 1e-6);
    match (a, b) {
        (Point(p), Point(q)) => {
            let d = sub(*q, *p);
            out.distance = Some(len(d));
            out.delta = Some(d); // the per-axis breakdown is meaningful only for two points
        }
        // A point and a plane: the distance along the normal. The sign is not shown, since which side it
        // falls on depends on where the face normal points, and that is a detail of the topology rather than
        // part of the measurement.
        (Point(p), Plane { origin, normal }) | (Plane { origin, normal }, Point(p)) => {
            out.distance = Some(dot(sub(*p, *origin), norm(*normal)).abs());
        }
        (Point(p), Line { origin, dir, .. }) | (Line { origin, dir, .. }, Point(p)) => {
            out.distance = Some(point_line(*p, *origin, *dir));
        }
        // A point and a cylinder or circle: measured to the surface rather than to the axis. Pointing at the
        // wall of a hole means asking for the clearance to that wall, not the distance to an imaginary axis
        // buried in the material.
        (Point(p), Cylinder { origin, axis, r }) | (Cylinder { origin, axis, r }, Point(p)) => {
            out.distance = Some((point_line(*p, *origin, *axis) - r).abs());
        }
        (Point(p), Circle { center, axis, r }) | (Circle { center, axis, r }, Point(p)) => {
            // the distance to the circle itself rather than to its centre: along the axis plus within the
            // plane
            let w = sub(*p, *center);
            let along = dot(w, norm(*axis));
            let radial = (len(w).powi(2) - along * along).max(0.0).sqrt() - r;
            out.distance = Some((along * along + radial * radial).sqrt());
        }
        (Plane { origin: o1, normal: n1 }, Plane { origin: o2, .. }) if parallel => {
            out.distance = Some(dot(sub(*o2, *o1), norm(*n1)).abs());
        }
        (Plane { origin, normal }, Line { origin: lo, dir, .. }) | (Line { origin: lo, dir, .. }, Plane { origin, normal }) => {
            // the line is parallel to the plane, its normal being perpendicular to the direction, so a
            // distance applies; otherwise the line crosses the plane and the distance is zero at the
            // intersection, so the angle is measured instead
            if dot(norm(*normal), norm(*dir)).abs() < 1e-9 {
                out.distance = Some(dot(sub(*lo, *origin), norm(*normal)).abs());
            }
            // the angle between a line and a plane is 90° minus the angle to the normal
            out.angle_deg = out.angle_deg.map(|a| (90.0 - a).abs());
        }
        (Plane { origin: po, normal }, Cylinder { origin: co, axis, r }) | (Cylinder { origin: co, axis, r }, Plane { origin: po, normal }) => {
            // The cylinder is parallel to the plane when its axis is perpendicular to the normal.
            if dot(norm(*normal), norm(*axis)).abs() < 1e-9 {
                let d_axis = dot(sub(*co, *po), norm(*normal)).abs();
                out.distance = Some((d_axis - r).max(0.0));
            }
            // The angle between a cylinder axis and a plane is 90 deg minus the angle to the normal.
            out.angle_deg = out.angle_deg.map(|a| (90.0 - a).abs());
        }
        (Line { origin: lo, .. }, Cylinder { origin: co, axis, r }) | (Cylinder { origin: co, axis, r }, Line { origin: lo, .. }) => {
            if parallel {
                let d_axis = point_line(*lo, *co, *axis);
                out.distance = Some((d_axis - r).abs());
            }
        }
        (Line { origin: o1, dir: d1, .. }, Line { origin: o2, dir: d2, .. }) => {
            out.distance = Some(line_line(*o1, *d1, *o2, *d2));
        }
        (Cylinder { origin: o1, axis: a1, r: r1 }, Cylinder { origin: o2, axis: a2, r: r2 }) if parallel => {
            let c2c = line_line(*o1, *a1, *o2, *a2);
            let (r_min, r_max) = if r1 < r2 { (*r1, *r2) } else { (*r2, *r1) };
            if c2c + r_min <= r_max {
                // One cylinder is inside the other (coaxial or nested): the gap between the walls.
                out.distance = Some(r_max - (c2c + r_min));
            } else if c2c < r1 + r2 {
                // The walls intersect.
                out.distance = Some(0.0);
            } else {
                // External cylinders.
                out.distance = Some(c2c - r1 - r2);
            }
        }
        (Circle { center: c1, axis: a1, .. }, Circle { center: c2, axis: a2, .. }) if parallel => {
            out.distance = Some(point_line(*c2, *c1, *a1).hypot(dot(sub(*c2, *c1), norm(*a1))).min(len(sub(*c2, *c1))));
            let _ = a2;
        }
        // Every other pair: the distance between representative points is not computed, since it depends on
        // which points are taken and would be a number about nothing. The angle, where there is one, is
        // already computed above.
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_points_give_distance_and_axis_deltas() {
        let r = measure_pair(&MeasureItem::Point([0.0, 0.0, 0.0]), &MeasureItem::Point([3.0, 4.0, 0.0]));
        assert!((r.distance.unwrap() - 5.0).abs() < 1e-9, "3-4-5: {r:?}");
        assert_eq!(r.delta.unwrap(), [3.0, 4.0, 0.0], "the per-axis breakdown is meaningful for points");
    }

    #[test]
    fn point_to_plane_is_the_perpendicular_distance() {
        let pl = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] };
        let r = measure_pair(&MeasureItem::Point([5.0, -7.0, 12.0]), &pl);
        assert!((r.distance.unwrap() - 12.0).abs() < 1e-9, "12 along the normal, got {:?}", r.distance);
    }

    #[test]
    fn parallel_planes_give_the_gap_between_them() {
        let a = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] };
        let b = MeasureItem::Plane { origin: [100.0, 50.0, 8.0], normal: [0.0, 0.0, -1.0] };
        let r = measure_pair(&a, &b);
        assert!((r.distance.unwrap() - 8.0).abs() < 1e-9, "a clearance of 8 regardless of which point of the face is taken: {r:?}");
        assert!(r.angle_deg.unwrap() < 1e-9, "opposing normals still describe parallel faces, giving 0° rather than 180°");
    }

    /// Non-parallel faces have an angle and no distance.
    ///
    /// The distance between converging planes depends on where it is measured, so showing a single number
    /// would be a lie.
    #[test]
    fn planes_at_an_angle_report_the_angle_and_no_distance() {
        let a = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] };
        let b = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [0.0, 1.0, 1.0] };
        let r = measure_pair(&a, &b);
        assert!((r.angle_deg.unwrap() - 45.0).abs() < 1e-9, "45°, got {:?}", r.angle_deg);
        assert!(r.distance.is_none(), "the distance between converging planes is not constant, so there must be no number");
    }

    #[test]
    fn skew_lines_give_the_common_normal_distance() {
        let a = MeasureItem::Line { origin: [0.0, 0.0, 0.0], dir: [1.0, 0.0, 0.0], len: 10.0 };
        let b = MeasureItem::Line { origin: [0.0, 0.0, 7.0], dir: [0.0, 1.0, 0.0], len: 10.0 };
        let r = measure_pair(&a, &b);
        assert!((r.distance.unwrap() - 7.0).abs() < 1e-9, "skew lines 7 apart: {r:?}");
        assert!((r.angle_deg.unwrap() - 90.0).abs() < 1e-9, "and at 90°");
    }

    #[test]
    fn parallel_lines_give_the_distance_between_them() {
        let a = MeasureItem::Line { origin: [0.0, 0.0, 0.0], dir: [1.0, 0.0, 0.0], len: 10.0 };
        let b = MeasureItem::Line { origin: [5.0, 3.0, 4.0], dir: [-2.0, 0.0, 0.0], len: 10.0 };
        let r = measure_pair(&a, &b);
        assert!((r.distance.unwrap() - 5.0).abs() < 1e-9, "3-4-5 across: {r:?}");
        assert!(r.angle_deg.unwrap() < 1e-9, "opposite directions still describe parallel edges, giving 0°");
    }

    /// A point and a cylinder: measured to the wall rather than to the axis. Pointing at a hole means asking
    /// for the clearance to it.
    #[test]
    fn point_to_cylinder_measures_to_the_wall() {
        let c = MeasureItem::Cylinder { origin: [0.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 4.0 };
        let r = measure_pair(&MeasureItem::Point([10.0, 0.0, 3.0]), &c);
        assert!((r.distance.unwrap() - 6.0).abs() < 1e-9, "10 from the axis minus a radius of 4 gives 6, got {:?}", r.distance);
    }

    #[test]
    fn parallel_cylinders_measure_wall_to_wall() {
        let a = MeasureItem::Cylinder { origin: [0.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 3.0 };
        let b = MeasureItem::Cylinder { origin: [20.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 5.0 };
        let r = measure_pair(&a, &b);
        assert!((r.distance.unwrap() - 12.0).abs() < 1e-9, "20 between the axes minus 3 and 5 gives 12, got {:?}", r.distance);
    }

    #[test]
    fn a_single_edge_reports_its_length_and_a_hole_its_diameter() {
        let l = measure_one(&MeasureItem::Line { origin: [0.0; 3], dir: [1.0, 0.0, 0.0], len: 17.5 });
        assert_eq!(l.value, Some(("m3-length", 17.5)));
        let c = measure_one(&MeasureItem::Cylinder { origin: [0.0; 3], axis: [0.0, 0.0, 1.0], r: 4.0 });
        assert_eq!(c.value, Some(("Ø", 8.0)), "a hole is reported by its diameter, which is what it is measured by");
    }

    /// A vertex and a plane have no size of their own: the result is empty rather than zero.
    #[test]
    fn a_point_or_plane_alone_has_no_size() {
        assert_eq!(measure_one(&MeasureItem::Point([1.0, 2.0, 3.0])).value, None);
        assert_eq!(measure_one(&MeasureItem::Plane { origin: [0.0; 3], normal: [0.0, 0.0, 1.0] }).value, None);
    }

    /// A line parallel to a plane has a distance and an angle of 0°.
    #[test]
    fn a_line_parallel_to_a_plane_has_a_distance() {
        let pl = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] };
        let ln = MeasureItem::Line { origin: [3.0, 3.0, 9.0], dir: [1.0, 1.0, 0.0], len: 5.0 };
        let r = measure_pair(&pl, &ln);
        assert!((r.distance.unwrap() - 9.0).abs() < 1e-9, "9 above the plane: {r:?}");
        assert!(r.angle_deg.unwrap() < 1e-9, "parallel to the plane means 0°, not 90° to the normal");
    }

    /// Two coaxial cylinders (e.g. counterbore and hole) must report the radial wall gap,
    /// never a negative distance.
    #[test]
    fn coaxial_cylinders_give_radial_wall_clearance() {
        let counterbore = MeasureItem::Cylinder { origin: [0.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 8.0 };
        let hole = MeasureItem::Cylinder { origin: [0.0, 0.0, -10.0], axis: [0.0, 0.0, 1.0], r: 4.0 };
        let r = measure_pair(&counterbore, &hole);
        assert_eq!(r.distance, Some(4.0), "radial wall gap between coaxial Ø16 and Ø8 is 4 mm");
        assert!(r.angle_deg.unwrap() < 1e-9, "coaxial cylinders are parallel");
    }

    /// An eccentric cylinder inside another must report the minimum radial wall gap.
    #[test]
    fn nested_eccentric_cylinders_give_shortest_wall_clearance() {
        let outer = MeasureItem::Cylinder { origin: [0.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 10.0 };
        let inner = MeasureItem::Cylinder { origin: [3.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 5.0 };
        let r = measure_pair(&outer, &inner);
        assert_eq!(r.distance, Some(2.0), "minimum wall clearance between nested cylinders is 2 mm");
    }

    /// A straight edge parallel to a hole axis reports the distance from the edge line to the hole wall,
    /// not an arbitrary distance between anchor endpoints.
    #[test]
    fn parallel_edge_and_cylinder_gives_wall_clearance() {
        let edge = MeasureItem::Line { origin: [15.0, 5.0, 20.0], dir: [0.0, 0.0, -1.0], len: 20.0 };
        let hole = MeasureItem::Cylinder { origin: [0.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], r: 3.0 };
        let r = measure_pair(&edge, &hole);
        let expected = 250.0_f64.sqrt() - 3.0;
        assert!((r.distance.unwrap() - expected).abs() < 1e-6, "wall distance must be ~12.811 mm, got {:?}", r.distance);
        assert!(r.angle_deg.unwrap() < 1e-9, "parallel edge and cylinder");
    }

    /// A planar face perpendicular to a hole axis has no constant distance and reports only the 90° angle.
    #[test]
    fn perpendicular_plane_and_cylinder_reports_no_distance() {
        let top_face = MeasureItem::Plane { origin: [0.0, 0.0, 20.0], normal: [0.0, 0.0, 1.0] };
        let hole = MeasureItem::Cylinder { origin: [10.0, 10.0, 0.0], axis: [0.0, 0.0, 1.0], r: 3.0 };
        let r = measure_pair(&top_face, &hole);
        assert_eq!(r.distance, None, "perpendicular plane and cylinder must not report a distance");
        assert_eq!(r.angle_deg, Some(90.0), "perpendicular plane and cylinder are at 90°");
    }

    /// A planar face parallel to a cylinder axis reports wall-to-plane clearance.
    #[test]
    fn parallel_plane_and_cylinder_reports_wall_clearance() {
        let side_face = MeasureItem::Plane { origin: [0.0, 0.0, 0.0], normal: [1.0, 0.0, 0.0] };
        let cylinder = MeasureItem::Cylinder { origin: [10.0, 5.0, 0.0], axis: [0.0, 0.0, 1.0], r: 3.0 };
        let r = measure_pair(&side_face, &cylinder);
        assert_eq!(r.distance, Some(7.0), "distance from plane X=0 to cylinder at X=10 with r=3 is 7 mm");
        assert_eq!(r.angle_deg, Some(0.0), "cylinder axis is parallel to plane");
    }
}
