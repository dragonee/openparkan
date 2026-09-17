<!-- Phase R research note R8: a placed object's scale. Where this note and docs/ disagree, docs/ wins: docs/04-missions.md#the-scale -->
# R8 engine: what the renderer and the sim must do with a placement's scale

Labels: **read**, **measured**, **guess**, **unknown**.

## Loading (parkan-world)

- `Instance::scale` was a STAND-IN that applied `object.scale[0]` to every kind. It is now
  `mission::Object::placed_scale`:
  - **kind 2 and 3 (vegetation, rock):** the record's scale. **Read:** World3D
    `AddNewObjectToGame` → AniMesh `SetScale`.
  - **kind 0 and 1 (building, unit):** always 1, whatever the record says. **Read:** iron3d
    builds them from the `.dat` with the matrix alone. This changes exactly two placements,
    the `tushka` animals on CAMPAIGN.02/Mission.03 at 1.5 and 1.25. The engine would
    currently draw them enlarged.
- The record's scale is uniform on all 864 placements (**measured**), so one `f32` is enough.
  If a non-uniform record should ever appear, the scale belongs in the object's own frame:
  the pose walk multiplies the root's axes by it (**read**, `AniMesh.dll:0x10008c8f`). Either
  way the order is untested, because the data is all uniform.
- The two zero words before `rotation` are rotation x and y, composed as R = Rz·Ry·Rx
  (**read**, `MisLoad.dll:0x10001d80`). All are 0 in the shipped data, so `Rz(rotation)`
  alone is exact. A reader that wants to be complete takes three angles.

## Renderer (parkan-render)

- The world matrix is `T(position) · Rz · Ry · Rx · S(scale)`, with the scale taken as above.
  `models.rs:80 placement()` already has this shape for z-only rotation and needs no change.
  Only the scale it is fed changes.
- A per-instance uniform scale leaves normals needing only renormalisation, which the
  shaders already do (read in the repo: `model.wgsl:50` normalises in the fragment stage).
- Scaled scenery stands on the ground only with the scale applied (**measured**). With it,
  2 of 134 scaled trees and 2 of 82 scaled stones float more than 0.25 above the ground.
  Without it, 63 and 44 do. Median base is 3.7 and 4.3 units under the ground (dug in).
  Do not "fix" floating scenery by snapping to the terrain. Apply the scale.

## Sim (hit test, collision, damage)

- **Hit test (read):** `SetScale` recomposes every node's matrices with the scale. The mesh
  test walks nodes in their own frames (`AniMesh.dll:0x10013ef0`, docs/26-damage.md), so
  rounds strike the scaled geometry. The engine's ray test must use the same scaled instance
  matrix as the renderer, not the unscaled model.
- **Bounds (read):** `SetScale` scales both bounding boxes (8 corners each), the bounding
  sphere and the cylinder (`AniMesh.dll:0x10014770`). Any broad-phase or walk-collision
  volume taken from the model must be multiplied by the scale. The sphere radius scales
  by the uniform factor.
- **Node area and volume getters (read):** ids 0xf and 0x10 return area × two scales and
  volume × three scales (`0x10005288`, `0x100052c7`). Anything that sizes an effect or a
  mass from them scales with it.
- **Hit points (read, for Control objects):** Control's volume scale is x·y·z of the mesh
  scale, and node life and maximum are rescaled by new / old (`Control.dll:0x10009ee0`).
  Units and buildings never receive the placement scale, so for them it stays 1 and needs
  no code. **Unknown:** whether vegetation or rock carries Control node life at all, i.e.
  whether a scaled tree is tougher. Nothing here decides it.
- **Unknown:** whether any script or other path calls `SetScale` on a unit at run time.
  Interface slot 15 is `[reg+0x3c]`, too common to sweep for.

## Tests to port

- `check_scale_field`: record version 10 on every mission; 218 uniform scaled placements;
  no scaled building; only the two animals among units.
- `check_scale_on_ground`: scaled kind 2/3 at their scale float in fewer than 10% of cases,
  against about half at scale 1. The control is the unit-scale placements.
- The engine's `install/scene.rs` bridge test already multiplies by `instance.scale`. It stays
  valid, since bridges are buildings at scale 1.
