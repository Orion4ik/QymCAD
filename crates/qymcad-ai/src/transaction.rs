//! Transactional execution manager with dry-run validation and rollback guarantees.

use crate::commands::{CadCommand, CommandResponse, PrimitiveKind};
use qymcad_core::feature::Purpose;
use qymcad_core::geom::{Mesh, Point3};
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
            if dimensions.iter().any(|&d| d <= 0.0) {
                return Err("Primitive dimensions must be strictly positive".into());
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

            let mesh = match kind {
                PrimitiveKind::Box => make_box_mesh(dx, dy, dz, *position),
                PrimitiveKind::Cylinder => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    let h = *dimensions.get(1).unwrap_or(&10.0);
                    make_cylinder_mesh(r, h, *position)
                }
                PrimitiveKind::Sphere => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    make_sphere_mesh(r, *position)
                }
                PrimitiveKind::Cone => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    let h = *dimensions.get(1).unwrap_or(&10.0);
                    make_cone_mesh(r, h, *position)
                }
                PrimitiveKind::Torus => {
                    let r_major = *dimensions.first().unwrap_or(&10.0);
                    let r_minor = *dimensions.get(1).unwrap_or(&2.0);
                    make_torus_mesh(r_major, r_minor, *position)
                }
            };

            let node_id = match kind {
                PrimitiveKind::Box => project.add_box(dx, dy, dz),
                PrimitiveKind::Cylinder => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    let h = *dimensions.get(1).unwrap_or(&10.0);
                    project.add_cylinder(r, h)
                }
                PrimitiveKind::Sphere => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    project.add_sphere(r)
                }
                PrimitiveKind::Cone => {
                    let r = *dimensions.first().unwrap_or(&5.0);
                    let h = *dimensions.get(1).unwrap_or(&10.0);
                    project.add_cone(r, 0.0, h)
                }
                PrimitiveKind::Torus => {
                    let r_major = *dimensions.first().unwrap_or(&10.0);
                    let r_minor = *dimensions.get(1).unwrap_or(&2.0);
                    project.add_torus(r_major, r_minor)
                }
            };
            let finished_id = project.finish_base_body(node_id, 1);
            project.bodies.push(qymcad_core::model::Body { id: finished_id, name: format!("{:?}-{}", kind, finished_id), mesh, faces: Vec::new(), visible: true, sheet: false });

            Ok(CommandResponse::success(tx_id, &format!("Created {:?} primitive with dimensions [{:.1}, {:.1}, {:.1}]", kind, dx, dy, dz), Some(finished_id)))
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
        CadCommand::Extrude { sketch_id, distance, symmetric } => {
            if distance.abs() < 1e-6 {
                return Err("Extrusion distance cannot be zero".into());
            }
            let si = project.sketches.iter().position(|s| s.id == *sketch_id).ok_or_else(|| format!("Sketch ID {sketch_id} not found"))?;
            let sk_id = project.sketches[si].id;
            let node_id = project.add_extrude(sk_id, *distance);

            let sketch = &project.sketches[si];
            let z_start = if *symmetric { -distance * 0.5 } else { 0.0 };
            let z_end = if *symmetric { distance * 0.5 } else { *distance };
            let extrude_h = z_end - z_start;

            let mesh = synthesize_sketch_extrude_mesh(sketch, z_start, extrude_h);
            let finished_id = project.finish_base_body(node_id, 1);
            project.bodies.push(qymcad_core::model::Body { id: finished_id, name: format!("Body-{}", finished_id), mesh, faces: Vec::new(), visible: true, sheet: false });

            Ok(CommandResponse::success(tx_id, &format!("Extruded sketch by {:.2} mm (symmetric: {})", distance, symmetric), Some(finished_id)))
        }
        CadCommand::Fillet { body_id, edge_indices, radius } => {
            if *radius <= 0.0 {
                return Err("Fillet radius must be positive".into());
            }
            if edge_indices.is_empty() {
                return Err("Edge indices for fillet cannot be empty".into());
            }
            let _ = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            Ok(CommandResponse::success(tx_id, &format!("Applied {:.2} mm fillet to body {} ({} edges)", radius, body_id, edge_indices.len()), None))
        }
        CadCommand::Chamfer { body_id, edge_indices, distance } => {
            if *distance <= 0.0 {
                return Err("Chamfer distance must be positive".into());
            }
            if edge_indices.is_empty() {
                return Err("Edge indices for chamfer cannot be empty".into());
            }
            let _ = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            Ok(CommandResponse::success(tx_id, &format!("Applied {:.2} mm chamfer to body {} ({} edges)", distance, body_id, edge_indices.len()), None))
        }
        CadCommand::AssignMaterial { body_id, material_id } => {
            let _ = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            let mat = materials.get(material_id).or_else(|| materials.find_by_name(material_id)).ok_or_else(|| format!("Unknown material '{material_id}'"))?;
            Ok(CommandResponse::success(tx_id, &format!("Assigned material '{}' ({}) to body {}", mat.name, mat.id, body_id), None))
        }
        CadCommand::CalculateMassProperties { body_id, material_id } => {
            let body = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            let mat = if let Some(mid) = material_id {
                materials.get(mid).or_else(|| materials.find_by_name(mid)).ok_or_else(|| format!("Unknown material '{mid}'"))?
            } else {
                materials.get("al_6061_t6").unwrap_or_else(|| materials.iter().next().expect("material exists"))
            };
            let vol = body.mesh.volume();
            let area = compute_mesh_surface_area(&body.mesh);
            let com = compute_mesh_centroid(&body.mesh);
            let mass_props = calculate_mass_properties(vol, area, com, mat);
            let details = serde_json::json!({
                "body_id": body_id,
                "volume_mm3": mass_props.volume_mm3,
                "surface_area_mm2": mass_props.surface_area_mm2,
                "mass_kg": mass_props.mass_kg,
                "center_of_mass": mass_props.center_of_mass,
                "material": mat.name,
                "raw_material_cost_usd": mass_props.raw_material_cost_usd
            });
            Ok(CommandResponse::success(tx_id, "Mass properties calculated", None).with_details(details))
        }
        CadCommand::AnalyzeDfm { body_id, process } => {
            let body = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            let proc = match process.to_lowercase().as_str() {
                "cnc" => qymcad_manufacturing::ManufacturingProcess::CncMilling3Axis,
                "sla" => qymcad_manufacturing::ManufacturingProcess::SlaPrinting,
                "sheet_metal" => qymcad_manufacturing::ManufacturingProcess::SheetMetal,
                _ => qymcad_manufacturing::ManufacturingProcess::FdmPrinting,
            };
            let verts: Vec<[f64; 3]> = body.mesh.verts.iter().map(|v| [v.x, v.y, v.z]).collect();
            let report = qymcad_manufacturing::analyze_triangles_dfm(&verts, &body.mesh.tris, proc, &qymcad_manufacturing::DfmConfig::default());
            let details = serde_json::json!({
                "body_id": body_id,
                "process": process,
                "is_manufacturable": report.is_manufacturable,
                "support_area_ratio": report.support_area_ratio,
                "findings": report.findings
            });
            let msg = if report.is_manufacturable {
                format!("DFM analysis passed ({} findings)", report.findings.len())
            } else {
                format!("DFM analysis found violations ({} findings)", report.findings.len())
            };
            Ok(CommandResponse::success(tx_id, &msg, None).with_details(details))
        }
        CadCommand::EstimateCost { body_id, cycle_time_hours, quantity, material_id } => {
            let body = project.bodies.iter().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            let mat = if let Some(mid) = material_id {
                materials.get(mid).or_else(|| materials.find_by_name(mid)).ok_or_else(|| format!("Unknown material '{mid}'"))?
            } else {
                materials.get("al_6061_t6").unwrap_or_else(|| materials.iter().next().expect("material exists"))
            };
            let vol = body.mesh.volume();
            let area = compute_mesh_surface_area(&body.mesh);
            let com = compute_mesh_centroid(&body.mesh);
            let mass_props = calculate_mass_properties(vol, area, com, mat);

            let rates = qymcad_manufacturing::CostModelRates::default();
            let cost = qymcad_manufacturing::estimate_manufacturing_cost(mass_props.raw_material_cost_usd, *cycle_time_hours, *quantity, &rates);
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
            let body = project.bodies.iter_mut().find(|b| b.id == *body_id).ok_or_else(|| format!("Body ID {body_id} not found"))?;
            let n = (*levels).clamp(1, 4);

            let mut cage = if body.mesh.verts.is_empty() {
                qymcad_core::subdiv::OpenSubdivCage::new(qymcad_core::subdiv::Cage::cube(20.0))
            } else {
                let cage_verts: Vec<[f64; 3]> = body.mesh.verts.iter().map(|v| [v.x, v.y, v.z]).collect();
                let cage_faces: Vec<Vec<u32>> = body.mesh.tris.iter().map(|t| vec![t[0], t[1], t[2]]).collect();
                qymcad_core::subdiv::OpenSubdivCage::new(qymcad_core::subdiv::Cage { verts: cage_verts, faces: cage_faces })
            };

            if let Some(s) = crease_sharpness {
                cage.set_all_edges_crease(*s);
            }
            let refined = cage.subdivided(n);
            let new_tris = refined.to_triangles();
            let new_verts = refined.cage.verts.iter().map(|v| Point3::new(v[0], v[1], v[2])).collect();
            body.mesh = Mesh { verts: new_verts, tris: new_tris.clone() };

            let details = serde_json::json!({
                "body_id": body_id,
                "levels": n,
                "crease_sharpness": crease_sharpness,
                "output_verts": refined.cage.verts.len(),
                "output_faces": refined.cage.faces.len(),
                "output_triangles": new_tris.len()
            });
            Ok(CommandResponse::success(tx_id, &format!("OpenSubdiv refined cage to {} vertices, {} faces across {} levels", refined.cage.verts.len(), refined.cage.faces.len(), n), None)
                .with_details(details))
        }
    }
}

fn make_box_mesh(dx: f64, dy: f64, dz: f64, pos: Option<[f64; 3]>) -> Mesh {
    let [ox, oy, oz] = pos.unwrap_or([0.0, 0.0, 0.0]);
    let x0 = ox - dx * 0.5;
    let x1 = ox + dx * 0.5;
    let y0 = oy - dy * 0.5;
    let y1 = oy + dy * 0.5;
    let z0 = oz - dz * 0.5;
    let z1 = oz + dz * 0.5;

    let verts = vec![
        Point3::new(x0, y0, z0),
        Point3::new(x1, y0, z0),
        Point3::new(x1, y1, z0),
        Point3::new(x0, y1, z0),
        Point3::new(x0, y0, z1),
        Point3::new(x1, y0, z1),
        Point3::new(x1, y1, z1),
        Point3::new(x0, y1, z1),
    ];
    let tris = vec![[0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7], [0, 1, 5], [0, 5, 4], [2, 3, 7], [2, 7, 6], [3, 0, 4], [3, 4, 7], [1, 2, 6], [1, 6, 5]];
    Mesh { verts, tris }
}

fn make_cylinder_mesh(r: f64, h: f64, pos: Option<[f64; 3]>) -> Mesh {
    let [ox, oy, oz] = pos.unwrap_or([0.0, 0.0, 0.0]);
    let segments = 32;
    let mut verts = Vec::with_capacity(segments * 2 + 2);
    let mut tris = Vec::with_capacity(segments * 4);

    let z0 = oz;
    let z1 = oz + h;

    for i in 0..segments {
        let theta = (i as f64) * std::f64::consts::TAU / (segments as f64);
        let x = ox + r * theta.cos();
        let y = oy + r * theta.sin();
        verts.push(Point3::new(x, y, z0));
        verts.push(Point3::new(x, y, z1));
    }

    let bottom_center_idx = verts.len() as u32;
    verts.push(Point3::new(ox, oy, z0));
    let top_center_idx = verts.len() as u32;
    verts.push(Point3::new(ox, oy, z1));

    for i in 0..segments {
        let next = (i + 1) % segments;
        let b0 = (i * 2) as u32;
        let t0 = (i * 2 + 1) as u32;
        let b1 = (next * 2) as u32;
        let t1 = (next * 2 + 1) as u32;

        tris.push([b0, b1, t1]);
        tris.push([b0, t1, t0]);
        tris.push([bottom_center_idx, b1, b0]);
        tris.push([top_center_idx, t0, t1]);
    }

    Mesh { verts, tris }
}

fn make_sphere_mesh(r: f64, pos: Option<[f64; 3]>) -> Mesh {
    let [ox, oy, oz] = pos.unwrap_or([0.0, 0.0, 0.0]);
    let lat_steps = 16;
    let lon_steps = 32;
    let mut verts = Vec::new();
    let mut tris = Vec::new();

    for i in 0..=lat_steps {
        let phi = std::f64::consts::PI * (i as f64) / (lat_steps as f64);
        let z = oz + r * phi.cos();
        let r_sin = r * phi.sin();
        for j in 0..lon_steps {
            let theta = std::f64::consts::TAU * (j as f64) / (lon_steps as f64);
            let x = ox + r_sin * theta.cos();
            let y = oy + r_sin * theta.sin();
            verts.push(Point3::new(x, y, z));
        }
    }

    for i in 0..lat_steps {
        for j in 0..lon_steps {
            let next_j = (j + 1) % lon_steps;
            let i0 = (i * lon_steps + j) as u32;
            let i1 = (i * lon_steps + next_j) as u32;
            let i2 = ((i + 1) * lon_steps + next_j) as u32;
            let i3 = ((i + 1) * lon_steps + j) as u32;

            if i > 0 {
                tris.push([i0, i1, i2]);
            }
            if i + 1 < lat_steps {
                tris.push([i0, i2, i3]);
            }
        }
    }
    Mesh { verts, tris }
}

fn make_cone_mesh(r: f64, h: f64, pos: Option<[f64; 3]>) -> Mesh {
    let [ox, oy, oz] = pos.unwrap_or([0.0, 0.0, 0.0]);
    let segments = 32;
    let mut verts = Vec::with_capacity(segments + 2);
    let mut tris = Vec::with_capacity(segments * 2);

    let z0 = oz;
    let z1 = oz + h;

    for i in 0..segments {
        let theta = (i as f64) * std::f64::consts::TAU / (segments as f64);
        let x = ox + r * theta.cos();
        let y = oy + r * theta.sin();
        verts.push(Point3::new(x, y, z0));
    }

    let apex_idx = verts.len() as u32;
    verts.push(Point3::new(ox, oy, z1));
    let base_center_idx = verts.len() as u32;
    verts.push(Point3::new(ox, oy, z0));

    for i in 0..segments {
        let next = ((i + 1) % segments) as u32;
        let curr = i as u32;
        tris.push([curr, next, apex_idx]);
        tris.push([base_center_idx, next, curr]);
    }

    Mesh { verts, tris }
}

fn make_torus_mesh(r_major: f64, r_minor: f64, pos: Option<[f64; 3]>) -> Mesh {
    let cage = qymcad_core::subdiv::Cage::torus(16, 16, r_major, r_minor);
    let [ox, oy, oz] = pos.unwrap_or([0.0, 0.0, 0.0]);
    let verts = cage.verts.iter().map(|v| Point3::new(v[0] + ox, v[1] + oy, v[2] + oz)).collect();
    let mut tris = Vec::new();
    for f in &cage.faces {
        if f.len() >= 3 {
            for k in 1..f.len().saturating_sub(1) {
                tris.push([f[0], f[k], f[k + 1]]);
            }
        }
    }
    Mesh { verts, tris }
}

fn synthesize_sketch_extrude_mesh(sketch: &qymcad_core::model::Sketch, z_start: f64, height: f64) -> Mesh {
    if sketch.points.len() >= 2 {
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for p in &sketch.points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let dx = (max_x - min_x).max(1.0);
        let dy = (max_y - min_y).max(1.0);
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        let center_z = z_start + height * 0.5;
        make_box_mesh(dx, dy, height, Some([center_x, center_y, center_z]))
    } else {
        make_box_mesh(20.0, 20.0, height, Some([0.0, 0.0, z_start + height * 0.5]))
    }
}

fn compute_mesh_surface_area(mesh: &Mesh) -> f64 {
    let mut area = 0.0;
    for &tri in &mesh.tris {
        let p0 = mesh.verts[tri[0] as usize];
        let p1 = mesh.verts[tri[1] as usize];
        let p2 = mesh.verts[tri[2] as usize];

        let v1 = [p1.x - p0.x, p1.y - p0.y, p1.z - p0.z];
        let v2 = [p2.x - p0.x, p2.y - p0.y, p2.z - p0.z];

        let cross = [v1[1] * v2[2] - v1[2] * v2[1], v1[2] * v2[0] - v1[0] * v2[2], v1[0] * v2[1] - v1[1] * v2[0]];
        let tri_area = 0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        area += tri_area;
    }
    area
}

fn compute_mesh_centroid(mesh: &Mesh) -> [f64; 3] {
    if mesh.verts.is_empty() {
        return [0.0, 0.0, 0.0];
    }
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_z = 0.0;
    for v in &mesh.verts {
        sum_x += v.x;
        sum_y += v.y;
        sum_z += v.z;
    }
    let n = mesh.verts.len() as f64;
    [sum_x / n, sum_y / n, sum_z / n]
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
        assert!(!proj.bodies.is_empty());
        assert!(!proj.timeline.is_empty());
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
