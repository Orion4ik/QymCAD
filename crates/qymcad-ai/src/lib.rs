//! AI-native engineering layer, CAD Copilot agent, command transactions, and Model Context Protocol (MCP) server.

pub mod commands;
pub mod copilot;
pub mod mcp;
pub mod transaction;

pub use commands::{CadCommand, CommandResponse, PrimitiveKind};
pub use copilot::parse_natural_language_intent;
pub use mcp::{dispatch_mcp_tool_call, get_mcp_tool_definitions, handle_jsonrpc_message, McpCallRequest, McpCallResponse, McpServer, McpToolDefinition};
pub use transaction::execute_transaction;

#[cfg(test)]
mod tests {
    use super::*;
    use qymcad_core::model::Project;
    use qymcad_materials::MaterialRegistry;

    #[test]
    fn complete_ai_natural_language_to_execution_pipeline() {
        let mut project = Project::default();
        let materials = MaterialRegistry::default();

        // 1. Natural language parse
        let cmd = parse_natural_language_intent("Create a 50 mm cube").expect("parsed command");

        // 2. Transaction execution
        let resp = execute_transaction(&mut project, &cmd, &materials, "copilot_tx_1");
        assert!(resp.success);
        assert!(!project.timeline.is_empty());

        // 3. MCP tool call for material assignment
        let mcp_call = McpCallRequest {
            name: "cad.assign_material".into(),
            arguments: serde_json::json!({
                "body_id": 1,
                "material_id": "al_6061_t6"
            }),
        };
        let mcp_resp = dispatch_mcp_tool_call(&mut project, &materials, &mcp_call);
        assert!(!mcp_resp.is_error);
    }
}
