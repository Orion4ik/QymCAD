# Subdivision surfaces

Subdivision surfaces let you shape organic and freeform models from a coarse control cage.

## How subdivision works

A polygonal cage made of quadrilaterals or triangles is smoothed across multiple refinement
levels. Faces divide into smaller quadrilaterals, producing curvature across the surface while
control points remain easy to move.

## Semi-sharp edge creases

Edges can be assigned crease sharpness values:

- **Sharpness 0.0**: completely smooth curved surface;
- **Sharpness between 0.0 and refinement level**: semi-sharp transition with a tight radius;
- **Sharpness above refinement level**: hard mechanical edge that preserves sharp corners.

This lets you combine organic transitions with flat faces on the same model.

## Sharp corners and features

Vertices surrounded by three or more creased edges stay fixed in place. They act as sharp corners,
ensuring that mechanical mounting flats and reference faces retain their dimensions while
neighbouring walls curve smoothly.

## Exporting and meshing

Subdivided cages can be converted into triangulated meshes or boundary surfaces for export into
standard formats or for direct toolpath preparation.
