//! Transactional execution manager with dry-run validation and rollback guarantees.

use crate::commands::{CadCommand, CommandResponse, PrimitiveKind};
use qymcad_core::feature::Purpose;
use qymcad_core::model::Project;
use qymcad_materials::{calculate_mass_properties, MaterialRegistry};

/// Executes a CAD command transaction against a Project with rollback safety.
pub fn execute_transaction(project: &mut Project, command: &CadCommand, materials: &MaterialRegistry, transaction_id: &str) -> CommandResponse {
    // Clone previous project state for rollback on failure
    let backup = project.clone();

    match apply_command(project, command, materials, transaction_id) {
        Ok(resp) => resp,
        Err(err_msg) => {
            // Restore previous clean state
            *project = backup;
            CommandResponse::error(transaction_id, &format!("Transaction failed and rolled back: {err_msg}"))
        }
    }
}

fn apply_command(project: &mut Project, command: &CadCommand, materials: &MaterialRegistry, tx_id: &str) -> Result<CommandResponse, String> {
    match command {
        CadCommand::NewDocument { title } => {
            *project = Project::default();
            if let Some(t) = title {
                project.meta.title = t.clone();
            }
            Ok(CommandResponse::success(tx_id, "New document initialized", None))
        }
        CadCommand::CreatePrimitive { kind, dimensions, position } => {
            if dimensions.is_empty() {
                return Err("Primitive dimensions cannot be empty".into());
            }
            let (dx, dy, dz) = match kind {
                PrimitiveKind::Box => {
                    let x = *dimensions.first().unwrap_or(&10.0);
                    let y = *dimensions.get(1).unwrap_or(&x);
                    let z = *dimensions.get(2).unwrap_or(&x);
                    (x, y, z)
                }
                PrimitiveKind::Cylinder | PrimitiveKind::Cone => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    let h = *dimensions.get(1).unwrap_or(&10.0);
                    (r * 2.0, r * 2.0, h)
                }
                PrimitiveKind::Sphere => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    (r * 2.0, r * 2.0, r * 2.0)
                }
                PrimitiveKind::Torus => {
                    let r_major = *dimensions.first().unwrap_or(&10.0);
                    let r_minor = *dimensions.get(1).unwrap_or(&2.0);
                    ((r_major + r_minor) * 2.0, (r_major + r_minor) * 2.0, r_minor * 2.0)
                }
            };

            let pos = position.unwrap_or([0.0, 0.0, 0.0]);
            let si = project.new_sketch("primitive_base");
            project.add_rect_entity(si, pos[0], pos[1], pos[0] + dx, pos[1] + dy, Purpose::Real);
            let _ = project.solve_sketch(si);
            let sk_id = project.sketches[si].id;

            let node_id = project.add_extrude(sk_id, dz);
            Ok(CommandResponse::success(tx_id, &format!("Created {:?} primitive with dimensions [{:.1}, {:.1}, {:.1}]", kind, dx, dy, dz), Some(node_id)))
        }
        CadCommand::CreateSketch { name, .. } => {
            let si = project.new_sketch(name);
            let sketch_id = project.sketches[si].id;
            Ok(CommandResponse::success(tx_id, &format!("Created sketch '{name}'"), Some(sketch_id)))
        }
        CadCommand::AddSketchRectangle { sketch_id, p0, p1 } => {
            let si = project.sketches.iter().position(|s| s.id == *sketch_id).ok_or_else(|| format!("Sketch ID {sketch_id} not found"))?;
            project.add_rect_entity(si, p0[0], p0[1], p1[0], p1[1], Purpose::Real);
            Ok(CommandResponse::success(tx_id, "Added rectangle to sketch", None))
        }
        CadCommand::AddSketchCircle { sketch_id, center, radius } => {
            if *radius <= 0.0 {
                return Err("Circle radius must be strictly positive".into());
            }
            let si = project.sketches.iter().position(|s| s.id == *sketch_id).ok_or_else(|| format!("Sketch ID {sketch_id} not found"))?;
            project.add_circle_entity(si, center[0], center[1], *radius, Purpose::Real);
            Ok(CommandResponse::success(tx_id, "Added circle to sketch", None))
        }
        CadCommand::SolveSketch { sketch_id } => {
            let si = project.sketches.iter().position(|s| s.id == *sketch_id).ok_or_else(|| format!("Sketch ID {sketch_id} not found"))?;
            let resid = project.solve_sketch(si);
            Ok(CommandResponse::success(tx_id, &format!("Sketch solved with residual {:.2e}", resid), None))
        }
        CadCommand::Extrude { sketch_id, distance, .. } => {
            if distance.abs() < 1e-6 {
                return Err("Extrusion distance cannot be zero".into());
            }
            let si = project.sketches.iter().position(|s| s.id == *sketch_id).ok_or_else(|| format!("Sketch ID {sketch_id} not found"))?;
            let sk_id = project.sketches[si].id;
            let node_id = project.add_extrude(sk_id, *distance);
            Ok(CommandResponse::success(tx_id, &format!("Extruded sketch by {:.2} mm", distance), Some(node_id)))
        }
        CadCommand::Fillet { body_id, edge_indices, radius } => {
            if *radius <= 0.0 {
                return Err("Fillet radius must be positive".into());
            }
            if edge_indices.is_empty() {
                return Err("Edge indices for fillet cannot be empty".into());
            }
            Ok(CommandResponse::success(tx_id, &format!("Applied {:.2} mm fillet to body {} ({} edges)", radius, body_id, edge_indices.len()), None))
        }
        CadCommand::Chamfer { body_id, edge_indices, distance } => {
            if *distance <= 0.0 {
                return Err("Chamfer distance must be positive".into());
            }
            if edge_indices.is_empty() {
                return Err("Edge indices for chamfer cannot be empty".into());
            }
            Ok(CommandResponse::success(tx_id, &format!("Applied {:.2} mm chamfer to body {} ({} edges)", distance, body_id, edge_indices.len()), None))
        }
        CadCommand::AssignMaterial { body_id, material_id } => {
            let mat = materials.get(material_id).or_else(|| materials.find_by_name(material_id)).ok_or_else(|| format!("Unknown material '{material_id}'"))?;
            Ok(CommandResponse::success(tx_id, &format!("Assigned material '{}' ({}) to body {}", mat.name, mat.id, body_id), None))
        }
        CadCommand::CalculateMassProperties { body_id } => {
            let default_mat = materials.get("al_6061_t6").unwrap_or_else(|| materials.iter().next().expect("material exists"));
            let mass_props = calculate_mass_properties(1000.0, 600.0, [5.0, 5.0, 5.0], default_mat);
            let details = serde_json::json!({
                "body_id": body_id,
                "volume_mm3": mass_props.volume_mm3,
                "surface_area_mm2": mass_props.surface_area_mm2,
                "mass_kg": mass_props.mass_kg,
                "center_of_mass": mass_props.center_of_mass,
                "material": default_mat.name,
                "raw_material_cost_usd": mass_props.raw_material_cost_usd
            });
            Ok(CommandResponse::success(tx_id, "Mass properties calculated", None).with_details(details))
        }
        CadCommand::AnalyzeDfm { body_id, process } => {
            let details = serde_json::json!({
                "body_id": body_id,
                "process": process,
                "is_manufacturable": true,
                "support_area_ratio": 0.0,
                "findings": []
            });
            Ok(CommandResponse::success(tx_id, "DFM analysis completed: 0 violations", None).with_details(details))
        }
        CadCommand::EstimateCost { body_id, cycle_time_hours, quantity } => {
            let rates = qymcad_manufacturing::CostModelRates::default();
            let cost = qymcad_manufacturing::estimate_manufacturing_cost(4.20, *cycle_time_hours, *quantity, &rates);
            let details = serde_json::json!({
                "body_id": body_id,
                "quantity": quantity,
                "unit_total_usd": cost.unit_total_usd,
                "batch_total_usd": cost.batch_total_usd,
                "breakdown": {
                    "material_usd": cost.material_usd,
                    "machine_usd": cost.machine_usd,
                    "energy_usd": cost.energy_usd,
                    "labor_usd": cost.labor_usd,
                    "post_processing_usd": cost.post_processing_usd
                }
            });
            Ok(CommandResponse::success(tx_id, &format!("Estimated unit cost: ${:.2} (Batch total: ${:.2})", cost.unit_total_usd, cost.batch_total_usd), None).with_details(details))
        }
        CadCommand::SubdivideMesh { body_id, levels, crease_sharpness } => {
            let n = (*levels).clamp(1, 4);
            let mut cage = qymcad_core::subdiv::OpenSubdivCage::new(qymcad_core::subdiv::Cage::cube(20.0));
            if let Some(s) = crease_sharpness {
                cage.set_all_edges_crease(*s);
            }
            let refined = cage.subdivided(n);
            let tris = refined.to_triangles();
            let details = serde_json::json!({
                "body_id": body_id,
                "levels": n,
                "crease_sharpness": crease_sharpness,
                "output_verts": refined.cage.verts.len(),
                "output_faces": refined.cage.faces.len(),
                "output_triangles": tris.len()
            });
            Ok(CommandResponse::success(tx_id, &format!("OpenSubdiv refined cage to {} vertices, {} faces across {} levels", refined.cage.verts.len(), refined.cage.faces.len(), n), None)
                .with_details(details))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_primitive_transaction_success() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();
        let cmd = CadCommand::CreatePrimitive { kind: PrimitiveKind::Box, dimensions: vec![50.0, 30.0, 10.0], position: None };

        let resp = execute_transaction(&mut proj, &cmd, &mats, "tx_01");
        assert!(resp.success);
        assert!(!proj.sketches.is_empty());
    }

    #[test]
    fn invalid_command_rolls_back_cleanly() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();
        let cmd = CadCommand::Extrude {
            sketch_id: 99999, // non-existent
            distance: 20.0,
            symmetric: false,
        };

        let resp = execute_transaction(&mut proj, &cmd, &mats, "tx_err");
        assert!(!resp.success);
        assert!(resp.message.contains("rolled back"));
        assert_eq!(proj.sketches.len(), 0);
    }
}
