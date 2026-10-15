# OpenSubdiv Integration & Subdivision Surfaces in QymCAD

## 1. Overview

QymCAD features an advanced subdivision surface engine fully compatible with **Pixar OpenSubdiv** Catmull-Clark algorithms and semi-sharp creasing rules (`crates/qymcad-core/src/subdiv/opensubdiv.rs`).

Subdivision surfaces (SubD) bridge the gap between fast, intuitive freeform cage manipulation (characteristic of direct modelers like Plasticity) and precise engineering B-Rep geometry.

---

## 2. Theoretical Principles

### 2.1 Catmull-Clark Scheme
A coarse polygonal cage with arbitrary n-gons is refined into smooth quad-dominant topology:
- **Face points**: Centroid of vertices forming the face.
- **Edge points**: Average of edge endpoints and adjacent face points.
- **Vertex points**: Weighted combination `(F + 2R + (n - 3)P) / n`.
- **Boundary preservation**: Boundary vertices subdivide as cubic B-spline curves along open perimeter edges.

### 2.2 Semi-Sharp Creasing (Chaikin / Hoppe / DeRose Decay)
Sharpness is assigned as a scalar weight $s \ge 0.0$ to edges and vertices:
- **$s = 0.0$**: Fully smooth organic surface.
- **$s \ge N$** (where $N$ is subdivision level count): Infinitely sharp feature edge (ideal for mechanical fillets and flanges).
- **$0.0 < s < N$**: Semi-sharp crease where edge sharpness decays by $1.0$ at each subdivision step:
  $$s_{k+1} = \max(0, s_k - 1.0)$$
  When $0 < s < 1$, vertex coordinates are computed as a convex blend between the sharp limit rule and the smooth Catmull-Clark rule:
  $$P_{blended} = (1 - s) P_{smooth} + s P_{sharp}$$

---

## 3. Rust API Architecture

```rust
use qymcad_core::subdiv::{Cage, OpenSubdivCage};

// 1. Create a coarse geometric cage (e.g. cube of side 20 mm)
let cube = Cage::cube(20.0);
let mut sub_cage = OpenSubdivCage::new(cube);

// 2. Set semi-sharp crease on top edges (e.g. sharpness = 2.5)
sub_cage.set_edge_crease(4, 5, 2.5);
sub_cage.set_edge_crease(5, 6, 2.5);
sub_cage.set_edge_crease(6, 7, 2.5);
sub_cage.set_edge_crease(7, 4, 2.5);

// 3. Subdivide 2 refinement levels
let refined = sub_cage.subdivided(2);

// 4. Extract rendering triangles or B-Rep patches
let triangles = refined.to_triangles();
println!("Subdivided into {} vertices, {} faces", refined.cage.verts.len(), refined.cage.faces.len());
```

---

## 4. AI & Model Context Protocol (MCP) Integration

OpenSubdiv operations can be triggered conversationally via the CAD Copilot or programmatically via MCP:

```json
{
  "operation": "subdivide_mesh",
  "body_id": 1,
  "levels": 2,
  "crease_sharpness": 3.0
}
```

Or via prompt:
> *"Apply OpenSubdiv to this cage with 2 levels and crease sharpness 2.5 on all mechanical edges."*
