//! INTERACTIVE 3D MANIPULATOR FOR HOLE PLACEMENT AND EDITING.

use super::{App, Sel};
use egui::Rect;

struct Fixture {
    app: App,
    #[allow(dead_code)]
    body: u64,
    mi: usize,
}

fn cuboid_body() -> Fixture {
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
    Fixture { app, body, mi }
}

fn rect() -> Rect {
    Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 800.0))
}

#[test]
fn hole_command_arms_interactive_manipulator_arrows() {
    let mut fix = cuboid_body();
    let top_face_idx = fix.app.project.bodies[fix.mi].faces.iter().position(|f| f.normal[2] > 0.9).expect("top face");
    fix.app.chosen.sel = Sel::Face(fix.mi, top_face_idx);

    fix.app.start_feat_cmd(7); // Hole command
    assert_eq!(fix.app.tools.armed.cmd_kind(), 7);

    let hs = qymcad_ui_state::handles(&fix.app.painting());
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
    let basis = fix.app.viewing.cam.basis();
    let scr = qymcad_ui_state::Screen { cam: &fix.app.viewing.cam, set: &fix.app.set, rect: rect(), basis: &basis };
    let step = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(20.0, 0.0) };
    qymcad_ui_state::drag_handle(&mut fix.app.tools.cmd, &mut fix.app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("off_u")), &scr, step);

    let new_u = qymcad_ui_state::cmd_val(&fix.app.tools.cmd, "off_u");
    assert!(new_u.abs() > 0.01, "dragging off_u handle must update off_u param");

    // Drag diameter handle
    let init_dia = qymcad_ui_state::cmd_val(&fix.app.tools.cmd, "diameter");
    let step_dia = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(30.0, 30.0) };
    qymcad_ui_state::drag_handle(&mut fix.app.tools.cmd, &mut fix.app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("diameter")), &scr, step_dia);
    let new_dia = qymcad_ui_state::cmd_val(&fix.app.tools.cmd, "diameter");
    assert_ne!(new_dia, init_dia, "dragging diameter arrow must update diameter param");
}

#[test]
fn edit_existing_hole_moves_offsets_and_point() {
    let mut fix = cuboid_body();
    let top_face_idx = fix.app.project.bodies[fix.mi].faces.iter().position(|f| f.normal[2] > 0.9).expect("top face");
    fix.app.chosen.sel = Sel::Face(fix.mi, top_face_idx);

    // Create Hole (cmd 7) at origin of top face
    fix.app.start_feat_cmd(7);
    assert_eq!(fix.app.tools.armed.cmd_kind(), 7);
    fix.app.apply_feat_cmd();
    qymcad_ui_state::rebuild_if_dirty(&mut fix.app.rebuild_ctx());

    let hole_node = fix.app.project.timeline.iter().find(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Hole { .. })).cloned().expect("hole feature node");
    let fid = hole_node.id;
    let initial_point = match hole_node.kind {
        qymcad_core::feature::FeatureKind::Hole { point, .. } => point,
        _ => unreachable!(),
    };

    // Reopen Hole for editing
    crate::gui::commands::start_feat_cmd_edit(&mut fix.app.part_ctx(), fid);
    assert_eq!(fix.app.tools.armed.cmd_kind(), 7);
    assert_eq!(fix.app.tools.cmd.edit, Some(fid));

    // When reopened, 4 manipulator arrows must be armed
    let hs = qymcad_ui_state::handles(&fix.app.painting());
    assert_eq!(hs.arrows.len(), 4, "hole manipulator must generate 4 interactive arrows during editing");

    // Drag off_u handle to shift hole
    let basis = fix.app.viewing.cam.basis();
    let scr = qymcad_ui_state::Screen { cam: &fix.app.viewing.cam, set: &fix.app.set, rect: rect(), basis: &basis };
    let step = qymcad_ui_state::PointerStep { at: egui::pos2(100.0, 100.0), by: egui::vec2(40.0, 0.0) };
    qymcad_ui_state::drag_handle(&mut fix.app.tools.cmd, &mut fix.app.regen, &hs, Some(qymcad_ui_state::Handle::Arrow("off_u")), &scr, step);

    let new_u = qymcad_ui_state::cmd_val(&fix.app.tools.cmd, "off_u");
    assert!(new_u.abs() > 0.01, "dragging off_u handle must update off_u param");

    // Set explicit shifts to verify precise translation
    if let Some(p) = fix.app.tools.cmd.params.iter_mut().find(|p| p.key == "off_u") {
        p.val = 15.0;
        p.txt = "15.0".into();
    }
    if let Some(p) = fix.app.tools.cmd.params.iter_mut().find(|p| p.key == "off_v") {
        p.val = -5.0;
        p.txt = "-5.0".into();
    }

    // Apply edited hole and rebuild
    fix.app.apply_feat_cmd();
    qymcad_ui_state::rebuild_if_dirty(&mut fix.app.rebuild_ctx());

    let updated_node = fix.app.project.timeline.iter().find(|n| n.id == fid).cloned().expect("updated hole node");
    let updated_point = match updated_node.kind {
        qymcad_core::feature::FeatureKind::Hole { point, .. } => point,
        _ => unreachable!(),
    };

    assert_ne!(updated_point, initial_point, "hole point must be updated after editing off_u/off_v");
    let dist = ((updated_point[0] - initial_point[0]).powi(2) + (updated_point[1] - initial_point[1]).powi(2) + (updated_point[2] - initial_point[2]).powi(2)).sqrt();
    assert!((dist - 15.0_f64.hypot(-5.0)).abs() < 1e-3, "hole point shift must match off_u and off_v distance");
}
