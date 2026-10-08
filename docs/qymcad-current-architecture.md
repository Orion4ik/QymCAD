# QymCAD Current Architecture Audit

## 1. Executive Summary

QymCAD is a parametric associative 3D CAD application written in Rust and powered by OpenCASCADE Technology (OCCT 7.8/7.9) as its B-Rep geometry kernel. It uses `egui` and `wgpu` (with software rasterizer fallbacks and custom 3D viewports) for an agile, desktop-native user interface.

The codebase adheres to strict engineering hygiene:
- **Rust Edition 2021** (target floor rustc 1.88+).
- **Workspace-wide strict lints:** `dead_code = "deny"`, `unused_must_use = "deny"`, `clippy::all = "deny"`.
- **Architectural Ratchets:** Formal tests (`god_object_ratchet.rs`, `dependency_ratchet.rs`, `comment_ratchet.rs`, `wide_signature_ratchet.rs`) that enforce code decoupling, prohibit bloated signatures (max 7 arguments, no nameless tuples), and monotonically reduce `App` state.
- **Associativity:** Solid bodies, features, sketches, and assembly constraints share an associative timeline with transactional rebuilds.

---

## 2. Workspace Crates Analysis

The repository is modularized into specialized crates:

| Crate | Role & Scope | Dependencies & Technology |
|---|---|---|
| **`qymcad-core`** | Fundamental CAD domain model, 2D sketch constraint solver, timeline DAG, B-Rep mesh structures, dimensional expressions, and assembly kinematics. | `nalgebra`, `petgraph`, `cavalier_contours`, `levenberg-marquardt`. (Pure Rust, NO C++ dependency) |
| **`qymcad-kernel`** | High-performance C-ABI bridge to OpenCASCADE Technology (OCCT). B-Rep primitives, booleans, sweeps, lofts, fillets, chamfers, and STEP/IGES low-level translation. | C++17 bridge (`TKernel`, `TKMath`, `TKBRep`, `TKTopAlgo`, `TKBool`, `TKFillet`, `TKOffset`, `TKFeat`, etc.) |
| **`qymcad-sketch`** | Sketching workbench logic and tool implementations (lines, arcs, circles, rectangles, splines, offset, trim, constraints). | `qymcad-core`, `qymcad-ui-state` |
| **`qymcad-part`** | 3D feature modeling workbench (Extrude, Revolve, Sweep, Loft, Fillet, Chamfer, Shell, Thread, Draft, Booleans, Patterns). | `qymcad-core`, `qymcad-kernel`, `qymcad-ui-state` |
| **`qymcad-assembly`**| Mechanical assembly workbench (joints, mates: coincident, concentric, parallel, revolute, cylindrical, slider, gears, pin-slot). | `qymcad-core`, `qymcad-ui-state` |
| **`qymcad-materials`** | Engineering material library (metals, alloys, polymers) and physical property calculation (mass, CG, moments, raw cost). | `qymcad-core`, `serde` |
| **`qymcad-io`** | Multi-format 2D/3D file converters: DXF (import/export), SVG, STL, OBJ, PLY, AMF, 3MF, glTF, and STEP metadata. | `dxf`, `roxmltree`, `serde`, `qymcad-core` |
| **`qymcad-meshfit`**| Mesh simplification, topology decimation, and geometric deviation analysis. | `qymcad-core`, `nalgebra` |
| **`qymcad-pick`** | Viewport raycasting, sub-element geometric selection (vertices, edges, silhouettes, faces, datum axes, coordinate connectors). | `qymcad-core`, `nalgebra` |
| **`qymcad-render`** | 3D rendering pipeline: interactive gizmos, dimension annotations, handle rings, mesh lighting, edge rendering, cap sections. | `egui`, `wgpu`, `qymcad-core`, `qymcad-ui-state` |
| **`qymcad-shell`** | UI container framework decoupling the shell layout from workbenches. | `egui` |
| **`qymcad-ui-state`**| Stateless UI presentation layer, command parameters, live input sessions, contextual tool options. | `egui`, `qymcad-core` |
| **`qymcad-i18n`** | Fluent-based translation system with automated terminology coverage checking and runtime locale switching. | `fluent-bundle`, `unic-langid` |
| **`qymcad-help`** | Interactive user manual and verified help articles with visual raster previews and contextual bindings. | `qymcad-ui-state` |
| **`qymcad-paths`** | OS-specific persistent directory paths (application data, recent files, material configs). | `directories` |
| **`qymcad-scheme`** | Dynamic theme tokens, color palettes, and contrast-checked styles. | `egui` |
| **`qymcad-update`** | Background update checker and release telemetry. | `ureq` |
| **`qymcad-testkit`** | Headless OCCT regression matrix harness and geometry validation suites. | `qymcad-core`, `qymcad-kernel` |
| **`qymcad-acceptance`**| End-to-end integration oracles and simulated interactive human session playback. | `qymcad`, `qymcad-core` |
| **`qymcad`** | Main application binary and GUI shell coordinating egui panels, viewports, command search, and event routing. | All workspace crates |

---

## 3. Core Geometry and Associativity Architecture

### 3.1 B-Rep Kernel Boundary
- `qymcad-core` maintains pure mathematical definitions and mesh caches without depending on the OCCT dynamic libraries.
- `qymcad-kernel` acts as a guarded boundary with mutex-protected C++ calls to OCCT.
- All persistent geometry handles use stable 64-bit identifiers (`Id`). Face, edge, and vertex lineage are tracked across rebuilds so sketches on faces do not break when upstream dimensions change.

### 3.2 Constraint Solver
- Analytical Jacobian Levenberg-Marquardt solver in `qymcad-core::solver`.
- Supports geometric constraints (coincidence, horizontal, vertical, parallel, perpendicular, tangent, concentric, midpoint, collinear) and driving dimensions (distance, horizontal/vertical delta, angle, radius, diameter).
- Real-time DoF (degrees of freedom) calculation and fast sparse update path for mouse drag frames.

### 3.3 Kinematic Assembly Solver
- Multi-body graph decomposition with joint degrees of freedom (`qymcad-core::asm`).
- Resolves kinematic chains with limits, gear relations, rack-and-pinion, and screw couplings.

---

## 4. UI/UX Architecture

- **Command Pattern & Command Catalog (`command_catalog.rs`):** Single source of truth for command IDs, shortcuts, help articles, and launch delegates.
- **Contextual In-Viewport Widgets:** Interactive radius handles, face pull arrows, and dynamic dimension fields right on the geometry.
- **Strict Decoupling:** Panels receive narrow contexts (`PartCtx`, `SketchCtx`, `TreeCtx`) rather than `&mut App`.
