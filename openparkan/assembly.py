"""What a placed mission object is made of, and where each visible part sits.

Two chains meet here.  Scenery names an ``objects.rlb`` record directly.  A
building or unit names a ``UNITS/**/*.dat`` assembly whose components form a
tree; a ``FORT`` root record carries no mesh of its own and names, in its
first slot, the record that does.

A child part bolts onto a node of its parent's mesh.  Mounting makes the
part's root node take that socket's pose, so its transform is the socket
composed with the inverse of the part's root, and it composes down the tree.
Armour, internal systems and ammunition are modelled but not drawn outside,
so only chassis, turrets and guns are parts here.  See ``docs/07-objects.md``.

This is the mount maths the engine ports (``engine/crates/parkan-world``), and
the golden cross-check compares the two.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from . import mesh as objmesh
from . import objects
from .nres import NResArchive

#: How many FORT-to-record hops a mesh lookup follows.
MAX_INDIRECTION = 3


@dataclass(frozen=True)
class Part:
    """One visible mesh of a placed object."""

    ref: objects.ResourceRef
    #: Where the part sits in the object's own frame.
    pose: objmesh.Pose
    #: Index of the part it hangs from, or -1.
    host: int
    #: The node of that part's mesh it hangs from, or -1.
    node: int


class Assembly:
    """Resolves objects to parts, reading each archive and mesh once."""

    def __init__(self, game: Path):
        self.game = game
        self.library = objects.ObjectLibrary(game / "objects.rlb")
        self._archives: dict[str, NResArchive] = {}
        self._meshes: dict[tuple[str, str], objmesh.ObjectMesh | None] = {}
        self._wears: dict[tuple[str, str], objmesh.Wear] = {}

    def archive(self, name: str) -> NResArchive:
        key = name.lower()
        if key not in self._archives:
            path = self.game / name
            if not path.exists():
                path = next(p for p in self.game.iterdir() if p.name.lower() == key)
            self._archives[key] = NResArchive.open(path)
        return self._archives[key]

    def record_mesh(self, record, depth: int = 0) -> objects.ResourceRef | None:
        """A record's ``.msh`` slot, following a FORT's first slot to its mesh."""
        if record is None or depth > MAX_INDIRECTION:
            return None
        if record.mesh:
            return record.mesh
        for slot in record.slots:
            if slot and not slot.suffix:
                found = self.record_mesh(self.library.get(slot.member), depth + 1)
                if found:
                    return found
        return None

    def unit_file(self, path: str) -> Path | None:
        """A ``UNITS\\...\\x.dat`` path, matched without regard to case."""
        at = self.game
        for part in path.replace("\\", "/").split("/"):
            exact = at / part
            if exact.exists():
                at = exact
                continue
            found = next((p for p in at.iterdir() if p.name.lower() == part.lower()), None) \
                if at.is_dir() else None
            if found is None:
                return None
            at = found
        return at

    def mesh(self, ref: objects.ResourceRef) -> objmesh.ObjectMesh | None:
        """A mesh parsed once, with its wear's material names."""
        key = (ref.library.lower(), ref.member.lower())
        if key not in self._meshes:
            try:
                archive = self.archive(ref.library)
                try:
                    wear = objmesh.parse_wear(
                        archive.read_name(ref.member.rsplit(".", 1)[0] + ".wea"))
                except KeyError:
                    wear = objmesh.Wear()
                self._wears[key] = wear
                self._meshes[key] = objmesh.parse(
                    archive.read_name(ref.member), ref.member, wear.materials)
            except (KeyError, ValueError, StopIteration, struct.error):
                self._meshes[key] = None
        return self._meshes[key]

    def wear(self, ref: objects.ResourceRef) -> objmesh.Wear:
        self.mesh(ref)
        return self._wears.get((ref.library.lower(), ref.member.lower()), objmesh.Wear())

    def parts(self, kind: int, path: str) -> list[Part]:
        """The visible parts of an object placed with this kind and path."""
        if kind in (2, 3):
            ref = self.record_mesh(self.library.get(path))
            return [Part(ref, objmesh.IDENTITY_POSE, -1, -1)] if ref else []
        f = self.unit_file(path)
        if f is None:
            return []
        try:
            unit = objects.load_unit(f)
            parents = unit.parents()
        except (objects.ObjectFormatError, OSError, struct.error):
            return []
        refs: list[objects.ResourceRef | None] = []
        poses: list[objmesh.Pose] = []
        slot_of: dict[int, int] = {}
        out: list[Part] = []
        for i, component in enumerate(unit.components):
            ref = self.record_mesh(self.library.get(component.ref.member))
            refs.append(ref)
            parent = parents[i]
            pose = objmesh.IDENTITY_POSE
            if parent >= 0:
                pose = poses[parent]
                host = self.mesh(refs[parent]) if refs[parent] else None
                part = self.mesh(ref) if ref else None
                if host and part and 0 <= component.attach_node < len(host.nodes):
                    socket = host.world_pose(component.attach_node)
                    mount = objmesh.compose(socket, objmesh.invert(part.root_pose()))
                    pose = objmesh.compose(pose, mount)
            poses.append(pose)
            if ref and component.is_external:
                slot_of[i] = len(out)
                out.append(Part(ref, pose, slot_of.get(parent, -1) if parent >= 0 else -1,
                                component.attach_node if parent >= 0 else -1))
        return out

    def joined_box(self, parts: list[Part]) -> tuple[tuple[float, ...], tuple[float, ...]] | None:
        """The box an agent joins from its parts' header boxes: its least and greatest corner.

        ``AniMesh.dll:0x10009510`` takes the eight corners of every part's
        stream-2 header box (``0x10009941``-``0x10009ac0``).  The agent's own
        mesh, part 0, gives them as they are.  A hung part's go through a
        matrix made of two nodes of the merged model: the rotation of the
        part's first own node -- its node 1, since its node 0 is left out of
        the merge -- and the place of that node's parent, the socket
        (``0x10009967``-``0x100099ba``, ``0x10009a12``).  The box is then the
        least and the greatest of them along each axis (``0x1000c3b0``).
        """
        low, high = [float("inf")] * 3, [float("-inf")] * 3
        for part in parts:
            mesh = self.mesh(part.ref)
            if mesh is None or mesh.volume is None:
                continue
            carried = None
            if part.host >= 0:
                first = mesh.world_pose(1) if len(mesh.nodes) > 1 else objmesh.IDENTITY_POSE
                carried = (part.pose[0], objmesh.compose(part.pose, first)[1])
            for corner in mesh.volume.corners:
                at = objmesh.apply(carried, corner) if carried else corner
                low = [min(a, b) for a, b in zip(low, at, strict=True)]
                high = [max(a, b) for a, b in zip(high, at, strict=True)]
        return (tuple(low), tuple(high)) if low[0] <= high[0] else None

    def cylinder(self, parts: list[Part]):
        """An agent's bounding cylinder: its far end, its near end and its radius, or ``None``.

        The cylinder of :meth:`joined_box` (:func:`openparkan.mesh.box_cylinder`),
        which interface ``0x18`` slot 12 hands out (``AniMesh.dll:0x100146a0``).
        A unit's record keeps the radius at ``+0x98`` as it is bound
        (``iron3d.dll:0x1007e5d6``).
        """
        box = self.joined_box(parts)
        return objmesh.box_cylinder(*box) if box else None
