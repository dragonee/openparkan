<!-- Phase R research note R7: Texture alpha, sky, fog and scene colour. Where this note and docs/ disagree, docs/ wins: docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured -->
# R7 engine description: textures, sky, fog, scene colour (Phase One, Mission 01 / Tut_1)

Labels:

- **faithful**: read or measured; the source is in doc.md and known.toml.
- **stand-in**: the behaviour is unknown and we choose it. Mark it in the code
  with `// STAND-IN (R7)` and list it in the README GUESS table.

Mission 01 is `MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01` on map `Tut_1`. Its
`sky.ske` has 21 keyframes and a 900-second day.

## 1. Textures

| source format | GPU format | alpha | status |
|---|---|---|---|
| 8888 | RGBA8 | yes (alpha test/blend per material flags, docs/07) | faithful |
| 4444 | RGBA8 (expand ×17) | yes | faithful |
| 888 | RGBA8, alpha 255 | no | faithful |
| 565 | RGBA8, alpha 255 | no | faithful |
| 0 (palettised) | RGBA8 from the embedded BGRX palette, alpha 255 | no | faithful |

- There is no colour key anywhere, and `COLORKEYENABLE` is never set.
  - If a header's `+0x14` has bit `0x1000000` or `0x2000000`, the texture
    goes to an alpha surface, and a palettised texture's index 0 becomes
    alpha 0 (`0x2000000` also makes a fade palette).
  - No shipped texture sets either bit, so it is fine to assert that and
    decode opaque.
- The alpha channel's meaning (cut-out vs gloss map) stays as docs/02 and
  docs/07 describe.
- Header `+0x14` bit `0x4000000` (81 `8888` textures) has an unknown use, so
  ignore it. **Stand-in.**

## 2. Per-frame sky values

Keep the atmosphere clock as docs/10 describes: keyframe time × day_seconds ÷
86400. Each frame:

```
k0, k1, t = keyframes bracketing now (wrap at the section end)
lerp_c(slot) = per channel: c0 + (c1 - c0) * t      (integer channels, then /255)
lerp_f(slot) = f0 + (f1 - f0) * t

horizon[0..3] = lerp_c(2), lerp_c(3), lerp_c(1), lerp_c(4)     # faithful
ring3[0..3]   = lerp_c(7), lerp_c(10), lerp_c(8), lerp_c(9)    # faithful
ring2[0..3]   = lerp_c(11), lerp_c(14), lerp_c(12), lerp_c(13) # faithful
apex          = lerp_c(15)                                    # faithful
fog_start     = 700 * lerp_f(5)      # 0 on every shipped keyframe   faithful
fog_end       = 700 * lerp_f(6)      # 70..700; Mission 01: 420..700 faithful
scene_colour  = lerp_c(20)           # rgb only                      faithful
sun_light     = lerp_c(19) * lerp_f(float 3)   # STAND-IN use: directional light colour
```

- Compass order of each group: index `k` is the direction `k × 90°` from +y
  towards +x (faithful for the dome).
- Game space has +X east and +Y north, so `k = 0` is north and `k = 1` east.

## 3. Fog colour

```
heading = camera yaw measured from +y towards +x, radians in [-pi, pi)
                                                      # STAND-IN: angle convention
deg = floor((heading + pi) * 180 / pi)
k   = (deg / 90 + 2) mod 4          # integer division, as the engine does   faithful
f   = (deg mod 90) / 90                                                    faithful
fog_colour = lerp(horizon[k], horizon[(k + 1) mod 4], f), alpha = 1        faithful
```

The engine also passes this colour through a shade transform when a flag is
set (`0x10079b1e`). That transform is unknown, so skip it. **Stand-in.**

## 4. Fog on geometry

- **Linear, range-based fog** (faithful). For each fragment:
  `d = |world_pos - eye|`, `fog = clamp((fog_end - d) / (fog_end - fog_start), 0, 1)`,
  `color = mix(fog_colour, color, fog)`.
- **Per-vertex vs per-pixel.** D3D range vertex fog, or the engine's software
  fog (`ForceSWFog` 1), is computed per vertex and interpolated. Per-pixel is
  a **stand-in** that looks the same except on huge triangles. Terrain faces
  are small, so either is fine.
- **Fog colour by blend mode** (faithful): additive materials (flags 8 →
  mode 2) fog toward black (`0,0,0`). Modes 3 and 5 would fog toward white
  and grey; no Mission 01 material uses them, but implement them anyway.
- **Clouds** use their own fog, start 5000 and end 11380.7 (faithful).
- **2D overlays** (HUD, text) have no fog (faithful).

## 5. Lighting: the scene colour

- **Material emissive** (faithful): `emissive_rgb = scene_colour + material_entry_emissive`
  (MAT0 entry emissive; `+0x14..+0x1c` in the loaded material).
- **Ambient term** is 0 (faithful): in `color = texture × (diffuse × light + emissive)`,
  `scene_colour` plays the role of ambient.
- **Diffuse** comes from the material's diffuse. The directional light's
  direction is the sun or moon direction from docs/10: sun (0.5, 0, 0.866),
  moon (0, −0.766, 0.643), whichever is up.
- **Light colour** is `sun_light`. **Stand-in:** how `CSun` feeds a D3D light
  was not traced, and `UseDXLighting` defaults to 0.
- Specular is enabled by default (`SpecularsOn` 1); use the material specular
  and power `1 << +0x44` (faithful for the material values).

## 6. The dome

**Mesh** (faithful):

```
W = 10000; A = pi/4; R = W / (2 * sin(A/2)^2)       # 34142.1
S = 16 (AtmSkyDetail 4 -> 2^4); N = 5 rings
v0 = (0, 0, W)                                        # apex
for j in 0..S, r in 1..=N:
    theta = r / N * A;  phi = j * 2*pi / S
    v = (R sin(theta) sin(phi), R sin(theta) cos(phi), R cos(theta) + W - R)   # rim r=N at z=0, radius 24142
index (j, r) = 1 + j*N + (r-1)
```

**Colours** (faithful). Per vertex, rebuilt when the keyframe values change:

- apex and ring 1: `apex`
- ring 2: `ring2`
- ring 3: `ring3`
- ring 4: `horizon`
- ring 5 (the rim): `fog_colour`, every frame

For rings 2–4, segment `j` lies in quadrant `q = j / (S/4)` at index
`i = j mod (S/4)`. Its colour is `lerp(group[q], group[(q+1)%4], i / (S/4))`.

The engine uses `(i+1)/(S/4)` from a table indexed at `i-1`, and sets each
quadrant's first segment to `group[q]` exactly. Reproduce it like this: segment
`q*S/4` gets `group[q]`, and segment `q*S/4 + m` for `m = 1..S/4-1` gets
`lerp(group[q], group[q+1], m/(S/4))`.

**Placement** (faithful): translate to the camera position, with no rotation.

**Drawing.** The dome is 34142 units in radius and the camera's far plane
(R3: value 1 = 1000) is much nearer, and fog would bury it. How the engine
avoids both is unknown. **Stand-in:** draw the dome first each frame, with
depth test and write off and without fog, using its own projection with
far = 50000 and the same FOV.

**Texture layers:** the nebula, stars and clouds UV layers are docs/10's and
R5's. Use the viewer's layering until they are read. **Stand-in.**

## 7. Render settings (faithful defaults, no shade.cfg)

| setting | value |
|---|---|
| ForceSWFog | 1 |
| LightingOn | 1 |
| SpecularsOn | 1 |
| AtmCloudsOn | 1 |
| AtmStarsOn | 1 |
| AtmSkyDetail | 4 |
| LensFlareOn | 1 |
| ContrLightOn | 1 |
| UseReflections | 1 |
| PortalNearDist | 75 |
| PortalFarDist | 95 |
| UseDXLighting | 0 |

The ini's `RENDER_QUALITY` preset may lower some of these; which preset
index maps to which value was not read. Use the defaults. **Stand-in.**

## Faithful vs stand-in summary

| piece | status |
|---|---|
| texture formats, no colour key | faithful (read + measured) |
| slot → horizon/rings/apex/fog/scene colour | faithful (read + measured) |
| fog linear range, start 0, end 700 × slot 6 | faithful |
| fog colour by heading quadrant lerp | faithful formula; heading convention stand-in |
| per-blend-mode fog colour (additive → black) | faithful |
| scene colour → emissive | faithful |
| sun light colour/direction use | stand-in |
| dome mesh and vertex colours, camera-centred | faithful |
| dome depth/far-plane/fog handling | stand-in |
| per-vertex vs per-pixel fog | stand-in (equivalent) |
