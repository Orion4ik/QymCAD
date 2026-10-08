# Materials and manufacturing

A part is not only a geometric shape; it also carries real engineering properties for physical
calculation and fabrication.

## Assigning an engineering material

Every body can be assigned a material from the engineering database. The catalogue holds verified
metals (such as 6061-T6 and 7075-T6 aluminum, 1018 and 4140 steel, 304 and 316L stainless steel,
titanium, brass) and engineering polymers (PLA, PETG, ABS, PA12, PC, PEEK).

Selecting a material fills in mechanical attributes, density, thermal limits, and market cost per
kilogram.

## Mass properties and calculation

Once a material is set, physical mass calculation reports:

- Total volume in cubic millimetres;
- Surface area in square millimetres;
- Mass in kilograms and grams;
- Coordinates of the centre of mass [X, Y, Z];
- Estimated cost of the raw stock.

Values update whenever dimensions change.

## Design for manufacturing and 3D printing

Manufacturing evaluation inspects the model before sending it to machines:

- **Overhang analysis**: checks surface slope angles against the build plate to indicate surfaces
  needing support;
- **Build envelope check**: validates whether the bounding dimensions fit within target printers
  such as Bambu Lab or Prusa build volumes;
- **Slicer dispatch**: automates configuration for external tools such as OrcaSlicer and PrusaSlicer.

## Cost estimation and CAM toolpaths

The manufacturing engine provides costing and machining preparation:

- **Cost estimation**: breaks down expenses across raw material, machine runtime, electricity,
  setup labor amortized across the batch quantity, and post-processing;
- **Toolpaths and G-code**: generates cutting trajectories for flat and ball end mills, outputting
  standard ISO G-code for CNC controllers.
