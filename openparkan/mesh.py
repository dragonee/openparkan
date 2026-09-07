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
     5       4  vertex      UV, uint16 8.8 fixed point
    18       4  vertex      lightmap UV, uint16 over 1024; only on the 21
                             buildings whose wear has a LIGHTMAPS section
     6       6  face        triangle, three uint16 indices *relative to the
                             first_vertex of the batch that covers them*
     7      16  face        face record, contents unresolved
     8     ---  ---         unresolved; 24 bytes on most meshes, 96 on some
     9      32  sub-object  sub-object name ("Base_TM", "leaf1_m1o1")
    10       4  sub-object  one uint32, zero throughout the shipped data
    13      20  batch       draw batch: material, index range, vertex range
    15       8  vertex      unresolved
    17      20  node        building interior path graph, see parse_path_graph
    19     ---  ---         unresolved; present on many meshes, empty on some

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
#: whose role is not established.  Within a block the first four triangle
#: counts fall monotonically on 1157 of 1161 chains; including the fifth drops
#: that to 869, which is what says it is not a level.
SLOTS_PER_VARIANT = 5
LOD_COUNT = 4
VARIANT_COUNT = 3
NO_SLOT = 0xFFFF
#: Bit 0 of a sub-object's flags marks interior geometry.
SUBOBJECT_INTERIOR = 0x0001
#: Bit 5 marks a collision hull -- geometry the engine tests against but never
#: draws.  It sits on 28 nodes across the shipped archives and on nothing else,
#: and every one of them is named ``CP_m1o1`` (19) or ``BTCP_m1o1`` (9).  They
#: are always leaves, and together they carry 18402 triangles that a renderer
#: must skip: excluding them is what takes the models that fit inside their own
#: authored bounding box from 422 to 434 of 434.
SUBOBJECT_COLLISION = 0x0020
NO_PARENT = 0xFFFF

BATCH_SIZE = 20
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

#: UVs use the same 8.8 fixed point as the terrain.
UV_FIXED_POINT_SCALE = 256.0

#: Lightmap UVs are the same uint16 over 1024, not 256: they address one page
#: of an atlas, so they never leave 0..1.  Every one of the 21 lightmapped
#: meshes tops out at exactly ``round((1 - 0.5 / width) * 1024)`` for its own
#: lightmap's width -- 1022 for a 256-pixel page, 1020 for a 128 -- which is
#: the half-texel inset an atlas is authored with.
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


@dataclass
class Subobject:
    """One named part of a model.

    Buildings carry their inside and their outside in the same mesh -- Parkan
    lets you walk into them -- so a model's parts are split between the two.
    Names say which (``o01_0_m1o1`` outside, ``i03_0_m1o1`` inside) and so does
    ``flags`` bit 0, which agrees with the naming on all 1564 sub-objects
    across six archives.  Bit 5 marks a collision hull, which is not drawn at
    all; see ``is_collision``.
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
    def is_collision(self) -> bool:
        """Whether this node is a collision hull rather than something to draw.

        A `CP_m1o1` hull is a crude box around the part it belongs to, and it
        is bigger than the part -- a turret's is nearly twice its height -- so
        drawing it puts a translucent slab over the model and pushes it
        outside the box the file itself states.
        """
        return bool(self.flags & SUBOBJECT_COLLISION)

    def slots_for_lod(self, lod: int = 0, variant: int = 0) -> list[int]:
        """The slot to draw this node at one level of detail.

        At most one slot: a level is a single entry, not the five-slot run the
        earlier reading took it for.  Drawing all five superimposes four
        levels of detail and is what makes a model look like scrambled
        geometry.

        28 nodes carry only the fifth slot of their variant, so level 0 falls
        back to the coarsest slot present rather than drawing nothing.
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
    first_vertex: int
    vertex_count: int

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

    def local_pose(self, node: int) -> Pose:
        """A node's pose in its parent's frame."""
        key = self.rest_key(node)
        return IDENTITY_POSE if key is None else self.keys[key].pose

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

    def posed_positions(self, lod: int = 0) -> list[tuple[float, float, float]]:
        """Vertex positions with each node's world pose applied.

        Mesh vertices are authored in their own node's frame; without this a
        multi-part model draws every part piled on the origin.  Vertices not
        reached by ``lod`` keep their raw position.
        """
        out = list(self.positions)
        for i, node in enumerate(self.nodes):
            pose = self.world_pose(i)
            if pose == IDENTITY_POSE:
                continue
            for si in node.slots_for_lod(lod):
                if si >= len(self.slots):
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

        Collision hulls are never returned; see ``Subobject.is_collision``.
        """
        out: list[tuple[int, int, int]] = []
        for node in self.nodes:
            if node.is_collision:
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
                     struct.unpack_from("<3f", raw_slots, o + 20))
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

    The record is nine float32.  In ``static.rlb`` and ``turrets.rlb`` the
    first triple is zero, the second is a position inside the model's bounding
    box and the third a unit direction -- but that reading does not hold in
    every archive (``guns.rlb`` stores a scalar width in a vector slot), so the
    triples are exposed as they are and named ``a``, ``position`` and
    ``direction`` only as the best-supported interpretation.
    """

    name: str
    a: tuple[float, float, float]
    position: tuple[float, float, float]
    direction: tuple[float, float, float]


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


@dataclass
class PathNode:
    """A waypoint inside a building."""

    position: tuple[float, float, float]
    a: int
    b: int


@dataclass
class PathLink:
    """A traversable connection between two waypoints."""

    start: int
    end: int
    #: Eight further slots, ``0xFFFFFFFF`` throughout the shipped data.
    extra: tuple[int, ...]


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
