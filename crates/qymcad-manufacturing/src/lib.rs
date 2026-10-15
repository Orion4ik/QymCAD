//! Manufacturing preparation, DFM analysis, 3D printing slicer adapters, CAM toolpaths, and costing.

pub mod cam;
pub mod cost;
pub mod dfm;
pub mod printing;

pub use cam::{generate_gcode, CncTool, PostDialect, StockDefinition, ToolType, Toolpath, ToolpathPoint};
pub use cost::{estimate_manufacturing_cost, CostBreakdown, CostModelRates};
pub use dfm::{analyze_triangles_dfm, DfmConfig, DfmFinding, DfmReport, ManufacturingProcess, Severity};
pub use printing::{PrinterProfile, SlicerKind, SlicingJob};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_manufacturing_workflow_pipeline() {
        // 1. DFM check
        let verts = vec![[0.0, 0.0, 0.0], [50.0, 0.0, 0.0], [0.0, 50.0, 0.0]];
        let tris = vec![[0, 1, 2]];
        let report = analyze_triangles_dfm(&verts, &tris, ManufacturingProcess::FdmPrinting, &DfmConfig::default());
        assert!(report.is_manufacturable);

        // 2. Cost estimation
        let cost = estimate_manufacturing_cost(4.20, 1.5, 5, &CostModelRates::default());
        assert!(cost.unit_total_usd > 4.20);

        // 3. 3D printer check
        let printer = PrinterProfile::bambu_x1();
        assert!(printer.fits_bounding_box([0.0, 0.0, 0.0], [150.0, 150.0, 150.0]));

        // 4. CAM toolpath & G-code
        let tool = CncTool::flat_6mm();
        let tp = Toolpath { tool_number: 1, points: vec![ToolpathPoint { pos: [0.0, 0.0, 0.0], is_rapid: true, feedrate: None }], estimated_time_seconds: 1.0 };
        let code = generate_gcode(&[tp], &[tool], PostDialect::Grbl, "part1");
        assert!(code.contains("G21"));
    }
}
