<!-- Phase R research note R5: Effects. Where this note and docs/ disagree, docs/ wins: docs/11-effects.md#how-an-effect-runs--read -->

> **Superseded since this note was written:**
>
> - R6 did not find what drives the hero's muzzle effects (time mode 4); it stays a stand-in.
> - Since read (docs/11): time mode 4 is the animation value of the mesh node action 14 names — the
>   barrel nodes for the guns' effects, the arm nodes for the `_sfx` — so nothing starts them per shot;
>   a round's hit on a unit plays the struck batch's material class (`mt` on Mission 01); type 1 is a
>   Direct3D-style light with (start, end) position, direction, colour and range and fixed attenuation
>   terms; a bolt's length is its start point to its position, one sprite per +36; a stream emits every
>   lerp(+24, +28) s; the fade value is start + (end − start)·x^power; bit 8 turns the depth test off
>   while the effect's tested point (header +0x24) is in view; header +0x14 is a settings switch, all
>   on at `RENDER_QUALITY=2`.
# R5 engine description: effects (Phase One, Mission 01)

**Faithful** means read or measured, with its source in doc.md and known.toml.
**Stand-in** means the behaviour is unknown and we choose it; mark it in code with
`// STAND-IN (R5): docs/11#…`.

## Data (all from the install)

| input | where | reader |
|---|---|---|
| effect templates | `effects.rlb` FXID: 60-byte header + typed emitter blocks | `effects` (+ patch: header, windows, phase) |
| explosions | `*.exp` in `weapon.rlb`, `system.rlb`, … (kind, damage, radius, placement, 12 slots) | `effects.parse_explosion` |
| which effects an object runs | `.ctl` section 5 groups + the 21-entry block + state `+0x90` | `control` (`groups`, `Reference.group`, patch: `action`, `args`) |
| surface of a struck face | `Material.lib` class byte of the face's layer-1 material | `materials.Material.surface` |
| sprites' look | material → texture chain, blend byte | `materials`, `texm` |
| sounds | `sounds.lib` WAV names on type-2 emitters | M5 |

## Objects

```
struct Template { header: Header, emitters: Vec<Emitter> }          // one per FXID, loaded once (faithful)
struct Instance {
    key: u32,                  // (part_high << 16) | id; 0x1000_0000 | counter for .exp effects (faithful)
    template: TemplateId,
    owner: ObjectId, attach: AttachPoint,    // control point / node; or World (faithful)
    local: Mat4,               // placement relative to the attach point (faithful: slot 0x28)
    scale: Vec3,               // requested size × header.scale (faithful)
    start_ms: u32, end_ms: u32, mode: TimeMode, manual: f32 /* 0 */, on: bool,
    restart_pending: bool, last_t: f32, value_point: Option<u32>,
}
```

## Clock (faithful)

- Keep effect time in integer milliseconds. `end = start + round(1000 × header.duration)`.
- Update an instance when `now − last_update ≥ 100 ms` or within 50 ms after any command
  to it (start/restart/on/off). **Stand-in:** we render every frame but only advance
  emitter state on those ticks, interpolating sprite phases per frame for smoothness
  (mark it) — or update every frame if the difference is invisible at 60 Hz.

## Effect time t (faithful, docs/11 "Effect time")

```
t = match mode {
  0 => manual,                                  // 0 unless set
  1 => (now-start)/(end-start),  2 => fract(that),  3 => 1-that,
  4 => owner.point_value(value_point),          // STAND-IN until R6 names it: the gun's fire channel 0..1
  5 => |v|/|v_max| (6,7,8 per axis),  9..12 => same for spin,
  13 => 1-owner.attach_value(attach),  14 => 1-owner.property(0x31),   // STAND-IN: 0
  15 => max(mode5, mode9),
  16 => rising-only mode 4 (resets at 0),  17 => falling-only mode 4 (resets at 1),
}
if flags & 0x200 { t *= linear }
if flags & 0x20  { t = if t < 0.5 { 2t } else { 2(1-t) } }
if flags & 0x1   { t += rand(-jitter/2, jitter/2) }           // the engine's generator is not ours: STAND-IN RNG
t = clamp(t, 0, 1)   // division by zero when end == start: treat linear as 1 (duration 0)
```
Beware duration 0 (dust, muzzle effects): mode 1 is then 1 at once.

## Lifecycle (faithful)

| trigger | do |
|---|---|
| section-5 action 3 | create instance: template by name, attach to control point v4, id v7 |
| action 4 | attach to the node control point v4 sits on; position = centroid of points v4, v5, v6; orientation from the three points (STAND-IN: use the part's frame until R6 reads the point vectors) |
| action 5 | world instance at the object's construction sphere, scaled by its radius (buildings only) |
| action 10 | `start = now; end = now + dur; mode = v5` |
| action 11 | `restart_pending = true`; on the next update with `now ≥ end`: `start = now; end = now+dur; mode = header.mode` |
| action 18 / 19 | on / off (off: every emitter sees t = −1 once, then nothing) |
| action 8 | delete |
| action 14 | `value_point = v5` |
| header flag 0x40 | instance starts off |
| header flag 0x2 and t ≥ 1 | delete |
| attach point hidden (owner state bits 5) and flag 0x10 clear | emitters off (STAND-IN: "hidden" = the attach node destroyed) |

Which groups run (faithful):
- block entry 0 at load;
- a state's `+0x90` group on entering it;
- block entries 10 + s when the ground surface id changes to s (≤ 10);
- a round: entry 2 on a face hit, entry 3 at the map edge, entry 4 when its range is spent.

## Explosion effect (faithful, docs/11 "What an explosion plays")

When a node's damage stage plays its `.exp`:
```
surface = contact.face.map(|f| material_class(f)).unwrap_or(if contact.exists { 10 } else { 0xFF })
slot    = if surface <= 10 && exp.slots[surface+1].is_some() { surface+1 } else { 0 }
pos     = node.bounding_sphere.centre
axis    = match exp.placement { 1|2 => node.x, 3 => node.z, 4 => obj.x, 5 => obj.y, 6 => obj.z,
                                7 => contact.face_vector /* STAND-IN: the face normal */, _ => node.y }
scale   = exp.radius × (if object is a round { 1 } else { node.bounding_radius })
spawn Instance { key: 0x1000_0000 | counter, attach: World(pos, axis), scale, mode: header.mode, start: now }
```
- A unit's own material class for a hit on a robot: **unknown** — STAND-IN: class of
  the struck triangle's material (Mission 01 dummies and hero are all class 5, `mt`).
- Rounds: face hit → action 17 kills the round → its node-0 `.exp` plays here with the
  surface of the face it hit; edge → action 15 removes it silently; range end →
  action 27 replaces node 0's `.exp` with `*_end.exp` and kills it (slot 0).

## Emitters

Common, faithful: each emitter is active only while `window.0 ≤ t ≤ window.1`
(`WINDOW_AT`: 1→+8, 2→+8, 3/4/9→+32, 5→+12, 7/10→+20, 8→+16). Local progress
`p = (t − lo)/(hi − lo)`.

| type | faithful | stand-in (mark it) |
|---|---|---|
| 1 light | on inside the window, off outside | point light at the attach point, colour +80..+88, radius +112; ignore +116. Or skip lights in M4 and add in M5. |
| 2 sound | play once when t crosses +8; near/far +64/+68 | kira spatial sound with linear falloff between near and far |
| 3 sprite | phase = a + (b−a)·x^g with a +8, b +12, g +16, x = p (or seconds since start when a < 0) | billboard at attach point + lerp(+40, +52, phase) along the effect's local axes; size = lerp(+100, +112, phase) (per-axis, x→width along the axis, y/z → height); alpha = lerp(+20, +24, phase^+28); material blend from `Material.lib`; flag bit 8 ignored |
| 4 sprite | as 3 | as 3 |
| 5 bolt | window +12..+16; phase from +40 (seconds when negative) | a camera-facing quad stretched along the round's velocity: width +24, length min(+36, speed × age) |
| 7 / 10 particles | window +20..+24; +36 × +40 particles | spawn all at start with position = +56..+64 ± +68..+76/2 and velocity from the (low, high) triples +80.. / +104..; age = (phase − spawn)/+28; size lerp +92→+104 range; alpha fade with age; no drag |
| 8 stream | window +16..+20 on t (dust: only above 20 % speed) | emit at 20/s × t while active; lifetime 1 s; velocity from +88..+96 / +100..+108; size +136..+144 → +148..+156 |
| 9 | as 3 | as 3 |

## Mission 01 content (measured)

- Hero turret `o_tur_ht_02` load group: `hero_cannon` on points (11,12,13) id 0, time from
  point 13; `hero_prifle` (15,16,17) id 1 ← 9; `hero_redlaser` (4,5,6) id 2 ← 22;
  `hero_helm_light` (19) id 3; `hero_breath` (20,22,21) id 4; sounds id 5–8 ← 11, 7, 19, 15.
- Muzzle effects are mode 4: they run 0 → 1 with the gun's point value (R6), scale 0.1.
- Rounds: `bb_h_01` → `hero_cannon_bullet`; `bl_h_01` → `hero_laser_bullet` (type 5 ×2);
  `bp_h_01` → `hero_prifle_bulletA/B`; `bm_h_01` → engine, smoke, launch effects.
- Hits: `bb_h_01.exp` / `bl_h_01.exp` / `bp_h_01.exp` kind 2, `bm_h_01.exp` kind 3; placement 7;
  slots `exp_H{sn,st,gr,sw,ic,mt,gr,wt,al,an,sh}_{bul,las,pls,mis}`; slot 0 `exp_m_bul`,
  `exp_Hmt_las`, `exp_Hsn_pls`, `exp_Hst_mis`.
- On Tut_1 ground: `L02` → class 1 → `st`; `L00` → 2 → `gr`; `WATER` → 7 → `wt`;
  `WATER_BOT` → 1 → `st`.
- Dummies: `r_h_01` nodes explode `explode_aim_S` (kind 1, r 5.5 × node radius) →
  `aim_exp_S`; `r_h_03` `explode_aim_L` (r 15) → `aim_exp_L`; 3 s, flag 0x2.
- Dust: the hero has none (its chassis sets no surface groups).

## Tests (engine)

- `t` for every mode on synthetic owners; ping-pong and ×linear flags; duration 0.
- window gating: `hero_cannon` at t = 0.3 has emitters 0, 1, 2 (sprites and light, window
  0.01–0.5), 6 (stream) and 7 (the sound, fired at 0.01) active; at 0.7 emitters 3, 4, 5, 6.
- `.exp` slot choice: surface 7 → slot 8; 0xFF → slot 0; 10 with an empty slot 11 → slot 0.
- action 11 after action 10 (mode 0) restores the header mode on the next tick past end.
- install test: every Mission 01 effect name above resolves and every window holds.
