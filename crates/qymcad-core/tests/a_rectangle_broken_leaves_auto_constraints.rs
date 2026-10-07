//! A RECTANGLE BROKEN WITH AUTO-CONSTRAINTS ON LEAVES ITS LINES WITH THE CONSTRAINTS THEY NOW PLAINLY HAVE:
//! Horizontal or Vertical on a line standing so, Parallel and Perpendicular between the lines that are,
//! Equal between lines of one length - every one added only where it is independent, so nothing is over-defined.
//! With Auto-constraints off they are left as plain lines.
//!
//! Reported (issue #72): a rectangle broken by deleting a side or one of its own constraints left plain lines
//! with no constraints at all, even though they still stood horizontal, vertical, parallel and square.

use qymcad_core::feature::Purpose;
use qymcad_core::geom::Point2;
use qymcad_core::model::{Constraint, EntityKind, Id, Project};
use qymcad_core::solver::DragPull2d;

fn drawn(way: usize) -> (Project, usize) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("S");
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    match way {
        0 => drop(p.add_rect_entity(si, 10.0, 10.0, 50.0, 40.0, Purpose::Real)),
        1 => drop(p.add_rect_from_centre(si, Point2::new(30.0, 25.0), Point2::new(50.0, 40.0), Purpose::Real)),
        _ => drop(p.add_rect3_entity(si, Point2::new(10.0, 10.0), Point2::new(10.0 + 40.0 * c, 10.0 + 40.0 * s), Point2::new(10.0 + 40.0 * c - 30.0 * s, 10.0 + 40.0 * s + 30.0 * c), Purpose::Real)),
    }
    p.solve_sketch(si);
    (p, si)
}

fn pt(p: &Project, si: usize, id: Id) -> (f64, f64) {
    let q = p.sketches[si].points.iter().find(|q| q.id == id).expect("a point");
    (q.x, q.y)
}

#[test]
fn rectangle_broken_by_deleting_side_with_auto_constrain_on_leaves_constraints() {
    let (mut p, si) = drawn(0);
    p.auto_constrain = true;

    // The top side is the one connecting (10, 40) and (50, 40)
    let top_side = {
        let s = &p.sketches[si];
        s.rects[0]
            .sides
            .iter()
            .copied()
            .find(|&eid| {
                let e = s.entities.iter().find(|x| x.id == eid).unwrap();
                if let EntityKind::Line { a, b } = e.kind {
                    let (ya, yb) = (pt(&p, si, a).1, pt(&p, si, b).1);
                    (ya - 40.0).abs() < 1e-4 && (yb - 40.0).abs() < 1e-4
                } else {
                    false
                }
            })
            .expect("top side")
    };

    p.delete_entities(si, &[top_side]);

    let s = &p.sketches[si];
    assert!(s.rects.is_empty(), "rectangle record is gone");
    assert_eq!(s.entities.len(), 3, "three lines left");

    // The bottom line is Horizontal, the two upright lines are Vertical (or Parallel and Perpendicular)
    let has_horiz = s.constraints.iter().any(|c| matches!(c, Constraint::Horizontal { .. }));
    let vert_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Vertical { .. })).count();
    let perp_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Perpendicular { .. })).count();
    let par_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Parallel { .. })).count();

    assert!(has_horiz, "bottom line has Horizontal constraint: {:?}", s.constraints);
    assert!(vert_count == 2 || (perp_count >= 1 && par_count >= 1), "upright lines are Vertical or Perpendicular/Parallel: {:?}", s.constraints);

    // Dragging a corner keeps the lines upright and level
    let top_right_corner = s.points.iter().find(|q| (q.x - 50.0).abs() < 1e-4 && (q.y - 40.0).abs() < 1e-4).map(|q| q.id).expect("top-right point");
    p.solve_sketch_drag(si, Some(DragPull2d::new(top_right_corner, 50.0, 55.0)));

    let s = &p.sketches[si];
    for e in &s.entities {
        if let EntityKind::Line { a, b } = e.kind {
            let (pa, pb) = (pt(&p, si, a), pt(&p, si, b));
            let dx = (pb.0 - pa.0).abs();
            let dy = (pb.1 - pa.1).abs();
            let is_level = dy < 1e-3;
            let is_upright = dx < 1e-3;
            assert!(is_level || is_upright, "line from ({}, {}) to ({}, {}) is neither upright nor level", pa.0, pa.1, pb.0, pb.1);
        }
    }
}

#[test]
fn rectangle_turned_by_30_deg_broken_leaves_parallel_and_perpendicular() {
    let (mut p, si) = drawn(2);
    p.auto_constrain = true;

    // Delete one side of the turned rectangle
    let side0 = p.sketches[si].rects[0].sides[0];
    p.delete_entities(si, &[side0]);

    let s = &p.sketches[si];
    assert!(s.rects.is_empty(), "rectangle record is gone");
    assert_eq!(s.entities.len(), 3, "three lines left");

    // No Horizontal or Vertical constraints
    assert!(!s.constraints.iter().any(|c| matches!(c, Constraint::Horizontal { .. } | Constraint::Vertical { .. })), "turned rectangle should not get Horizontal or Vertical: {:?}", s.constraints);

    // Parallel and Perpendicular constraints instead
    let perp_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Perpendicular { .. })).count();
    let par_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Parallel { .. })).count();
    assert!(perp_count >= 1 && (par_count >= 1 || perp_count >= 2), "expected Perpendicular and Parallel constraints: {:?}", s.constraints);
}

#[test]
fn rectangle_broken_by_deleting_own_constraint_gets_same() {
    let (mut p, si) = drawn(0);
    p.auto_constrain = true;

    // Delete one of rectangle's own constraints
    let ci = p.sketches[si].constraints.iter().position(|c| matches!(c, Constraint::Orientation { .. })).expect("orientation constraint");
    assert!(p.delete_sketch_constraint(si, ci));

    let s = &p.sketches[si];
    assert!(s.rects.is_empty(), "rectangle record is gone");
    assert_eq!(s.entities.len(), 4, "four lines left");

    let has_horiz = s.constraints.iter().any(|c| matches!(c, Constraint::Horizontal { .. }));
    let has_vert = s.constraints.iter().any(|c| matches!(c, Constraint::Vertical { .. }));
    assert!(has_horiz && has_vert, "broken rectangle got Horizontal and Vertical constraints: {:?}", s.constraints);
}

#[test]
fn rectangle_broken_with_auto_constrain_off_leaves_plain_lines() {
    let (mut p, si) = drawn(0);
    p.auto_constrain = false;

    let side = p.sketches[si].rects[0].sides[0];
    p.delete_entities(si, &[side]);

    let s = &p.sketches[si];
    assert!(s.rects.is_empty(), "rectangle record is gone");
    assert_eq!(s.entities.len(), 3, "three lines left");
    assert!(s.constraints.is_empty(), "no constraints added when auto_constrain is false: {:?}", s.constraints);
}
