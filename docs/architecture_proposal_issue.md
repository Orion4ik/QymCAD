# [Architecture Proposal] Moving QymCAD from Monolithic GUI (`App`) to a Modular, Clean Multi-Crate Architecture

## Context & Motivation

As QymCAD evolves into a professional, AI-native engineering CAD platform (combining direct modeling, parametric B-Rep history, assemblies, CAM, simulation, and MCP/AI Copilot), our codebase is facing scaling pressure. 

Currently, while our core geometry layer is nicely modularized across workspace crates (`qymcad-core`, `qymcad-kernel`, `qymcad-sketch`, etc.), the main desktop application crate (`qymcad`) still suffers from a **God Object (`App`)** pattern. Panels, windows, and tools are implemented as large method blocks directly hanging on `App`, granting them access to global state and making parallel teamwork difficult.

When multiple engineers work simultaneously on features like the AI/MCP layer, CAM toolpaths, or sketcher performance improvements, this monolithic GUI structure leads to high merge conflict friction, tightly coupled state, and harder testing.

---

## Proposed Architecture Evolution

To ensure QymCAD remains scalable, clean, and pleasant for a multi-developer team, we propose moving toward a strict **Clean Architecture with Modular Workbenches**:

### 1. Zero-Tolerance for the "God Object" (`App`)
* **Strict Enforced Ratchets:** Continue tightening `god_object_ratchet.rs` ceilings. No new methods or UI panels should be added directly to `impl App`.
* **Narrow Contexts:** Enforce the free-function pattern with narrow view/rebuild contexts (`WinCtx`, `BarCtx`, `StatusCtx`, `DrawCtx`), keeping UI panels entirely decoupled from the application root.

### 2. Workbench Crate Decomposing
Decouple independent workbenches and domains into dedicated workspace crates:
```text
crates/
├── qymcad-core          # Domain model, geometry, timeline
├── qymcad-kernel        # OCCT C++ bridge
├── qymcad-sketcher      # 2D constraint solver & sketching
├── qymcad-workbench-part     # Part design workbench
├── qymcad-workbench-assembly # Assembly & kinematics
├── qymcad-workbench-cam      # CAM & G-code generator
├── qymcad-workbench-sim      # CAE / FEA simulation
├── qymcad-ai-copilot         # LLM Copilot & MCP server
└── qymcad-ui-desktop         # Thin egui/wgpu shell & main loop
```
* **Team Benefit:** A developer working on CAM or MCP operates inside their dedicated crate without touching `gui.rs`, completely eliminating merge conflicts.

### 3. Unified Command & Transaction Bus
* Both GUI button clicks and external AI agent requests (via Model Context Protocol) must funnel through **exactly the same command catalog and transaction engine** (`CadCommand` -> `execute_transaction` -> `regenerate_all`).
* This guarantees identical semantics, error handling, and undo/redo support for human users and AI co-pilots alike.

### 4. Strict Dependency Inversion (Clean Architecture)
```text
[UI / Desktop Shell] 
       ↓
[Application Services / Workbenches] 
       ↓
[Domain Model (qymcad-core)] 
       ↓
[Geometric Kernel (OpenCASCADE)]
```
* The core domain and kernel must never depend on UI frameworks (egui), rendering backends (wgpu), or MCP wire protocols. This enables instantaneous headless testing of complex CAD pipelines.

---

## Action Plan & Next Steps

1. **Review and Discuss:** Open this proposal for team review.
2. **Isolate AI/MCP & CAM:** Ensure new modules (`qymcad-ai`, `qymcad-manufacturing`) strictly adhere to the narrow context rule.
3. **Refactor GUI Panels:** Gradually extract remaining panels out of `gui.rs` into isolated free-standing modules.
4. **Enforce CI Quality Gates:** Keep our fast gate (`tools/gate.py fast`) green with zero clippy warnings and strict ratchet enforcement.

Looking forward to feedback from the team and lead maintainers!
