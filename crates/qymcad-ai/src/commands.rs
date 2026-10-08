//! Strongly-typed CAD command schema for AI, Copilot, and MCP tool automation.

use serde::{Deserialize, Serialize};

/// Supported 3D CAD modeling commands.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum CadCommand {
    /// Initializes a new empty CAD project.
    NewDocument {
        /// Document title.
        title: Option<String>,
    },
    /// Creates a 3D geometric primitive solid.
    CreatePrimitive {
        /// Kind of primitive solid.
        kind: PrimitiveKind,
        /// Dimensions in millimeters [X, Y, Z] or [radius, height].
        dimensions: Vec<f64>,
        /// Optional location offset [X, Y, Z].
        position: Option<[f64; 3]>,
    },
    /// Creates a 2D sketch plane.
    CreateSketch {
        /// Name of the new sketch.
        name: String,
        /// Reference plane: "XY", "XZ", "YZ", or datum plane ID.
        plane: String,
    },
    /// Adds a 2D rectangle to a sketch.
    AddSketchRectangle {
        /// ID of target sketch.
        sketch_id: u64,
        /// First corner [X, Y].
        p0: [f64; 2],
        /// Opposite corner [X, Y].
        p1: [f64; 2],
    },
    /// Adds a 2D circle to a sketch.
    AddSketchCircle {
        /// ID of target sketch.
        sketch_id: u64,
        /// Center point [X, Y].
        center: [f64; 2],
        /// Radius in mm.
        radius: f64,
    },
    /// Solves all geometric constraints in a sketch.
    SolveSketch {
        /// ID of the sketch to solve.
        sketch_id: u64,
    },
    /// Extrudes a sketch contour into a solid body.
    Extrude {
        /// Source sketch ID.
        sketch_id: u64,
        /// Extrusion distance in mm.
        distance: f64,
        /// Extrude symmetrically about sketch plane.
        symmetric: bool,
    },
    /// Applies a constant-radius fillet to body edges.
    Fillet {
        /// Target body ID.
        body_id: u64,
        /// Edge indices to round.
        edge_indices: Vec<u32>,
        /// Fillet radius in mm.
        radius: f64,
    },
    /// Applies a chamfer bevel to body edges.
    Chamfer {
        /// Target body ID.
        body_id: u64,
        /// Edge indices to bevel.
        edge_indices: Vec<u32>,
        /// Chamfer distance in mm.
        distance: f64,
    },
    /// Assigns an engineering material to a body.
    AssignMaterial {
        /// Target body ID.
        body_id: u64,
        /// Unique material identifier (e.g. "al_6061_t6", "pla").
        material_id: String,
    },
    /// Computes volume, mass, CG, and raw material cost.
    CalculateMassProperties {
        /// Target body ID.
        body_id: u64,
    },
    /// Runs Design For Manufacturing (DFM) analysis.
    AnalyzeDfm {
        /// Target body ID.
        body_id: u64,
        /// Manufacturing process ("fdm", "cnc", "sla").
        process: String,
    },
    /// Estimates full manufacturing cost (material, machine, energy, labor).
    EstimateCost {
        /// Target body ID.
        body_id: u64,
        /// Manufacturing cycle time in hours.
        cycle_time_hours: f64,
        /// Production batch quantity.
        quantity: u32,
    },
    /// Performs OpenSubdiv Catmull-Clark subdivision with optional edge crease sharpness.
    SubdivideMesh {
        /// Target body ID.
        body_id: u64,
        /// Number of subdivision refinement steps (1..4).
        levels: usize,
        /// Optional crease sharpness to apply across cage edges.
        crease_sharpness: Option<f64>,
    },
}

/// Solid primitive geometry kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    /// Rectangular cuboid / block.
    Box,
    /// Upright cylinder.
    Cylinder,
    /// Spherical solid.
    Sphere,
    /// Conical solid.
    Cone,
    /// Torus ring.
    Torus,
}

/// Execution outcome of a CAD command.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Unique transaction identifier.
    pub transaction_id: String,
    /// High-level status or error message.
    pub message: String,
    /// Optional generated entity ID (body, feature, sketch).
    pub created_id: Option<u64>,
    /// Diagnostic details or metric values.
    pub details: Option<serde_json::Value>,
}

impl CommandResponse {
    /// Creates a successful response.
    pub fn success(transaction_id: &str, message: &str, created_id: Option<u64>) -> Self {
        Self { success: true, transaction_id: transaction_id.to_string(), message: message.to_string(), created_id, details: None }
    }

    /// Creates an error response.
    pub fn error(transaction_id: &str, message: &str) -> Self {
        Self { success: false, transaction_id: transaction_id.to_string(), message: message.to_string(), created_id: None, details: None }
    }

    /// Attaches structured details to the response.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}
