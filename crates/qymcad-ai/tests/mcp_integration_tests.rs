//! Integration tests for Model Context Protocol (MCP) server in QymCAD.

use qymcad_ai::{handle_jsonrpc_message, McpServer};
use qymcad_core::model::Project;
use qymcad_materials::MaterialRegistry;
use serde_json::json;

#[test]
fn mcp_session_lifecycle_handshake() {
    let mut server = McpServer::new();

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0.0" }
        }
    });
    let init_res_str = server.handle_message(&init_req.to_string()).expect("response to initialize");
    let init_res: serde_json::Value = serde_json::from_str(&init_res_str).expect("valid json response");

    assert_eq!(init_res["jsonrpc"], "2.0");
    assert_eq!(init_res["id"], 1);
    assert_eq!(init_res["result"]["protocolVersion"], "2024-11-05");
    assert!(init_res["result"]["capabilities"]["tools"].is_object());
    assert_eq!(init_res["result"]["serverInfo"]["name"], "qymcad-mcp");

    // 2. Initialized notification (must not return a response)
    let init_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    assert!(server.handle_message(&init_notif.to_string()).is_none());

    // 3. Ping
    let ping_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });
    let ping_res_str = server.handle_message(&ping_req.to_string()).expect("response to ping");
    let ping_res: serde_json::Value = serde_json::from_str(&ping_res_str).expect("valid json response");
    assert_eq!(ping_res["id"], 2);
    assert!(ping_res["result"].is_object());
}

#[test]
fn mcp_tools_list_schema_compliance() {
    let mut server = McpServer::new();

    let list_req = json!({
        "jsonrpc": "2.0",
        "id": "req-tools-list",
        "method": "tools/list"
    });
    let res_str = server.handle_message(&list_req.to_string()).expect("response");
    let res: serde_json::Value = serde_json::from_str(&res_str).expect("valid json");

    assert_eq!(res["id"], "req-tools-list");
    let tools = res["result"]["tools"].as_array().expect("tools array");
    assert!(tools.len() >= 7);

    // Verify MCP spec compliance: must use inputSchema (camelCase)
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        assert!(name.starts_with("cad."));
        assert!(tool["description"].is_string());
        assert!(tool.get("inputSchema").is_some(), "Tool {name} must have inputSchema");
        assert!(tool.get("input_schema").is_none(), "Tool {name} must not expose input_schema");
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }
}

#[test]
fn mcp_end_to_end_design_workflow() {
    let mut server = McpServer::new();

    // 1. Create a 40x20x10 mm solid box
    let create_req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "cad.create_primitive",
            "arguments": {
                "kind": "box",
                "dimensions": [40.0, 20.0, 10.0]
            }
        }
    });
    let create_res_str = server.handle_message(&create_req.to_string()).expect("create response");
    let create_res: serde_json::Value = serde_json::from_str(&create_res_str).unwrap();
    assert_eq!(create_res["result"]["isError"], false);
    let content_text = create_res["result"]["content"][0]["text"].as_str().unwrap();
    let create_data: serde_json::Value = serde_json::from_str(content_text).unwrap();
    assert!(create_data["success"].as_bool().unwrap());
    let body_id = create_data["created_id"].as_u64().expect("body id");

    // 2. Assign material: Aluminum 6061-T6
    let mat_req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "cad.assign_material",
            "arguments": {
                "body_id": body_id,
                "material_id": "al_6061_t6"
            }
        }
    });
    let mat_res_str = server.handle_message(&mat_req.to_string()).expect("material response");
    let mat_res: serde_json::Value = serde_json::from_str(&mat_res_str).unwrap();
    assert_eq!(mat_res["result"]["isError"], false);

    // 3. Calculate mass properties on the real geometry
    let mass_req = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "cad.calculate_mass_properties",
            "arguments": {
                "body_id": body_id
            }
        }
    });
    let mass_res_str = server.handle_message(&mass_req.to_string()).expect("mass response");
    let mass_res: serde_json::Value = serde_json::from_str(&mass_res_str).unwrap();
    assert_eq!(mass_res["result"]["isError"], false);
    let mass_text = mass_res["result"]["content"][0]["text"].as_str().unwrap();
    let mass_data: serde_json::Value = serde_json::from_str(mass_text).unwrap();
    let details = &mass_data["details"];

    // Box 40x20x10 -> Volume = 8000 mm^3, Area = 2*(800 + 400 + 200) = 2800 mm^2
    let vol = details["volume_mm3"].as_f64().unwrap();
    let area = details["surface_area_mm2"].as_f64().unwrap();
    assert!((vol - 8000.0).abs() < 1e-3, "Volume expected 8000.0, got {vol}");
    assert!((area - 2800.0).abs() < 1e-3, "Area expected 2800.0, got {area}");
    let mass_kg = details["mass_kg"].as_f64().unwrap();
    // 8000 mm^3 = 8e-6 m^3 * 2700 kg/m^3 = 0.0216 kg
    assert!((mass_kg - 0.0216).abs() < 1e-5);

    // 4. Run DFM analysis
    let dfm_req = json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "tools/call",
        "params": {
            "name": "cad.analyze_dfm",
            "arguments": {
                "body_id": body_id,
                "process": "fdm"
            }
        }
    });
    let dfm_res_str = server.handle_message(&dfm_req.to_string()).expect("dfm response");
    let dfm_res: serde_json::Value = serde_json::from_str(&dfm_res_str).unwrap();
    assert_eq!(dfm_res["result"]["isError"], false);

    // 5. Estimate production cost
    let cost_req = json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "tools/call",
        "params": {
            "name": "cad.estimate_cost",
            "arguments": {
                "body_id": body_id,
                "cycle_time_hours": 0.5,
                "quantity": 25
            }
        }
    });
    let cost_res_str = server.handle_message(&cost_req.to_string()).expect("cost response");
    let cost_res: serde_json::Value = serde_json::from_str(&cost_res_str).unwrap();
    assert_eq!(cost_res["result"]["isError"], false);

    // 6. Subdivide mesh
    let subdiv_req = json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "tools/call",
        "params": {
            "name": "cad.subdivide_mesh",
            "arguments": {
                "body_id": body_id,
                "levels": 2,
                "crease_sharpness": 1.5
            }
        }
    });
    let subdiv_res_str = server.handle_message(&subdiv_req.to_string()).expect("subdiv response");
    let subdiv_res: serde_json::Value = serde_json::from_str(&subdiv_res_str).unwrap();
    assert_eq!(subdiv_res["result"]["isError"], false);
}

#[test]
fn mcp_error_handling_and_rollback_safety() {
    let mut server = McpServer::new();

    // 1. Parse error on invalid JSON (-32700)
    let parse_err = server.handle_message("{{{invalid json").unwrap();
    let val: serde_json::Value = serde_json::from_str(&parse_err).unwrap();
    assert_eq!(val["error"]["code"], -32700);

    // 2. Invalid Request (-32600)
    let inv_req = server.handle_message(r#"{"id": 100, "jsonrpc": "1.0", "method": "test"}"#).unwrap();
    let val: serde_json::Value = serde_json::from_str(&inv_req).unwrap();
    assert_eq!(val["error"]["code"], -32600);

    // 3. Method not found (-32601)
    let not_found = server.handle_message(r#"{"jsonrpc": "2.0", "id": 101, "method": "unknown_tool_cmd"}"#).unwrap();
    let val: serde_json::Value = serde_json::from_str(&not_found).unwrap();
    assert_eq!(val["error"]["code"], -32601);

    // 4. Invalid params (-32602)
    let inv_params = server.handle_message(r#"{"jsonrpc": "2.0", "id": 102, "method": "tools/call", "params": {}}"#).unwrap();
    let val: serde_json::Value = serde_json::from_str(&inv_params).unwrap();
    assert_eq!(val["error"]["code"], -32602);

    // 5. Tool execution error on non-existent body ID with rollback guarantee
    let initial_bodies_count = server.project.bodies.len();
    let tool_err_req = json!({
        "jsonrpc": "2.0",
        "id": 103,
        "method": "tools/call",
        "params": {
            "name": "cad.calculate_mass_properties",
            "arguments": {
                "body_id": 999999
            }
        }
    });
    let err_res_str = server.handle_message(&tool_err_req.to_string()).unwrap();
    let err_res: serde_json::Value = serde_json::from_str(&err_res_str).unwrap();
    assert_eq!(err_res["result"]["isError"], true);
    assert!(err_res["result"]["content"][0]["text"].as_str().unwrap().contains("not found"));
    assert_eq!(server.project.bodies.len(), initial_bodies_count);

    // 6. Invalid dimensions (negative) rejected
    let neg_dim_req = json!({
        "jsonrpc": "2.0",
        "id": 104,
        "method": "tools/call",
        "params": {
            "name": "cad.create_primitive",
            "arguments": {
                "kind": "box",
                "dimensions": [-10.0, 20.0, 30.0]
            }
        }
    });
    let neg_res_str = server.handle_message(&neg_dim_req.to_string()).unwrap();
    let neg_res: serde_json::Value = serde_json::from_str(&neg_res_str).unwrap();
    assert_eq!(neg_res["result"]["isError"], true);
}

#[test]
fn mcp_extrude_symmetric_and_asymmetric_dispatch() {
    let mut proj = Project::default();
    let mats = MaterialRegistry::default();

    // 1. Create sketch
    let sk_req = json!({
        "jsonrpc": "2.0",
        "id": 201,
        "method": "tools/call",
        "params": {
            "name": "cad.create_sketch",
            "arguments": { "name": "base_sk", "plane": "XY" }
        }
    });
    let sk_res = handle_jsonrpc_message(&sk_req.to_string(), &mut proj, &mats).unwrap();
    let sk_val: serde_json::Value = serde_json::from_str(&sk_res).unwrap();
    assert_eq!(sk_val["result"]["isError"], false);
    let sk_id = proj.sketches[0].id;

    // 2. Add rectangle
    let rect_req = json!({
        "jsonrpc": "2.0",
        "id": 202,
        "method": "tools/call",
        "params": {
            "name": "cad.add_sketch_rectangle",
            "arguments": {
                "sketch_id": sk_id,
                "p0": [0.0, 0.0],
                "p1": [50.0, 25.0]
            }
        }
    });
    let rect_res = handle_jsonrpc_message(&rect_req.to_string(), &mut proj, &mats).unwrap();
    let rect_val: serde_json::Value = serde_json::from_str(&rect_res).unwrap();
    assert_eq!(rect_val["result"]["isError"], false);

    // 3. Extrude symmetrically without requiring symmetric parameter (defaults to false)
    let ext_req1 = json!({
        "jsonrpc": "2.0",
        "id": 203,
        "method": "tools/call",
        "params": {
            "name": "cad.extrude",
            "arguments": {
                "sketch_id": sk_id,
                "distance": 15.0
            }
        }
    });
    let ext_res1 = handle_jsonrpc_message(&ext_req1.to_string(), &mut proj, &mats).unwrap();
    let ext_val1: serde_json::Value = serde_json::from_str(&ext_res1).unwrap();
    assert_eq!(ext_val1["result"]["isError"], false);

    // 4. Extrude with explicit symmetric: true
    let ext_req2 = json!({
        "jsonrpc": "2.0",
        "id": 204,
        "method": "tools/call",
        "params": {
            "name": "cad.extrude",
            "arguments": {
                "sketch_id": sk_id,
                "distance": 20.0,
                "symmetric": true
            }
        }
    });
    let ext_res2 = handle_jsonrpc_message(&ext_req2.to_string(), &mut proj, &mats).unwrap();
    let ext_val2: serde_json::Value = serde_json::from_str(&ext_res2).unwrap();
    assert_eq!(ext_val2["result"]["isError"], false);
}

#[test]
fn mcp_stdio_server_process_run() {
    let binary_path = env!("CARGO_BIN_EXE_qymcad_mcp");
    let mut child = std::process::Command::new(binary_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn qymcad_mcp process");

    let mut stdin = child.stdin.take().expect("stdin handle");
    let stdout = child.stdout.take().expect("stdout handle");
    let mut reader = std::io::BufReader::new(stdout);

    use std::io::{BufRead, Write};

    // 1. Send initialize
    let init_line = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "protocolVersion": "2024-11-05" }
    })
    .to_string();
    writeln!(stdin, "{}", init_line).expect("write to stdin");
    stdin.flush().expect("flush stdin");

    let mut response_line = String::new();
    reader.read_line(&mut response_line).expect("read response");
    let init_res: serde_json::Value = serde_json::from_str(&response_line).expect("parse response");
    assert_eq!(init_res["jsonrpc"], "2.0");
    assert_eq!(init_res["id"], 1);
    assert_eq!(init_res["result"]["protocolVersion"], "2024-11-05");

    // 2. Send ping
    let ping_line = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    })
    .to_string();
    writeln!(stdin, "{}", ping_line).expect("write ping");
    stdin.flush().expect("flush stdin");

    response_line.clear();
    reader.read_line(&mut response_line).expect("read ping response");
    let ping_res: serde_json::Value = serde_json::from_str(&response_line).expect("parse ping");
    assert_eq!(ping_res["id"], 2);

    // 3. Drop stdin to signal EOF and verify clean server process termination
    drop(stdin);
    let status = child.wait().expect("child exit");
    assert!(status.success());
}
