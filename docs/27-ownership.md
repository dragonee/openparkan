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
| `0x8` | a mine's loading place ("Mined") | `0x10019482` |
| `0x10` | a storage's unloading place ("Stored") | `0x100195b8` |
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
| hangar | 1 | — | 1 (`0x10000620`) |
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
named a repair depot: the hangar is the one building left with a ground-level
dock, so that it is the depot is a *guess* by elimination. Every dock repairs,
not only the hangar's.

## What a dock gives — *read*

For each unit in a dock that belongs to the building's clan or an ally
(`0x10019318`), every building tick adds, per second of game time:

- **10% of a full battery** (`0x10019372`, the device manager's value 1);
- **10% of full life** and 10% of the device manager's value 7 (`0x10018100`);
- to **each gun**, 10% of its magazine rounded down but at least one round,
  and 10% of its capacitor (`0x100181e0`).

So a unit sitting in a dock is full in ten seconds, whatever it is.

## What sends a bot to a dock — *read*

The orders `ORDER_ROBOT_RELOAD` (8) and `ORDER_ROBOT_REPARE` (9) of
`varset.var` make the same task, `M_Task_Reload` (vtable `0x10059be0`, built by
`MTaskStack::CreateTaskFromOrder`). It walks to a place with any bit of
`0x620` (`0x1002ea87`), waits there until life, charge and ammunition are all
at 98% (`0x1002ed3b`), and leaves.

**It only picks a ground-level dock.** `MakeInsideDest` (`0x10001270`) asks for
the ground-level bit on every place except the pod (`0x10001357`). So a bot
sent to reload goes to a generator's two outdoor docks, a hangar's, or a
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

**The capturer walks to the pod** (`0x1003094f`) and the task ends when the
building's clan is its own: "Building [..] captured" (`0x10030474`).

**Taking the building** is done in `iron3d.dll`, by a callback it registers on
every building (`0x10032e99`, entry `0x10061050`): if the capturer's clan is
not the building's, it calls the building's `MBehaviour::Capture` with the
capturer's clan (`0x10061145`) and broadcasts the change through the queue
(`IQueue::ChangeOwner`, `World3D.dll:0x10004f50`, message
`GMSG_CHANGE_OBJECT_OWNER` `0x80000007`, whose handler calls `Capture` on the
other machines, `0x1000672b`).

**What changes** (`MBehaviour::Capture`, `Behavior.dll:0x10008e40`): the
building's task list is dropped, it leaves its old clan's power distributor,
takes the new clan, and joins the new clan's distributor and SuperAI. The
clan's minds do not come into it — buildings hold none.

**What the player hears** (`iron3d.dll:0x100a48a0`): `VOICE_NBUILD_CAPTURE`
when the player takes a building from a neutral or an ally,
`VOICE_EBUILD_CAPTURE` from an enemy, and `VOICE_BUILD_CAPTURE` when the
player loses one. The cursor over a building a selected capturer can take is
`CAPTURE`, `ui/capture.ani` (`ui/cursor.cfg`, *measured*).

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
come first and enemies second. `ArealMap.dll` keeps a type per clan
(`SystemArealMap::GetClanType`, `0x10020a10`, and `SetClanType`), and a clan it
does not know is type 3 — that this is the word the file carries is a *guess*
the names above make a strong one; the load path between them was not traced.
The code treats 3 as neutral: `iron3d.dll` tests for it (`0x100394a0`) to pick
the voice above, and a neutral or nature clan's bots never order themselves to
a dock (`Behavior.dll:0x10017dbd`). A radar module drops contacts of both kinds
([25-sensors.md](25-sensors.md#what-the-ai-does-with-it--read)), so the AI does
not pick animals or neutrals as targets on its own.

**Neutrals own things** (*measured*): 22 units and 28 buildings across the
shipped missions — factories, a hangar, a bunker and a generator in the first campaign's
tutorials, bridges, and on four multiplayer maps a research centre, a mine or
generators between the players. `Multi.05`'s `Ntrl` is the exception: type 2.

## Not established

- **What fires the capture callback, and when**: how long a capturer must
  stand in the pod, and whether defenders inside or the building's damage
  stop it. The callback sits on the building's interface `0x17`; nothing in
  `Behavior.dll`, `Terrain.dll`, `AniMesh.dll`, `ArealMap.dll`, `Control.dll`,
  `World3D.dll` or `Effect.dll` answers that interface by a compare or a jump
  table found so far.
- **How a neutral bot is captured.** `MBehaviour::Capture` accepts a unit (it
  only changes its clan), and a captured unit takes a mind
  ([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured));
  but no path from a pod, a touch or a hero action to it was found. The log
  string "Cannot capture warbot" is referenced by nothing.
- Whether a unit must be standing still to count as in a place: the
  occupancy test (`0x10018310`) compares a vector of the unit's to 2.0, and
  that it is the velocity is a *guess*.
- Why `behpsp.res` gives `Task_Charge` to the generator's profile and to no
  other building's, when factories, hangars, bunkers and towers have docks.
- The mission property `ChargeRadius` is 10000 and locked on all 463 placed
  buildings and units; nothing found reads it for docking.
