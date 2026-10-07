//! AN ARC, A SLOT AND AN ELLIPSE ARE DRAGGED BY THEIR CENTRE AS A WHOLE.
//!
//! Reported behaviour: dragging an arc, a slot or an ellipse by its centre did not move the shape
//! as a whole. Rectangles carried their corners before the solve; ellipses, slots and arcs now carry
//! their points too.

use qymcad_core::feature::{Purpose, Winding};
use qymcad_core::geom::Point2;
use qymcad_core::model::{EntityKind, Project, Sketch};
use qymcad_core::solver::DragPull2d;

fn xy(p: &Project, si: usize, id: u64) -> (f64, f64) {
    let q = p.sketches[si].points.iter().find(|q| q.id == id).expect("the point is there");
    (q.x, q.y)
}

#[test]
fn an_arc_dragged_by_its_centre_carries_its_endpoints() {
    let mut p = Project::default();
    p.sketches.push(Sketch::default());
    let si = 0;
    // Standalone arc: centre at (20, 20), start at (40, 20), end at (20, 40)
    p.add_arc_entity(si, Point2::new(20.0, 20.0), Point2::new(40.0, 20.0), Point2::new(20.0, 40.0), Winding::Ccw, Purpose::Real);
    let arc = p.sketches[si]
        .entities
        .iter()
        .find_map(|e| match e.kind {
            EntityKind::Arc { center, a, b, .. } => Some((center, a, b)),
            _ => None,
        })
        .expect("an arc is there");
    let (c, a, b) = arc;
    let (c0, a0, b0) = (xy(&p, si, c), xy(&p, si, a), xy(&p, si, b));
    for k in 1..=10 {
        let t = k as f64 / 10.0;
        p.solve_sketch_drag_fast(si, Some(DragPull2d::new(c, c0.0 + 12.0 * t, c0.1 + 6.0 * t)));
    }
    p.solve_sketch(si);
    let (c1, a1, b1) = (xy(&p, si, c), xy(&p, si, a), xy(&p, si, b));
    assert!((c1.0 - c0.0 - 12.0).hypot(c1.1 - c0.1 - 6.0) < 1e-6, "centre moved: {c0:?} -> {c1:?}");
    assert!((a1.0 - a0.0 - 12.0).hypot(a1.1 - a0.1 - 6.0) < 1e-6, "endpoint a moved: {a0:?} -> {a1:?}");
    assert!((b1.0 - b0.0 - 12.0).hypot(b1.1 - b0.1 - 6.0) < 1e-6, "endpoint b moved: {b0:?} -> {b1:?}");
}

#[test]
fn an_ellipse_dragged_by_its_centre_carries_its_axes() {
    let mut p = Project::default();
    p.sketches.push(Sketch::default());
    let si = 0;
    // Ellipse: centre at (30, 30), major axis length 20, minor axis length 10, rot 0
    let c = p.add_ellipse_entity(si, Point2::new(30.0, 30.0), 20.0, 10.0, 0.0, Purpose::Real);
    let (ma, mi) = match p.sketches[si].entities[0].kind {
        EntityKind::Ellipse { ma, mi, .. } => (ma, mi),
        _ => panic!("an ellipse"),
    };
    let (c0, ma0, mi0) = (xy(&p, si, c), xy(&p, si, ma), xy(&p, si, mi));
    for k in 1..=10 {
        let t = k as f64 / 10.0;
        p.solve_sketch_drag_fast(si, Some(DragPull2d::new(c, c0.0 + 12.0 * t, c0.1 + 6.0 * t)));
    }
    p.solve_sketch(si);
    let (c1, ma1, mi1) = (xy(&p, si, c), xy(&p, si, ma), xy(&p, si, mi));
    assert!((c1.0 - c0.0 - 12.0).hypot(c1.1 - c0.1 - 6.0) < 1e-6, "centre moved: {c0:?} -> {c1:?}");
    assert!((ma1.0 - ma0.0 - 12.0).hypot(ma1.1 - ma0.1 - 6.0) < 1e-6, "major axis moved: {ma0:?} -> {ma1:?}");
    assert!((mi1.0 - mi0.0 - 12.0).hypot(mi1.1 - mi0.1 - 6.0) < 1e-6, "minor axis moved: {mi0:?} -> {mi1:?}");
}

#[test]
fn a_slot_dragged_by_its_centre_carries_the_whole_slot() {
    let mut p = Project::default();
    p.sketches.push(Sketch::default());
    let si = 0;
    // Slot between (20, 20) and (50, 20) with r = 10
    p.add_slot_entity(si, Point2::new(20.0, 20.0), Point2::new(50.0, 20.0), 10.0, Purpose::Real);
    let c1 = p.sketches[si]
        .entities
        .iter()
        .find_map(|e| match e.kind {
            EntityKind::Arc { center, .. } => Some(center),
            _ => None,
        })
        .expect("slot arc centre");
    let mut pts: Vec<u64> = Vec::new();
    for e in &p.sketches[si].entities {
        match e.kind {
            EntityKind::Line { a, b } => {
                pts.push(a);
                pts.push(b);
            }
            EntityKind::Arc { center, a, b, .. } => {
                pts.push(center);
                pts.push(a);
                pts.push(b);
            }
            _ => {}
        }
    }
    pts.sort_unstable();
    pts.dedup();
    assert_eq!(pts.len(), 6, "slot has 6 points");
    let before: Vec<(f64, f64)> = pts.iter().map(|id| xy(&p, si, *id)).collect();
    let c0 = xy(&p, si, c1);
    for k in 1..=10 {
        let t = k as f64 / 10.0;
        p.solve_sketch_drag_fast(si, Some(DragPull2d::new(c1, c0.0 + 12.0 * t, c0.1 + 6.0 * t)));
    }
    p.solve_sketch(si);
    let after: Vec<(f64, f64)> = pts.iter().map(|id| xy(&p, si, *id)).collect();
    for (b, a) in before.iter().zip(&after) {
        assert!((a.0 - b.0 - 12.0).hypot(a.1 - b.1 - 6.0) < 1e-6, "slot point moved: {b:?} -> {a:?}");
    }
}
