//! THE DIMENSIONS OF A SKETCH RECTANGLE GO WITH IT WHEN TURNED.
//!
//! Reported behaviour: a rectangle with its width and height dimensioned, turned with Rotate
//! (or by an angle dimension on a side): the rectangle turns as a whole, but its dimensions
//! stay horizontal and vertical where they stood before the turn, measuring from there.
//!
//! Expected: the dimensions go with their sides: each stays along the side it measures,
//! at the distance from it it had, turned with the rectangle, and still says the same width
//! and height. Turned back by -30 deg, the dimensions stand where they stood at first.

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::{Purpose, SketchPlane};
    use qymcad_core::model::{Constraint, EntityKind};

    fn sheet(app: &App) -> qymcad_ui_state::Sheet {
        qymcad_ui_state::Sheet { view: app.viewing.view, rect: app.viewing.view_rect }
    }

    /// The direction and offset of a linear dimension's line on the screen.
    fn dim_line_angle_and_caption(app: &App, si: usize, ci: usize) -> (f64, String) {
        let c = &app.project.sketches[si].constraints[ci];
        let (la, lb, _) = qymcad_ui_state::linear_dim_line(&app.project, si, c, &sheet(app)).expect("a linear dimension line");
        let dir = lb - la;
        // In screen coordinates y goes down, so invert y for math angle:
        let ang = (-(dir.y as f64)).atan2(dir.x as f64).to_degrees().rem_euclid(180.0);
        let cap = qymcad_ui_state::dim_caption(&app.project, si, c, &app.set).expect("a caption");
        (ang, cap)
    }

    fn worst(app: &App, si: usize) -> f64 {
        app.project.sketch_residuals(si).into_iter().fold(0.0_f64, f64::max)
    }

    #[test]
    fn the_dimensions_of_a_rectangle_go_with_it_when_turned() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);

        // Draw rectangle 40 x 30 without typing sizes
        Hand::new(&mut app).sk_tool(2).click2d(0.0, 0.0).click2d(40.0, 30.0).key(egui::Key::Escape);

        // Take dimension tool
        let mut hand = Hand::new(&mut app);
        qymcad_ui_state::set_dim_tool(&mut qymcad_ui_state::tools_of!(hand.app), &mut hand.app.viewing.mode_3d, &hand.app.project, hand.app.chosen.sel, hand.app.sketch_ses, &mut hand.app.status, 1);
        // Click bottom side at (20, 0) and pull down to (20, -10)
        hand.click2d(20.0, 0.0);
        let b = hand.on_screen2d((20.0, -10.0));
        hand.frame(vec![egui::Event::PointerMoved(b)]);
        hand.click2d(20.0, -10.0);

        // Click right side at (40, 15) and pull right to (50, 15)
        hand.click2d(40.0, 15.0);
        let d = hand.on_screen2d((50.0, 15.0));
        hand.frame(vec![egui::Event::PointerMoved(d)]);
        hand.click2d(50.0, 15.0);
        hand.key(egui::Key::Escape);

        let dims: Vec<usize> = app.project.sketches[si].constraints.iter().enumerate().filter_map(|(i, c)| if matches!(c, Constraint::Distance { .. }) { Some(i) } else { None }).collect();
        assert_eq!(dims.len(), 2, "setup: rectangle has width and height dimensions");
        match &app.project.sketches[si].constraints[dims[0]] {
            Constraint::Distance { axis, .. } => assert_eq!(*axis, 1, "width dimension has horizontal axis"),
            _ => panic!("expected distance constraint"),
        }
        match &app.project.sketches[si].constraints[dims[1]] {
            Constraint::Distance { axis, .. } => assert_eq!(*axis, 2, "height dimension has vertical axis"),
            _ => panic!("expected distance constraint"),
        }

        // Before rotate: width dimension is horizontal (0 deg), height is vertical (90 deg)
        let (ang0_before, cap0_before) = dim_line_angle_and_caption(&app, si, dims[0]);
        let (ang1_before, cap1_before) = dim_line_angle_and_caption(&app, si, dims[1]);
        assert!((ang0_before - 0.0).abs() < 1e-4 || (ang0_before - 180.0).abs() < 1e-4, "width dimension before turn is horizontal: {ang0_before}");
        assert!((ang1_before - 90.0).abs() < 1e-4, "height dimension before turn is vertical: {ang1_before}");
        assert!(cap0_before.starts_with("40"));
        assert!(cap1_before.starts_with("30"));

        // Rotate by 30 deg about centre (20, 15) by picking the bottom side at (20, 0)
        Hand::new(&mut app).sk_rotate((20.0, 0.0), (20.0, 15.0), 30.0);

        assert!(worst(&app, si) < 1e-6, "sketch solved after 30 deg turn: {:.2e}", worst(&app, si));

        // After turn: dimensions lie along turned sides (30 deg and 120 deg)
        let (ang0_turned, cap0_turned) = dim_line_angle_and_caption(&app, si, dims[0]);
        let (ang1_turned, cap1_turned) = dim_line_angle_and_caption(&app, si, dims[1]);
        assert!((ang0_turned - 30.0).abs() < 1e-3, "width dimension turned by 30 deg lies along side: got {ang0_turned} deg");
        assert!((ang1_turned - 120.0).abs() < 1e-3, "height dimension turned by 30 deg lies along side: got {ang1_turned} deg");
        assert!(cap0_turned.starts_with("40"));
        assert!(cap1_turned.starts_with("30"));

        // Turn back by -30 deg
        let side0 = app.project.sketches[si].rects[0].sides[0];
        let spot = Hand::new(&mut app).spot2d((1, side0)).expect("bottom side");
        Hand::new(&mut app).sk_rotate(spot, (20.0, 15.0), -30.0);

        assert!(worst(&app, si) < 1e-6, "sketch solved after turning back: {:.2e}", worst(&app, si));

        let (ang0_back, cap0_back) = dim_line_angle_and_caption(&app, si, dims[0]);
        let (ang1_back, cap1_back) = dim_line_angle_and_caption(&app, si, dims[1]);
        assert!((ang0_back - 0.0).abs() < 1e-4 || (ang0_back - 180.0).abs() < 1e-4, "width dimension turned back is horizontal: {ang0_back}");
        assert!((ang1_back - 90.0).abs() < 1e-4, "height dimension turned back is vertical: {ang1_back}");
        assert!(cap0_back.starts_with("40"));
        assert!(cap1_back.starts_with("30"));
    }

    #[test]
    fn dimensions_follow_rectangle_turned_by_angle_dimension() {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);

        // Fixed reference line at (0, 0) -> (50, 0)
        let la = app.project.add_line_entity(si, 0.0, 0.0, 50.0, 0.0, Purpose::Real);
        let line_pts = match app.project.sketches[si].entities.iter().find(|e| e.id == la).map(|e| e.kind) {
            Some(EntityKind::Line { a, b }) => (a, b),
            _ => panic!("reference line"),
        };
        app.project.sketches[si].constraints.extend([Constraint::Fixed { p: line_pts.0 }, Constraint::Fixed { p: line_pts.1 }]);

        // Rectangle 40 x 30
        Hand::new(&mut app).sk_tool(2).click2d(10.0, 10.0).click2d(50.0, 40.0).key(egui::Key::Escape);
        let side0 = app.project.sketches[si].rects[0].sides[0];
        assert!(app.project.dimension_rect(si, side0), "dimensions laid on rectangle");
        let r = app.project.sketches[si].rects[0].clone();
        for c in app.project.sketches[si].constraints.iter_mut() {
            if let Constraint::Distance { a, b, axis, .. } = c {
                if (*a == r.corners[0] && *b == r.corners[1]) || (*a == r.corners[1] && *b == r.corners[0]) {
                    *axis = 1;
                } else if (*a == r.corners[1] && *b == r.corners[2]) || (*a == r.corners[2] && *b == r.corners[1]) {
                    *axis = 2;
                }
            }
        }
        app.project.solve_sketch(si);
        let angle_dim = Constraint::AngleLines { a: line_pts.0, b: line_pts.1, c: r.corners[0], d: r.corners[1], deg: 30.0, expr: String::new(), driven: false, off: 0.0, at: None };
        app.project.give_rect_turn_to(si, &angle_dim);
        app.project.sketches[si].constraints.push(angle_dim);
        app.project.solve_sketch(si);

        let dims: Vec<usize> = app.project.sketches[si].constraints.iter().enumerate().filter_map(|(i, c)| if matches!(c, Constraint::Distance { .. }) { Some(i) } else { None }).collect();
        assert_eq!(dims.len(), 2, "setup: rectangle has width and height dimensions");

        assert!(worst(&app, si) < 1e-6, "sketch solved with angle dimension on rectangle side: {:.2e}", worst(&app, si));

        // Dimensions should lie along turned sides
        let (ang0_turned, cap0_turned) = dim_line_angle_and_caption(&app, si, dims[0]);
        let (ang1_turned, cap1_turned) = dim_line_angle_and_caption(&app, si, dims[1]);
        assert!((ang0_turned - 30.0).abs() < 1e-3, "width dimension turned by angle dim lies along side: got {ang0_turned} deg");
        assert!((ang1_turned - 120.0).abs() < 1e-3, "height dimension turned by angle dim lies along side: got {ang1_turned} deg");
        assert!(cap0_turned.starts_with("40"));
        assert!(cap1_turned.starts_with("30"));
    }
}
