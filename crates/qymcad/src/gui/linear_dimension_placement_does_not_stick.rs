//! A LINEAR DIMENSION BEING PLACED DOES NOT STICK TO ITS OWN LINE.
//!
//! Reported behaviour: when a linear dimension was placed on a line, the pointer was caught
//! by the snaps of the very line being dimensioned (its middle, a point on it). The dimension
//! line stuck to the line, was drawn red, and took a firm pull of the hand to tear it off.
//!
//! Snapping is for the points a dimension is put between, not for where its line is laid.
//! The dimension being placed follows the pointer as it is, not a snapped point: it is led off
//! the line at once and stands where the pointer is, without sticking.

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::Constraint;

    #[test]
    fn linear_dimension_placement_follows_pointer_without_sticking_to_line() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);

        // Draw a line from (0, 0) to (40, 0)
        Hand::new(&mut app).sk_tool(1).click2d(0.0, 0.0).click2d(40.0, 0.0);
        Hand::new(&mut app).key(egui::Key::Escape);

        // Take the dimension tool
        let mut hand = Hand::new(&mut app);
        qymcad_ui_state::set_dim_tool(&mut qymcad_ui_state::tools_of!(hand.app), &mut hand.app.viewing.mode_3d, &hand.app.project, hand.app.chosen.sel, hand.app.sketch_ses, &mut hand.app.status, 1);
        assert_eq!(hand.app.tools.armed.dim_kind(), 1, "setup: dimension tool armed");

        // Click the line at its middle (20, 0)
        hand.mouse2d(20.0, 0.0);
        assert!(hand.app.tools.place.dim.is_some(), "dimension placement active after clicking line");

        let a = hand.on_screen2d((20.0, 0.0));
        let b = hand.on_screen2d((20.0, -10.0));
        let steps = ((b - a).length() / 3.0).ceil().max(1.0) as usize;

        // Lead the pointer straight down in steps of 3 px
        let mut last_off = 0.0;
        for k in 1..=steps {
            let p = a + (b - a) * (k as f32 / steps as f32);
            hand.frame(vec![egui::Event::PointerMoved(p)]);

            let ci = hand.app.tools.place.dim.expect("dimension placement stays active while leading pointer");
            let (off, axis) = match hand.app.project.sketches[si].constraints[ci] {
                Constraint::Distance { off, axis, .. } => (off, axis),
                _ => panic!("expected Distance constraint"),
            };

            // Must follow the pointer at every step, with no frames left stuck on the line
            assert!(off.abs() > 0.0, "step {k}/{steps}: dimension stuck on line (off is 0.0)");
            assert!(off.abs() >= last_off, "step {k}/{steps}: dimension did not advance (off {off} vs previous {last_off})");
            last_off = off.abs();

            // Must not be drawn red / conflict on the way
            let diag = qymcad_ui_state::sketch_diag(&hand.app.cache, &hand.app.project, si);
            assert!(!diag.conflicts.contains(&ci), "step {k}/{steps}: dimension conflicts and is red");
            assert_eq!(axis, 1, "step {k}/{steps}: axis flipped unexpectedly");
        }

        // Placed with a second click 10 mm below the line
        let press = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        hand.frame(vec![press(b, true)]);
        hand.frame(vec![press(b, false)]);

        assert!(hand.app.tools.place.dim.is_none(), "dimension placed with second click");

        // Stands 10 mm below it
        let placed_dim = hand.app.project.sketches[si]
            .constraints
            .iter()
            .find_map(|c| match c {
                Constraint::Distance { off, .. } => Some(*off),
                _ => None,
            })
            .expect("placed dimension in sketch");

        assert!((placed_dim.abs() - 10.0).abs() < 0.5, "dimension stands at {placed_dim}, expected ~10.0 mm");
    }
}
