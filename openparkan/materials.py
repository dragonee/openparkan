"""Reader for ``Material.lib`` -- the ``MAT0`` material database.

905 materials, each naming one or more textures.  A model's wear (its
``.wea``) lists material names; a mesh batch picks one of them by index; the
material names the texture.  That completes the chain from a triangle to a
pixel::

    mesh stream 13 batch -> wear entry -> Material.lib MAT0 -> Textures.lib Texm

Terrain works the same way: ``Land1.wea`` and ``Land2.wea`` name *materials*,
not textures, which is why ``WATER``, ``B_S0`` and ``ENV_NLAVA`` cannot be
found in ``Textures.lib`` -- they are here, and they name ``WATER0.0``,
``B_FOUND.0`` and ``LAV00.0``.

The record
----------

A record is a 14-byte header, then the entries, then a table of animation
tracks over them::

    +0   uint16   entry count
    +2   uint16   track count      (the engine refuses more than 20)
    +4   uint8    class            \\
    +5   uint8    unused            |  version-gated; see below
    +6   float32                    |
    +10  uint32                    /
    +14  entry[]  34 bytes each
         track[]  a uint32, a uint16 key count, then 6 bytes per key

**All 905 records parse to the byte with nothing left over.**

The last four header fields are gated on a **version** that is not in the
record at all -- it is the archive directory entry's second count field
(``NResEntry.link_count``), and the parser substitutes a default below each
threshold: 0xFF for the two bytes at version < 2, 1.0 for the float at < 3, 0
for the dword at < 4.  Every shipped record declares **version 6**, so all
four are present and the header is 14 bytes on every one.  The float is 1.0
on all 905 and the dword 0 on 901 -- the other four hold a float, 1000.0 or
9999.0.

The entry
---------

An entry is a **D3DMATERIAL7 written as bytes**, and that is why it is 34
long::

    +0   uint8[3] ambient  rgb      +3  uint8 ambient  alpha, per cent
    +4   uint8[3] diffuse  rgb      +7  uint8 diffuse  alpha, per cent
    +8   uint8[3] specular rgb      +11 uint8 specular alpha, per cent
    +12  uint8[3] emissive rgb      +15 uint8 emissive alpha, per cent
    +16  uint8    specular power
    +17  int8     sub-image, -1 for the whole texture
    +18  char[16] texture name

The colours are bytes over 255 and the alphas **per cent** -- the parser
multiplies the four alpha bytes by 0.01 and the twelve colour bytes by
1/255.  Not one of the 12572 alpha bytes exceeds 100, which no wrong offset
survives.  The longest shipped name is 12 characters and seven entries name
nothing at all, which the engine reads as "no texture".

Getting the stride right is what makes every material resolve.  The names
used to be extracted by pattern, and the pattern swallowed whatever
alphanumeric byte happened to sit in front of a name: it read ``qqds.7`` out
of ``B_MTP_04``, whose real texture is ``MTP_04.0``, and ``0FAIR.0`` ..
``7FAIR.0`` out of the ``FIRE_SMOKE`` animations, where those digits are the
cell bytes of fourteen frames that all name ``FAIR.0``.  Read by offset, **all
905 materials name a texture that is in Textures.lib**, against 891 by
pattern.

The diffuse colour matters on screen: ``WATER``'s texture is a neutral grey
ripple and the blue is entirely in its ``#4d6aff``, and lava is a dull red
pattern tinted ``#b41e00``.  761 materials carry a diffuse other than white.
The ambient alpha is the only one that ever varies -- 3138 of the 3143
entries are at 100 and the exception is ``FIRESTORM``, whose entries run 0,
60, 80, 90, 95, 100 across its frames, which is a fade-in.

The byte before the name selects a **sub-image**: -1 means the whole texture,
and anything else is a cell.  630 of the 3143 entries take the whole texture.
The cell is **not** a grid index -- it indexes the texture's own ``Page``
table, a list of sub-image rectangles appended to the Texm payload; see
``texm.parse_pages``.  That is why ``SUN.0`` behaves like a 2 x 2 grid (its
four pages *are* the four quadrants) while ``EFFECT6.0`` does not: its 26
pages are four 128 x 32 strips, eight 64 x 64 tiles, eight 30 x 30 discs and
five 16 x 16 icons.  All 62 textures a material indexes carry a table, and
every one of the **2513** cells asked for is inside its own.

How a material draws
--------------------

Neither the record's class byte nor anything else inside the record says how
a material blends.  The **archive directory** does.  Its first count field --
where every other archive keeps an element count -- is a flags byte here, and
it is the one the loader branches on: bit 1 into one field of the loaded
material, bits 2 to 5 into another, bit 0 into a local flag that one record
sets and bit 6 into one that none does.  It takes five values, and they sort
the library by how the material is drawn:

===== ==== =========================================================
flags    n what it holds
===== ==== =========================================================
    0   54 opaque and lit.  52 of the 54 name a texture with no alpha
           channel at all, and the other two name no texture
    2  417 the ordinary lit skin: **3140 of the 3143** references from
           a model's wear land here, and 261 carry a specular colour
    4  219 see-through: smoke, dust and most of the sky.  175 of the
           219 carry a black diffuse, so they are drawn unlit
    5    1 ``ENV_STARS``, which is 4 with bit 0 as well
    8  214 **additive**: 44 of the 46 materials the artists named
           ``*_add`` are here, along with every ``JET*``, ``SHOOT*``,
           ``LASER_*`` and ``SPLASH*``.  210 of the 214 carry a black
           diffuse and **not one** carries a specular
===== ==== =========================================================

Who names what agrees: the terrain's layer tables are 196 at 0 against 74
elsewhere, a model's wear is 3140 at 2, `sky.wea`'s slots are 4, 5 and 8, and
the materials an effect's emitters name are **2519 at 8 and 980 at 4** -- the
additive glows and the smoke.

"Additive" is read off the data rather than out of the engine: the naming, the
black diffuse, the absent specular and the population all say it, and
``Ngi32.dll``'s phase table has the `ADD` mode to do it with.  What the
record's own class byte (+4) means is still open; this field is the one a
renderer needs.

The tracks
----------

The second uint16 is **not a layer count**, which an earlier reading took it
for.  It is the number of **animation tracks**, and the engine caps it at 20
with the message "Too many animations for material."  Each track is a flags
word, a key count, and one 6-byte key per keyframe: the entry to show, the
time to show it, and a word that is zero on all 3147 keys.  Playing a track
means walking its keys and interpolating the two entries that bracket the
clock, which is why an entry is a whole material and not just a name.

So the counts read the other way round from the old guess: ``WATER_M`` is one
track of ten keys 200 apart -- ``WATER0.0`` through ``WATER9.0`` -- and
``WATER_BOT`` is **two tracks of one key each**, one naming ``L20.0`` and the
other ``L20M.0``.  860 materials have a single track, 43 have two and two
have eight.

The 43 two-track ones are the ground, and on all 43 the second track's entry
names the first's texture with an ``M`` inserted.  The twin is **not a second
texture layer**: nothing binds the two together, and a caller asking for
track 1 gets one texture exactly as a caller asking for track 0 does.  What
separates them is the lighting: on 37 of the 43 the first entry carries a
white diffuse over a black ambient and the second a black diffuse over a
white ambient, so the base is lit by the scene and **the twin is drawn
unlit** -- which is what a texture flattened towards mid-grey is for.  See
``docs/03-terrain.md``.

The two eight-track materials, ``B_LBL_01`` and ``R_LBL_01``, are the same
mechanism used for variants rather than frames: eight tracks of one key,
naming cells 0 to 7 of one insignia sheet, blue and red.

**Nothing asks for a track.**  The manager exposes two ways to fetch a
material: ``GetMaterialPhase`` at vtable index 5, which takes a track index,
and a sibling at index 3 that takes none.  Only three modules can hold a
manager pointer -- ``World3D.dll``, which makes it, and ``Terrain.dll`` and
``AniMesh.dll``, which import ``LoadMatManager`` -- and between them there is
**no five-argument call through index 5 at all**, while index 3 *is* called,
from ``Terrain.dll`` at ``0x10046917``, with its selectors zero.  So reading
track 0 is not this library guessing a default: the engine's own fetch has no
track parameter to pass.  See ``analysis/README.md`` for how that negative was
controlled.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from .nres import NResArchive

MATERIAL_TAG = "MAT0"

#: Two counts, the class byte, an unused byte, a float and a dword.  The last
#: four are version-gated and every shipped record declares version 6, so the
#: header is this long on all 905.
HEADER_SIZE = 14

#: The archive directory's ``link_count`` is the record version, and 6 is what
#: every shipped material carries.
RECORD_VERSION = 6

#: One entry: a ``D3DMATERIAL7`` written as bytes, then the sub-image and the
#: name.  4 x (rgb + alpha) + power + cell + 16 = 34.
ENTRY_STRIDE = 34

AMBIENT_OFFSET = 0
DIFFUSE_OFFSET = 4
SPECULAR_OFFSET = 8
EMISSIVE_OFFSET = 12
#: Each colour's alpha sits behind its three bytes.
ALPHA_STEP = 3
POWER_OFFSET = 16
CELL_OFFSET = 17
NAME_OFFSET = 18
NAME_FIELD = ENTRY_STRIDE - NAME_OFFSET

#: The four alphas are per cent -- the engine multiplies each by 0.01.
ALPHA_FULL = 100

#: A cell of -1 asks for the whole texture rather than one of its pages.
WHOLE_TEXTURE = -1

#: What the parser writes into the two version-gated bytes when the record is
#: too old to carry them, and so the engine's own "not set".
UNSET = 0xFF

#: "Too many animations for material." -- the engine's own limit.
MAX_TRACKS = 20

#: The archive directory's *first* count field is a flags byte, and it is what
#: the loader branches on: bit 1 goes into one field of the loaded material,
#: bits 2 to 5 into another, bit 0 into a local flag one record sets and bit 6
#: into one none does.  It takes five values across the library and they sort
#: it by **how the material draws**; see the module docstring.
BLEND_OPAQUE = 0
BLEND_LIT = 2
BLEND_ALPHA = 4
BLEND_ADD = 8
#: Bit 0, set on ``ENV_STARS`` alone.
BLEND_BIT0 = 1

#: A track's header: a flags word then a key count.
TRACK_HEADER = 6
#: One keyframe: the entry to show, when to show it, and a word that is zero
#: on every one of the 3147 shipped keys.
KEY_STRIDE = 6


@dataclass(frozen=True)
class Key:
    """One keyframe of a track."""

    entry: int
    time: int
    unread: int = 0


@dataclass(frozen=True)
class Track:
    """One animation of a material, over its entries."""

    #: The low three bits of the track's word.  0 on 821 of the 918 tracks and
    #: on every track of every multi-track material, so it distinguishes
    #: playback rather than role.
    kind: int
    #: The rest of that word.  0 on 848.
    param: int
    keys: list[Key] = field(default_factory=list)


@dataclass(frozen=True)
class MaterialEntry:
    """One entry of a material: a ``D3DMATERIAL7`` and the texture it wears."""

    texture: str
    #: Sub-image of that texture, or ``WHOLE_TEXTURE``.
    cell: int = WHOLE_TEXTURE
    #: Diffuse colour, as ``(r, g, b)``.  What the scene light multiplies.
    colour: tuple[int, int, int] = (255, 255, 255)
    #: Ambient colour: what the surface shows when nothing lights it.
    ambient: tuple[int, int, int] = (0, 0, 0)
    specular: tuple[int, int, int] = (0, 0, 0)
    emissive: tuple[int, int, int] = (0, 0, 0)
    #: The four alphas in 0..1.  Only the ambient one ever varies.
    ambient_alpha: float = 1.0
    diffuse_alpha: float = 0.0
    specular_alpha: float = 0.0
    emissive_alpha: float = 0.0
    #: Specular power: 0, 3, 4, 5 or 6.
    power: int = 0

    @property
    def lit(self) -> bool:
        """Whether the scene light reaches this entry.

        An entry with a black diffuse and a white ambient shows the texture at
        full brightness whatever the lighting; that is what the ground's ``M``
        twins carry.
        """
        return self.colour != (0, 0, 0)


@dataclass
class Material:
    name: str
    entry_count: int
    #: Animation tracks over the entries, **not** texture layers.
    track_count: int
    #: The archive directory's flags byte: how the material draws.
    blend: int = BLEND_OPAQUE
    #: Every entry in file order.
    entries: list[MaterialEntry] = field(default_factory=list)
    #: The tracks themselves.
    tracks: list[Track] = field(default_factory=list)

    @property
    def additive(self) -> bool:
        """Whether the material adds its colour rather than covering with it."""
        return self.blend & BLEND_ADD != 0

    @property
    def blended(self) -> bool:
        """Whether it draws see-through at all."""
        return self.blend & (BLEND_LIT | BLEND_ALPHA | BLEND_ADD) != 0

    @property
    def colour(self) -> tuple[int, int, int]:
        """Diffuse colour of the first entry; white when there is none."""
        return self.entries[0].colour if self.entries else (255, 255, 255)

    @property
    def cell(self) -> int:
        """Sub-image the first entry asks for."""
        return self.entries[0].cell if self.entries else WHOLE_TEXTURE

    @property
    def textures(self) -> list[str]:
        """The entries' texture names, skipping the entries that name none."""
        return [e.texture for e in self.entries if e.texture]

    @property
    def whole_texture(self) -> bool:
        return self.cell == WHOLE_TEXTURE

    @property
    def texture(self) -> str | None:
        return self.textures[0] if self.textures else None

    def variant(self, track: int = 0) -> MaterialEntry | None:
        """The entry a track's first key names.

        Materials with more than one track use the extra ones as variants of
        the same surface rather than as frames: the ground's ``M`` twin is
        track 1, and an insignia's team colour is one track per team.
        """
        if not 0 <= track < len(self.tracks) or not self.tracks[track].keys:
            return None
        at = self.tracks[track].keys[0].entry
        return self.entries[at] if at < len(self.entries) else None

    @property
    def frames(self) -> list[str]:
        """The first track's texture per keyframe, in order.

        This is the animation: ``WATER_M`` gives its ten ripple frames and a
        still material gives its one.  The other tracks are variants, not
        later frames, so they are not in here.
        """
        if not self.tracks:
            return self.textures[:1]
        out = []
        for key in self.tracks[0].keys:
            if key.entry < len(self.entries):
                texture = self.entries[key.entry].texture
                if texture:
                    out.append(texture)
        return out or self.textures[:1]

    @property
    def frame_count(self) -> int:
        return len(self.frames)


def _colour(blob: bytes, at: int) -> tuple[int, int, int]:
    return (blob[at], blob[at + 1], blob[at + 2])


def parse_entries(data: bytes, count: int) -> list[MaterialEntry]:
    """The ``count`` entries of a MAT0 record."""
    out = []
    for i in range(count):
        at = HEADER_SIZE + i * ENTRY_STRIDE
        blk = data[at : at + ENTRY_STRIDE]
        if len(blk) < ENTRY_STRIDE:
            break
        out.append(
            MaterialEntry(
                texture=blk[NAME_OFFSET:].split(b"\0")[0].decode("latin-1"),
                cell=struct.unpack_from("<b", blk, CELL_OFFSET)[0],
                colour=_colour(blk, DIFFUSE_OFFSET),
                ambient=_colour(blk, AMBIENT_OFFSET),
                specular=_colour(blk, SPECULAR_OFFSET),
                emissive=_colour(blk, EMISSIVE_OFFSET),
                ambient_alpha=blk[AMBIENT_OFFSET + ALPHA_STEP] / ALPHA_FULL,
                diffuse_alpha=blk[DIFFUSE_OFFSET + ALPHA_STEP] / ALPHA_FULL,
                specular_alpha=blk[SPECULAR_OFFSET + ALPHA_STEP] / ALPHA_FULL,
                emissive_alpha=blk[EMISSIVE_OFFSET + ALPHA_STEP] / ALPHA_FULL,
                power=blk[POWER_OFFSET],
            )
        )
    return out


def parse_tracks(data: bytes, at: int, count: int) -> tuple[list[Track], int]:
    """The animation tracks that follow a record's entries.

    Returns the tracks and the offset just past them, which is the end of the
    record on all 905.
    """
    out = []
    for _ in range(count):
        if at + TRACK_HEADER > len(data):
            break
        word, keys = struct.unpack_from("<IH", data, at)
        at += TRACK_HEADER
        track = Track(word & 7, word >> 3, [])
        for _ in range(keys):
            if at + KEY_STRIDE > len(data):
                break
            track.keys.append(Key(*struct.unpack_from("<3H", data, at)))
            at += KEY_STRIDE
        out.append(track)
    return out, at


def parse(name: str, data: bytes, blend: int = BLEND_OPAQUE) -> Material:
    """Parse one MAT0 record.

    ``blend`` is the archive directory entry's first count field, which the
    record itself does not carry.
    """
    count, tracks = struct.unpack_from("<2H", data, 0)
    entries = parse_entries(data, count)
    return Material(
        name,
        count,
        tracks,
        blend,
        entries,
        parse_tracks(data, HEADER_SIZE + count * ENTRY_STRIDE, tracks)[0],
    )


class MaterialLibrary:
    """``Material.lib``, keyed by material name (case-insensitively)."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.materials: dict[str, Material] = {}
        for entry in self.archive:
            if entry.tag != MATERIAL_TAG:
                continue
            self.materials[entry.name.upper()] = parse(
                entry.name, self.archive.read(entry), entry.element_count
            )

    def get(self, name: str) -> Material | None:
        return self.materials.get(name.upper())

    def texture_for(self, name: str) -> str | None:
        """Base texture name for a material, or None if it has none."""
        material = self.get(name)
        return material.texture if material else None

    def colour_for(self, name: str) -> tuple[int, int, int]:
        """Diffuse colour of a material; white when it has none."""
        material = self.get(name)
        return material.colour if material else (255, 255, 255)

    def frames_for(self, name: str) -> list[str]:
        """Animation frames for a material, or an empty list if unknown."""
        material = self.get(name)
        return material.frames if material else []

    def __len__(self) -> int:
        return len(self.materials)
