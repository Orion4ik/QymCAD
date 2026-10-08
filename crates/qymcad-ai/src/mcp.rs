//! Model Context Protocol (MCP) server endpoints and tool registry.

use crate::commands::CadCommand;
use crate::transaction::execute_transaction;
use qymcad_core::model::Project;
use qymcad_materials::MaterialRegistry;
use serde::{Deserialize, Serialize};

/// Tool definition in the standard Model Context Protocol schema.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Unique tool name (e.g. "cad.create_primitive").
    pub name: String,
    /// Human/AI description of what the tool accomplishes.
    pub description: String,
    /// JSON schema describing the required and optional input arguments.
    pub input_schema: serde_json::Value,
}

/// Incoming tool call request formatted according to MCP specifications.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpCallRequest {
    /// Name of tool being invoked.
    pub name: String,
    /// JSON argument values passed by the LLM.
    pub arguments: serde_json::Value,
}

/// Outgoing result produced by an MCP tool invocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpCallResponse {
    /// Whether the execution completed successfully.
    pub is_error: bool,
    /// Content blocks returned to the model.
    pub content: Vec<McpContentBlock>,
}

/// Content payload block inside an MCP response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpContentBlock {
    /// Type of content (typically "text").
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text payload (often serialized JSON or diagnostic report).
    pub text: String,
}

/// Returns the registry of all CAD MCP tools exposed by QymCAD.
pub fn get_mcp_tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "cad.create_primitive".into(),
            description: "Creates a solid geometric primitive (box, cylinder, sphere, cone, torus) with specified dimensions.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["box", "cylinder", "sphere", "cone", "torus"] },
                    "dimensions": { "type": "array", "items": { "type": "number" }, "description": "Dimensions in mm [X, Y, Z] or [radius, height]" }
                },
                "required": ["kind", "dimensions"]
            }),
        },
        McpToolDefinition {
            name: "cad.extrude".into(),
            description: "Extrudes a sketch profile along its normal into a 3D solid body.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "sketch_id": { "type": "integer" },
                    "distance": { "type": "number", "description": "Extrusion distance in mm" },
                    "symmetric": { "type": "boolean", "default": false }
                },
                "required": ["sketch_id", "distance"]
            }),
        },
        McpToolDefinition {
            name: "cad.assign_material".into(),
            description: "Assigns an engineering material (e.g. aluminum 6061-T6, AISI 1018, PLA) to a body.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_id": { "type": "integer" },
                    "material_id": { "type": "string", "description": "Material identifier or name" }
                },
                "required": ["body_id", "material_id"]
            }),
        },
        McpToolDefinition {
            name: "cad.calculate_mass_properties".into(),
            description: "Calculates volume, surface area, mass, center of mass, and raw material cost.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_id": { "type": "integer" }
                },
                "required": ["body_id"]
            }),
        },
        McpToolDefinition {
            name: "cad.analyze_dfm".into(),
            description: "Runs Design For Manufacturing checks (overhang angles, support area, min thickness).".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_id": { "type": "integer" },
                    "process": { "type": "string", "enum": ["fdm", "cnc", "sla"] }
                },
                "required": ["body_id"]
            }),
        },
        McpToolDefinition {
            name: "cad.estimate_cost".into(),
            description: "Estimates manufacturing cost including material, machine time, labor, and setup.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_id": { "type": "integer" },
                    "cycle_time_hours": { "type": "number" },
                    "quantity": { "type": "integer" }
                },
                "required": ["body_id", "cycle_time_hours", "quantity"]
            }),
        },
        McpToolDefinition {
            name: "cad.subdivide_mesh".into(),
            description: "Applies OpenSubdiv Catmull-Clark subdivision with optional edge crease sharpness.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "body_id": { "type": "integer" },
                    "levels": { "type": "integer", "minimum": 1, "maximum": 4 },
                    "crease_sharpness": { "type": "number", "minimum": 0.0 }
                },
                "required": ["body_id", "levels"]
            }),
        },
    ]
}

/// Dispatches an MCP tool call to the internal CAD transaction engine.
pub fn dispatch_mcp_tool_call(project: &mut Project, materials: &MaterialRegistry, call: &McpCallRequest) -> McpCallResponse {
    let tx_id = format!("mcp_{}", call.name);

    let cmd_result: Result<CadCommand, String> = match call.name.as_str() {
        "cad.create_primitive" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("create_primitive");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.extrude" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("extrude");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.assign_material" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("assign_material");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.calculate_mass_properties" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("calculate_mass_properties");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.analyze_dfm" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("analyze_dfm");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.estimate_cost" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("estimate_cost");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        "cad.subdivide_mesh" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("subdivide_mesh");
            serde_json::from_value(val).map_err(|e| e.to_string())
        }
        unknown => Err(format!("Unknown MCP tool: {unknown}")),
    };

    match cmd_result {
        Ok(cmd) => {
            let resp = execute_transaction(project, &cmd, materials, &tx_id);
            let text = serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.message.clone());
            McpCallResponse { is_error: !resp.success, content: vec![McpContentBlock { content_type: "text".into(), text }] }
        }
        Err(err) => McpCallResponse { is_error: true, content: vec![McpContentBlock { content_type: "text".into(), text: format!("Argument error: {err}") }] },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_mcp_tools_returns_cad_tools() {
        let tools = get_mcp_tool_definitions();
        assert!(tools.len() >= 6);
        assert!(tools.iter().any(|t| t.name == "cad.create_primitive"));
        assert!(tools.iter().any(|t| t.name == "cad.estimate_cost"));
    }

    #[test]
    fn dispatch_mcp_create_primitive() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        let req = McpCallRequest {
            name: "cad.create_primitive".into(),
            arguments: serde_json::json!({
                "kind": "box",
                "dimensions": [40.0, 20.0, 10.0]
            }),
        };

        let resp = dispatch_mcp_tool_call(&mut proj, &mats, &req);
        assert!(!resp.is_error);
        assert!(resp.content[0].text.contains("primitive"));
    }

    #[test]
    fn dispatch_mcp_subdivide_mesh() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        let req = McpCallRequest {
            name: "cad.subdivide_mesh".into(),
            arguments: serde_json::json!({
                "body_id": 1,
                "levels": 2,
                "crease_sharpness": 2.5
            }),
        };

        let resp = dispatch_mcp_tool_call(&mut proj, &mats, &req);
        assert!(!resp.is_error);
        assert!(resp.content[0].text.contains("OpenSubdiv"));
    }
}
