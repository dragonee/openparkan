<!-- Phase R research note R6: Firing, from button to round. Where this note and docs/ disagree, docs/ wins: docs/29-weapons.md#firing-from-button-to-round--read-and-measured -->
# R6 → engine: firing the hero's guns (Mission 01)

Tags: **F** means faithful (read or measured; the sources are in doc.md and
known.toml). **S** means stand-in (unknown, we choose); mark it in code with
`// STAND-IN (R6)`. Axes are model space, y forward and z up.

## 0. Data

| what | where | hero values |
|---|---|---|
| guns | `turrets.rlb/o_tur_ht_02.ctl`, class-2 components in order | 1 cannon `bb_h_01`, 2 plasma `bp_h_01`, 3 laser `bl_h_01`, 4 missile `bm_h_01` |
| gun values | component values 0–3 | mag 500/150/−1/4; capacitor 20/38/200/0.8; energy/shot 0.1/1.9/5.5/0.2; interval ms 0/250/200/1250 |
| barrels | the gun's entries → section-2 channels (node +0, frames +4/+8, point +0x14, rate +0x18, flags +0x20) | ch3 `Mgun_d` r4; ch8 `Plaz_d` r2; ch22 `Laz_d` r4; ch26 `Roc_d1` r1 and ch27 `Roc_d2` r2.5 (flag 4: timed, not posed) |
| arms | class-24 components 8–11, one per gun, frames 42–48 | cosmetic |
| sight points | yaw channel +0x10, pitch channel +0x14 → `.cpt` | `TurretCenter` (node 3), `TargetDirect` (node 34, +y) |
| rounds | `weapon.rlb/<round>.ctl`: top speed = triple 3 y, range = +108, mode +104, frame flags +116, block entries 2/3/4 | speeds 350/150/10000/70; ranges 500/150/1000/350 |
| round explosions | `.ndp` node 0 `.exp` (hit), action 27's `.exp` (range) | bb/bp/bl kind 2, 1.0; bm hit kind 3 170 r7, range `bm_h_01r.exp` kind 3 200 r10 |

## 1. Input → guns (F)

```
selected[gun] initial:
    round.frame_flags & 4  -> true      (cannon, laser)
    else if unit is hero   -> false     (plasma, missile)
key '1'..'4' press: toggle selected[n]; on select: gun.reset()  (on deselect: gun.state = 0)
key '0' press:      selected[*] = true; gun.reset() for all
LMB press:   for g in guns if selected[g]: g.state = CONTINUE (0x100)
LMB release: for g in guns if selected[g]: g.state = OFF (0)
```

`gun.reset()` (state `0x1000`, `0x1002a100`) clears the gun's internal status and delay; it leaves the state word alone. Arm
animations on select: **S**, play frames 42→48 on select and 48→42 on
deselect, or skip them in Phase One.

## 2. The gun's schedule (F)

Each gun keeps `start_ms` and `next_ms` (the component's +0xc and +0x10) and a
`started` flag. Per sim tick (`Control.dll:0x1002d260`), which can run several
events in one tick:

```
while now_ms > gun.next_ms or not gun.started:
    if gun.next_ms == gun.start_ms or not gun.started:
        gun.next_ms = now_ms              # idle: the event is now
    gun.start_ms = gun.next_ms
    fire_step(gun, gun.start_ms)          # may push gun.next_ms later
    gun.started = true
```

```
fire_step(gun, t):
  b = gun.barrels[gun.cur]
  if b.step != 0: advance(gun, b, t); return          # a stroke always finishes
  if gun.rounds == 0: return
  if gun.capacitor_full > 0 and gun.charge < gun.shot_energy: return
  if gun.state == 0: return
  b.step = 4; advance(gun, b, t)

advance(gun, b, t):
  b.step -= 1
  match b.step:
    3: b.channel_to(0.5);  gun.next_ms = t + 1000*|0.5-b.value|/b.rate
    2: spawn_round(gun, b, t); gun.next_ms = t        # value 4 = 0
    1: b.channel_to(1.0);  gun.next_ms = t + 1000*|1.0-0.5|/b.rate
    0: b.value = 0; if gun.magazine != -1 and gun.rounds > 0: gun.rounds -= 1
       gun.charge -= gun.shot_energy                    # if capacitor_full > 0 and charge >= energy
       gun.next_ms = t + gun.interval_ms
       gun.cur = next barrel (round-robin, skip channels with flag 0x40)
       if gun.state == SINGLE: gun.state = 0
```

For a salvo gun (record +8 & 0x2000000), advance every barrel together. The
hero has none.

The barrel node's pose is `frame = first + value·(last − first)`, with value
linear between `channel_to` calls over the step (R4). Resulting rates: cannon
4/s, plasma 1.33/s, laser 2.22/s, missiles 0.51/s (2250 then 1650 ms).

The capacitor recharge is docs/23's energy draw. **S** for Phase One: refill
at the battery rate.

## 3. spawn_round (F unless marked)

```
p, d = world_point(b.point)                # the .cpt point on its node, composed through the pose
d = normalize(d)
if gun.on_turret and round.mode == 0:      # all hero rounds
    o = world_point(turret.TurretCenter).pos
    s = normalize(world_point(turret.TargetDirect).dir)
    hit = world.segment_first_hit(o + 5*s, o + 1e6*s, filter=everything_solid)   # S: which geometry
    if hit:
        P = hit.point
        if |P - o| < 100: P = o + 100*s
        d = normalize(P - p)
round = spawn(kind=9, pos=p, forward=d, up=(0,0,1))
round.owner = hero.id                      # the hit test skips it
round.level_ratio = gun.level_ratio        # 1.0
round.velocity_world = d * round.top_speed + hero.velocity_world
round.free_flight = true                   # no approach to the command; speed held
round.seeker_target = gun.target           # S: hero target source unknown; None -> flies straight
```

- **S, the ray geometry:** test the terrain (R1 §5) and object meshes (R1 §4)
  with the round filter, and skip the hero itself.
- **S, the hero target:** Phase One leaves `gun.target = None`, so the plasma
  and the missile fly straight. Optionally, lock the object under the
  crosshair when fire is pressed.

## 4. The round in flight (F)

- Move `pos += velocity·dt`, with no gravity (mode 0). Sweep and test per R1.
- **Sideways damping:** the round's own-frame velocity x and z each move toward
  0 by `0.003·dt_ms`.
- **Range:** `remaining -= |moved|`; clamp at the range end (R1, docs/29).
- **Guided rounds** with a target turn per docs/29: spin toward the seeker's
  heading, capped by the turn triple, times the speed fraction. The velocity
  is in the round's frame, so it turns with the body. **S:** the steering
  scale `0x100430d4` is runtime; use 1.

## 5. How it ends (F)

| event | group | engine does |
|---|---|---|
| face or bubble hit | `+0x4e4` | stop; stop the tracer effect (19) or start a hit effect (10); **kill**: apply node 0's `.exp` at the point (docs/26), remove |
| map edge | `+0x4e8` | stop, stop the tracer, remove without an explosion |
| range end | `+0x4ec` | stop, stop the tracer, **explode with the named `.exp`** (`<round>_end.exp`, or `bm_h_01r.exp`) at the position, remove |

## 6. Effects and sound

- **F:** each hero round's load group creates its flight effect at spawn
  (`hero_cannon_bullet`, `hero_prifle_bulletA` and `B`, `hero_laser_bullet`,
  and the missile's engine, smoke and gunfire). Attach it to the round's
  points 0–2.
- **S:** the muzzle flash and shot sound (`hero_cannon` on `Mgun_*`,
  `hero_prifle` on `Plaz_*`, `hero_redlaser` on `Laz_*`, and the `*_sfx` at
  `GH_*_sfx`) are created by the turret's load group, but what starts them per
  shot is unknown. Play each once at barrel step 1 (the moment a gun with a
  shot group would run it).

## 7. Faithful vs stand-in

| piece | status |
|---|---|
| fire and select rows, the selected set, initial selection | F |
| event schedule, 4-step barrel stroke, stroke + interval timing | F |
| muzzle from the barrel control point; convergence on the sight ray, ≥ 100 m | F (the ray's geometry S) |
| owner, launch velocity (+ shooter velocity), free flight, side damping | F |
| end groups: kill / remove / range explosion | F |
| hero turret target → missile guidance | S (unknown) |
| muzzle flash and shot sound trigger | S (unknown) |
| gun ready byte from follower channels (`0x10028200`) | S: always ready |
| arm fold/unfold on select | S |
