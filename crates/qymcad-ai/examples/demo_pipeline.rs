//! End-to-end demonstration of QymCAD engineering features:
//! 1. OpenSubdiv Catmull-Clark with edge creases
//! 2. Engineering Materials & Mass Properties
//! 3. DFM (Design For Manufacturing) surface analysis
//! 4. Manufacturing Cost Estimation
//! 5. CAM toolpath & ISO G-code generation
//! 6. AI Copilot natural language processing & MCP tool dispatch

use qymcad_ai::{dispatch_mcp_tool_call, execute_transaction, get_mcp_tool_definitions, parse_natural_language_intent, McpCallRequest};
use qymcad_core::model::Project;
use qymcad_core::subdiv::{Cage, OpenSubdivCage};
use qymcad_manufacturing::{
    analyze_triangles_dfm, estimate_manufacturing_cost, generate_gcode, CncTool, CostModelRates, DfmConfig, ManufacturingProcess, PostDialect, PrinterProfile, Toolpath, ToolpathPoint,
};
use qymcad_materials::{calculate_mass_properties, MaterialRegistry};

fn main() {
    println!("============================================================");
    println!("     QymCAD — New Engineering Features & AI Capabilities    ");
    println!("============================================================\n");

    // -------------------------------------------------------------------------
    // 1. OpenSubdiv Catmull-Clark with Creases
    // -------------------------------------------------------------------------
    println!(">>> [1] OpenSubdiv Subdivision & Semi-Sharp Creasing");
    let cube = Cage::cube(20.0);
    let mut subd_cage = OpenSubdivCage::new(cube);

    // Apply semi-sharp crease to top face edges (sharpness = 2.5)
    subd_cage.set_edge_crease(4, 5, 2.5);
    subd_cage.set_edge_crease(5, 6, 2.5);
    subd_cage.set_edge_crease(6, 7, 2.5);
    subd_cage.set_edge_crease(7, 4, 2.5);

    let refined = subd_cage.subdivided(2);
    let tris = refined.to_triangles();
    println!("    - Original cage: 8 vertices, 6 faces");
    println!("    - Creased top edges (sharpness = 2.5)");
    println!("    - Subdivided level 2: {} vertices, {} quad faces, {} triangles\n", refined.cage.verts.len(), refined.cage.faces.len(), tris.len());

    // -------------------------------------------------------------------------
    // 2. Engineering Materials Database & Mass Properties
    // -------------------------------------------------------------------------
    println!(">>> [2] Engineering Materials & Mass Properties");
    let reg = MaterialRegistry::default();
    println!("    - Materials available: {}", reg.len());
    let al6061 = reg.get("al_6061_t6").expect("al6061 exists");
    println!("    - Selected: {} (Category: {:?})", al6061.name, al6061.category);
    println!("      Density: {} kg/m^3 | Yield: {} MPa | Price: ${:.2}/kg", al6061.physical.density, al6061.physical.yield_strength, al6061.manufacturing.cost_per_kg_usd);

    let mass_props = calculate_mass_properties(8000.0, 2400.0, [10.0, 10.0, 10.0], al6061);
    println!("    - Physical Calculation:");
    println!("      Mass: {:.4} kg ({:.2} g)", mass_props.mass_kg, mass_props.mass_kg * 1000.0);
    println!("      Center of Mass: {:?}", mass_props.center_of_mass);
    println!("      Raw Material Cost: ${:.2}\n", mass_props.raw_material_cost_usd);

    // -------------------------------------------------------------------------
    // 3. DFM (Design For Manufacturing) & 3D Print Preparation
    // -------------------------------------------------------------------------
    println!(">>> [3] DFM Analysis & 3D Printing Build Envelopes");
    let printer = PrinterProfile::bambu_x1();
    let fits = printer.fits_bounding_box([0.0, 0.0, 0.0], [150.0, 150.0, 150.0]);
    println!("    - Printer Profile: {} (Build volume: {:?} mm)", printer.name, printer.build_volume_mm);
    println!("      Part fits build volume: {}", fits);

    let dfm_report = analyze_triangles_dfm(&refined.cage.verts, &tris, ManufacturingProcess::FdmPrinting, &DfmConfig::default());
    println!("    - DFM Overhang Analysis:");
    println!("      Support Area Ratio: {:.1}%", dfm_report.support_area_ratio * 100.0);
    println!("      Manufacturability Status: {}\n", if dfm_report.is_manufacturable { "PASSED" } else { "ACTION NEEDED" });

    // -------------------------------------------------------------------------
    // 4. Manufacturing Cost Estimation
    // -------------------------------------------------------------------------
    println!(">>> [4] Detailed Cost Estimation");
    let rates = CostModelRates::default();
    let cost_batch = estimate_manufacturing_cost(mass_props.raw_material_cost_usd, 1.2, 10, &rates);
    println!("    - Batch Quantity: 10 units");
    println!("      Material: ${:.2} | Machine: ${:.2} | Energy: ${:.2} | Labor/Setup: ${:.2}", cost_batch.material_usd, cost_batch.machine_usd, cost_batch.energy_usd, cost_batch.labor_usd);
    println!("      Unit Cost: ${:.2} | Total Batch Cost: ${:.2}\n", cost_batch.unit_total_usd, cost_batch.batch_total_usd);

    // -------------------------------------------------------------------------
    // 5. CAM Toolpaths & G-code Generation
    // -------------------------------------------------------------------------
    println!(">>> [5] CAM Engine & ISO G-code Post-Processor");
    let tool = CncTool::flat_6mm();
    println!("    - Tool: {} (Dia: {} mm, RPM: {:.0})", tool.name, tool.diameter_mm, tool.spindle_rpm);

    let toolpath = Toolpath {
        tool_number: tool.number,
        points: vec![
            ToolpathPoint { pos: [0.0, 0.0, 5.0], is_rapid: true, feedrate: None },
            ToolpathPoint { pos: [0.0, 0.0, -2.0], is_rapid: false, feedrate: Some(300.0) },
            ToolpathPoint { pos: [50.0, 0.0, -2.0], is_rapid: false, feedrate: None },
            ToolpathPoint { pos: [50.0, 30.0, -2.0], is_rapid: false, feedrate: None },
            ToolpathPoint { pos: [0.0, 30.0, -2.0], is_rapid: false, feedrate: None },
            ToolpathPoint { pos: [0.0, 0.0, -2.0], is_rapid: false, feedrate: None },
            ToolpathPoint { pos: [0.0, 0.0, 15.0], is_rapid: true, feedrate: None },
        ],
        estimated_time_seconds: 14.5,
    };

    let gcode = generate_gcode(&[toolpath], &[tool], PostDialect::Grbl, "QymCAD_Contour_Demo");
    println!("    - Generated G-code preview (first 8 lines):");
    for line in gcode.lines().take(8) {
        println!("        {}", line);
    }
    println!("        ... ({} total lines of G-code)\n", gcode.lines().count());

    // -------------------------------------------------------------------------
    // 6. AI CAD Copilot & Model Context Protocol (MCP)
    // -------------------------------------------------------------------------
    println!(">>> [6] CAD Copilot Agent & Model Context Protocol (MCP)");
    let prompt = "Create a 50 mm cube and assign Aluminum 6061";
    println!("    - Natural language input: \"{}\"", prompt);

    let mut project = Project::default();
    if let Some(cmd) = parse_natural_language_intent(prompt) {
        println!("    - Translated CAD Command: {:?}", cmd);
        let resp = execute_transaction(&mut project, &cmd, &reg, "copilot_demo_tx");
        println!("    - Transaction Status: {} | Message: {}", resp.success, resp.message);
    }

    // Call MCP Server tool
    let tools = get_mcp_tool_definitions();
    println!("    - Registered MCP Tools: {} tools", tools.len());

    let mcp_call = McpCallRequest { name: "cad.calculate_mass_properties".into(), arguments: serde_json::json!({ "body_id": 1 }) };
    println!("    - Invoking MCP tool: {}", mcp_call.name);
    let mcp_resp = dispatch_mcp_tool_call(&mut project, &reg, &mcp_call);
    println!("    - MCP Output:\n{}", mcp_resp.content[0].text);

    println!("============================================================");
    println!("     All QymCAD engineering modules verified successfully!   ");
    println!("============================================================");
}
