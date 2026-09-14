# The shipped engine

Useful as a map of what a reimplementation has to cover, and as a guide to
where each behaviour lives if you disassemble.

## Binaries

`iron_3d.exe` is a 13 KB stub; everything is in DLLs. All are 32-bit x86 (PE
machine type `0x14c`).

| Module | Code | Exports | Role suggested by name and imports |
|---|---|---|---|
| `iron3d.dll` | 929 KB | 8 | game core; pulls in every other module |
| `Terrain.dll` | 623 KB | 13 | terrain, building placement, render settings |
| `Behavior.dll` | 357 KB | 3 | the research tree (`ResTree`, the `.trf` archives) |
| `services.dll` | 232 KB | 6 | support services |
| `Control.dll` | 235 KB | 5 | controllers — `IControl`, [read](13-control.md) |
| `ArealMap.dll` | 226 KB | 9 | areal map — navigation / regions (`Land.map`) |
| `ai.dll` | 207 KB | 2 | AI, and the loader for the `.scr` behaviour scripts, `varset.var` and `.fml` |
| `Ngi32.dll` | 195 KB | 145 | **Nikita Graphics Interface** — the platform layer |
| `AniMesh.dll` | 126 KB | 2 | animated meshes |
| `World3D.dll` | 126 KB | 72 | world / scene management |
| `Effect.dll` | 115 KB | 2 | particle and visual effects |
| `Net.dll` | 52 KB | 37 | multiplayer |
| `MisLoad.dll` | 52 KB | 2 | mission loading |
| `Wizard.dll` | 43 KB | 1 | mission/scenario wizard |
| `Joystick.dll` | 15 KB | 6 | joystick input |

**Total: 3.59 MB of x86 code.**

The module split is free architecture documentation — it maps almost
one-to-one onto the subsystems a reimplementation needs.

Note the export counts. `Ngi32` (145) and `World3D` (72) expose flat C APIs;
the rest export 1–13 symbols, which is the signature of a factory function
returning a COM-style vtable. That makes "replace one DLL at a time on
Windows" possible in principle, but only after reversing those vtables.

## Graphics

`Ngi32.dll` imports **`DDRAW`** and **`DSOUND`** only, and exports:

```
niCreate3DRender      niGet3DRender
niGetD3DDriverAmount  niGetD3DDriverCaps
niGetD3DVideoModeList niSelectD3DDriver
```

plus a `DisableD3DCalls` switch and a `Software\Nikita\NgiTool` registry key.
`World3D.dll` imports `DINPUT`. So the stack is DirectDraw + Direct3D
immediate mode + DirectSound + DirectInput — fixed-function DirectX 7 with a
software fallback; the interface is pinned to `IDirect3DDevice7` below.

That is good news for a reimplementation: there are no shaders to reproduce.
`Iron_3D.ini` shows the whole feature set —

```
RENDER_QUALITY=2
REFLECTIONS=1
EMBOSS_BUMP=0
EMBM=1
BITDEPTH=32
```

`EMBM` is environment-mapped bump mapping, a DX6-era feature. Everything here
maps onto modern Metal / Vulkan / WebGPU without difficulty. `REFLECTIONS` and
`EMBM` are what water is drawn with: see [03-terrain.md](03-terrain.md#water-reflects--read-and-measured).

### The device is `IDirect3DDevice7`

`Ngi32.dll` binds DirectDraw by name at run time (`DirectDrawCreate`,
`DirectDrawCreateEx`) and drives everything through one COM-style object
whose vtable is at `0x100315e0`; `niGet3DRender` hands it out and every other
DLL calls it. Which Direct3D interface it holds falls out of the call sites:
counting the vtable offsets it calls through gives `SetRenderState` at index
20, `SetTransform` at 11, `SetViewport` at 13, `DrawPrimitive` at 25,
`SetTexture` at 35, `SetTextureStageState` at 37, `Begin`/`EndStateBlock` at
22/23 and `ApplyStateBlock` at 39 — the **`IDirect3DDevice7`** layout exactly,
with no offset left over. The driver GUID it compares against at `0x10031378`
is `IID_IDirect3DHALDevice`.

The engine declares **six vertex formats**, and their FVF codes and strides
check each other:

| FVF | Meaning | Stride |
|---|---|---|
| `0x1c2` | `XYZ \| DIFFUSE \| SPECULAR \| TEX1` | 28 |
| `0x2c2` | `XYZ \| DIFFUSE \| SPECULAR \| TEX2` | 36 |
| `0x1c4` | `XYZRHW \| DIFFUSE \| SPECULAR \| TEX1` | 32 |
| `0x2c4` | `XYZRHW \| DIFFUSE \| SPECULAR \| TEX2` | 40 |
| `0x112` | `XYZ \| NORMAL \| TEX1` | 32 |
| `0x212` | `XYZ \| NORMAL \| TEX2` | 40 |

Every stride is what its bits imply, to the byte. Three layouts, each with a
one-texture form and its two-texture twin — so multitexturing is not
incidental to the engine, it has its own vertex declaration.

### The render phase table

`Ngi32.dll` holds a table of **20 records of 44 bytes at `0x10036a30`**,
covering **14 render phases** numbered 0 to 13. A record carries the phase it
implements, a capability requirement, a filtering-quality index, a list of
`{stage, state, value}` triples, and a four-entry array saying which of the
call's texture arguments each stage takes. At start-up the engine walks the
table once per phase and keeps the **first record whose requirement the device
meets**, recording its triples into a D3D state block — so a phase that needs
more than the hardware offers is re-expressed over more stages rather than
dropped. `ApplyStateBlock` is then all a draw costs.

Setting a phase is one call, `render->SetPhase(mode, tex0, tex1)`, vtable
index 30. The stage array decides the binding, and **every two-texture phase
reads `[0, 1, …]` — argument 0 to stage 0, argument 1 to stage 1.** The number
of stages it will fill is `min(device stages, 4)`, held in the render object
and forced to 1 by the registry value `Disable MultiTexturing` under
`HKCU\Software\Nikita\NgiTool` (alongside `DisableMipmap`,
`Force 16-bit textures`, `DisableD3DCalls`, `UseFirstCard` and `ForceCpu`).
`Iron_3D.ini`'s `BITDEPTH` and `RENDER_QUALITY` have nothing to do with it —
they live in `iron3d.dll` and never reach a stage.

What the two-texture phases compute:

| Phase | Result |
|---|---|
| 3 | `tex0 · tex1 · diffuse` |
| 4 | `lerp(tex0, tex1, tex1.a) · diffuse` |
| 5 | `lerp(tex0, tex1, diffuse.a) · diffuse` |
| 9 | `tex0 · tex1 · 2 · diffuse` — `MODULATE2X`, whose identity value is 128 |
| 10, 11 | `BUMPENVMAP` / `BUMPENVMAPLUMINANCE`, the `EMBM=1` path |
| 12, 13 | three textures: modulate, bump, then add |

Phases 1, 2, 7 and 8 are the single-texture ones and 0 is untextured. Phase 6
is the odd one: it binds **one** texture to stages 0, 1 and 3 with
`TEXCOORDINDEX` 0, 1 and 0 — two UV sets over one image.

`Terrain.dll`'s `CShade::ConfigureTextureAndAlphaBlendModes` is the consumer.
It asks the renderer which phases exist on this device (`IsPhaseSupported`,
vtable index 31) and caches one mode per drawing role, falling back to phase 1
where the device cannot do better; it warns *"TEXTUREMODE_MODULATE not
supported"* if even phase 1 is missing. Where two textures have to be
combined it either sets phase 3 and puts the second texture in stage 1, **or**
— when phase 3 is missing — builds a second surface and draws a second pass.

A surface, as the renderer sees it, is five fields: two textures, a cell for
each (`-1` for the whole texture), and the phase. `CCamera::DrawMaterialStrided2`
reads exactly those and makes one `SetPhase` call. A `MAT0` entry supplies one
texture and one cell, so a material fills half of one — see
[03-terrain.md](03-terrain.md#the-m-twin-is-the-materials-second-track-drawn-unlit).

These are facts about the binaries rather than about the shipped data, so
`uv run openparkan verify` does not cover them; the addresses above are where
to look. See [09-method.md](09-method.md).

`Iron_3D.ini` also proves the Steam build already runs at modern resolutions
(`DISPLAY_WIDTH=1920`, `DISPLAY_HEIGHT=1080`), so resolution is not among the
reasons to reimplement.

## The engine's own class names

A release build with no symbols still carries its assertion text, and an
assertion names the function it sits in. `analysis/coverage.py --names`
recovers **119 function names over 40 classes** that way, each tied to one
address — and they are the developers' names, not ours. Two spellings give
that away: `CLadnscape` is a typo living alongside `CLandscape`, and
`CBuilding::AlikItem2VladItem` converts between two programmers' idea of what
an item is.

The classes sort the engine cleanly, with the `C` prefix on the world and the
`M` prefix on the simulation:

| module | classes |
|---|---|
| `Terrain.dll` | `CLandscape`, `CTerrain`, `CWorld`, `CBuilding`, `CAtmosphere`, `CAtmData`, `CSun`, `CRain`, `CLightning`, `CCamera`, `CBufferingCamera`, `ICamera`, `CShade`, `CPrimBuffer`, `CStridedPrimitive`, `CLightManager`, `CDynamicPageHeap`, `CSettings` |
| `Behavior.dll` | `MBehaviour`, `MWalker`, `MTaskStack`, `M_Task_Attack`, `M_Task_Construct`, `M_Task_Mine`, `M_Task_Reload`, `M_Task_Research`, `MGraph`, `MWorldGraph`, `MResearchCenter`, `MVarSet` |
| `ArealMap.dll` | `M_ISystemArealMap`, `M_IArealMap`, `MHallWay`, `MLandHexaGris`, `MBrokenAreal` |
| `AniMesh.dll`, `MisLoad.dll`, `Terrain.dll` | `CGameObject` |

Several answer or sharpen questions asked elsewhere in these notes.
`CShade::InitAlphaBlendModeTranslateTable` is the renderer turning a blend
mode into a device state, which is the other end of the `MAT0` flags byte in
[07-objects.md](07-objects.md). `CWorld::PlaceObjectOnWorldFace`,
`FindWorldFace` and `GetWorldFace` are the placement path behind the vertical
datum in [04-missions.md](04-missions.md), and
`CLandscape::CheckMaxBasementAngle` says *basement* is the engine's word for a
building's footing. `CGameObject::SetParent`, `PlaceObject`, `GetPlacement`
and `GetChildren` are the attachment tree. `MGraph::FormPath` and
`MWorldGraph::AddNeighbourToFront` are the pathfinder, and the `M_Task_*`
family is the order vocabulary the `.scr` scripts drive
([15-behaviour.md](15-behaviour.md)).

A name is a place to start, not an answer: the ledger in
`analysis/known.toml` marks these `read = false` until somebody has actually
read the function.

## Data layout

```
Textures.lib   57 MB   393 Texm textures
voices.lib     23 MB   speech
sounds.lib     10 MB   167 RIFF/WAVE effects
MUSIC/         70 MB   9 Ogg Vorbis tracks
fortif.rlb    7.1 MB   fortification objects
static.rlb    3.3 MB   static objects
turrets.rlb   2.9 MB   turrets
lightmap.lib  2.7 MB   lightmaps
bases.rlb     2.7 MB   bases
intsys.rlb    2.1 MB   interface system
DATA/MAPS/     33 map directories
UNITS/         unit and building definitions (*.dat)
MISSIONS/      29 missions
```

Music is already Ogg Vorbis and the game ships `libogg`/`libvorbis` DLLs, so
audio needs no format work at all.
