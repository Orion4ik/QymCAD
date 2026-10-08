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
        assert!(app.face_arrow_geometry().is_some(), "arrow gizmo must be available immediately");
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
        app.dragged.edge_handle_drag = Some(before);

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
}
