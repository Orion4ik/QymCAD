# Automation and AI

QymCAD includes an automation layer supporting natural language instructions and tool integration
via the Model Context Protocol (MCP).

## Natural language commands

The engineering assistant translates written requests into structured actions. Prompts such as:

- "Create a 50 mm cube"
- "Add a 3 mm fillet to edges"
- "Assign 6061 aluminum material"
- "Estimate cost for 10 units"

turn directly into parameters and geometric operations.

## Safe transactional updates

Operations from external scripts or assistants run through a protected boundary:

- Commands are validated against model rules before changes are applied;
- If an operation fails, the document automatically restores its previous clean state;
- Geometry is never altered partially or left in an inconsistent condition.

## Model Context Protocol tools

The system exposes verified operations through the standard Model Context Protocol:

- `cad.create_primitive`: generates solids with specified dimensions;
- `cad.extrude`: creates 3D bodies from sketches;
- `cad.assign_material`: links material properties to bodies;
- `cad.calculate_mass_properties`: computes volume, mass, and center of mass;
- `cad.analyze_dfm`: checks overhangs and printability;
- `cad.estimate_cost`: produces detailed cost breakdowns;
- `cad.subdivide_mesh`: applies subdivision surface refinement.

## Connecting external assistants

Any MCP-compatible client can connect to the tool endpoints by running the server command,
allowing design workflows to be driven from chat interfaces and custom automation pipelines.
