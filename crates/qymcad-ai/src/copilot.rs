//! Engineering Copilot agent: translates natural language instructions into structured CAD commands.

use crate::commands::{CadCommand, PrimitiveKind};

/// Parses a natural language user request into a structured CAD command.
pub fn parse_natural_language_intent(prompt: &str) -> Option<CadCommand> {
    let text = prompt.trim().to_lowercase();

    // 1. Cube / Box primitive
    if text.contains("cube") || (text.contains("box") && (text.contains("create") || text.contains("make"))) {
        let size = extract_first_number(&text).unwrap_or(20.0);
        return Some(CadCommand::CreatePrimitive { kind: PrimitiveKind::Box, dimensions: vec![size, size, size], position: None });
    }

    // 2. Cylinder primitive
    if text.contains("cylinder") && (text.contains("create") || text.contains("make")) {
        let nums = extract_all_numbers(&text);
        let radius = nums.first().copied().unwrap_or(10.0);
        let height = nums.get(1).copied().unwrap_or(30.0);
        return Some(CadCommand::CreatePrimitive { kind: PrimitiveKind::Cylinder, dimensions: vec![radius, height], position: None });
    }

    // 3. Extrude
    if text.contains("extrude") {
        let distance = extract_first_number(&text).unwrap_or(10.0);
        let symmetric = text.contains("symmetric") || text.contains("both sides");
        return Some(CadCommand::Extrude { sketch_id: 1, distance, symmetric });
    }

    // 4. Fillet
    if text.contains("fillet") || text.contains("round") {
        let radius = extract_first_number(&text).unwrap_or(2.0);
        return Some(CadCommand::Fillet { body_id: 1, edge_indices: vec![0, 1, 2, 3], radius });
    }

    // 5. Chamfer
    if text.contains("chamfer") || text.contains("bevel") {
        let distance = extract_first_number(&text).unwrap_or(1.0);
        return Some(CadCommand::Chamfer { body_id: 1, edge_indices: vec![0, 1, 2, 3], distance });
    }

    // 6. Material assignment
    if text.contains("material") || text.contains("assign") || text.contains("make it") {
        if text.contains("aluminum") || text.contains("6061") {
            return Some(CadCommand::AssignMaterial { body_id: 1, material_id: "al_6061_t6".into() });
        }
        if text.contains("steel") || text.contains("1018") {
            return Some(CadCommand::AssignMaterial { body_id: 1, material_id: "steel_1018".into() });
        }
        if text.contains("pla") {
            return Some(CadCommand::AssignMaterial { body_id: 1, material_id: "pla".into() });
        }
        if text.contains("petg") {
            return Some(CadCommand::AssignMaterial { body_id: 1, material_id: "petg".into() });
        }
        if text.contains("titanium") {
            return Some(CadCommand::AssignMaterial { body_id: 1, material_id: "ti_6al_4v".into() });
        }
    }

    // 7. Mass properties
    if text.contains("mass") || text.contains("weight") || text.contains("center of mass") {
        return Some(CadCommand::CalculateMassProperties { body_id: 1, material_id: None });
    }

    // 8. DFM / Printability check
    if text.contains("dfm") || text.contains("printability") || text.contains("check support") || text.contains("overhang") {
        return Some(CadCommand::AnalyzeDfm { body_id: 1, process: "fdm".into() });
    }

    // 9. Cost estimation
    if text.contains("cost") || text.contains("price") || text.contains("estimate") {
        let qty = extract_first_number(&text).map(|n| n as u32).unwrap_or(1);
        return Some(CadCommand::EstimateCost { body_id: 1, cycle_time_hours: 1.5, quantity: qty, material_id: None });
    }

    // 10. OpenSubdiv / Subdivision
    if text.contains("subdiv") || text.contains("smooth mesh") {
        let levels = extract_first_number(&text).map(|n| n as usize).unwrap_or(2);
        let crease = if text.contains("crease") || text.contains("sharp") {
            let nums = extract_all_numbers(&text);
            nums.get(1).copied()
        } else {
            None
        };
        return Some(CadCommand::SubdivideMesh { body_id: 1, levels, crease_sharpness: crease });
    }

    None
}

fn extract_first_number(s: &str) -> Option<f64> {
    extract_all_numbers(s).into_iter().next()
}

fn extract_all_numbers(s: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && !current.contains('.')) {
            current.push(c);
        } else if !current.is_empty() {
            if let Ok(num) = current.parse::<f64>() {
                numbers.push(num);
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        if let Ok(num) = current.parse::<f64>() {
            numbers.push(num);
        }
    }

    numbers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cube_prompt() {
        let cmd = parse_natural_language_intent("Create a 100 mm cube").expect("parsed");
        match cmd {
            CadCommand::CreatePrimitive { kind, dimensions, .. } => {
                assert_eq!(kind, PrimitiveKind::Box);
                assert_eq!(dimensions, vec![100.0, 100.0, 100.0]);
            }
            _ => panic!("wrong command variant"),
        }
    }

    #[test]
    fn parse_fillet_prompt() {
        let cmd = parse_natural_language_intent("Add a 5 mm fillet to external edges").expect("parsed");
        match cmd {
            CadCommand::Fillet { radius, .. } => assert_eq!(radius, 5.0),
            _ => panic!("wrong command variant"),
        }
    }

    #[test]
    fn parse_material_prompt() {
        let cmd = parse_natural_language_intent("Assign Aluminum 6061 material").expect("parsed");
        match cmd {
            CadCommand::AssignMaterial { material_id, .. } => assert_eq!(material_id, "al_6061_t6"),
            _ => panic!("wrong command variant"),
        }
    }
}
