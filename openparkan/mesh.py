"""Reader for object geometry -- the ``MESH`` members of ``static.rlb`` & co.

An object mesh is itself an NRes archive nested inside an archive member, with
the same numeric-type-as-stream-selector convention as the terrain
(``landmesh.py``), and the same per-vertex encodings.  All 68 meshes in
``static.rlb`` carry the identical set of 14 streams:

    id  stride  indexed by  contents
    --  ------  ----------  ---------------------------------------------
     1      38  node        flags, parent, and slot_index[lod * 5 + group]
     2      68  slot        a 140-byte header, then geometry slots
     3      12  vertex      position, float32 x/y/z
     4       4  vertex      normal, int8 x/y/z / 127, then one padding byte
     5       4  vertex      UV, uint16 over 1024
    18       4  vertex      lightmap UV, uint16 over 1024; only on the 21
                             buildings whose wear has a LIGHTMAPS section
     6       6  face        triangle, three uint16 indices *relative to the
                             first_vertex of the batch that covers them*
     7      16  face        flags, three edge neighbours, the face normal as
                             int16 over 32767, and the winged-edge link
     8      24  pose key    a node's placement: float32[3] translation,
                             float32 frame time, int16[4] rotation over 32767
     9      32  sub-object  sub-object name ("Base_TM", "leaf1_m1o1")
    10       4  sub-object  one uint32, zero throughout the shipped data
    13      20  batch       draw batch: material, index range, vertex range
    15       8  vertex      unresolved
    17      20  node        building interior path graph, see parse_path_graph
    19       2  frame       per animated node, ``frame_count`` indices into
                             stream 8; the archive entry's second count is
                             ``frame_count``

Materials are assigned per *batch*, not per face: stream 13 groups runs of the
index buffer and names a material for each, which is why no field of the face
record ever held a texture index.

Indices in stream 6 are **relative to the batch that covers them**, the
DirectX ``DrawIndexedPrimitive`` convention: the real vertex is
``batch.first_vertex + index``.  ``ObjectMesh.triangles`` has this already
applied; ``raw_triangles`` keeps the file's own values.
"""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass, field

from .nres import NResArchive

STREAM_BOUNDS = 2
STREAM_POSITION = 3
STREAM_NORMAL = 4
STREAM_UV = 5
STREAM_TRIANGLE = 6
STREAM_FACE = 7
#: One 16-byte record per triangle: flags, three edge neighbours, the face's
#: own normal as int16 over NORMAL_SCALE, and the winged-edge link.
FACE_RECORD_STRIDE = 16
#: A face record's normal is int16 over this, as the terrain's is.
NORMAL_SCALE = 32767.0
#: No neighbour across this edge.
NO_FACE = 0xFFFF
#: The record's last word is the terrain's field 13 again: three 2-bit codes,
#: edge ``e`` at ``(link >> 2 * e) & 3``, each **the index of the matching edge
#: back in the neighbouring face**, 3 where there is none.  Checked by geometry
#: rather than by mutuality -- the code names the neighbour's edge with the
#: same two vertex positions on **674206 of 674206** in-range neighbours, and
#: it is 3 on all 51455 open edges.  Only the low six bits are the link: 1387
#: faces carry more above them, and they recur at the same record positions
#: across unrelated models (faces 120, 123 and 124 of a node's run on most
#: trees), which is what a buffer the exporter never cleared looks like.
FACE_LINK_BITS = 2
FACE_LINK_MASK = 0x3F
FACE_NO_TWIN = 3
#: Face flag ``0x02`` is the **walkable surface** of the buildings a unit walks
#: through: it sits on exactly the 29 meshes that carry a path graph, all in
#: ``fortif.rlb``, and on nothing else.  All 6166 of its faces lie in a level-0
#: slot, and posed into model space 6100 stand above the engine's own cos-80
#: degree ground threshold (``landmesh.WALKABLE_NORMAL_Z``) -- ramps and stairs
#: as well as flat floor -- of which 4258 are within 10 degrees of level.  It is
#: a chosen subset: 1802 more of the same meshes' level-0 faces point within
#: 10 degrees of up and carry nothing.  The collision push-out reads it
#: (``Control.dll:0x1001dbce``); the ground search does not (*measured*, and
#: *read*; ``docs/07-objects.md``).  4 and 32 are the faces a round passes
#: through (``ROUND_SKIPS_FACE``); ``FACE_DOOR_LEAF`` is 16.
FACE_BUILDING_FLOOR = 0x02
#: Face flag ``0x10`` is the **broad face of a door leaf**, which the collision
#: pass's door test requires (``AniMesh.dll:0x1000dbba``).  All 384 are vertical
#: once posed, on 52 interior nodes of 20 ``fortif.rlb`` buildings, 50 of them
#: animated; within a leaf's level-0 slot only its two broad sides carry it,
#: never the slab's rim.
FACE_DOOR_LEAF = 0x10
STREAM_SUBOBJECT_NAME = 9
STREAM_SUBOBJECT_HEADER = 1
STREAM_BATCH = 13
STREAM_PATH_GRAPH = 17
STREAM_POSE_KEY = 8
STREAM_FRAME_MAP = 19
STREAM_LIGHTMAP_UV = 18

SUBOBJECT_HEADER_SIZE = 38
SLOT_HEADER_SIZE = 0x8C
SLOT_SIZE = 68
#: A node selects geometry with ``slot_index[variant * SLOTS_PER_VARIANT + lod]``.
#: Each block of five is one variant: four levels of detail and a fifth slot
#: that is **what the unit's own first-person view draws**.  Within a block the
#: first four triangle counts fall monotonically on 1157 of 1161 chains;
#: including the fifth drops that to 869, which is what says it is not a level.
#:
#: What reads it -- *read*.  A turret's camera component registers the view it
#: creates with the unit's mesh (``Control.dll:0x1002399a``, IAnimation slot
#: 33), and the mesh draw asks whether the view drawing it is one of those
#: (``AniMesh.dll:0x10014bdb``, slot 34).  If it is, every node draws slot
#: ``variant * 5 + 4`` instead of the level the distance picks
#: (``0x10014be5``), and a node with nothing there is not drawn at all.
#:
#: The 28 nodes that carry *only* a fifth slot -- flag ``0x20``, named
#: ``CP_m1o1`` or ``BTCP_m1o1`` -- are therefore geometry only that view sees,
#: and they are where the view sits: on all 54 turret records whose mesh has
#: one, the ``CameraCenter`` control point is on it.  So they are the cockpit.
#: An earlier reading called them collision hulls and this slot collision
#: geometry; a round's hit test takes level 0 (``HIT_LOD``) and never this
#: slot.  On the 288 ordinary nodes that carry both, the fifth is a separate
#: slot: a same-sized copy of level 0 on 141, coarser on 137, finer on 10.
COCKPIT_LOD = 4
#: The name this slot had while it was read as collision geometry.
COLLISION_SLOT = COCKPIT_LOD
SLOTS_PER_VARIANT = 5
LOD_COUNT = 4
VARIANT_COUNT = 3
#: The three five-slot blocks are damage states, not a second level-of-detail
#: axis.  1479 nodes fill block 0, 135 fill block 1 and 15 fill block 2, and no
#: node ever fills a later block without the earlier ones.  A later block is
#: the same part with pieces missing: `s_tree_0_04`'s crown drops from 212
#: triangles topping out at z 22.12 to 104 at 10.39, and `fr_l_gener`'s pylons
#: go 88 / 66 / 14 triangles at z 22.70 / 9.31 / -16.88, sinking into the
#: ground.  The `.ndp` damage table settles it: every one of those nodes names
#: an explosion, and the tree's is `explode_tree.exp`.
VARIANT_INTACT = 0
NO_SLOT = 0xFFFF

#: The level a round's hit test reads, in the node's current variant
#: (``AniMesh.dll:0x10010c33`` asks ``0x100124d0`` for lod 0).
HIT_LOD = 0
#: Stream-7 face flags a round passes through (``Control.dll:0x1001d9fa``):
#: 4 and 32.  Flags 2 and 16 are struck.
ROUND_SKIPS_FACE = 0x24
#: Stream-13 batch bits a round passes through: 8 always, 0x200 unless the
#: round's type carries ``ROUND_TESTS_BATCH_200`` (``0x1001da02``).
ROUND_SKIPS_BATCH = 0x8
ROUND_SKIPS_BATCH_200 = 0x200
ROUND_TESTS_BATCH_200 = 0x4000000
#: Bit 0 of a sub-object's flags marks interior geometry.
SUBOBJECT_INTERIOR = 0x0001
#: Bit 5 marks the **cockpit**: a node whose only geometry is its fifth slot,
#: so only the unit's own first-person view draws it (``COCKPIT_LOD``).  It
#: sits on 28 nodes, all in ``turrets.rlb``, every one named ``CP_m1o1`` (19)
#: or ``BTCP_m1o1`` (9); they are always leaves, and the turret's
#: ``CameraCenter`` sits on them.  Their 18402 triangles are not what anyone
#: else sees: excluding them is what takes the models that fit inside their
#: own authored bounding box from 422 to 434 of 434.  When a mesh draws its
#: fifth slot it tells ``CShade``'s mesh draw mode 2 for a node with this bit
#: and 1 for any other (``AniMesh.dll:0x10014e9a``), which that draw files
#: under layer 10 or 9 where an ordinary surface gets 0, or 5 see-through.
SUBOBJECT_COCKPIT = 0x0020
#: The name this bit had while the nodes were read as collision hulls.
SUBOBJECT_COLLISION = SUBOBJECT_COCKPIT
NO_PARENT = 0xFFFF

BATCH_SIZE = 20
#: Bit 1 of a batch's flags dword: its triangles are tested from **both** sides.
#: ``AniMesh.dll:0x1001110c`` runs the plane test once as the segment lies and,
#: failing, once with its ends swapped, so a round or a sight ray reaches the
#: face from behind.  It is set on 1477 of the 15153 shipped batches, 946 of them
#: in ``static.rlb`` (the trees) and 440 in ``fortif.rlb``.
BATCH_TWO_SIDED = 0x2
#: High byte of a batch's material word.  It says whether the batch is lit by
#: the model's lightmap: 0x00 on the 972 batches that are, 0xFF on the other
#: 14181.  Every vertex a 0x00 batch reaches carries a non-zero lightmap UV
#: (51324 of 51324), against 1% of the vertices a 0xFF batch reaches.
BATCH_MATERIAL_MASK = 0xFF
BATCH_LIT = 0x00

PATH_NODE_SIZE = 20
PATH_LINK_SIZE = 40
NO_LINK = 0xFFFFFFFF

NAME_FIELD = 32
FACE_STRIDE = 16

POSE_KEY_SIZE = 24
#: A pose quaternion is int16 over this scale, so 32767 reads as 1.0.
QUATERNION_SCALE = 32767.0
#: A node with no entry in the frame map.
NO_ANIMATION = 0xFFFF

#: ``node_of_vertex`` where no node's slot reaches a vertex.
NO_NODE = -1

#: UVs are uint16 over 1024, not the terrain's 8.8 fixed point: the strided
#: expansion multiplies stream 5 by 1/1024 (``Terrain.dll:0x10038a01``), so a
#: material's cell is one cell of its page (docs/07-objects.md, "How a material
#: reaches the device").
UV_FIXED_POINT_SCALE = 1024.0

#: Lightmap UVs are the same uint16 over 1024: they address one page of an
#: atlas, so they never leave 0..1.  Every one of the 21 lightmapped meshes tops
#: out at exactly ``round((1 - 0.5 / width) * 1024)`` for its own lightmap's
#: width -- 1022 for a 256-pixel page, 1020 for a 128 -- which is the half-texel
#: inset an atlas is authored with.
LIGHTMAP_UV_SCALE = 1024.0


#: A pose: a translation and a rotation quaternion ``(w, x, y, z)``.
Pose = tuple[tuple[float, float, float], tuple[float, float, float, float]]

IDENTITY_POSE: Pose = ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0, 0.0))


def quaternion_multiply(
    a: tuple[float, float, float, float], b: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    """Compose two rotations, ``a`` applied after ``b``."""
    w1, x1, y1, z1 = a
    w2, x2, y2, z2 = b
    return (
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
    )


def quaternion_rotate(
    q: tuple[float, float, float, float], v: tuple[float, float, float]
) -> tuple[float, float, float]:
    """Rotate a vector by a unit quaternion."""
    w, x, y, z = q
    vx, vy, vz = v
    tx = 2.0 * (y * vz - z * vy)
    ty = 2.0 * (z * vx - x * vz)
    tz = 2.0 * (x * vy - y * vx)
    return (
        vx + w * tx + (y * tz - z * ty),
        vy + w * ty + (z * tx - x * tz),
        vz + w * tz + (x * ty - y * tx),
    )


def compose(parent: Pose, child: Pose) -> Pose:
    """Place a child pose in its parent's frame."""
    (px, py, pz), pq = parent
    ct, cq = child
    rx, ry, rz = quaternion_rotate(pq, ct)
    return ((px + rx, py + ry, pz + rz), quaternion_multiply(pq, cq))


def quaternion_slerp(
    a: tuple[float, float, float, float], b: tuple[float, float, float, float], t: float
) -> tuple[float, float, float, float]:
    """Interpolate two unit rotations the short way round."""
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    if dot < 0.0:
        b, dot = tuple(-x for x in b), -dot
    if dot > 0.9995:
        out = tuple(x + t * (y - x) for x, y in zip(a, b, strict=True))
    else:
        theta = math.acos(dot)
        sa, sb = math.sin((1.0 - t) * theta), math.sin(t * theta)
        out = tuple((sa * x + sb * y) / math.sin(theta) for x, y in zip(a, b, strict=True))
    norm = math.sqrt(sum(x * x for x in out)) or 1.0
    return tuple(x / norm for x in out)


def blend(a: Pose, b: Pose, t: float) -> Pose:
    """A lerp of the translations and a slerp of the rotations."""
    (ta, qa), (tb, qb) = a, b
    return (tuple(x + t * (y - x) for x, y in zip(ta, tb, strict=True)),
            quaternion_slerp(qa, qb, t))


def invert(pose: Pose) -> Pose:
    """The pose that undoes ``pose``."""
    (x, y, z), (w, qx, qy, qz) = pose
    inverse = (w, -qx, -qy, -qz)
    back = quaternion_rotate(inverse, (-x, -y, -z))
    return back, inverse


def apply(pose: Pose, point: tuple[float, float, float]) -> tuple[float, float, float]:
    """Transform a point by a pose."""
    (tx, ty, tz), q = pose
    x, y, z = quaternion_rotate(q, point)
    return (x + tx, y + ty, z + tz)


@dataclass
class PoseKey:
    """One entry of stream 8: where a node sits, and when.

    24 bytes: a ``float32[3]`` translation, a ``float32`` time in frames, and
    the rotation as ``int16[4]`` over 32767 in ``(w, x, y, z)`` order -- 34038
    of the 34049 keys the game ships are unit length to within 0.1%, and the
    eleven that are not are all zero or unnormalised, so they are repaired on
    read.

    A node's rest pose is its ``fallback_key``.  Animated nodes walk the frame
    map instead; their fallback is the animation's last frame, which is why
    using it for a rest pose leaves turrets pointing wherever they stopped.

    ``rotation`` is stored conjugated relative to the file, because the game
    is left-handed; see ``parse``.
    """

    translation: tuple[float, float, float]
    time: float
    rotation: tuple[float, float, float, float]

    @property
    def pose(self) -> Pose:
        return self.translation, self.rotation


@dataclass
class BoundingVolume:
    """The model's authored extent, from the 140-byte stream-2 header.

    Eight box corners, a bounding sphere, then the axis and radius of a
    bounding cylinder.  The box is the useful part: it is stated in *posed*
    space, so it is the oracle that proves a pose reading right or wrong.
    """

    corners: list[tuple[float, float, float]]
    centre: tuple[float, float, float]
    radius: float
    axis_low: tuple[float, float, float]
    axis_high: tuple[float, float, float]
    axis_radius: float

    @property
    def minimum(self) -> tuple[float, float, float]:
        return tuple(min(c[a] for c in self.corners) for a in range(3))

    @property
    def maximum(self) -> tuple[float, float, float]:
        return tuple(max(c[a] for c in self.corners) for a in range(3))


@dataclass
class Slot:
    """A run of triangles and batches that a node can select."""

    first_triangle: int
    triangle_count: int
    first_batch: int
    batch_count: int
    aabb_min: tuple[float, float, float]
    aabb_max: tuple[float, float, float]
    #: +0x20: a bounding sphere, centre and radius.
    sphere: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    #: +0x30: the geometry's area -- ``AniMesh.dll:0x100051f0`` id 0xf, times
    #: two of the object's scales.  Armour's first value is weighed by it.
    area: float = 0.0
    #: +0x34: the bounding box's volume -- id 0x10, times all three scales.
    #: A node's ``.ndp`` density times this is its mass.
    volume: float = 0.0


@dataclass
class Subobject:
    """One named part of a model.

    Buildings carry their inside and their outside in the same mesh -- Parkan
    lets you walk into them -- so a model's parts are split between the two.
    Names say which (``o01_0_m1o1`` outside, ``i03_0_m1o1`` inside) and so does
    ``flags`` bit 0, which agrees with the naming on all 1564 sub-objects
    across six archives.  Bit 5 marks the cockpit, which only the unit's own
    first-person view draws; see ``is_cockpit``.
    """

    name: str
    flags: int
    parent: int
    #: Start of this node's run in the frame map (stream 19), or NO_ANIMATION.
    #: The run is ``frame_count`` long and each entry indexes a pose key.
    anim_start: int
    fallback_key: int
    #: 15 slot indices, addressed as ``[variant * 5 + lod]``; NO_SLOT where
    #: the node has no geometry for that combination.
    slot_index: list[int]

    @property
    def is_animated(self) -> bool:
        return self.anim_start != NO_ANIMATION

    @property
    def is_interior(self) -> bool:
        return bool(self.flags & SUBOBJECT_INTERIOR)

    @property
    def is_cockpit(self) -> bool:
        """Whether this node is the cockpit, drawn only to the unit's own view.

        A ``CP_m1o1`` node is a box around the eye, bigger than the part -- a
        turret's is nearly twice its height -- so drawing it in a survey puts a
        slab over the model and pushes it outside the box the file states.
        """
        return bool(self.flags & SUBOBJECT_COCKPIT)

    #: The name ``is_cockpit`` had while these nodes were read as hulls.
    is_collision = is_cockpit

    def cockpit_slot(self, variant: int = 0) -> int | None:
        """The fifth slot of a variant: what the unit's own view draws.

        The mesh draws it in place of a level of detail for a view the
        turret's camera component registered (``COCKPIT_LOD``).  A round's hit
        test does not read it (``hit_slot``).
        """
        index = self.slot_index[variant * SLOTS_PER_VARIANT + COCKPIT_LOD]
        return None if index == NO_SLOT else index

    #: The name ``cockpit_slot`` had while the slot was read as collision.
    collision_slot = cockpit_slot

    def hit_slot(self, variant: int = 0) -> int | None:
        """The slot a round is tested against: level 0 of the variant, no fallback.

        None on every cockpit node, which carries no level 0.
        """
        index = self.slot_index[variant * SLOTS_PER_VARIANT + HIT_LOD]
        return None if index == NO_SLOT else index

    def slots_for_lod(self, lod: int = 0, variant: int = 0) -> list[int]:
        """The slot to draw this node at one level of detail.

        At most one slot: a level is a single entry, not the five-slot run the
        earlier reading took it for.  Drawing all five superimposes four
        levels of detail and is what makes a model look like scrambled
        geometry.

        28 nodes carry only the fifth slot of their variant, and level 0
        falls back to it.  Those 28 are exactly the cockpit nodes, which the
        game draws only to the unit's own view, so the fallback never reaches
        a survey picture -- ``select`` and the viewer skip them -- but it is
        how their geometry is found at all.
        """
        base = variant * SLOTS_PER_VARIANT
        block = self.slot_index[base : base + SLOTS_PER_VARIANT]
        if lod < len(block) and block[lod] != NO_SLOT:
            return [block[lod]]
        if lod == 0:
            fallback = next((i for i in block if i != NO_SLOT), NO_SLOT)
            return [fallback] if fallback != NO_SLOT else []
        return []


@dataclass
class Batch:
    """A run of the index buffer drawn with one material.

    ``material`` indexes the model's wear (its ``.wea`` palette).
    """

    material: int
    #: High byte of the material word; ``BATCH_LIT`` when the batch takes the
    #: model's lightmap.
    flag: int
    first_index: int
    index_count: int
    #: The base the batch's indices are relative to -- D3D's
    #: ``BaseVertexIndex``.
    first_vertex: int
    #: How far past that base its indices reach -- D3D's ``NumVertices``, the
    #: span the driver has to transform.  It is **exactly** the largest
    #: relative index plus one, on all 15153 batches of all 435 meshes, which
    #: is why it never tiled the vertex array: it is a draw hint, and two
    #: batches are free to overlap.
    vertex_count: int
    #: The record's first dword, which a query's batch masks test
    #: (``AniMesh.dll:0x100081fa``).  ``ROUND_SKIPS_BATCH`` and
    #: :data:`BATCH_TWO_SIDED` live in it.
    flags: int = 0

    @property
    def two_sided(self) -> bool:
        """Whether its triangles are struck from behind as well."""
        return bool(self.flags & BATCH_TWO_SIDED)

    @property
    def is_lit(self) -> bool:
        """Whether this batch takes the model's baked lightmap."""
        return self.flag == BATCH_LIT

    @property
    def triangles(self) -> tuple[int, int]:
        """``(first triangle, triangle count)`` into the mesh's triangle list."""
        return self.first_index // 3, self.index_count // 3


@dataclass
class ObjectMesh:
    name: str
    positions: list[tuple[float, float, float]]
    normals: list[tuple[float, float, float]]
    uv: list[tuple[float, float]]
    #: Absolute vertex indices, with each batch's first_vertex already added.
    triangles: list[tuple[int, int, int]]
    #: Lightmap UVs, empty on a model with no baked lighting.
    lightmap_uv: list[tuple[float, float]] = field(default_factory=list)
    #: The file's own batch-relative indices.
    raw_triangles: list[tuple[int, int, int]] = field(default_factory=list)
    #: Stream 7, one per triangle.  ``face_normal`` is the face's own normal,
    #: unit length on all 241887 faces of the 435 meshes and agreeing with the
    #: cross product on 241879; ``face_adjacency`` its three edge neighbours,
    #: ``NO_FACE`` where there is none, and every in-range neighbour shares an
    #: edge -- 674200 of 674206.  ``face_flags`` is 0 on 233714 faces and
    #: takes 2, 4, 16, 32 or 34 on the rest; ``face_class`` is the winged-edge
    #: link in its low six bits (``edge_twin``), with leftovers above them on
    #: 1387 faces.
    face_normal: list[tuple[float, float, float]] = field(default_factory=list)
    face_adjacency: list[tuple[int, int, int]] = field(default_factory=list)
    face_flags: list[int] = field(default_factory=list)
    face_class: list[int] = field(default_factory=list)
    subobjects: list[str] = field(default_factory=list)
    texture_names: list[str] = field(default_factory=list)
    batches: list[Batch] = field(default_factory=list)
    nodes: list[Subobject] = field(default_factory=list)
    slots: list[Slot] = field(default_factory=list)
    #: Stream 8, the pose keys every node draws its placement from.
    keys: list[PoseKey] = field(default_factory=list)
    #: Stream 19, ``frame_count`` key indices per animated node.
    frame_map: tuple[int, ...] = ()
    frame_count: int = 0
    #: The authored bounding volume from the stream-2 header, if present.
    volume: BoundingVolume | None = None

    def collision_radius(self, scale: tuple[float, float, float] = (1.0, 1.0, 1.0)) -> float:
        """The radius of an agent's swept sphere when this mesh is all it has.

        A part keeps its stream-2 header's sphere (``AniMesh.dll:0x1000a891``);
        the agent's sphere is those spheres' union at the current pose, times
        the largest of the object's three scales (``0x10009510``), and that
        radius is what the collision object sweeps (``Control.dll:0x1001fec0``).
        With a single part -- every round -- the union is the header's own
        sphere.  0 when the mesh carries no header.
        """
        return self.volume.radius * max(scale) if self.volume else 0.0

    def rest_key(self, node: int) -> int | None:
        """Index of the pose key that puts a node in its rest position.

        An animated node's ``fallback_key`` is the *last* frame of its
        animation, not its rest pose -- a turret's fallback leaves it swung
        round to wherever the animation ended -- so an animated node takes
        frame 0 of its own run in the frame map instead.
        """
        n = self.nodes[node]
        if n.is_animated and self.frame_count and n.anim_start < len(self.frame_map):
            return self.frame_map[n.anim_start]
        if n.fallback_key < len(self.keys):
            return n.fallback_key
        return None

    def root_pose(self) -> Pose:
        """The pose of the model's root node.

        A part is mounted by making this pose equal the socket's, so it is
        what a host's socket replaces.  Its translation is zero on all 1318
        turret and gun meshes the game mounts, which is why taking only the
        socket's position was indistinguishable from the full pose except
        where the two rotations disagree.
        """
        for i, node in enumerate(self.nodes):
            if node.parent == NO_PARENT:
                return self.local_pose(i)
        return IDENTITY_POSE

    def local_pose(self, node: int) -> Pose:
        """A node's pose in its parent's frame."""
        key = self.rest_key(node)
        return IDENTITY_POSE if key is None else self.keys[key].pose

    def node_volume(self, node: int, variant: int = 0) -> float:
        """What ``Control.dll`` weighs a node by: its level-0 slot's volume, or 0.

        Not ``slots_for_lod``: that falls back to another slot where a node has
        no level 0, and the engine gets nothing there.
        """
        index = self.nodes[node].slot_index[variant * SLOTS_PER_VARIANT]
        return 0.0 if index == NO_SLOT or index >= len(self.slots) else self.slots[index].volume

    def world_pose(self, node: int) -> Pose:
        """A node's pose in model space, composed down the parent chain."""
        pose = self.local_pose(node)
        seen = {node}
        parent = self.nodes[node].parent
        while parent != NO_PARENT and parent < len(self.nodes) and parent not in seen:
            seen.add(parent)
            pose = compose(self.local_pose(parent), pose)
            parent = self.nodes[parent].parent
        return pose

    def node_of_triangle(self, index: int, lod: int = 0) -> int | None:
        """Which node draws a triangle, via the slot that covers it."""
        for i, node in enumerate(self.nodes):
            for si in node.slots_for_lod(lod):
                if si >= len(self.slots):
                    continue
                slot = self.slots[si]
                if slot.first_triangle <= index < slot.first_triangle + slot.triangle_count:
                    return i
        return None

    @property
    def animated(self) -> bool:
        """Whether this mesh carries an animation worth playing."""
        return self.frame_count > 1 and any(n.is_animated for n in self.nodes)

    def node_of_vertex(self) -> list[int]:
        """Which node poses each vertex, or ``NO_NODE``.

        The same walk ``posed_positions`` makes, kept as a table instead of
        applied: a vertex belongs to exactly one node -- no vertex of any of
        the 435 meshes is reached by two slots whose nodes pose it
        differently -- so a model animates **rigidly per node**, with no
        skinning weights to recover.
        """
        out = [NO_NODE] * len(self.positions)
        for i, node in enumerate(self.nodes):
            for si in node.slot_index:
                if si == NO_SLOT or si >= len(self.slots):
                    continue
                slot = self.slots[si]
                stop = slot.first_triangle + slot.triangle_count
                for tri in self.triangles[slot.first_triangle : stop]:
                    for v in tri:
                        if v < len(out):
                            out[v] = i
        return out

    def pose_at(self, node: int, frame: float) -> Pose:
        """A node's pose at a fractional frame, as ``AniMesh.dll:0x10012880`` finds it.

        The key is the run's entry at ``round(frame - 0.5)``.  Past the run,
        on a node that is not animated, or where the entry is at or beyond the
        node's fallback key, the *fallback key alone* is the pose
        (``0x10012ba2``): it is the last key of the node's own run, so the key
        after it belongs to the next node and is never blended in.  Otherwise,
        at a key's time or the next key's in stream 8, that key is taken
        whole, and between them the two are blended by time.
        """
        n = self.nodes[node]
        index = None
        k = int(math.floor(frame - 0.5 + 0.5))
        if n.is_animated and 0 <= k < self.frame_count:
            entry = self.frame_map[n.anim_start + k]
            if entry < n.fallback_key:
                index = entry
        if index is None:
            if n.fallback_key >= len(self.keys):
                return IDENTITY_POSE
            return self.keys[n.fallback_key].pose
        if index >= len(self.keys):
            return IDENTITY_POSE
        key = self.keys[index]
        if frame == key.time or index + 1 >= len(self.keys):
            return key.pose
        nxt = self.keys[index + 1]
        if frame == nxt.time or nxt.time == key.time:
            return nxt.pose if frame == nxt.time else key.pose
        return blend(key.pose, nxt.pose, (frame - key.time) / (nxt.time - key.time))

    def blended_pose(self, node: int, frame_a: float, frame_b: float, weight: float) -> Pose:
        """Two frames and a weight, as a controller hands them (``0x10012560``).

        Frame A alone at weight 0 or when B is negative, frame B alone at
        weight 1 or when A is negative, a blend between otherwise.
        """
        use_a = weight < 1.0 and frame_a >= 0
        use_b = weight > 0.0 and frame_b >= 0
        if use_a and use_b:
            return blend(self.pose_at(node, frame_a), self.pose_at(node, frame_b), weight)
        if use_b:
            return self.pose_at(node, frame_b)
        return self.pose_at(node, max(frame_a, 0.0))

    def track(self, node: int) -> list[int]:
        """The pose key a node takes at each frame, or ``[]`` if it is still.

        Stream 19 lays the runs out consecutively, ``frame_count`` entries per
        animated node starting at its ``anim_start``, and each entry indexes
        stream 8.  A key's ``time`` is its frame number: they run 0 to
        ``frame_count - 1`` with one key per frame.
        """
        n = self.nodes[node]
        if not n.is_animated or self.frame_count < 2:
            return []
        run = self.frame_map[n.anim_start : n.anim_start + self.frame_count]
        return list(run) if len(run) == self.frame_count else []

    def posed_positions(self) -> list[tuple[float, float, float]]:
        """Vertex positions with each node's world pose applied.

        Mesh vertices are authored in their own node's frame; without this a
        multi-part model draws every part piled on the origin.

        **Every** slot is posed, not just one level's.  That is unambiguous:
        no vertex of any of the 435 meshes is reached by two slots whose nodes
        have different world poses.  It used to pose only the slots one level
        reached, which quietly left the coarser levels in raw node-local
        space -- and reading them back with the wrong level made them look as
        though they lived in a different frame altogether.  They do not: posed,
        each level sits on level 0's centre to within a median 0.000 to 0.011
        of the model's size.
        """
        out = list(self.positions)
        for i, node in enumerate(self.nodes):
            pose = self.world_pose(i)
            if pose == IDENTITY_POSE:
                continue
            for si in node.slot_index:
                if si == NO_SLOT or si >= len(self.slots):
                    continue
                slot = self.slots[si]
                stop = slot.first_triangle + slot.triangle_count
                for tri in self.triangles[slot.first_triangle : stop]:
                    for v in tri:
                        if v < len(out):
                            out[v] = apply(pose, self.positions[v])
        return out

    @property
    def has_interior(self) -> bool:
        return any(n.is_interior for n in self.nodes)

    def select(self, lod: int = 0, interior: bool | None = None) -> list[tuple[int, int, int]]:
        """Triangles for one level of detail.

        A model holds up to three levels of detail in one mesh, and drawing
        them superimposed is what makes a building look like scrambled
        geometry.  Selecting a single LOD fixes that.

        ``interior`` filters on the node flag when given.  Leave it None --
        the default -- to draw the whole model: a building's tall structure
        lives in its ``i*`` nodes, so excluding them makes buildings far too
        short.  The flag marks internal *components*, not a separate indoor
        model.

        Cockpit nodes are never returned; see ``Subobject.is_cockpit``.
        """
        out: list[tuple[int, int, int]] = []
        for node in self.nodes:
            if node.is_cockpit:
                continue
            if interior is not None and node.is_interior != interior:
                continue
            for index in node.slots_for_lod(lod):
                if index >= len(self.slots):
                    continue
                slot = self.slots[index]
                stop = slot.first_triangle + slot.triangle_count
                out += self.triangles[slot.first_triangle : stop]
        return out

    def material_of_triangle(self, index: int) -> int | None:
        """Material index for a triangle, via the batch that covers it."""
        for b in self.batches:
            first, count = b.triangles
            if first <= index < first + count:
                return b.material
        return None

    def edge_twin(self, face: int, edge: int) -> int | None:
        """Which edge of the face across ``edge`` is the shared one.

        ``None`` where the edge is open.  The face record's last word is the
        same winged-edge link the terrain keeps in its field 13; only its low
        six bits are the link (``FACE_LINK_MASK``).
        """
        code = (self.face_class[face] >> (FACE_LINK_BITS * edge)) & 3
        return None if code == FACE_NO_TWIN else code

    @property
    def vertex_count(self) -> int:
        return len(self.positions)

    @property
    def triangle_count(self) -> int:
        return len(self.triangles)

    def bounds(self) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
        xs = [p[0] for p in self.positions]
        ys = [p[1] for p in self.positions]
        zs = [p[2] for p in self.positions]
        return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))


def parse(blob: bytes, name: str = "<mesh>", texture_names: list[str] | None = None) -> ObjectMesh:
    """Parse a ``MESH`` payload.  ``texture_names`` comes from the sibling
    ``.wea`` member and is carried through for callers that want it."""
    inner = NResArchive(blob, name)
    stream = {e.type_id: inner.read(e) for e in inner}
    entries = {e.type_id: e for e in inner}

    raw_pos = stream[STREAM_POSITION]
    nv = len(raw_pos) // 12
    positions = [struct.unpack_from("<3f", raw_pos, i * 12) for i in range(nv)]

    raw_n = stream[STREAM_NORMAL]
    normals = [
        tuple((v - 256 if v > 127 else v) / 127.0 for v in raw_n[i * 4 : i * 4 + 3])
        for i in range(nv)
    ]

    def read_uv(raw: bytes, scale: float) -> list[tuple[float, float]]:
        return [
            (
                struct.unpack_from("<H", raw, i * 4)[0] / scale,
                struct.unpack_from("<H", raw, i * 4 + 2)[0] / scale,
            )
            for i in range(nv)
        ]

    uv = read_uv(stream[STREAM_UV], UV_FIXED_POINT_SCALE)
    raw_lm = stream.get(STREAM_LIGHTMAP_UV, b"")
    lightmap_uv = read_uv(raw_lm, LIGHTMAP_UV_SCALE) if len(raw_lm) >= nv * 4 else []

    raw_tri = stream[STREAM_TRIANGLE]
    raw_triangles = [
        struct.unpack_from("<3H", raw_tri, i * 6) for i in range(len(raw_tri) // 6)
    ]

    raw_names = stream.get(STREAM_SUBOBJECT_NAME, b"")
    subobjects = [
        raw_names[i * NAME_FIELD : (i + 1) * NAME_FIELD].split(b"\0")[0].decode("latin-1")
        for i in range(len(raw_names) // NAME_FIELD)
    ]

    headers = stream.get(STREAM_SUBOBJECT_HEADER, b"")
    parts = []
    header_entry = entries.get(STREAM_SUBOBJECT_HEADER)
    n_parts = header_entry.element_count if header_entry else 0
    if len(headers) == n_parts * SUBOBJECT_HEADER_SIZE:
        for i in range(n_parts):
            words = struct.unpack_from("<19H", headers, i * SUBOBJECT_HEADER_SIZE)
            parts.append(
                Subobject(
                    name=subobjects[i] if i < len(subobjects) else "",
                    flags=words[0],
                    parent=words[1],
                    anim_start=words[2],
                    fallback_key=words[3],
                    slot_index=list(words[4:19]),
                )
            )

    # Stream 7: one record per triangle.
    raw_records = stream.get(STREAM_FACE, b"")
    face_normal: list[tuple[float, float, float]] = []
    face_adjacency: list[tuple[int, int, int]] = []
    face_flags: list[int] = []
    face_class: list[int] = []
    for i in range(len(raw_records) // FACE_RECORD_STRIDE):
        at = i * FACE_RECORD_STRIDE
        face_flags.append(struct.unpack_from("<H", raw_records, at)[0])
        face_adjacency.append(struct.unpack_from("<3H", raw_records, at + 2))
        face_normal.append(tuple(
            v / NORMAL_SCALE
            for v in struct.unpack_from("<3h", raw_records, at + 8)
        ))
        face_class.append(struct.unpack_from("<H", raw_records, at + 14)[0])

    slots = []
    slot_entry = entries.get(STREAM_BOUNDS)
    raw_slots = stream.get(STREAM_BOUNDS, b"")
    n_slots = slot_entry.element_count if slot_entry else 0
    if len(raw_slots) == SLOT_HEADER_SIZE + n_slots * SLOT_SIZE:
        for i in range(n_slots):
            o = SLOT_HEADER_SIZE + i * SLOT_SIZE
            ts, tc, bs, bc = struct.unpack_from("<4H", raw_slots, o)
            slots.append(
                Slot(ts, tc, bs, bc,
                     struct.unpack_from("<3f", raw_slots, o + 8),
                     struct.unpack_from("<3f", raw_slots, o + 20),
                     struct.unpack_from("<4f", raw_slots, o + 0x20),
                     *struct.unpack_from("<2f", raw_slots, o + 0x30))
            )

    keys = []
    key_entry = entries.get(STREAM_POSE_KEY)
    raw_keys = stream.get(STREAM_POSE_KEY, b"")
    for i in range(key_entry.element_count if key_entry else 0):
        o = i * POSE_KEY_SIZE
        if o + POSE_KEY_SIZE > len(raw_keys):
            break
        translation = struct.unpack_from("<3f", raw_keys, o)
        time = struct.unpack_from("<f", raw_keys, o + 12)[0]
        w, x, y, z = struct.unpack_from("<4h", raw_keys, o + 16)
        length = math.sqrt(sum((v / QUATERNION_SCALE) ** 2 for v in (w, x, y, z)))
        if length < 1e-6:
            rotation = (1.0, 0.0, 0.0, 0.0)
        else:
            scale = QUATERNION_SCALE * length
            # The game is left-handed (Z up, DirectX): the matrix it builds
            # from a quaternion is the transpose of the right-handed one, so
            # the same four numbers denote the conjugate rotation here.
            rotation = (w / scale, -x / scale, -y / scale, -z / scale)
        keys.append(PoseKey(translation, time, rotation))

    frame_entry = entries.get(STREAM_FRAME_MAP)
    raw_frames = stream.get(STREAM_FRAME_MAP, b"")
    frame_map = struct.unpack_from(f"<{len(raw_frames) // 2}H", raw_frames, 0)
    frame_count = frame_entry.link_count if frame_entry else 0

    volume = None
    if len(raw_slots) >= SLOT_HEADER_SIZE:
        v = struct.unpack_from("<35f", raw_slots, 0)
        volume = BoundingVolume(
            corners=[v[i * 3 : i * 3 + 3] for i in range(8)],
            centre=v[24:27],
            radius=v[27],
            axis_low=v[28:31],
            axis_high=v[31:34],
            axis_radius=v[34],
        )

    batches = []
    raw_batch = stream.get(STREAM_BATCH, b"")
    for i in range(entries[STREAM_BATCH].element_count if STREAM_BATCH in entries else 0):
        f = struct.unpack_from("<10H", raw_batch, i * BATCH_SIZE)
        batches.append(
            Batch(
                material=f[2] & BATCH_MATERIAL_MASK,
                flag=f[2] >> 8,
                index_count=f[4],
                first_index=f[5],
                vertex_count=f[7],
                first_vertex=f[8],
                flags=f[0] | f[1] << 16,
            )
        )

    # Resolve batch-relative indices to absolute ones.  A mesh without batches
    # has nothing to resolve against, so its indices are taken as they are.
    triangles = list(raw_triangles)
    for b in batches:
        first, count = b.triangles
        for t in range(first, min(first + count, len(triangles))):
            a, bb, c = raw_triangles[t]
            triangles[t] = (a + b.first_vertex, bb + b.first_vertex, c + b.first_vertex)

    return ObjectMesh(
        name=name,
        positions=positions,
        normals=normals,
        uv=uv,
        triangles=triangles,
        lightmap_uv=lightmap_uv,
        raw_triangles=raw_triangles,
        face_normal=face_normal,
        face_adjacency=face_adjacency,
        face_flags=face_flags,
        face_class=face_class,
        subobjects=subobjects,
        texture_names=texture_names or [],
        batches=batches,
        nodes=parts,
        slots=slots,
        keys=keys,
        frame_map=frame_map,
        frame_count=frame_count,
        volume=volume,
    )


CONTROL_POINT_NUMERIC = 36
CONTROL_POINT_NAME = 32


@dataclass
class ControlPoint:
    """A named point on a model: an attachment, a light, an effect origin.

    The record is nine float32: a triple that is **exactly zero on 3432 of
    the 3599** shipped points, a position, and a vector.

    That vector is a direction whose **length carries a magnitude**, which is
    what an earlier reading missed when it called ``guns.rlb`` and
    ``parts.rlb`` "scalars in a vector slot".  On a frame or an aim point it
    is unit length -- 553 of the 570 named ``*_X``, ``*_Y``, ``*_Z``,
    ``*Direct`` or ``*Center`` -- and on a size it is an axis times the size:
    **all 191 points named Width, Height or Size have exactly one non-zero
    component**, so ``Width_1`` at ``(0, 0, 0.42)`` is 0.42 across the model's
    z, not a scalar written into a vector.
    """

    name: str
    a: tuple[float, float, float]
    position: tuple[float, float, float]
    direction: tuple[float, float, float]

    @property
    def nodes(self) -> tuple[int, int]:
        """The first triple's second and third slots, read as the int32 they are.

        The first is **the node the point sits on** -- the only one the
        control system keeps when it loads the file (``Control.dll:0x1000b22a``),
        and the one its world position and direction come from.  It names a
        node of the same-stem mesh on all but one of the points that have one.

        The second is the same number on 3338 of the 3599.  Where it differs it
        is **the node that carries the point**: for a contact point the ground
        contact marks that node (``0x1001a3aa``) and stops counting the point
        once that node is destroyed (``0x1001ac0d``).  On a chassis it is the
        wheel or leg below the body node the point sits on -- 69 of the 78
        chassis points whose pair differs.  See ``placed_on`` and ``carrier``.
        """
        return tuple(struct.unpack("<i", struct.pack("<f", v))[0] for v in self.a[1:])

    @property
    def placed_on(self) -> int:
        """The node whose pose places the point."""
        return self.nodes[0]

    @property
    def carrier(self) -> int:
        """The node a contact point lives and dies with; often ``placed_on``."""
        return self.nodes[1]


def parse_control_points(blob: bytes, source: str = "<cpt>") -> list[ControlPoint]:
    """Parse a ``CTPT`` payload.

    The layout is two parallel arrays, not an array of records::

        uint32  count
        count x float32[9]     numeric data
        count x char[32]       names

    which is why the total is always ``4 + count * 68``.
    """
    count = struct.unpack_from("<I", blob, 0)[0]
    expected = 4 + count * (CONTROL_POINT_NUMERIC + CONTROL_POINT_NAME)
    if expected != len(blob):
        raise ValueError(
            f"{source}: {count} control points implies {expected} bytes, have {len(blob)}"
        )
    names_at = 4 + count * CONTROL_POINT_NUMERIC
    out = []
    for i in range(count):
        v = struct.unpack_from("<9f", blob, 4 + i * CONTROL_POINT_NUMERIC)
        raw = blob[names_at + i * CONTROL_POINT_NAME : names_at + (i + 1) * CONTROL_POINT_NAME]
        out.append(ControlPoint(raw.split(b"\0")[0].decode("latin-1"), v[0:3], v[3:6], v[6:9]))
    return out


#: Hall-way vertex flags that make a vertex a *place*.  ``Behavior.dll``
#: walks a building's places every tick (``0x10018ac0``).
#: The control pod, where a capturer goes (``0x1003094c``).
PLACE_POD = 0x40
#: A dock: charges, repairs and rearms whoever of the clan or its allies stands
#: in it (``0x10019251``).
PLACE_DOCK = 0x20 | 0x200 | 0x400
#: A mine's loading place and a storage's unloading place.
PLACE_MINE = 0x8
PLACE_STORE = 0x10
#: A ground-level place, 10 wide and 12 high instead of 5 and 3
#: (``0x100184f0``) -- the only kind a unit sent to reload will pick.
PLACE_GROUND = 0x10000000
#: The main teleport's places (in 0x8000, out 0x4000, exit 0x10000, and two
#: more bits the same test takes).
PLACE_TELEPORT = 0x7C000
#: A unit counts as in a place only while its world speed is at most this, in
#: m/s (``Behavior.dll:0x10018492``, property 0x27): it must stand still.  At
#: a teleport place the bound is 1000 (``0x1001851f``).
PLACE_SPEED = 2.0
PLACE_TELEPORT_SPEED = 1000.0
#: The size gate a vertex puts on a unit walking to it
#: (``Behavior.dll:0x10042d08``).  ``PLACE_GROUND`` lets any unit through,
#: ``VERTEX_BUILDING_SIZE`` one no bigger than the building's own size class,
#: and a vertex with neither only a unit of ``VERTEX_SMALL`` or less.
VERTEX_ANY_SIZE = PLACE_GROUND
VERTEX_BUILDING_SIZE = 0x20000000
VERTEX_SMALL = 2
#: A link the search will not let a walker cross, and one only a flyer crosses
#: (``Behavior.dll:0x10042c90``, ``0x10036934``).
LINK_SHUT = 0x20000
LINK_FLYER_ONLY = 0x10000
#: Which of a link's eight tail slots decide that (``ArealMap.dll:0x1000a274``,
#: ``0x1000a294``): the live link's ``+0xc`` and ``+0x1c``.
LINK_GATE = (1, 5)


@dataclass
class PathNode:
    """A waypoint inside a building."""

    position: tuple[float, float, float]
    #: The vertex's flags: ``PLACE_*`` make it somewhere a unit stands.
    a: int
    b: int

    @property
    def flags(self) -> int:
        return self.a

    def fits(self, size: int, building: int) -> bool:
        """Whether a unit of size class ``size`` may walk to this vertex.

        ``building`` is the building's own size class, its property ``0x201``.
        """
        if self.a & VERTEX_ANY_SIZE:
            return True
        if self.a & VERTEX_BUILDING_SIZE:
            return building >= size
        return size <= VERTEX_SMALL


@dataclass
class PathLink:
    """A traversable connection between two waypoints."""

    start: int
    end: int
    #: Eight further slots.  The loader copies them to the live 56-byte link's
    #: ``+8``..``+0x24`` (``ArealMap.dll:0x1000a568``), and slots 1 and 5 are
    #: the link's own gate: both zero makes it ``0x20000``, crossed by nothing,
    #: and the first zero alone ``0x10000``, crossed only by a flyer
    #: (``0x1000a274``, ``0x1000a294``).  *Measured*: all eight are
    #: ``0xFFFFFFFF`` on 1034 of the 1096 shipped links, 18 read (0, -1) on the
    #: three mines and the three factories, and 44 carry a node index with 1
    #: beside it four times over.  See ``docs/24-motion.md``, "The hall-way
    #: gates, in the shipped buildings".
    extra: tuple[int, ...]

    @property
    def gate(self) -> int:
        """``LINK_SHUT``, ``LINK_FLYER_ONLY`` or 0, as the search reads it."""
        first, second = (self.extra[i] for i in LINK_GATE)
        if first:
            return 0
        return LINK_SHUT if not second else LINK_FLYER_ONLY


@dataclass
class PathGraph:
    nodes: list[PathNode]
    links: list[PathLink]


def parse_path_graph(blob: bytes, node_count: int, link_count: int) -> PathGraph:
    """Parse stream 17, the interior path graph the engine calls a *hall way*.

    The two counts are not in the payload; they are the element-count and the
    following field of the stream's own NRes directory entry.  Only buildings
    carry one -- 29 of the 30 meshes in ``fortif.rlb``, and nothing anywhere
    else, because only a building has an inside to walk around.
    """
    expected = node_count * PATH_NODE_SIZE + link_count * PATH_LINK_SIZE
    if len(blob) != expected:
        raise ValueError(
            f"path graph: {node_count} nodes and {link_count} links imply "
            f"{expected} bytes, have {len(blob)}"
        )
    nodes = []
    for i in range(node_count):
        x, y, z, a, b = struct.unpack_from("<3f2I", blob, i * PATH_NODE_SIZE)
        nodes.append(PathNode((x, y, z), a, b))
    base = node_count * PATH_NODE_SIZE
    links = []
    for i in range(link_count):
        values = struct.unpack_from("<10I", blob, base + i * PATH_LINK_SIZE)
        links.append(PathLink(values[0], values[1], values[2:]))
    return PathGraph(nodes, links)


def read_path_graph(archive: NResArchive) -> PathGraph | None:
    """Read the path graph out of an already-opened object mesh, if it has one."""
    for entry in archive:
        if entry.type_id == STREAM_PATH_GRAPH and entry.size:
            return parse_path_graph(
                archive.read(entry), entry.element_count, entry.link_count
            )
    return None


@dataclass
class Wear:
    """A model's ``.wea``: its material palette, and its baked lighting.

    The file is one or more keyword sections, each a count then ``index name``
    pairs.  The first section has no keyword and holds the materials a draw
    batch indexes.  ``LIGHTMAPS`` names members of ``lightmap.lib``, and
    exactly the 21 meshes that carry one also carry mesh stream 18 -- the
    lightmap's UV set.  No mesh has one without the other.
    """

    materials: list[str] = field(default_factory=list)
    lightmaps: list[str] = field(default_factory=list)


def _read_table(tokens: list[str], start: int) -> tuple[list[str], int]:
    """Read ``count`` then ``index name`` pairs; returns the table and where
    reading stopped."""
    if start >= len(tokens) or not tokens[start].lstrip("-").isdigit():
        return [], start
    count = int(tokens[start])
    names = [""] * count
    i = start + 1
    while i + 1 < len(tokens) and tokens[i].lstrip("-").isdigit():
        index = int(tokens[i])
        if 0 <= index < count:
            names[index] = tokens[i + 1]
        i += 2
    return names, i


def parse_wear(blob: bytes) -> Wear:
    """Parse a ``.wea`` in full, materials and lightmaps."""
    tokens = blob.decode("latin-1").split()
    materials, at = _read_table(tokens, 0)
    lightmaps: list[str] = []
    while at < len(tokens):
        keyword = tokens[at].upper()
        table, at = _read_table(tokens, at + 1)
        if keyword == "LIGHTMAPS":
            lightmaps = table
        if not table:
            at += 1
    return Wear(materials, lightmaps)


def read_wea(blob: bytes) -> list[str]:
    """The material palette of a ``.wea``; see ``parse_wear`` for the rest."""
    return parse_wear(blob).materials
