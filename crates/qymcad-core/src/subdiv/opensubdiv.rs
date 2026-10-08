//! OpenSubdiv-compatible Catmull-Clark subdivision with semi-sharp edge and vertex creases.
//!
//! Standard Catmull-Clark smooths all edges unconditionally, turning a cube into a sphere-like shape.
//! Industrial surface design requires controlling sharpness:
//!
//! * **Sharpness 0.0**: completely smooth Catmull-Clark surface;
//! * **Sharpness >= levels**: infinitely sharp crease (hard mechanical feature edge);
//! * **Fractional sharpness (0 < s < levels)**: semi-sharp crease with controlled roundness,
//!   evaluated via the Chaikin/Hoppe/DeRose crease decay rule.
//!
//! At each subdivision step, edge sharpness decrements by 1.0: `s_{k+1} = max(0, s_k - 1)`. When sharpness
//! is between 0 and 1, the new vertex position is a linear blend between the sharp rule and the smooth rule.

use super::{edge_key, Cage};
use std::collections::HashMap;

/// Edge crease specification: an unordered pair of vertex indices and a sharpness weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeCrease {
    /// Vertex indices of the edge.
    pub edge: (u32, u32),
    /// Sharpness value (0.0 = smooth, 1.0..10.0 = semi-sharp, >= 10.0 = infinitely sharp).
    pub sharpness: f64,
}

impl EdgeCrease {
    /// Creates a new edge crease with normalized vertex order.
    pub fn new(a: u32, b: u32, sharpness: f64) -> Self {
        Self { edge: edge_key(a, b), sharpness: sharpness.max(0.0) }
    }
}

/// A subdivision cage augmented with edge and vertex crease sharpness maps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenSubdivCage {
    /// Base geometric cage (vertices and faces).
    pub cage: Cage,
    /// Sharpness weight per normalized edge.
    pub edge_creases: HashMap<(u32, u32), f64>,
    /// Sharpness weight per vertex index.
    pub vert_creases: HashMap<u32, f64>,
}

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn mid3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    scale3(add3(a, b), 0.5)
}

fn blend3(smooth: [f64; 3], sharp: [f64; 3], s: f64) -> [f64; 3] {
    let t = s.clamp(0.0, 1.0);
    add3(scale3(smooth, 1.0 - t), scale3(sharp, t))
}

impl OpenSubdivCage {
    /// Wraps a base cage with empty crease maps.
    pub fn new(cage: Cage) -> Self {
        Self { cage, edge_creases: HashMap::new(), vert_creases: HashMap::new() }
    }

    /// Sets the sharpness of an edge.
    pub fn set_edge_crease(&mut self, a: u32, b: u32, sharpness: f64) {
        let key = edge_key(a, b);
        if sharpness > 1e-6 {
            self.edge_creases.insert(key, sharpness);
        } else {
            self.edge_creases.remove(&key);
        }
    }

    /// Sets the sharpness of all edges on a given face.
    pub fn set_face_edges_crease(&mut self, face_index: usize, sharpness: f64) {
        if let Some(face) = self.cage.faces.get(face_index).cloned() {
            for k in 0..face.len() {
                let a = face[k];
                let b = face[(k + 1) % face.len()];
                self.set_edge_crease(a, b, sharpness);
            }
        }
    }

    /// Sets sharpness on all edges of the cage (creates an infinitely sharp or bevelled polyhedron).
    pub fn set_all_edges_crease(&mut self, sharpness: f64) {
        let faces = self.cage.faces.clone();
        for f in &faces {
            for k in 0..f.len() {
                let a = f[k];
                let b = f[(k + 1) % f.len()];
                self.set_edge_crease(a, b, sharpness);
            }
        }
    }

    /// Performs one OpenSubdiv Catmull-Clark subdivision step with crease evaluation.
    pub fn subdivide(&self) -> OpenSubdivCage {
        let t = self.cage.topo();
        let mut new_verts = Vec::with_capacity(self.cage.verts.len() * 4);
        let mut next_edge_creases = HashMap::new();
        let mut next_vert_creases = HashMap::new();

        // 1) Vertex points
        for (vi, &p) in self.cage.verts.iter().enumerate() {
            let edges = &t.vert_edges[vi];
            let faces = &t.vert_faces[vi];

            // Count sharp incident edges
            let mut sharp_edges: Vec<((u32, u32), f64)> = Vec::new();
            for &e in edges {
                if let Some(&s) = self.edge_creases.get(&e) {
                    if s > 1e-4 {
                        sharp_edges.push((e, s));
                    }
                }
            }

            let vert_sharpness = self.vert_creases.get(&(vi as u32)).copied().unwrap_or(0.0);
            let boundary: Vec<(u32, u32)> = edges.iter().copied().filter(|e| t.edge_faces[e].len() == 1).collect();

            let other = |e: (u32, u32)| if e.0 as usize == vi { e.1 } else { e.0 };

            // Base smooth Catmull-Clark vertex
            let p_smooth = if !boundary.is_empty() {
                if boundary.len() == 2 {
                    let a = self.cage.verts[other(boundary[0]) as usize];
                    let b = self.cage.verts[other(boundary[1]) as usize];
                    scale3(add3(add3(a, b), scale3(p, 6.0)), 1.0 / 8.0)
                } else {
                    p
                }
            } else {
                let n = edges.len() as f64;
                if n < 3.0 {
                    p
                } else {
                    let f_avg = scale3(faces.iter().fold([0.0; 3], |acc, &fi| add3(acc, self.cage.face_point(&self.cage.faces[fi]))), 1.0 / faces.len() as f64);
                    let r_avg = scale3(edges.iter().fold([0.0; 3], |acc, &(a, b)| add3(acc, mid3(self.cage.verts[a as usize], self.cage.verts[b as usize]))), 1.0 / n);
                    scale3(add3(add3(f_avg, scale3(r_avg, 2.0)), scale3(p, n - 3.0)), 1.0 / n)
                }
            };

            // Crease rules:
            // k = 0 or 1: smooth / dart
            // k = 2: crease rule (cubic B-spline along crease)
            // k >= 3 or corner: fixed corner point
            let (p_final, updated_vert_s) = if vert_sharpness > 0.0 || sharp_edges.len() >= 3 {
                // Corner vertex: remains fixed
                (p, (vert_sharpness - 1.0).max(0.0))
            } else if sharp_edges.len() == 2 {
                // Crease vertex: average along the two creased edges
                let a = self.cage.verts[other(sharp_edges[0].0) as usize];
                let b = self.cage.verts[other(sharp_edges[1].0) as usize];
                let p_crease = scale3(add3(add3(a, b), scale3(p, 6.0)), 1.0 / 8.0);
                let avg_s = (sharp_edges[0].1 + sharp_edges[1].1) * 0.5;
                let blended = blend3(p_smooth, p_crease, avg_s);
                (blended, 0.0)
            } else if sharp_edges.len() == 1 {
                // Dart vertex: smoothly blends into surrounding surface
                (p_smooth, 0.0)
            } else {
                (p_smooth, 0.0)
            };

            if updated_vert_s > 1e-4 {
                next_vert_creases.insert(vi as u32, updated_vert_s);
            }
            new_verts.push(p_final);
        }

        // 2) Face points
        let mut face_idx = Vec::with_capacity(self.cage.faces.len());
        for f in &self.cage.faces {
            face_idx.push(new_verts.len() as u32);
            new_verts.push(self.cage.face_point(f));
        }

        // 3) Edge points
        let mut edge_idx = HashMap::new();
        for (&(a, b), fs) in &t.edge_faces {
            let m = mid3(self.cage.verts[a as usize], self.cage.verts[b as usize]);
            let s = self.edge_creases.get(&(a, b)).copied().unwrap_or(0.0);

            let p_smooth = if fs.len() == 2 {
                let fp = add3(self.cage.face_point(&self.cage.faces[fs[0]]), self.cage.face_point(&self.cage.faces[fs[1]]));
                scale3(add3(scale3(m, 2.0), fp), 0.25)
            } else {
                m
            };

            // Sharp edge rule: exact edge midpoint
            let p_edge = if s > 0.0 { blend3(p_smooth, m, s) } else { p_smooth };

            let e_center_id = new_verts.len() as u32;
            edge_idx.insert((a, b), e_center_id);
            new_verts.push(p_edge);

            // Decrement edge sharpness for child segments
            let child_s = (s - 1.0).max(0.0);
            if child_s > 1e-4 {
                next_edge_creases.insert(edge_key(a, e_center_id), child_s);
                next_edge_creases.insert(edge_key(e_center_id, b), child_s);
            }
        }

        // 4) New quad faces
        let mut new_faces = Vec::with_capacity(self.cage.faces.iter().map(|f| f.len()).sum());
        for (fi, f) in self.cage.faces.iter().enumerate() {
            let c = face_idx[fi];
            for (k, &v) in f.iter().enumerate() {
                let prev = f[(k + f.len() - 1) % f.len()];
                let next = f[(k + 1) % f.len()];
                let e_prev = edge_idx[&edge_key(prev, v)];
                let e_next = edge_idx[&edge_key(v, next)];
                new_faces.push(vec![c, e_prev, v, e_next]);
            }
        }

        OpenSubdivCage { cage: Cage { verts: new_verts, faces: new_faces }, edge_creases: next_edge_creases, vert_creases: next_vert_creases }
    }

    /// Subdivides the cage `levels` times.
    pub fn subdivided(&self, levels: usize) -> OpenSubdivCage {
        let mut cur = self.clone();
        for _ in 0..levels {
            cur = cur.subdivide();
        }
        cur
    }

    /// Converts all quadrilateral and polygon faces into flat triangle indices for rendering or export.
    pub fn to_triangles(&self) -> Vec<[u32; 3]> {
        let mut tris = Vec::new();
        for f in &self.cage.faces {
            if f.len() >= 3 {
                for k in 1..f.len().saturating_sub(1) {
                    tris.push([f[0], f[k], f[k + 1]]);
                }
            }
        }
        tris
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncreased_cube_matches_smooth_catmull_clark() {
        let cube = Cage::cube(10.0);
        let sub_cage = OpenSubdivCage::new(cube.clone());

        let refined_sub = sub_cage.subdivided(2);
        let refined_pure = cube.subdivided(2);

        assert_eq!(refined_sub.cage.faces.len(), refined_pure.faces.len());
        assert_eq!(refined_sub.cage.verts.len(), refined_pure.verts.len());

        let vol_diff = (refined_sub.cage.volume() - refined_pure.volume()).abs();
        assert!(vol_diff < 1e-6, "volume difference: {vol_diff}");

        let (lo_sub, hi_sub) = refined_sub.cage.bounds();
        let (lo_pure, hi_pure) = refined_pure.bounds();
        for k in 0..3 {
            assert!((lo_sub[k] - lo_pure[k]).abs() < 1e-6);
            assert!((hi_sub[k] - hi_pure[k]).abs() < 1e-6);
        }
    }

    #[test]
    fn infinitely_sharp_cube_preserves_corners_and_bounds() {
        let cube = Cage::cube(10.0);
        let mut sub_cage = OpenSubdivCage::new(cube);
        // Set sharpness to 10.0 (infinitely sharp for 2 subdivision steps)
        sub_cage.set_all_edges_crease(10.0);

        let refined = sub_cage.subdivided(2);
        let (lo, hi) = refined.cage.bounds();

        for k in 0..3 {
            assert!((lo[k] - (-5.0)).abs() < 1e-9, "min bound along {k} was corrupted: {}", lo[k]);
            assert!((hi[k] - 5.0).abs() < 1e-9, "max bound along {k} was corrupted: {}", hi[k]);
        }
    }

    #[test]
    fn semi_sharp_crease_interpolates_between_sharp_and_smooth() {
        let cube = Cage::cube(10.0);

        let smooth = OpenSubdivCage::new(cube.clone()).subdivided(1);

        let mut semi = OpenSubdivCage::new(cube.clone());
        semi.set_all_edges_crease(0.5);
        let semi_res = semi.subdivided(1);

        let mut sharp = OpenSubdivCage::new(cube);
        sharp.set_all_edges_crease(5.0);
        let sharp_res = sharp.subdivided(1);

        // Semi-sharp vertex positions must strictly lie between smooth and sharp positions
        let p_smooth = smooth.cage.verts[0];
        let p_semi = semi_res.cage.verts[0];
        let p_sharp = sharp_res.cage.verts[0];

        for k in 0..3 {
            let min_val = p_smooth[k].min(p_sharp[k]);
            let max_val = p_smooth[k].max(p_sharp[k]);
            assert!(p_semi[k] >= min_val - 1e-5 && p_semi[k] <= max_val + 1e-5);
        }
    }
}
