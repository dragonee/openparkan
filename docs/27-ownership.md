# Docks, control pods and ownership

A building's inside is a path graph — the *hall way*, mesh stream 17 of
`fortif.rlb` ([07-objects.md](07-objects.md#buildings-carry-an-interior-path-graph)).
Its vertices are not only waypoints. The first of the two words after a
vertex's position is a **flag word**, and the flags make some vertices
*places*: somewhere a unit stands to be charged, to load ore, to teleport, or
to take the building for its clan.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## The places — *read*, and *measured*

`Behavior.dll` walks every place of a building on each of its ticks
(`0x10018ac0`), and `ArealMap.dll` finds places by flag
(`MHallWay::GetBestVertexOfType`, `0x1000a7e0`: the first vertex with any bit
of one mask and every bit of another).

| bit | place | read at |
|---|---|---|
| `0x40` | the **control pod** — where a capturer goes | `0x1003094c` |
| `0x20`, `0x200`, `0x400` | a **dock**: charges, repairs and rearms who stands in it | `0x10019251` |
| `0x8` | a mine's loading place ("Mined"): exactly one, ground-level, on each mine model and on no other building ([32-builder.md](32-builder.md)) | `0x10019482` |
| `0x10` | a storage's unloading place ("Stored"): exactly one, ground-level, on each storage model | `0x100195b8` |
| `0x8000`, `0x4000`, `0x10000` | main teleport in, out, and the exit vertex | `0x10018bb1`, `0x1001914f` |
| `0x80`, `0x800` | a factory's creation node | `0x100299ea` |
| `0x10000000` | **ground level**: a place 10 wide and 12 high instead of 5 and 3 | `0x100184f0` |

The engine's own debug view draws the ore places blue, the docks red and the
pod green (`0x10018574`). That is what the game looks like too: a red charging
station, and a green pod beside it.

*Measured*, on the building models in `UNITS/BUILDS` through each one's `.bas`
mesh:

| building | pods | indoor docks | ground-level docks |
|---|---|---|---|
| bunker (small, medium, large) | 1 | 1 (`0x620`) | — |
| tower (both meshes) | 1 | 1 (`0x620`) | — |
| factory (small, medium, large) | 1 | 1 (`0x600`) | 1 (`0x10000400`) |
| generator | 1 | 1 (`0x620`) | 2 (`0x10000620`, `0x10000020`) |
| Outpost (`fr_l_angar`) | 1 | — | 1 (`0x10000620`) |
| research centre, mine, storage, main teleport | 1 | — | — |
| large ruin | — | 1 (`0x620`) | — |
| bridges, the other ruins | — | — | — |

Every one of the 25 pods is indoors; not one carries the ground-level bit. The
large ruin is the odd one out: a dock with no pod, so a charger nobody can
capture. It is the free-standing charging station of *The Last Gate*: the only
building the player's clan starts with there is `b_ruin.dat`, and no other
mission places one.

Against what the game looked like: docks in factories, a repair depot, power
stations, bunkers and towers, small ones indoors and large ones outside. The
data agrees on all but two points. Only one generator model ships
(`fr_l_gener`; the `L Power Maste` has no hall way at all). And no building is
named a repair depot in the data: that building is the **Outpost** —
*posterunek* in Polish — the one building left with a ground-level dock. Its
files call it a hangar (`fr_l_angar`, "Small Hangar", `BUILDING_HANGAR`), and
the command menu's "Build Outpost" and "Upgrade Outpost" push that Type
(`iron3d.dll:0x1007baf7`, [31-packages.md](31-packages.md)). Every dock
repairs, not only the Outpost's.

## What a dock gives — *read*

For each unit in a dock that belongs to the building's clan or an ally
(`0x10019318`), every building tick adds, per second of game time:

- **10% of a full battery** (`0x10019372`, the device manager's value 1);
- **10% of full life** and 10% of the device manager's value 7 (`0x10018100`);
- to **each gun**, 10% of its magazine rounded down but at least one round,
  and 10% of its capacitor (`0x100181e0`).

So a unit sitting in a dock is full in ten seconds, whatever it is.

**A unit must stand still to be in a place** (*read*, and *measured*):

- **What is tested.** The occupancy refresh (`0x10018310`) takes each object
  the hall way finds in the place's box. It reads the object's property `0x27`,
  which is its world velocity (`Control.dll:0x1000deee`, `+0x200`). The object
  counts only if that vector's length is at most the place's bound
  (`0x10018492`).
- **The bound** is set with the box (`0x100184f0`): 2 m/s, or 1000 at a place
  with any bit of `0x7c000`, a teleport's (`0x1001851f`).
- **Which places.** Five places carry such a bit: four on the main teleport
  `fr_m_mtp` and one teleport-out place on the large ruin `fr_b_ruin`
  (*measured*). The other 43 pods, docks and ore places count a unit only while
  it stands.
- **So a dock charges nobody driving through it**, and a capturer must stop on
  the pod (*derived*).

**No profile flag gates a dock** (*derived*). `behpsp.res` gives `Task_Charge`
to the generator's profile, `Task_Mine` to the mine's and `Task_Construct` to
the plant's. Each building profile names the one task its building does. None of
the fourteen task flags is read by any code
([31-packages.md](31-packages.md)), and a dock charges by its place flags alone
(`0x10019251`). So the generator's `Task_Charge` has no effect, and a factory,
Outpost, bunker or tower charges without it.

**`ChargeRadius` is a constant** (*read*, and *measured*). The mission property
is kind 6 of `MBehaviour`'s eleven (`0x1000b315`). Its getter returns 10000
whatever is stored (`0x1000b688`), and its setter stores nothing (`0x1000b575`).
All 463 placements hold 10000, locked. No mission could change it, and nothing
docks by it.

## What sends a bot to a dock — *read*

The orders `ORDER_ROBOT_RELOAD` (8) and `ORDER_ROBOT_REPARE` (9) of
`varset.var` make the same task, `M_Task_Reload` (vtable `0x10059be0`, built by
`MTaskStack::CreateTaskFromOrder`). It walks to a place with any bit of
`0x620` (`0x1002ea87`), waits there until life, charge and ammunition are all
at 98% (`0x1002ed3b`), and leaves. **With no dock to go to, it fails at its
start** (`0x1002e800`: "No Where to reX...", or "(for flyeing)" for a flyer
after a second try). The unit is not left waiting
([31-packages.md](31-packages.md)).

**It only picks a ground-level dock.** `MakeInsideDest` (`0x10001270`) asks for
the ground-level bit on every place except the pod (`0x10001357`). So a bot
sent to reload goes to a generator's two outdoor docks, an Outpost's, or a
factory's — never to a bunker's or a tower's, which serve whoever walks in.

**A bot orders itself there** (`0x10017d50`) when any of these holds:

- its life is under half (under 90% for a building) (`0x1001c700`);
- its battery is under half (`0x1001cbe0`);
- more than 80% of its guns are under 20% of their magazine (`0x1001ca40`).

— provided `Behavior.ini`'s `DeterminMode` is 0, as shipped (`0x10003384`), and
its clan is neither neutral nor nature (clan type 3 or 0, below). The self-order
becomes the same `M_Task_Reload` (`0x10034510`, reason 3).

## Capture — *read*

**A capturer is tiny or small.** The capture order (`ORDER_ROBOT_CAPTURE`, 17)
is refused unless the unit's size class is at most 2 (`0x100301a9`) — the
chassis letters `t`, `l` and `h` of [23-economy.md](23-economy.md), so every
hero, all 19 of whose chassis are `h` (*measured*). It is refused on a
building of the capturer's own clan, and on a main teleport (`0x10030252`).
`MakeInsideDest` refuses it again for anything bigger: "TypedSizes
missmached".

**The capturer walks to the pod** (`0x1003094f`): the goal is the hall-way
vertex with bit `0x40`, carried into the world through its node. A capturer that
can fly first lands at the nearest corner of the building's `.bas` outer ring
that stands on a walkable areal. Its flying state still places it on the faces
under it, so over the pod's floor it is the building's child like a walker
([31-packages.md](31-packages.md#the-capture-tick-by-tick--read-measured-and-seen)).
When the building's clan is its own the task logs "Building [..] captured"
(`0x10030474`); what follows depends on the order
([31-packages.md](31-packages.md#the-escape--read)):

- **Capture building**, or a script's capture by logic id, **ends**, and the
  unit — idle, standing in the building — is given an **escape** by its own takt
  (`0x10005408`), which walks it to open ground within 150 and ends there;
- **Search and capture** does not end: it plans the next building at once and
  walks there;
- a small unit sent to **guard** another clan's building captures it, escapes
  from it and then guards it (`0x1002da7a`).

**What fires it is the building's computer** — *read*, and *measured*. The
building object is `Terrain.dll`'s `CBuilding`, and interface `0x17` is its
**`IBuilding`** (`CTerrain::PlaceBuilding` asks for it by that name,
`0x1000e5fc`; the inner `QueryInterface` at `0x10057c20` answers it). Slot 3
of `IBuilding` (`0x1005b250`) is where `iron3d.dll` stores its callback
(`0x10032e99`).

- **The pod is a `CICLS_COMPUTER` part.** When a building starts,
  `CBuilding` walks its controller's items and files class 12 as doors and
  class 13 as computers (`0x100580b0`). *Measured:* the 21 `fortif.rlb`
  controllers with a class-13 part are exactly the 21 buildings whose hall way
  has a pod. Doors are no substitute: the large ruin has four and no pod.
- **Who counts as inside.** The building keeps the position of every object
  standing on it — its children, as each is added and every time it moves —
  and forgets one that leaves, provided the object has a mesh and a life
  system (`0x10059f40`; the same notification opens the doors,
  [24-motion.md](24-motion.md#walking-into-a-building--read-and-measured)).
  The position is the centre of the object's bounding sphere.
  On each tick (`CBuilding::SendMsg`, `0x10057550`) it
  looks for one standing in the **first** computer's zone: within 0.8 of that
  part's bounding radius across the ground, and in height inside the box of
  that part's parent node (`0x10059d80`,
  [below](#the-zones-height-is-the-pod-nodes-parents-box--read-and-measured)).
  The test reads only the position: no clan, size, order or
  speed is checked here.
- **The pod opens, then fires.** With someone in the zone and the pod idle,
  the building switches the computer on, and it starts opening. When the item
  reports done, the building checks that the same object is still in the zone.
  Only then does it call the callback with the building and that object
  (`0x10057a41`). The pod closes when the object leaves, and it can fire again
  5 seconds after it has closed. **How long the opening takes.** The computer is an item, and an item's update
  (`Control.dll:0x10020900`) moves a 0-to-1 progress by 0.45 a step while
  switched on, back by 0.45 while switched off, and at the end clears the state
  word (property `0x600`) the building waits on. A pod's rate is 1 (its record's
  flags are 0 and its factor 1.0, *measured*), so **it opens in three steps**:
  0.45, 0.9, 1. **A step lasts as long as its slowest channel needs**
  (*read*). The time driver (`0x1002d260`) starts the next step when the last
  one ends. Each step sets its end to 1000 × |Δvalue| ÷ (factor × channel rate)
  ms after its start, taking the longest over the item's channels
  (`0x10022120`). A step that moves nothing lasts 100 ms (`0x10020d72`). The
  channels interpolate linearly across the step (`0x10021a30`) and play the
  node's frames. So an item opens fully in **1 ÷ rate seconds**, and the state
  word clears when the third step starts, at **0.9 ÷ rate**. That is when the
  building sees the pod open.

  *Measured*, the first class-13 part of each of the 21 buildings (item factor
  1 and flags 0 on all 95 doors and pods):

  | Buildings | Pod rate | Open | Capture fires |
  |---|---:|---:|---:|
  | the three power plants, `fr_m_bunker` | 0.2 | 5 s | 4.5 s |
  | `fr_b_bunker`, `fr_l_bunker`, `fr_b_inst`, `fr_e_inst`, `fr_l_inst`, `fr_m_inst` | 0.25 | 4 s | 3.6 s |
  | `fr_l_angar` | 0.3 | 3.33 s | 3 s |
  | `fr_m_mtp` | 0.4 | 2.5 s | 2.25 s |
  | the three mines, three stores and `fr_b_tower`, `fr_m_tower` | 0.5 | 2 s | 1.8 s |
  | `fr_l_gener` | 0.7 | 1.43 s | 1.29 s |

  Doors open in 1 to 5 s the same way. Up to 100 ms more pass before the
  first step starts.

  **Computer 0 is the file's first class-13 part** (*read*, and *measured*):

  - `CBuilding` files its items in the item manager's order, appending each
    class-13 item to its computers as it meets it (`Terrain.dll:0x100583a2`).
  - The item manager's order is the control system's component list. The
    loader appends one component per section-4 record, in file order
    (`Control.dll:0x1000905f`, `0x1002d792`).
  - It matters on 8 of the 18 buildings with two class-13 parts: the three
    mines, the three stores and `fr_b_tower`, `fr_m_tower`. Their second part is
    a wrapping channel at rate 0.2, which would capture at 4.5 s. Their pods
    capture at 1.8 s.
  - On the other 10, both parts have the same rate.
- **The callback takes the building** (`iron3d.dll:0x10061050`).
  - **Same clan** (`0x100610c2`): nothing is captured. `0x10062630` runs
    instead, and **it opens the building for the player** (*read*):
    - It acts only if a state the player's interface keeps is not 7
      (`0x1006267e`, through `0x10044190`), and only for the player's own
      unit: the unit's record must be the player's clan, and
      either its `+0xa2` must be set (the player drives it,
      [below](#a-neutral-unit-is-taken-by-the-hero--read-and-measured)) or it
      must be a hero. An AI unit in its own pod does nothing.
    - It then switches the view by the building's Type through `0x10062bc0`:
      - the plant (`0x80000010`) to state 5 with page 5, and the institute
        (`0x80000400`) to state 5 with page 4 (`0x10084d80`);
      - the three bunkers to state 4;
      - the medium and large towers to state 6, unless `0x10033e40` refuses;
      - the generator, mine, storage and Outpost to `0x1007d0a0` and
        `0x100a5660` instead.
    - State 5 with page 5 is the plant's own screen, the factory screen
      (*read*, [36-factory.md](36-factory.md)). What the other states show was
      not read.
    - Before switching, it sends command 740, `CMD_JAMES_WINGMAN_MENU`, through
      the command handler when an interface object's flag is set
      (`0x100626dd`) — which reads as closing the wingman menu when it is up
      (*guess*). By `Type`
      (`0x10062708`): `0x80000010`, the plant, goes to state 5 and page 5;
      `0x80000002`, `0x80000004`, `0x80000008` and `0x80000040` — generator,
      mine, storage and Outpost — go to `0x10062732`; `0x80000020`, which no
      shipped building carries, does nothing (*measured*).
    - **A generator, mine, storage or Outpost opens no screen** (*read*).
      `0x1007d0a0` puts the building into the player's selection, saying
      `VOICE_SELECTED` if it was not selected, and `0x100a5660` makes it the
      interface's current building (game `+0xaec`) and recomputes the view
      state (`0x100a1c30`).
  - **Any other clan, single player** (`0x10061192`): the building's record
    changes owner and calls `MBehaviour::Capture` with the newcomer's clan
    (`0x10032fd0`), then the voice below plays. **There is no check of
    alliance, damage, power or defenders.** An ally's building is taken the
    same way ([below](#capture--read) for the voice it plays).
  - **The taker then has the building opened at once** (*read*). The
    ownership change (`0x100a48a0`) shows string 5039, *"Building is
    captured"*, as a System line (`0x1007eb60`), plays the voice, and always
    ends calling the opening above with the building and its taker
    (`0x100a4e2d`). So the pod's one firing both takes a building and, for the
    player's own unit, opens it: a hero that takes a plant sees the plant's
    screen with the message. The recording of *The Constructor* shows both in
    the same frame, 106.3 s in
    ([24-motion.md](24-motion.md#walking-into-a-building--read-and-measured)).
  - **Network games:** only the machine that owns the building (*guess*, from
    the check against `+0xad4`) does the capture, then broadcasts it:
    `IQueue::ChangeOwner` (`World3D.dll:0x10004f50`) sends message
    `GMSG_CHANGE_OBJECT_OWNER` `0x80000007`, whose handler calls `Capture` on
    the other machines (`0x1000672b`).

So what keeps a large bot from capturing is not the capture code. The capture
order refuses anything bigger than size class 2, and `MakeInsideDest` will not
route one to a pod (above). Whether a large bot driven by the player could reach
a pod is not established.

**What changes** (`MBehaviour::Capture`, `Behavior.dll:0x10008e40`): the
building's task list is dropped, it leaves its old clan's power distributor,
takes the new clan, and joins the new clan's distributor and SuperAI. The
clan's minds do not come into it — buildings hold none.

**What the player hears** (`iron3d.dll:0x100a48a0`, *read*). The routine
first compares the taker's clan with the player's (`0x100a4950`).

- **The player's clan gains the building.** 5039 shows as a System line, and
  the voice goes by the old clan:

  | the old clan | voice |
  |---|---|
  | a neutral type, 3 (`0x100394a0`) | `VOICE_NBUILD_CAPTURE` |
  | its word towards the taker is 1 (`0x10039460`) | `VOICE_NBUILD_CAPTURE` |
  | its word is 0, hostile (`0x10039440`) | `VOICE_EBUILD_CAPTURE` |
  | any other word, 2 allied among them | `VOICE_BUILD_CAPTURE` |

  The taker need not be the hero. An AI unit of the player's clan gets the
  same line and voice: *Teleport*'s helicopter is announced at 175.5 s
  ([31-packages.md](31-packages.md#the-capture-tick-by-tick--read-measured-and-seen)).
- **The player's clan loses the building** (`0x100a4c40`):
  `VOICE_BUILD_CAPTURE` plays, and no System line shows.

These are the words of [25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured),
where 1 is neutral and 2 allied. So an ally's building does not give the
neutral's voice.

The cursor over a building a selected capturer can take is
`CAPTURE`, `ui/capture.ani` (`ui/cursor.cfg`, *measured*).

### The zone's height is the pod node's parent's box — *read*, and *measured*

**Computer 0 keeps two node references** (*read*). Each holds the building and
a node index:

- `+8`, **the first node the pod's item plays**, filed with the item
  (`Terrain.dll:0x100583a2`–`0x100584e8`);
- `+0x48`, **that node's parent**. The filing (`0x100585df`) hands `+8` to the
  building's `IAnimation` (`CBuilding +0x24`), slot 29
  (`AniMesh.dll:0x10005b40`). That slot fills a record about the node whose
  `+0x14` is the node record's `+0x18` (`0x10005bb9`), the parent
  ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
  The filing stores it beside the building (`0x1005860d`–`0x10058662`).

**The zone test** (`0x10059d80`, *read*) asks the building's `IJointMesh`
(`CBuilding +0x38`, interface `0x20`, `0x10055fb3`) about both, in world space
(query 2):

1. **Across the ground.** Slot 3 (`AniMesh.dll:0x1000f3b0`) gives `+8`'s
   sphere: its level-0 slot's sphere through the node's matrix. The object's x
   and y must lie within 0.8 × that radius of the centre (`0x10059de9`).
2. **In height.** Slot 4 (`0x1000f760`) gives the eight corners of `+0x48`'s
   level-0 slot box. The box is the current damage variant's
   (`0x100124d0` with level 0 and variant −1), scaled by the object's scale and
   carried through the node's matrix. Corner 0 is the box's minimum and corner
   7 its maximum (`0x10011850`). The object's z must lie between corner 0's z
   and corner 7's, whichever is higher (`0x10059ec8`–`0x10059f26`).

**The object's position is its bounding-sphere centre** (*read*). The
notification that files a child (`0x10059ff1`) keeps `IMesh2` slot 9's sphere
in world space, the same centre a door measures. A hero's bounding sphere
spans its chassis's and turret's header spheres. On `tut3_p.dat`, as on
`tut2_p.dat`, its radius is 2.01 and its centre 1.62 above the foot of its
chassis box at rest, which is where it stands (*derived*).

**On every building** (*measured*, model space at rest). Take the pod node's
sphere centre across the ground and a hero standing on each floor under it.
Exactly one of the floors puts the hero's centre inside the parent's box, and
no item plays the parent, so the box stays where it rests. On all 21 buildings
with a pod that floor is the pod room's:

| building | pod node | parent | parent's box, z | the floor inside | the pod node's own sphere, z centre and radius |
|---|---:|---:|---|---:|---|
| `fr_l_gener`, the Small Generator (`gener01.dat`) | 4 | 3 | −12.48 … −6.72 | −12.48 | −3.84, 6.39 |
| `fr_l_bunker`, the Small Bunker (`sbunk01.dat`) | 5 | 4 | −13.96 … −3.41 | −12.04 | −9.13, 5.96 |
| `fr_l_store`, the Small Warehouse (`sstore01.dat`) | 11 | 10 | −29.24 … −18.69 | −27.32 | −24.41, 5.53 |
| `fr_b_plant`, the Large Factory (`lplant01.dat`) | 25 | 23 | −14.32 … −3.77 | −12.40 | −9.67, 4.77 |
| `fr_l_angar`, the Outpost (`shang01.dat`) | 2 | 1 | 0 … 36.86 | 0.48 | 4.79, 6.37 |

*Derived:* the pod node's own sphere is not the bound. It happens to hold the
hero on 20 of the 21. It does not on the generator: the pod's item plays the
platform `ic22`, which rests 5.8 m above the floor. Its sphere reaches down to
−10.23, while the hero's centre on the pod room's floor is at −10.86.

## A neutral unit is taken by the hero — *read*, and *measured*

Capturing a bot is not done at a pod. It is the hero's **Enter** —
`CMD_ENTER_STATE`, 730, "Enter warbot/HQ", bound to `SCAN_W_ENTER`
(*measured*) — in `iron3d.dll`'s command handler (`0x10071cd0`, case
`0x10071f08`):

1. **The player must be driving the hero.** The game view must be in state 1
   or 3, and the player's hero record must have its flag `+0xa2` set.
   `iron3d.dll`'s takeover (`0x10074ff0`) sets that flag when it hands a unit
   to the player (`0x100750c9`, with message 7 and 1) and clears it when it
   hands the unit back (`0x10075092`, `0x10075148`)
   ([29-weapons.md](29-weapons.md#who-may-drive-a-units-guns--read)). The
   flag's other writers are the record's constructor and binding, a
   briefing's start and end, and the help screen as it opens and closes
   (`0x1006788a`, `0x1006792a`), not the hero's body
   ([39-boarding.md](39-boarding.md#the-other-writers-of-0xa2--read)).
2. **The target must be a unit within 20.** It is the hero's current target
   ([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)). A
   neutral unit makes itself that target the first time the hero comes within
   its sensor range ("Vacant vehicle detected...").
   Its `Type` must have no bit outside `0x103e000` (`0x10071fad`): transport,
   builder, warrior, HQ or hero. It must be within 20 of the hero across the
   ground (`0x10071fe7`).
3. **If the target's clan is neutral (type 3),** the unit is captured on the
   spot: `MBehaviour::Capture` with the player's clan (`0x1007202a`), a new
   owner on its record, and a place in the player's clan list. If it is a bot
   the hero can board, the hero then enters it: the view goes to state 1 with
   that bot (`0x100720e8`), as boarding one's own bot does
   ([39-boarding.md](39-boarding.md)). Otherwise a message is shown.
   - **A bot the hero can board** (*read*, `0x10071ff8`) is a record whose
     `+0x30` is 4 and which `0x10076d30` does not refuse.
   - That test refuses a missing record, an object already removed (its class
     word is `0xfffe`), a unit with no class-1 turret, and one whose turret's
     node has no life left (property `0x52`, the component's node life).
   - So **a bot whose turret is shot off cannot be boarded**.
4. **If not,** Enter only boards a bot of the player's own clan. An enemy's or
   an ally's bot cannot be taken this way.

For a unit, `Capture` changes only its clan, SuperAI and areal map
(`Behavior.dll:0x10009051`). The mind it then needs is taken the way any
placed or captured bot takes one
([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
Nothing else found calls `Capture` on a unit: the AI clans have no way to take
a neutral bot (a search, not a proof).

*Measured:* the 22 units owned by neutral clans, all in the campaign, are
18 warriors (`0x1008000`), 2 HQs (`0x1010000`), a builder and a transport.
Every one passes the handler's unit test, and none of the neutral clans' 28
buildings does. One of the HQs is Mission 04's `tut4_hq.dat`, on an `R_B_03`
chassis (size class 4) with a class-1 turret, so the Enter that captures it
also boards it. A second Enter aboard then opens its command view
([40-command-mode.md](40-command-mode.md#an-hqs-command-mode-mode-3--read-and-seen))
(*measured*, `check_hq_command_mode`; the boarding *derived*).

## The clan word is a type — *measured*, and *read*

The clan record's word after the base position, which
[04-missions.md](04-missions.md) once called a 1-based index, is **the clan's type**:

| value | clans | named |
|---|---|---|
| 0 | 15 | `Anml`, `Natur`, `Bird1`, `nature2` — the animals |
| 1 | 37 | the players' clans |
| 2 | 30 | the enemies, and `Multi.05`'s `Ntrl` |
| 3 | 19 | `Ntrl`, `neutral`, `Ntr`, `Clan IIIn` — the neutrals |

It is the 1-based position of the clan only on 61 of the 101, because players
come first and enemies second.

**The file's word is the engine's clan type** — *read*. `MisLoad.dll` hands it
out first in a clan's info (`0x10001320`), and `iron3d.dll` stores it at `+0xc`
of its clan record (`0x10038ea0`). `ArealMap.dll` keeps a type per clan too
(`SystemArealMap::GetClanType`, `0x10020a10`, and `SetClanType`), and a clan it
does not know is type 3. The code treats 3 as neutral: `iron3d.dll` tests the
record's field against it (`0x100394a0`) to pick the voice above and to let the
hero take a unit (above), and a neutral or nature clan's bots never order
themselves to a dock (`Behavior.dll:0x10017dbd`). A radar module drops contacts
of both kinds ([25-sensors.md](25-sensors.md#what-the-ai-does-with-it--read)),
so the AI does not pick animals or neutrals as targets on its own.

**Neutrals own things** (*measured*): 22 units and 28 buildings across the
shipped missions — factories, an Outpost, a bunker and a generator in the first campaign's
tutorials, bridges, and on four multiplayer maps a research centre, a mine or
generators between the players. `Multi.05`'s `Ntrl` is the exception: type 2.

## Not established

- What the game view's states 1, 3 and 6 show (`iron3d.dll:0x10062bc0`),
  beyond state 1 being the one boarding a bot enters (state 5 with page 5 is
  the factory screen, [36-factory.md](36-factory.md); ~~state 4~~ is a bunker's
  command view, [40-command-mode.md](40-command-mode.md)); and what
  `0x10033e40` refuses on a tower. ~~What `0x1007d0a0` and `0x100a5660` open for a
  generator, mine, storage or Outpost~~ — **read**: no screen; they select the
  building and make it the interface's current one
  ([Capture](#capture--read)).
- The hero's target field (record `+0x38`, `+4`) and what sets it.
- ~~The other four writers of a unit record's `+0xa2` (`0x1005e7e8`,
  `0x10074dbf`, `0x1007e2ad`, `0x100a2a73`).~~ The briefing's end, the
  record's binding and constructor, and the briefing's start
  ([39-boarding.md](39-boarding.md#the-other-writers-of-0xa2--read)).
