//! Model Context Protocol (MCP) server endpoints, session lifecycle, and tool registry.

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
    #[serde(rename = "inputSchema", alias = "input_schema")]
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
    #[serde(rename = "isError", alias = "is_error")]
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

/// Stateful MCP server managing protocol handshake, session state, and CAD transactions.
pub struct McpServer {
    /// Active CAD document state.
    pub project: Project,
    /// Engineering materials catalogue.
    pub materials: MaterialRegistry,
    /// Whether initialize handshake was completed.
    pub initialized: bool,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    /// Creates a new MCP server with a clean project and default material registry.
    pub fn new() -> Self {
        Self { project: Project::default(), materials: MaterialRegistry::default(), initialized: false }
    }

    /// Creates an MCP server wrapping an existing CAD project.
    pub fn with_project(project: Project) -> Self {
        Self { project, materials: MaterialRegistry::default(), initialized: false }
    }

    /// Returns true if client has completed protocol initialization.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Handles a single incoming JSON-RPC 2.0 message according to the Model Context Protocol.
    pub fn handle_message(&mut self, msg: &str) -> Option<String> {
        let req_val: serde_json::Value = match serde_json::from_str(msg) {
            Ok(v) => v,
            Err(e) => {
                let err_res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": serde_json::Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                });
                return Some(err_res.to_string());
            }
        };

        let req = match req_val.as_object() {
            Some(obj) => obj,
            None => {
                let err_res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": serde_json::Value::Null,
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request: expected JSON object"
                    }
                });
                return Some(err_res.to_string());
            }
        };

        let id = req.get("id").cloned();
        let is_notification = id.is_none();

        // Validate JSON-RPC version
        if req.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            if is_notification {
                return None;
            }
            let err_res = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: 'jsonrpc' must be exactly '2.0'"
                }
            });
            return Some(err_res.to_string());
        }

        let method = match req.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                if is_notification {
                    return None;
                }
                let err_res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request: 'method' must be a string"
                    }
                });
                return Some(err_res.to_string());
            }
        };

        match method {
            "initialize" => {
                self.initialized = true;
                let res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "qymcad-mcp",
                            "version": "0.1.0"
                        }
                    }
                });
                Some(res.to_string())
            }
            "notifications/initialized" | "initialized" => {
                self.initialized = true;
                None
            }
            "notifications/cancelled" => None,
            "ping" => {
                let res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                });
                Some(res.to_string())
            }
            "tools/list" => {
                let tools = get_mcp_tool_definitions();
                let res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools
                    }
                });
                Some(res.to_string())
            }
            "tools/call" => {
                let params = match req.get("params").and_then(|p| p.as_object()) {
                    Some(p) => p,
                    None => {
                        let err_res = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32602,
                                "message": "Invalid params: 'params' object is required"
                            }
                        });
                        return Some(err_res.to_string());
                    }
                };

                let name = match params.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n.to_string(),
                    None => {
                        let err_res = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32602,
                                "message": "Invalid params: tool 'name' is required"
                            }
                        });
                        return Some(err_res.to_string());
                    }
                };

                let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                let call_req = McpCallRequest { name, arguments };
                let call_res = dispatch_mcp_tool_call(&mut self.project, &self.materials, &call_req);
                let res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": call_res.content,
                        "isError": call_res.is_error
                    }
                });
                Some(res.to_string())
            }
            _ => {
                if is_notification {
                    return None;
                }
                let res = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {method}")
                    }
                });
                Some(res.to_string())
            }
        }
    }
}

/// Returns the registry of all CAD MCP tools exposed by QymCAD.
pub fn get_mcp_tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "cad.create_primitive".into(),
            description: "Creates a solid geometric primitive (box, cylinder, sphere, cone, torus) with specified dimensions and optional position.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["box", "cylinder", "sphere", "cone", "torus"] },
                    "dimensions": { "type": "array", "items": { "type": "number" }, "description": "Dimensions in mm [X, Y, Z] or [radius, height]" },
                    "position": { "type": "array", "items": { "type": "number" }, "description": "Optional location offset [X, Y, Z] in mm" }
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
                    "sketch_id": { "type": "integer", "description": "Identifier of the sketch to extrude" },
                    "distance": { "type": "number", "description": "Extrusion distance in mm" },
                    "symmetric": { "type": "boolean", "default": false, "description": "Extrude equally in both directions" }
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
                    "body_id": { "type": "integer", "description": "Target body ID" },
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
                    "body_id": { "type": "integer", "description": "Target body ID" },
                    "material_id": { "type": "string", "description": "Optional material identifier to use for density and cost" }
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
                    "body_id": { "type": "integer", "description": "Target body ID" },
                    "process": { "type": "string", "enum": ["fdm", "cnc", "sla"], "default": "fdm", "description": "Manufacturing process" }
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
                    "body_id": { "type": "integer", "description": "Target body ID" },
                    "cycle_time_hours": { "type": "number", "description": "Machining or print cycle time in hours" },
                    "quantity": { "type": "integer", "minimum": 1, "description": "Production batch size" },
                    "material_id": { "type": "string", "description": "Optional material override" }
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
                    "body_id": { "type": "integer", "description": "Target body ID" },
                    "levels": { "type": "integer", "minimum": 1, "maximum": 4, "description": "Subdivision refinement depth (1..4)" },
                    "crease_sharpness": { "type": "number", "minimum": 0.0, "description": "Edge crease sharpness weight" }
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
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.extrude" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("extrude");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.assign_material" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("assign_material");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.calculate_mass_properties" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("calculate_mass_properties");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.analyze_dfm" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("analyze_dfm");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.estimate_cost" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("estimate_cost");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.subdivide_mesh" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("subdivide_mesh");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.create_sketch" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("create_sketch");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.add_sketch_rectangle" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("add_sketch_rectangle");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.add_sketch_circle" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("add_sketch_circle");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.solve_sketch" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("solve_sketch");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.fillet" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("fillet");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.chamfer" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("chamfer");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        "cad.new_document" => {
            let mut val = call.arguments.clone();
            val["operation"] = serde_json::json!("new_document");
            serde_json::from_value(val).map_err(|e| format!("Invalid argument: {e}"))
        }
        unknown => Err(format!("Tool not found: {unknown}")),
    };

    match cmd_result {
        Ok(cmd) => {
            let resp = execute_transaction(project, &cmd, materials, &tx_id);
            let text = serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.message.clone());
            McpCallResponse { is_error: !resp.success, content: vec![McpContentBlock { content_type: "text".into(), text }] }
        }
        Err(err) => McpCallResponse { is_error: true, content: vec![McpContentBlock { content_type: "text".into(), text: err }] },
    }
}

/// Handles a single JSON-RPC 2.0 message according to the Model Context Protocol.
pub fn handle_jsonrpc_message(msg: &str, project: &mut Project, materials: &MaterialRegistry) -> Option<String> {
    let mut server = McpServer { project: std::mem::take(project), materials: materials.clone(), initialized: true };
    let response = server.handle_message(msg);
    *project = server.project;
    response
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

        // Verify that inputSchema serializes properly as camelCase
        let val = serde_json::to_value(&tools[0]).expect("serialize tool");
        assert!(val.get("inputSchema").is_some());
        assert!(val.get("input_schema").is_none());
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

        // Create base box primitive first
        let create_req = McpCallRequest {
            name: "cad.create_primitive".into(),
            arguments: serde_json::json!({
                "kind": "box",
                "dimensions": [20.0, 20.0, 20.0]
            }),
        };
        let create_res = dispatch_mcp_tool_call(&mut proj, &mats, &create_req);
        assert!(!create_res.is_error);
        let body_id = proj.bodies[0].id;

        let req = McpCallRequest {
            name: "cad.subdivide_mesh".into(),
            arguments: serde_json::json!({
                "body_id": body_id,
                "levels": 2,
                "crease_sharpness": 2.5
            }),
        };

        let resp = dispatch_mcp_tool_call(&mut proj, &mats, &req);
        assert!(!resp.is_error);
        assert!(resp.content[0].text.contains("OpenSubdiv"));
    }

    #[test]
    fn jsonrpc_parse_error_returns_32700() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        let res = handle_jsonrpc_message("not a json string", &mut proj, &mats);
        assert!(res.is_some());
        let val: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
        assert_eq!(val["error"]["code"], -32700);
    }

    #[test]
    fn jsonrpc_invalid_request_returns_32600() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        // Missing jsonrpc: "2.0"
        let res = handle_jsonrpc_message(r#"{"id": 1, "method": "test"}"#, &mut proj, &mats);
        assert!(res.is_some());
        let val: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
        assert_eq!(val["error"]["code"], -32600);
    }

    #[test]
    fn jsonrpc_notification_never_responds_with_error() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        // Unknown method without ID (notification)
        let res = handle_jsonrpc_message(r#"{"jsonrpc": "2.0", "method": "unknown_notification"}"#, &mut proj, &mats);
        assert!(res.is_none());
    }

    #[test]
    fn jsonrpc_tools_call_missing_params_returns_32602() {
        let mut proj = Project::default();
        let mats = MaterialRegistry::default();

        let res = handle_jsonrpc_message(r#"{"jsonrpc": "2.0", "id": 42, "method": "tools/call"}"#, &mut proj, &mats);
        assert!(res.is_some());
        let val: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
        assert_eq!(val["error"]["code"], -32602);
    }
}
