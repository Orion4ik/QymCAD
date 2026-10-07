//! Pieces of a split body share the cut face in OpenCASCADE.
//!
//! When features are added to both pieces, the rebuild must not dispatch them to separate
//! worker threads at the same time: meshing writes into the shared face's triangulation,
//! corrupting memory.

use qymcad_core::model::Project;

#[test]
fn split_pieces_do_not_rebuild_side_by_side() {
    qymcad_kernel::set_parallel(true, 0);
    let mut p = Project::default();
    p.new_document();
    let root = p.root;
    p.set_active_component(Some(root));
    let part = p.add_part("Part 0");
    p.set_active_component(Some(part));

    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, 20.0, 20.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);

    let body = p.add_extrude(sid, 20.0);
    let pieces = p.add_split_body(body, 0, 0, 10.0, 2);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "split setup failed: {:?}", report.errors);
    assert_eq!(pieces.len(), 2, "expected 2 pieces from split");

    let e0 = p.regen_edges.get(&pieces[0]).and_then(|edges| edges.first().map(|e| e.id)).expect("piece 0 has edges");
    let e1 = p.regen_edges.get(&pieces[1]).and_then(|edges| edges.first().map(|e| e.id)).expect("piece 1 has edges");

    p.add_fillet(pieces[0], 1.0, vec![e0]);
    p.add_fillet(pieces[1], 1.0, vec![e1]);

    p.mark_all_dirty();
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "fillet rebuild failed: {:?}", report.errors);
    assert!(!report.waves.contains(&2), "split pieces share the cut face and must not be rebuilt side-by-side in a wave of 2: got waves {:?}", report.waves);
}
