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
| `Behavior.dll` | 357 KB | 3 | behaviour graph interpreter (`.scr`) |
| `services.dll` | 232 KB | 6 | support services |
| `Control.dll` | 235 KB | 5 | controllers (`.ctl` data) |
| `ArealMap.dll` | 226 KB | 9 | areal map — navigation / regions (`Land.map`) |
| `ai.dll` | 207 KB | 2 | AI |
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
immediate mode + DirectSound + DirectInput — fixed-function DirectX 6/7 with a
software fallback.

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
maps onto modern Metal / Vulkan / WebGPU without difficulty.

`Iron_3D.ini` also proves the Steam build already runs at modern resolutions
(`DISPLAY_WIDTH=1920`, `DISPLAY_HEIGHT=1080`), so resolution is not among the
reasons to reimplement.

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
