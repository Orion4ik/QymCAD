//! AUTO-CONSTRAINTS DO NOT TIE A NEW LINE TO DISTANT UNRELATED LINES.
//!
//! Reported (issue #94): Auto-constraints tied a new line to any line of the sketch at a similar
//! angle or length, however far away, without hovering over it. Two shapes drawn one after the
//! other ended up tied together.
//!
//! Expected: Two shapes drawn without hovering over each other share no constraints.
//! Hovering over an existing line while drawing infers Parallel and Equal to that line with a live glyph.

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::{Constraint, Id};

    fn points_of_constraint(c: &Constraint) -> Vec<Id> {
        match c {
            Constraint::Horizontal { a, b } | Constraint::Vertical { a, b } => vec![*a, *b],
            Constraint::Distance { a, b, .. } => vec![*a, *b],
            Constraint::AngleLines { a, b, c, d, .. } => vec![*a, *b, *c, *d],
            Constraint::Parallel { a, b, c, d } | Constraint::Perpendicular { a, b, c, d } | Constraint::Equal { a, b, c, d } => vec![*a, *b, *c, *d],
            Constraint::PointOnLine { p, a, b } => vec![*p, *a, *b],
            Constraint::Coincident { a, b } => vec![*a, *b],
            Constraint::Midpoint { p, a, b } => vec![*p, *a, *b],
            _ => Vec::new(),
        }
    }

    #[test]
    fn two_triangles_drawn_with_nothing_hovered_share_no_constraints() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.set.auto_constrain = true;

        // Draw first triangle at (10, 10), (30, 40), (40, 10), (10, 10)
        Hand::new(&mut app)
            .sk_tool(1)
            .click2d(10.0, 10.0)
            .click2d(30.0, 40.0)
            .click2d(40.0, 10.0)
            .click2d(10.0, 10.0)
            .key(egui::Key::Escape)
            .click2d(-40.0, 10.0)
            .click2d(-20.0, 40.0)
            .click2d(-10.0, 10.0)
            .click2d(-40.0, 10.0)
            .key(egui::Key::Escape);

        let s = &app.project.sketches[si];
        let t1_points: Vec<Id> = s.points.iter().filter(|p| p.x > 0.0).map(|p| p.id).collect();
        let t2_points: Vec<Id> = s.points.iter().filter(|p| p.x < 0.0).map(|p| p.id).collect();

        // No constraint should link a point of triangle 1 to a point of triangle 2
        let shared: Vec<_> = s
            .constraints
            .iter()
            .filter(|c| {
                let pts = points_of_constraint(c);
                let has_t1 = pts.iter().any(|p| t1_points.contains(p));
                let has_t2 = pts.iter().any(|p| t2_points.contains(p));
                has_t1 && has_t2
            })
            .collect();

        assert!(shared.is_empty(), "the two triangles share constraints: {:?}", shared);
    }

    #[test]
    fn hovering_over_an_existing_line_while_drawing_infers_parallel() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.set.auto_constrain = true;

        // Draw first line from (10, 10) to (30, 40)
        Hand::new(&mut app)
            .sk_tool(1)
            .click2d(10.0, 10.0)
            .click2d(30.0, 40.0)
            .key(egui::Key::Escape)
            // Start second line at (-40, 10)
            .click2d(-40.0, 10.0)
            // Hover over the first line at midpoint (20, 25)
            .hover2d(20.0, 25.0)
            // Click endpoint at (-20, 40), parallel to first line
            .click2d(-20.0, 40.0)
            .key(egui::Key::Escape);

        let s = &app.project.sketches[si];
        let has_parallel = s.constraints.iter().any(|c| matches!(c, Constraint::Parallel { .. }));
        assert!(has_parallel, "a parallel constraint was created between hovered line and new line: {:?}", s.constraints);
    }

    #[test]
    fn hovering_over_an_existing_line_while_drawing_infers_equal() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.set.auto_constrain = true;

        // Draw first line from (10, 10) to (30, 40) with length sqrt(20^2 + 30^2)
        Hand::new(&mut app)
            .sk_tool(1)
            .click2d(10.0, 10.0)
            .click2d(30.0, 40.0)
            .key(egui::Key::Escape)
            // Start second line at (-40, 10)
            .click2d(-40.0, 10.0)
            // Hover over the first line at midpoint (20, 25)
            .hover2d(20.0, 25.0)
            // Click endpoint at (-10, 30), different angle but same length sqrt(30^2 + 20^2)
            .click2d(-10.0, 30.0)
            .key(egui::Key::Escape);

        let s = &app.project.sketches[si];
        let has_equal = s.constraints.iter().any(|c| matches!(c, Constraint::Equal { .. }));
        assert!(has_equal, "an equal length constraint was created between hovered line and new line: {:?}", s.constraints);
    }
}
