//! A RECTANGLE BROKEN WITH AUTO-CONSTRAINTS ON LEAVES ITS LINES WITH CONSTRAINTS.
//!
//! Reported (issue #72): deleting a side or a constraint of a rectangle with Auto-constraints on
//! leaves plain lines with no constraints, instead of Horizontal/Vertical or Parallel/Perpendicular.

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::{Constraint, EntityKind};

    #[test]
    fn rectangle_broken_through_window_with_auto_constrain_on_leaves_constraints() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.set.auto_constrain = true;

        // Draw rectangle 40 x 30 by corners
        Hand::new(&mut app).sk_tool(2).click2d(0.0, 0.0).click2d(40.0, 30.0).key(egui::Key::Escape);

        // Click top side at (20, 30) to select it, then press Delete
        Hand::new(&mut app).click2d(20.0, 30.0).key(egui::Key::Delete);

        let s = &app.project.sketches[si];
        assert!(s.rects.is_empty(), "rectangle record is gone");
        assert_eq!(s.entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).count(), 3, "three lines left");

        let has_horiz = s.constraints.iter().any(|c| matches!(c, Constraint::Horizontal { .. }));
        let vert_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Vertical { .. })).count();
        let perp_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Perpendicular { .. })).count();
        let par_count = s.constraints.iter().filter(|c| matches!(c, Constraint::Parallel { .. })).count();

        assert!(has_horiz, "bottom line has Horizontal constraint: {:?}", s.constraints);
        assert!(vert_count == 2 || (perp_count >= 1 && par_count >= 1), "upright lines have constraints: {:?}", s.constraints);
    }

    #[test]
    fn rectangle_broken_through_window_with_auto_constrain_off_leaves_plain_lines() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.set.auto_constrain = false;

        // Draw rectangle 40 x 30 by corners
        Hand::new(&mut app).sk_tool(2).click2d(0.0, 0.0).click2d(40.0, 30.0).key(egui::Key::Escape);

        // Click top side at (20, 30) to select it, then press Delete
        Hand::new(&mut app).click2d(20.0, 30.0).key(egui::Key::Delete);

        let s = &app.project.sketches[si];
        assert!(s.rects.is_empty(), "rectangle record is gone");
        assert_eq!(s.entities.iter().filter(|e| matches!(e.kind, EntityKind::Line { .. })).count(), 3, "three lines left");
        assert!(
            !s.constraints
                .iter()
                .any(|c| matches!(c, Constraint::Horizontal { .. } | Constraint::Vertical { .. } | Constraint::Parallel { .. } | Constraint::Perpendicular { .. } | Constraint::Equal { .. })),
            "no constraints on lines when auto_constrain is false: {:?}",
            s.constraints
        );
    }
}
