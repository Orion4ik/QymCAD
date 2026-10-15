//! EDGE RADIUS HANDLE GIZMO FLOW
//!
//! Fillet (cmd 4) and Chamfer (cmd 5) radius handles directly on selected edges,
//! with smooth mouse drag, real-time preview update, and face/edge pre-selection.
#[cfg(test)]
mod tests {
    use super::super::{App, Sel};

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 700.0))
    }

    /// Setup a cuboid body in a part.
    fn cuboid_body() -> (App, u64, usize) {
        let mut app = App::default();
        let part = app.project.add_part("part");
        app.enter_component(part);
        let si = app.create_sketch_on(qymcad_core::feature::SketchPlane::default());
        app.project.add_rect_entity(si, 0.0, 0.0, 60.0, 40.0, qymcad_core::feature::Purpose::Real);
        app.project.regen_sketch(si);
        app.finish_sketch_edit();
        app.chosen.sel = Sel::Sketch(si);
        app.start_feat_cmd(1);
        if let Some(p) = app.tools.cmd.params.iter_mut().find(|p| p.key == "height") {
            p.val = 20.0;
            p.txt = "20".into();
        }
        app.apply_feat_cmd();
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        let body = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the body");
        let mi = app.project.mesh_index(body).expect("mesh index");
        (app, body, mi)
    }

    #[test]
    fn preselected_face_arms_push_face_arrow_immediately() {
        let (mut app, body, mi) = cuboid_body();
        // Pre-select top face
        let top_face_idx = app.project.bodies[mi].faces.iter().position(|f| f.normal[2] > 0.9).expect("top face");
        app.chosen.sel = Sel::Face(mi, top_face_idx);

        // Open push face command
        app.start_feat_cmd(25);
        assert_eq!(app.tools.armed.cmd_kind(), 25);
        let face_id = app.project.bodies[mi].faces[top_face_idx].id;
        assert!(app.tools.gsel.faces.contains(&face_id), "pre-selected face must be kept in gsel.faces");
        assert_eq!(app.tools.gsel.faces_body, Some(body), "faces_body must be set to the body");
        assert!(qymcad_ui_state::face_arrow_geometry(&app.painting()).is_some(), "arrow gizmo must be available immediately");
    }

    #[test]
    fn edge_radius_geometry_produces_handle_on_selected_edge() {
        let (mut app, body, _mi) = cuboid_body();
        qymcad_ui_state::select_body(&mut app.project, &mut app.chosen.sel, &mut app.viewing.view, body);
        app.start_feat_cmd(4); // Fillet
        crate::gui::commands::refresh_edges(&mut app.part_ctx());
        assert!(!app.edges.ids.is_empty(), "edge cache must have edges");

        let first_edge_id = app.edges.ids[0];
        app.tools.gsel.edges.insert(first_edge_id);

        let gizmo = qymcad_ui_state::edge_radius_geometry(&app.painting()).expect("edge radius gizmo must exist");
        assert_eq!(gizmo.param_key, "radius");
        assert!((gizmo.radius - 2.0).abs() < 1e-4, "default radius must be 2.0");

        let r = rect();
        let basis = app.viewing.cam.basis();
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: r, basis: &basis };
        let s_tip = scr.at(gizmo.tip).0;

        assert!(qymcad_ui_state::edge_handle_hit(&app.painting(), r, s_tip, &basis), "cursor at gizmo tip must hit handle");
        assert!(!qymcad_ui_state::edge_handle_hit(&app.painting(), r, s_tip + egui::vec2(200.0, 200.0), &basis), "cursor far away must not hit");
    }

    #[test]
    fn edge_radius_drag_updates_radius_and_field() {
        let (mut app, body, _mi) = cuboid_body();
        qymcad_ui_state::select_body(&mut app.project, &mut app.chosen.sel, &mut app.viewing.view, body);
        app.start_feat_cmd(4); // Fillet
        crate::gui::commands::refresh_edges(&mut app.part_ctx());
        let first_edge_id = app.edges.ids[0];
        app.tools.gsel.edges.insert(first_edge_id);

        let r = rect();
        let basis = app.viewing.cam.basis();
        let before = qymcad_ui_state::cmd_val(&app.tools.cmd, "radius");
        app.dragged.handle_drag = Some(qymcad_ui_state::Handle::Arrow("radius"));

        let gizmo = qymcad_ui_state::edge_radius_geometry(&app.painting()).expect("gizmo should exist");
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: r, basis: &basis };
        qymcad_ui_state::edge_radius_drag_to(gizmo, &scr, &mut app.tools.cmd, &mut app.regen, egui::vec2(20.0, -20.0));
        let after = qymcad_ui_state::cmd_val(&app.tools.cmd, "radius");
        assert!((after - before).abs() > 0.01, "dragging handle must change the radius: before {before}, after {after}");

        let txt = app.tools.cmd.params.iter().find(|p| p.key == "radius").map(|p| p.txt.clone()).unwrap_or_default();
        assert!((txt.trim().parse::<f64>().unwrap_or(f64::NAN) - after).abs() < 0.05, "param text field must stay in sync with value");
    }

    #[test]
    fn preselected_edge_arms_fillet_immediately() {
        let (mut app, body, _mi) = cuboid_body();
        qymcad_ui_state::select_body(&mut app.project, &mut app.chosen.sel, &mut app.viewing.view, body);
        crate::gui::commands::refresh_edges(&mut app.part_ctx());
        assert!(!app.edges.ids.is_empty(), "edge cache should have edges");
        let edge_id = app.edges.ids[0];
        app.chosen.sel = Sel::Edge(body, edge_id);

        app.start_feat_cmd(4); // Fillet
        assert!(app.tools.gsel.edges.contains(&edge_id), "pre-selected edge must be automatically added to gsel.edges");
        assert!(qymcad_ui_state::edge_radius_geometry(&app.painting()).is_some(), "edge radius handle must be available immediately");
    }

    #[test]
    fn axis_locking_switches_active_axis_or_ring() {
        let (mut app, _body, mi) = cuboid_body();
        let (o, _) = qymcad_ui_state::body_gizmo_geometry(&app.dragged.body_giz, app.viewing.cam, &app.project, &app.set, mi);
        app.dragged.body_giz.axis = Some(0);
        app.dragged.body_giz.drag = Some((mi, o, 0.0));

        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key { key: egui::Key::Y, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
        let _ = ctx.run_ui(raw, |ui| {
            let r = rect();
            let basis = app.viewing.cam.basis();
            let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: r, basis: &basis };
            let resp = ui.label("test");
            crate::gui::body_gizmo_drag_step(&mut app.dragged.body_giz, &app.set, ui.ctx(), &resp, &scr, &app.project, &mut app.regen);
        });
        assert_eq!(app.dragged.body_giz.axis, Some(1), "pressing Y must lock translation to Y axis");
    }

    #[test]
    fn hole_command_arms_interactive_manipulator_arrows() {
        let (mut app, _body, mi) = cuboid_body();
        let top_face_idx = app.project.bodies[mi].faces.iter().position(|f| f.normal[2] > 0.9).expect("top face");
        app.chosen.sel = Sel::Face(mi, top_face_idx);

        app.start_feat_cmd(7); // Hole command
        assert_eq!(app.tools.armed.cmd_kind(), 7);

        let hs = qymcad_ui_state::handles(&app.painting());
        assert_eq!(hs.arrows.len(), 4, "hole manipulator must generate 4 interactive arrows");

        let keys: Vec<&str> = hs.arrows.iter().map(|a| a.key).collect();
        assert!(keys.contains(&"depth"), "must contain depth handle");
        assert!(keys.contains(&"off_u"), "must contain off_u position handle");
        assert!(keys.contains(&"off_v"), "must contain off_v position handle");
        assert!(keys.contains(&"diameter"), "must contain diameter handle");

        // Diameter arrow base should be on the perimeter (distinct from center origin)
        let dia_arrow = hs.arrows.iter().find(|a| a.key == "diameter").expect("diameter arrow");
        let off_u_arrow = hs.arrows.iter().find(|a| a.key == "off_u").expect("off_u arrow");
        let dist = ((dia_arrow.base[0] - off_u_arrow.base[0]).powi(2) + (dia_arrow.base[1] - off_u_arrow.base[1]).powi(2) + (dia_arrow.base[2] - off_u_arrow.base[2]).powi(2)).sqrt();
        assert!(dist > 0.05, "diameter arrow must be placed on rim, separated from center origin");

        // Drag off_u handle
        let basis = app.viewing.cam.basis();
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
        let step = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(20.0, 0.0) };
        qymcad_ui_state::drag_handle(&mut app.tools.cmd, &mut app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("off_u")), &scr, step);

        let new_u = qymcad_ui_state::cmd_val(&app.tools.cmd, "off_u");
        assert!(new_u.abs() > 0.01, "dragging off_u handle must update off_u param");

        // Drag diameter handle
        let init_dia = qymcad_ui_state::cmd_val(&app.tools.cmd, "diameter");
        let step_dia = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(30.0, 30.0) };
        qymcad_ui_state::drag_handle(&mut app.tools.cmd, &mut app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("diameter")), &scr, step_dia);
        let new_dia = qymcad_ui_state::cmd_val(&app.tools.cmd, "diameter");
        assert_ne!(new_dia, init_dia, "dragging diameter arrow must update diameter param");
    }

    #[test]
    fn subdiv_command_interactive_manipulator_and_preview() {
        let (mut app, _body, mi) = cuboid_body();
        app.chosen.sel = Sel::Mesh(mi);

        app.start_feat_cmd(37); // Subdiv command
        assert_eq!(app.tools.armed.cmd_kind(), 37);

        let hs = qymcad_ui_state::handles(&app.painting());
        assert_eq!(hs.arrows.len(), 1, "subdiv manipulator must generate crease sharpness arrow");
        assert_eq!(hs.arrows[0].key, "sharp", "handle must be for crease sharpness");

        // Drag crease sharpness handle
        let basis = app.viewing.cam.basis();
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
        let step = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(0.0, -30.0) };
        qymcad_ui_state::drag_handle(&mut app.tools.cmd, &mut app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("sharp")), &scr, step);

        let new_sharp = qymcad_ui_state::cmd_val(&app.tools.cmd, "sharp");
        assert!(new_sharp > 0.0, "dragging sharp handle must increase crease sharpness");

        // Applying subdiv creates a new subdivided body in the timeline
        let prev_len = app.project.timeline.len();
        app.apply_feat_cmd();
        assert_eq!(app.project.timeline.len(), prev_len + 1, "applying subdiv creates a new timeline node");
    }

    #[test]
    fn edit_existing_hole_moves_offsets_and_point() {
        let (mut app, _body, mi) = cuboid_body();
        let top_face_idx = app.project.bodies[mi].faces.iter().position(|f| f.normal[2] > 0.9).expect("top face");
        app.chosen.sel = Sel::Face(mi, top_face_idx);

        // Create Hole (cmd 7) at origin of top face
        app.start_feat_cmd(7);
        assert_eq!(app.tools.armed.cmd_kind(), 7);
        app.apply_feat_cmd();
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());

        let hole_node = app.project.timeline.iter().find(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Hole { .. })).cloned().expect("hole feature node");
        let fid = hole_node.id;
        let initial_point = match hole_node.kind {
            qymcad_core::feature::FeatureKind::Hole { point, .. } => point,
            _ => unreachable!(),
        };

        // Reopen Hole for editing
        crate::gui::commands::start_feat_cmd_edit(&mut app.part_ctx(), fid);
        assert_eq!(app.tools.armed.cmd_kind(), 7);
        assert_eq!(app.tools.cmd.edit, Some(fid));

        // When reopened, 4 manipulator arrows must be armed
        let hs = qymcad_ui_state::handles(&app.painting());
        assert_eq!(hs.arrows.len(), 4, "hole manipulator must generate 4 interactive arrows during editing");

        // Drag off_u handle to shift hole
        let basis = app.viewing.cam.basis();
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
        let step = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(40.0, 0.0) };
        qymcad_ui_state::drag_handle(&mut app.tools.cmd, &mut app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("off_u")), &scr, step);

        let new_u = qymcad_ui_state::cmd_val(&app.tools.cmd, "off_u");
        assert!(new_u.abs() > 0.01, "dragging off_u handle must update off_u param");

        // Set explicit shifts to verify precise translation
        if let Some(p) = app.tools.cmd.params.iter_mut().find(|p| p.key == "off_u") {
            p.val = 15.0;
            p.txt = "15.0".into();
        }
        if let Some(p) = app.tools.cmd.params.iter_mut().find(|p| p.key == "off_v") {
            p.val = -5.0;
            p.txt = "-5.0".into();
        }

        // Apply edited hole and rebuild
        app.apply_feat_cmd();
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());

        let updated_node = app.project.timeline.iter().find(|n| n.id == fid).cloned().expect("updated hole node");
        let updated_point = match updated_node.kind {
            qymcad_core::feature::FeatureKind::Hole { point, .. } => point,
            _ => unreachable!(),
        };

        assert_ne!(updated_point, initial_point, "hole point must be updated after editing off_u/off_v");
        let dist = ((updated_point[0] - initial_point[0]).powi(2) + (updated_point[1] - initial_point[1]).powi(2) + (updated_point[2] - initial_point[2]).powi(2)).sqrt();
        assert!((dist - 15.0_f64.hypot(-5.0)).abs() < 1e-3, "hole point shift must match off_u and off_v distance");
    }
}
