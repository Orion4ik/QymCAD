# QymCAD Professional CAD Evolution Roadmap & Gap Analysis

## 1. Benchmarking & Competitive Gap Analysis

| Capability Area | Industry Reference | Current QymCAD Status | Evolution Target |
|---|---|---|---|
| **Direct Modeling** | **Plasticity** (fluid selection, push/pull, live tangent fillets, immediate numeric entry) | Edge radius handles and face-push vectors exist in UI state. | First-class direct modeling mode: drag face to offset/extrude, instant live preview, seamless transition to parametric tree. |
| **Parametric Part Modeling** | **SolidWorks** (feature tree rollback, configurations, derived equations, multi-body booleans) | Rich feature tree, rollback bar, parametric expressions, multi-body booleans. | Feature tree reordering diagnostics, design configurations/variants, sheet metal base flanges, surface knit/thicken. |
| **Assemblies & Kinematics** | **SolidWorks / Fusion 360** (rigid/revolute/cylindrical joints, collision detection, interference check) | Robust kinematic solver (`qymcad-core::asm`), gear/rack relations, limits. | Mesh interference/collision analysis, dynamic motion envelope visualization. |
| **Materials & Mass Properties** | **SolidWorks** (engineering database, CG, inertia tensor) | `qymcad-materials` with metals, polymers, physical/thermal/manufacturing properties. | UI material assignment panel, live CG indicator in 3D viewport, BOM weight integration. |
| **Manufacturing: 3D Printing** | **Fusion 360** (print orientation, overhang detection, slicer dispatch) | Basic mesh export (STL, 3MF). | First-class 3D print preparation: overhang angle analysis, wall thickness validation, slicer pluggable integration (OrcaSlicer, PrusaSlicer). |
| **Manufacturing: CAM & DFM** | **Fusion 360** (2.5D/3D milling toolpaths, G-code export) | Basic 2D dogbone/contour offsets. | Dedicated CAM architecture: Stock -> Tools -> Setups -> Operations (Facing, Pocketing, Contouring, Drilling) -> Post-processors. |
| **Simulation / CAE** | **SolidWorks Simulation / FreeCAD FEM** (linear static FEA, Von Mises stress) | Pure geometric modeler without mesh solver. | Pluggable simulation solver abstraction: tetrahedral meshing interface, boundary conditions, CalculiX / external solver adapters. |
| **AI & Automation** | **Next-Gen AI-Native CAD** (Copilot, Model Context Protocol) | Command catalog with headless invocation. | Native MCP server (`qymcad-mcp`), structured JSON CAD Command API, conversational CAD copilot with transactional validation and atomic rollback. |

---

## 2. Strategic Implementation Phases

### Phase 1: Core Geometry & Manufacturing Foundations (Current)
- Material database integration (`qymcad-materials`) with physical, thermal, and manufacturing metrics.
- Physical mass properties calculation (Volume, Surface Area, Center of Mass, Inertia Tensor, Raw Material Cost).
- DFM (Design For Manufacturing) analysis: minimum wall thickness, overhang angles, unsupported features.

### Phase 2: Additive Manufacturing & 3D Print Preparation
- Dedicated 3D printing preparation module (`qymcad-printing`):
  - Overhang surface classification (critical vs non-critical angles).
  - Pluggable slicer export abstraction (OrcaSlicer, PrusaSlicer, BambuStudio CLI).
  - Print time and filament consumption estimation.

### Phase 3: CAM Architecture (Computer-Aided Manufacturing)
- Modular CAM pipeline crate (`qymcad-cam`):
  - Work coordinate system (WCS) and stock bounding volume.
  - Tool library (flat endmills, ball endmills, chamfer tools, drills).
  - 2.5D toolpath generation: facing, adaptive 2D pocketing, profile contouring, peck drilling.
  - Standard G-code generator (Marlin, Grbl, LinuxCNC, Fanuc post-processors).

### Phase 4: Structural Simulation (CAE) Abstraction
- Finite Element Analysis foundation (`qymcad-simulation`):
  - Surface and volume mesh generation abstraction.
  - Loads, constraints, and material property mapping.
  - Static structural analysis results data model (displacement field, Von Mises stress, Safety Factor).

### Phase 5: AI-Native Layer & Model Context Protocol (MCP)
- Structured CAD Command API:
  - Strongly typed JSON command definitions matching UI operations.
  - Transactional boundary with dry-run validation and atomic rollback.
- Full MCP Server implementation (`qymcad-mcp`):
  - Tool endpoints: `cad.create_sketch`, `cad.extrude`, `cad.fillet`, `cad.assign_material`, `cad.measure`, `cad.analyze_printability`, `cad.export_step`.
  - Engineering Copilot agent with prompt-to-CAD translation.
