//! A SHAPE DRAGGED BY ITS CENTRE GOES WITH IT AS A WHOLE, AS A CIRCLE DOES.
//!
//! Reported behaviour: an arc by three points, a slot and an ellipse were not dragged by their centre
//! as a whole. The centres of the arc and the slot were refused, while the ellipse dragged by its centre
//! pulled only the centre, distorting and turning the shape. Fillet arcs round a corner and must stay
//! undraggable by their centre.

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::EntityKind;

    #[test]
    fn an_arc_dragged_by_its_centre_goes_as_a_whole() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        // Centre at (20, 20), start at (40, 20), end at (20, 40)
        Hand::new(&mut app).sk_tool(4).click2d(20.0, 20.0).click2d(40.0, 20.0).click2d(20.0, 40.0);
        Hand::new(&mut app).sk_tool(0);
        let arc = app.project.sketches[si]
            .entities
            .iter()
            .find_map(|e| match e.kind {
                EntityKind::Arc { center, a, b, .. } => Some((center, a, b)),
                _ => None,
            })
            .expect("an arc is drawn");
        let at = |app: &App, id: u64| {
            let q = app.project.sketches[si].points.iter().find(|p| p.id == id).unwrap();
            (q.x, q.y)
        };
        let (c_before, a_before, b_before) = (at(&app, arc.0), at(&app, arc.1), at(&app, arc.2));
        Hand::new(&mut app).drag2d(c_before, (c_before.0 + 12.0, c_before.1 + 6.0));
        let (c_after, a_after, b_after) = (at(&app, arc.0), at(&app, arc.1), at(&app, arc.2));
        assert!((c_after.0 - c_before.0 - 12.0).hypot(c_after.1 - c_before.1 - 6.0) < 0.5, "centre moved: {c_before:?} -> {c_after:?}");
        assert!((a_after.0 - a_before.0 - 12.0).hypot(a_after.1 - a_before.1 - 6.0) < 0.5, "start point moved: {a_before:?} -> {a_after:?}");
        assert!((b_after.0 - b_before.0 - 12.0).hypot(b_after.1 - b_before.1 - 6.0) < 0.5, "end point moved: {b_before:?} -> {b_after:?}");
    }

    #[test]
    fn a_slot_dragged_by_its_centre_goes_as_a_whole() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        // Slot centres at (20, 20) and (50, 20), width point at (50, 30)
        Hand::new(&mut app).sk_tool(7).click2d(20.0, 20.0).click2d(50.0, 20.0).click2d(50.0, 30.0);
        Hand::new(&mut app).sk_tool(0);
        let mut pts: Vec<u64> = Vec::new();
        for e in &app.project.sketches[si].entities {
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
        assert_eq!(pts.len(), 6, "slot has 6 points: 2 centres and 4 corners");
        let at = |app: &App, id: u64| {
            let q = app.project.sketches[si].points.iter().find(|p| p.id == id).unwrap();
            (q.x, q.y)
        };
        let before: Vec<(f64, f64)> = pts.iter().map(|id| at(&app, *id)).collect();
        Hand::new(&mut app).drag2d((20.0, 20.0), (32.0, 26.0));
        let after: Vec<(f64, f64)> = pts.iter().map(|id| at(&app, *id)).collect();
        for (b, a) in before.iter().zip(&after) {
            assert!((a.0 - b.0 - 12.0).hypot(a.1 - b.1 - 6.0) < 0.5, "slot point moved: {b:?} -> {a:?}");
        }
    }

    #[test]
    fn an_ellipse_dragged_by_its_centre_goes_as_a_whole() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        // Centre at (30, 30), major axis at (50, 30), minor axis point at (30, 40)
        Hand::new(&mut app).sk_tool(8).click2d(30.0, 30.0).click2d(50.0, 30.0).click2d(30.0, 40.0);
        Hand::new(&mut app).sk_tool(0);
        let ellipse = app.project.sketches[si]
            .entities
            .iter()
            .find_map(|e| match e.kind {
                EntityKind::Ellipse { c, ma, mi } => Some((c, ma, mi)),
                _ => None,
            })
            .expect("an ellipse is drawn");
        let at = |app: &App, id: u64| {
            let q = app.project.sketches[si].points.iter().find(|p| p.id == id).unwrap();
            (q.x, q.y)
        };
        let (c_before, ma_before, mi_before) = (at(&app, ellipse.0), at(&app, ellipse.1), at(&app, ellipse.2));
        Hand::new(&mut app).drag2d((30.0, 30.0), (42.0, 36.0));
        let (c_after, ma_after, mi_after) = (at(&app, ellipse.0), at(&app, ellipse.1), at(&app, ellipse.2));
        assert!((c_after.0 - c_before.0 - 12.0).hypot(c_after.1 - c_before.1 - 6.0) < 0.5, "centre moved: {c_before:?} -> {c_after:?}");
        assert!((ma_after.0 - ma_before.0 - 12.0).hypot(ma_after.1 - ma_before.1 - 6.0) < 0.5, "major axis moved: {ma_before:?} -> {ma_after:?}");
        assert!((mi_after.0 - mi_before.0 - 12.0).hypot(mi_after.1 - mi_before.1 - 6.0) < 0.5, "minor axis moved: {mi_before:?} -> {mi_after:?}");
    }

    #[test]
    fn the_centre_of_a_fillet_arc_is_not_dragged() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        let l1 = app.project.add_line_entity(si, 10.0, 30.0, 30.0, 30.0, qymcad_core::feature::Purpose::Real);
        let l2 = app.project.add_line_entity(si, 30.0, 30.0, 30.0, 10.0, qymcad_core::feature::Purpose::Real);
        assert!(app.project.fillet_lines(si, l1, l2, 5.0), "fillet applied");
        Hand::new(&mut app).sk_tool(0);
        let arc = app.project.sketches[si]
            .entities
            .iter()
            .find_map(|e| match e.kind {
                EntityKind::Arc { center, .. } => Some(center),
                _ => None,
            })
            .expect("fillet arc created");
        let at = |app: &App, id: u64| {
            let q = app.project.sketches[si].points.iter().find(|p| p.id == id).unwrap();
            (q.x, q.y)
        };
        let c_before = at(&app, arc);
        Hand::new(&mut app).drag2d(c_before, (c_before.0 + 12.0, c_before.1 + 6.0));
        let c_after = at(&app, arc);
        assert!((c_after.0 - c_before.0).hypot(c_after.1 - c_before.1) < 0.1, "fillet centre must not move: was {c_before:?}, became {c_after:?}");
    }
}
