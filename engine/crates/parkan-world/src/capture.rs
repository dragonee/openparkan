//! A small warbot's capture, the world's side: the places a capture search goes to — a
//! building's contour and its pod — the ways into a building and out of it along its hall way,
//! and what the player hears when a building changes owner. See `docs/31-packages.md`, "The
//! capture, tick by tick", and `docs/27-ownership.md`, "Capture".

use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, LINK_FLYER_ONLY, LINK_SHUT, Link, PLACE_POD};
use parkan_formats::mission::KIND_BUILDING;
use parkan_sim::behaviour::{Places, Task};
use parkan_sim::orders::{self, Order};
use parkan_sim::path::{CORNER_CLEARANCE, ESCAPE_SPEED, ESCAPE_SPEED_ANIMAL, LEAVE_SPEED, Refusal, Way};

use crate::construction::placed;
use crate::play::{
    BUILDING_BRIDGE, CLAN_NEUTRAL, Play, RELATION_HOSTILE, RELATION_NEUTRAL, VOICE_BUILD_CAPTURE,
    VOICE_EBUILD_CAPTURE, VOICE_NBUILD_CAPTURE,
};

/// A hall way's exit: flag 1 (docs/24, "The way to the pod").
pub const PLACE_EXIT: u32 = 0x1;
/// The contour while a construction sphere runs: eight points on a circle of the sphere's
/// radius ÷ cos(π/8) + 20 (`0x1000a611`).
pub const OCTAGON_MARGIN: f32 = 20.0;
/// How near its pod a unit walked in holds (the go task's arrival at an object, `0x1002b670`).
pub const POD_ARRIVED: f32 = 1.5;
/// A unit this near a hall-way vertex joins the way there rather than at an exit.
pub const WAY_JOIN: f32 = 5.0;
/// How far over the ground under it a door may stand and still be a way in. A walker reaches
/// a door from the areal under it (docs/24, "What the links cost": a walkable areal and a
/// building's exit over it), and the areal map carries no way up a building's own ramps, so a
/// door on an upper storey is one no walk outside can deliver it to. *Measured*: every exit of
/// every building the shipped campaigns place stands within 15 of the ground under it but the
/// two mines' upper doors, which stand 28 and 29 over it, their ground-level doors 5 and 10.
pub const WAY_DOOR_STEP: f32 = 20.0;
/// How [`Play::round_walls`] looks for a wall: within this of a building's sphere, along a leg in
/// pieces this long, this far over the landscape.
pub const WALL_MARGIN: f32 = 10.0;
pub const WALL_PIECE: f32 = 10.0;
pub const WALL_WAIST: f32 = 2.0;

/// The buildings' hall ways by path, and which building each unit walked into.
#[derive(Clone, Debug, Default)]
pub struct Ways {
    hall_ways: HashMap<String, Rc<HallWay>>,
    /// A unit's target: the building target it was last sent into along the hall way, or was
    /// made in, or was put on a vertex of: the building its walker's place holds a vertex of
    /// (docs/31, "The escape").
    pub entered: HashMap<usize, usize>,
    /// A unit's target, while it escapes: when its escape began, and none for one from a named
    /// building, which has no 20-second check.
    pub leaving: HashMap<usize, Option<f64>>,
}

/// How long an escape with no target walks on a building before it is put on the building's
/// hall way (the float at `Behavior.dll:0x1005960c`, tested at `0x1002c222`).
pub const LEAVE_TOO_LONG_MS: f64 = 20_000.0;

/// `IAnimation` slot 16 for a node (`AniMesh.dll:0x100057f0`): its stage held below its count, 0
/// on a node with a single stage however hurt it is, and on a node no life knows of.
pub fn node_value(life: Option<&parkan_sim::damage::Life>, node: usize) -> usize {
    life.and_then(|l| l.nodes.get(node)).map_or(0, parkan_sim::damage::NodeLife::block)
}

/// `MHallWay` slot 27 (`ArealMap.dll:0x1000be50`): of the vertices on node `joint`, whatever their
/// flags, the one nearest `at` in all three axes. None when no vertex is on it.
pub fn nearest_on_joint(h: &HallWay, points: &[Option<Vec3>], joint: usize, at: Vec3) -> Option<usize> {
    h.vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| v.joint as usize == joint)
        .filter_map(|(i, _)| Some((i, points.get(i).copied().flatten()?.distance(at))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// What a change of owner says to the player (`0x100a48a0`): with the player's clan the taker,
/// string 5039 and a voice by the old clan — a neutral clan's, or one whose word towards the
/// taker is 1, `VOICE_NBUILD_CAPTURE`; 0, `VOICE_EBUILD_CAPTURE`; any other,
/// `VOICE_BUILD_CAPTURE`. With the player's clan the loser, `VOICE_BUILD_CAPTURE` alone. With
/// neither, nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Announcement {
    pub text: bool,
    pub voice: Option<&'static str>,
}

/// The announcement for a building of `old` taken by `taker`, the player's clan being
/// `player`; `old_kind` is the old clan's type and `word` its relation word towards the taker.
pub fn announcement(
    player: i64,
    taker: i64,
    old: Option<i64>,
    old_kind: Option<u32>,
    word: Option<u32>,
) -> Announcement {
    if taker == player {
        let voice = if old_kind.is_none_or(|k| k == CLAN_NEUTRAL) || word == Some(RELATION_NEUTRAL) {
            VOICE_NBUILD_CAPTURE
        } else if word == Some(RELATION_HOSTILE) {
            VOICE_EBUILD_CAPTURE
        } else {
            VOICE_BUILD_CAPTURE
        };
        Announcement { text: true, voice: Some(voice) }
    } else if old == Some(player) {
        Announcement { text: false, voice: Some(VOICE_BUILD_CAPTURE) }
    } else {
        Announcement { text: false, voice: None }
    }
}

/// Whether a unit crosses hall-way link `link`: never one flagged [`LINK_SHUT`], and one flagged
/// [`LINK_FLYER_ONLY`] only if it flies (`Behavior.dll:0x10042c90`, `0x10036934`; docs/24, "The
/// hall-way gates"). *Measured*: the Large Factory's east exit and its upper west exit join its
/// inside only across four such links, so a walker's one way to its pod is the low west door.
pub fn crosses(link: &Link, flyer: bool) -> bool {
    let flags = link.flags();
    flags & LINK_SHUT == 0 && (flyer || flags & LINK_FLYER_ONLY == 0)
}

/// The shortest ways along a hall way's links from `from` to every vertex, over the links a
/// unit that does or does not fly may cross: each vertex's distance and the vertex before it,
/// `positions` giving the vertices in the world.
fn shortest(
    hall_way: &HallWay,
    positions: &[Option<Vec3>],
    from: usize,
    flyer: bool,
) -> (Vec<f32>, Vec<Option<usize>>) {
    let n = hall_way.vertices.len();
    let (mut dist, mut prev, mut done) = (vec![f32::INFINITY; n], vec![None; n], vec![false; n]);
    if from >= n {
        return (dist, prev);
    }
    dist[from] = 0.0;
    let next = |dist: &[f32], done: &[bool]| {
        (0..n).filter(|&i| !done[i] && dist[i].is_finite()).min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
    };
    while let Some(u) = next(&dist, &done) {
        done[u] = true;
        for link in hall_way.links.iter().filter(|l| crosses(l, flyer)) {
            let (a, b) = (link.start as usize, link.end as usize);
            let v = if a == u {
                b
            } else if b == u {
                a
            } else {
                continue;
            };
            let (Some(pu), Some(pv)) =
                (positions.get(u).copied().flatten(), positions.get(v).copied().flatten())
            else {
                continue;
            };
            let d = dist[u] + pu.distance(pv);
            if v < n && d < dist[v] {
                dist[v] = d;
                prev[v] = Some(u);
            }
        }
    }
    (dist, prev)
}

/// The walker search's random draw, 0..1, off the play's own 32-bit xorshift in `seed`.
fn xorshift(seed: &mut u32) -> f32 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *seed = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// The vertices from `to` back along `prev` to the way's start, start first.
fn walk_back(prev: &[Option<usize>], to: usize) -> Vec<usize> {
    let mut out = vec![to];
    while let Some(p) = prev.get(*out.last().expect("one")).copied().flatten() {
        out.push(p);
    }
    out.reverse();
    out
}

impl Play {
    /// Target `t`'s hall way, read once for its path.
    fn hall_way(&mut self, t: usize) -> Option<Rc<HallWay>> {
        let path = self.commander.paths.get(t)?.to_ascii_lowercase();
        if let Some(h) = self.construction.ways.hall_ways.get(&path) {
            return Some(h.clone());
        }
        let parts = self.assembly.parts(KIND_BUILDING, &path);
        let root = parts.iter().find(|p| p.host == -1)?.clone();
        let blob =
            self.assembly.archive(&root.reference.library)?.read_name(&root.reference.member).ok()?.to_vec();
        let h = Rc::new(hallway::parse(&blob, &root.reference.member).unwrap_or_default());
        self.construction.ways.hall_ways.insert(path, h.clone());
        Some(h)
    }

    /// Target `t`'s hall-way vertices in the world, through their nodes' poses.
    pub fn hall_way_points(&mut self, t: usize) -> Option<(Rc<HallWay>, Vec<Option<Vec3>>)> {
        let h = self.hall_way(t)?;
        let part = self.battle.combat.targets.get(t)?.parts.first()?;
        let points = h.vertices.iter().map(|v| crate::factory::vertex_world(v, part)).collect();
        Some((h, points))
    }

    /// Building `t`'s contour in the world (variable `0x203`, `0x1000a611`): its `.bas` outer
    /// ring placed where it stands, or, while its construction sphere runs, eight points on a
    /// circle of the sphere's radius ÷ cos(π/8) + 20 about it. Each point stands on the ground
    /// under it.
    pub fn contour(&mut self, t: usize) -> Vec<Vec3> {
        let on_ground = |play: &Play, x: f32, y: f32| {
            let z = play.ground.below(x, y, 1.0e5).map_or(0.0, |h| h.point.z);
            Vec3::new(x, y, z)
        };
        if let Some(sphere) = self.construction.spheres.iter().find(|s| s.target == t) {
            let (centre, r) =
                (sphere.centre, sphere.radius / (std::f32::consts::PI / 8.0).cos() + OCTAGON_MARGIN);
            return (0..8)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    on_ground(self, centre.x + r * a.cos(), centre.y + r * a.sin())
                })
                .collect();
        }
        let Some(&(at, yaw)) = self.construction.placements.get(&t) else { return Vec::new() };
        let Some(path) = self.commander.paths.get(t).cloned() else { return Vec::new() };
        self.plan(&path)
            .outer
            .iter()
            .map(|p| {
                let [x, y] = placed([p[0], p[1]], at, yaw);
                on_ground(self, x, y)
            })
            .collect()
    }

    /// Whether `at` lies inside building `t`'s outer ring across the ground.
    fn within_contour(&mut self, t: usize, at: Vec3) -> bool {
        let ring = self.contour(t);
        let mut inside = false;
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            if (a.y > at.y) != (b.y > at.y) && at.x < a.x + (at.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                inside = !inside;
            }
        }
        inside
    }

    /// Whether a unit whose origin is at `at` stands inside building `t`: within its outer ring
    /// and under one of its floors. A unit on the roof is within the ring too, with nothing of
    /// the building over it.
    fn inside(&mut self, t: usize, at: Vec3) -> bool {
        self.within_contour(t, at)
            && self.ground.solids.get(t).is_some_and(|s| s.walk_face(at, true).is_some())
    }

    /// Every live building's places for a capture search: whether it is finished, its pod and
    /// the contour vertices a flyer may land at, those on an areal whose first flag word is set
    /// (on a map with no areal map, those over ground above any water).
    pub fn capture_places(&mut self) -> Vec<Places> {
        let buildings: Vec<usize> = (0..self.units.len())
            .filter(|&t| {
                self.units[t].kind == KIND_BUILDING
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
            })
            .collect();
        buildings
            .into_iter()
            .map(|t| {
                let pod = self.hall_way_points(t).and_then(|(h, points)| {
                    h.vertices.iter().position(|v| v.flags & PLACE_POD != 0).and_then(|i| points[i])
                });
                let contour = self
                    .contour(t)
                    .into_iter()
                    .filter(|p| match &self.graph {
                        Some(graph) => graph.usable(p.x, p.y),
                        None => self.ground.water(p.x, p.y, p.z).is_none_or(|w| w <= p.z),
                    })
                    .collect();
                Places { id: self.units[t].logical_id, complete: !self.building_itself(t), pod, contour }
            })
            .collect()
    }

    /// The way into building `t` for a unit at `from` bound for the hall-way vertex nearest
    /// `goal` — its pod, for a capture, or one of its docks, for a refit: the hall way's
    /// shortest way there from the door, or a vertex within 5 of `from`, that makes the whole
    /// way shortest, counting the way to it straight; that vertex first. A walker (`flyer`
    /// false) crosses no link only a flyer may ([`crosses`]).
    ///
    /// A door the walk outside cannot reach is passed over: one standing more than
    /// [`WAY_DOOR_STEP`] over the ground under it is on an upper storey, up the building's own
    /// ramps, which the areal map does not carry, so the unit goes round to a door at ground
    /// level instead. With no such door left it takes the nearest anyway, as it did before.
    ///
    /// STAND-IN: docs/24-motion.md#how-a-walk-joins-a-hall-way--read-and-measured -- this is no
    /// walk of the game's, which has the one search ([`Play::search_in`]). It stands for a unit
    /// already by a vertex, its place on the building not being kept, and where the search gives
    /// no way, as past a vertex's gate: straight to that vertex, then along the links.
    pub fn way_in(&mut self, t: usize, from: Vec3, goal: Vec3, flyer: bool) -> Option<Vec<Vec3>> {
        let (h, points) = self.hall_way_points(t)?;
        let near = |i: &usize| points[*i].map_or(f32::INFINITY, |p| p.distance(goal));
        let end = (0..h.vertices.len()).min_by(|a, b| near(a).total_cmp(&near(b)))?;
        let (dist, prev) = shortest(&h, &points, end, flyer);
        let to = |i: usize| points[i].map_or(f32::INFINITY, |p| p.distance(from));
        let whole = |i: usize| to(i) + dist[i];
        let joins: Vec<usize> = (0..h.vertices.len())
            .filter(|&i| (h.vertices[i].flags & PLACE_EXIT != 0 || to(i) <= WAY_JOIN) && dist[i].is_finite())
            .collect();
        // A vertex the unit already stands by needs no ground: it is there. A door does.
        let reachable = |i: usize| {
            to(i) <= WAY_JOIN
                || points[i].is_some_and(|p| {
                    self.ground
                        .below(p.x, p.y, p.z + WAY_DOOR_STEP)
                        .is_some_and(|g| g.point.z >= p.z - WAY_DOOR_STEP)
                })
        };
        let nearest = |list: &[usize]| list.iter().copied().min_by(|&a, &b| whole(a).total_cmp(&whole(b)));
        let ground_level: Vec<usize> = joins.iter().copied().filter(|&i| reachable(i)).collect();
        let start = nearest(&ground_level).or_else(|| nearest(&joins))?;
        // `prev` leads back to the goal vertex, so the way from the start runs along it.
        let mut way = walk_back(&prev, start);
        way.reverse();
        way.into_iter().map(|i| points[i]).collect()
    }

    /// The way out of building `t` for a unit at `from` bound for `to`: from its nearest
    /// hall-way vertex along the links it may cross ([`crosses`]) to the exit that makes the
    /// way to `to` shortest, counting the rest straight. The hall way is the one path a building
    /// has: the walker's search starts from the vertex its place holds (docs/31, "The escape").
    ///
    /// STAND-IN: docs/31-packages.md#which-paths-a-unit-leaves-a-building-by--read-and-measured
    /// -- the vertex the walker's place holds is not kept along a walk; the one nearest the unit
    /// stands for it.
    pub fn way_out(&mut self, t: usize, from: Vec3, to: Vec3, flyer: bool) -> Option<Vec<Vec3>> {
        let (h, points) = self.hall_way_points(t)?;
        let near = |i: &usize| points[*i].map_or(f32::INFINITY, |p| p.distance(from));
        let start = (0..h.vertices.len()).min_by(|a, b| near(a).total_cmp(&near(b)))?;
        let (dist, prev) = shortest(&h, &points, start, flyer);
        let total =
            |i: usize| dist[i] + points[i].map_or(f32::INFINITY, |p| p.truncate().distance(to.truncate()));
        let exit = (0..h.vertices.len())
            .filter(|&i| h.vertices[i].flags & PLACE_EXIT != 0 && dist[i].is_finite())
            .min_by(|&a, &b| total(a).total_cmp(&total(b)))?;
        walk_back(&prev, exit).into_iter().map(|i| points[i]).collect()
    }

    /// Every live bridge's hall way in the world, with its target, as the walker's search links
    /// it: each vertex that has a point, and the links between them.
    ///
    /// STAND-IN: docs/24-motion.md#the-global-path--read -- the areal map links the exits of
    /// every building's hall way to the areals under them; only a bridge's are linked here,
    /// since a way through any other building's inside is not walked.
    pub fn bridge_ways(&mut self) -> Vec<(usize, Way)> {
        let bridges: Vec<usize> = (0..self.units.len())
            .filter(|&t| {
                self.units[t].kind == KIND_BUILDING
                    && self.units[t].type_word == BUILDING_BRIDGE
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
            })
            .collect();
        bridges
            .into_iter()
            .filter_map(|t| Some((t, self.way_of(t, false)?)))
            .collect()
    }

    /// Building `t`'s hall way in the world as the search links it for a unit that does or does
    /// not fly: each vertex that has a point, the links it may cross, and each vertex's first
    /// link whoever may cross it.
    fn way_of(&mut self, t: usize, flyer: bool) -> Option<Way> {
        let (h, points) = self.hall_way_points(t)?;
        let kept: Vec<usize> = (0..points.len()).filter(|&i| points[i].is_some()).collect();
        let index = |i: u32| kept.iter().position(|&k| k == i as usize);
        let mut first = vec![None; kept.len()];
        for link in &h.links {
            if let (Some(a), Some(b)) = (index(link.start), index(link.end)) {
                first[a] = first[a].or(Some(b));
                first[b] = first[b].or(Some(a));
            }
        }
        Some(Way {
            points: kept.iter().filter_map(|&i| points[i]).collect(),
            flags: kept.iter().map(|&i| h.vertices[i].flags).collect(),
            links: h
                .links
                .iter()
                .filter(|l| crosses(l, flyer))
                .filter_map(|l| Some((index(l.start)?, index(l.end)?)))
                .collect(),
            first,
            size: self.units[t].designation.size_class,
        })
    }

    /// The points robot target `t` walks from `from` to `goal` across the ground: the walker's
    /// global path for a unit that does not fly, straight for a flyer or on a map with no areal
    /// map (docs/24, "The global path"). A goal the walker refuses, or one it finds no way to,
    /// gives none, and the unit holds, its queues emptied as `SetTarget` empties them first and
    /// the walker's next takt stops it (`Behavior.dll:0x1003d31e`). A unit in a hole, or on an
    /// areal no link leaves, walks straight out at the walk's own speed, which [`Play::walk_out`]
    /// names, its way on to the goal left until it is there (docs/24, "Walking out").
    ///
    /// STAND-IN: docs/24-motion.md#the-global-path--read -- the game's `rand()` is not followed:
    /// a 32-bit xorshift of the play's own.
    pub fn route(&mut self, t: usize, from: Vec3, goal: Vec3) -> Vec<Vec3> {
        match self.walker_search(t, from, goal) {
            None => vec![goal],
            Some(Ok(legs)) => legs,
            Some(Err(Refusal::Goal | Refusal::NoWay)) => Vec::new(),
            Some(Err(refusal)) => self.walk_out_of(t, from, refusal),
        }
    }

    /// The walk out of what `refusal` says robot target `t` stands in, the speed set in
    /// [`Play::walk_out`].
    pub(crate) fn walk_out_of(&mut self, t: usize, from: Vec3, refusal: Refusal) -> Vec<Vec3> {
        let animal = self.units[t].type_word & crate::play::CLASS_ANIMAL != 0;
        let Play { graph, walk_seed, walk_out, .. } = self;
        let Some(graph) = graph.as_ref() else { return Vec::new() };
        let (point, speed) = if refusal == Refusal::Obstacle {
            (graph.leave_hole(from), LEAVE_SPEED)
        } else {
            let speed = if animal { ESCAPE_SPEED_ANIMAL } else { ESCAPE_SPEED };
            (graph.escape(from, animal, &mut || xorshift(walk_seed)), speed)
        };
        *walk_out = point.map(|_| speed);
        point.into_iter().collect()
    }

    /// The walker's global search for robot target `t` from `from` to `goal` (`MWorldGraph`,
    /// docs/24, "The global path"): its legs, or why it refuses. `None` where no search runs:
    /// `t` is no robot, it flies, or the map has no areal map.
    pub fn walker_search(&mut self, t: usize, from: Vec3, goal: Vec3) -> Option<Result<Vec<Vec3>, Refusal>> {
        let (_, robot) = self.robots.iter().find(|(rt, _)| *rt == t)?;
        if robot.flyer || self.graph.is_none() {
            return None;
        }
        let standing = robot.walker.ground.and_then(|h| h.solid).map(|s| s.0);
        let size = robot.size_class;
        let ways = self.bridge_ways();
        let on = |b: Option<usize>| b.and_then(|b| ways.iter().position(|(w, _)| *w == b));
        let aboard = on(standing);
        let ways: Vec<Way> = ways.into_iter().map(|(_, w)| w).collect();
        let Play { graph, walk_seed, .. } = self;
        let graph = graph.as_ref().expect("tested above");
        Some(graph.route(from, goal, &ways, aboard, CORNER_CLEARANCE, size, &mut || xorshift(walk_seed)))
    }

    /// The search's way for robot target `t` from `from` into building `b`, to the hall-way
    /// vertex nearest `goal`, over the areals and the hall ways together, each vertex's gate
    /// and link flags as they stand for it: the points up to its approach, then the rest. For a
    /// flyer the first part's points are raised as the walker raises any place of word 0
    /// (docs/24, "How a walk joins a hall way").
    ///
    /// STAND-IN: docs/24-motion.md#how-a-walk-joins-a-hall-way--read-and-measured -- what a
    /// walker does with an exit on an upper storey is not read: for one, an exit standing more
    /// than [`WAY_DOOR_STEP`] over the ground under it is no exit. The areal map carries no way up
    /// a building's own ramps.
    pub fn search_in(&mut self, t: usize, b: usize, from: Vec3, goal: Vec3) -> Option<(Vec<Vec3>, Vec<Vec3>)> {
        let (_, robot) = self.robots.iter().find(|(rt, _)| *rt == t)?;
        let (size, flyer) = (robot.size_class, robot.flyer);
        let standing = robot.walker.ground.and_then(|h| h.solid).map(|s| s.0);
        self.graph.as_ref()?;
        let mut way = self.way_of(b, flyer)?;
        if !flyer {
            for (v, flags) in way.flags.iter_mut().enumerate() {
                let p = way.points[v];
                let level = self
                    .ground
                    .below(p.x, p.y, p.z + WAY_DOOR_STEP)
                    .is_some_and(|g| g.point.z >= p.z - WAY_DOOR_STEP);
                if !level {
                    *flags &= !PLACE_EXIT;
                }
            }
        }
        let bridges = self.bridge_ways();
        let aboard = standing.and_then(|s| bridges.iter().position(|(w, _)| *w == s));
        let mut ways: Vec<Way> = bridges.into_iter().filter(|(w, _)| *w != b).map(|(_, w)| w).collect();
        let at = ways.len();
        let vertex = (0..way.points.len())
            .min_by(|&i, &j| way.points[i].distance(goal).total_cmp(&way.points[j].distance(goal)))?;
        ways.push(way);
        let Play { graph, walk_seed, .. } = self;
        let graph = graph.as_ref()?;
        let legs = graph
            .route_into(from, at, vertex, &ways, aboard, CORNER_CLEARANCE, size, &mut || xorshift(walk_seed))
            .ok()?;
        // The first point on the way itself is the exit, its approach just before it.
        let exit = legs.iter().position(|p| ways[at].points.contains(p))?;
        let (mut outside, inner) = (legs[..exit].to_vec(), legs[exit..].to_vec());
        if flyer {
            for p in &mut outside {
                p.z = self.flight_height(p.x, p.y, false);
            }
        }
        Some((outside, inner))
    }

    /// The building a robot stands on and its part and node: the face the ground contact holds
    /// names them, and the placement hangs the unit there (`Terrain.dll:0x10025fc0`), so they are
    /// its parent and its joint (docs/31, "The node test").
    pub(crate) fn standing_on(&self, r: usize) -> Option<(usize, usize, usize)> {
        let (b, face) = self.robots.get(r)?.1.walker.ground.and_then(|h| h.solid)?;
        if self.units.get(b)?.kind != KIND_BUILDING {
            return None;
        }
        let node = self.ground.solids.get(b)?.nodes.iter().find(|n| n.faces.contains(&face))?;
        Some((b, node.part, node.node))
    }

    /// Robot `r` put on the vertex of building `b`'s hall way nearest it on node `joint` of its
    /// root part (`Behavior.dll:0x1002c41f`, `0x10006c8d`), as the "Teleport to" in its log says:
    /// its translation written, its walk cleared, and the vertex in its walker's place
    /// (`0x1003c3d0`). False where no vertex is on that node, and it stays.
    pub(crate) fn put_on_way(&mut self, r: usize, b: usize, part: usize, joint: usize) -> bool {
        let at = self.robots[r].1.walker.body.position;
        let Some((h, points)) = self.hall_way_points(b) else { return false };
        let vertex = (part == 0).then(|| nearest_on_joint(&h, &points, joint, at)).flatten();
        let Some(point) = vertex.and_then(|v| points[v]) else { return false };
        let (t, robot) = &mut self.robots[r];
        let w = &mut robot.walker;
        w.body.position = point;
        w.from = (point, w.body.yaw);
        w.ground = None;
        robot.wizard.clear();
        self.construction.ways.entered.insert(*t, b);
        true
    }

    /// The takt of an escape with no target (`Behavior.dll:0x1002c216`-`0x1002c5df`): one that has
    /// walked for more than 20 s and still stands on a building is put on the building's hall
    /// way where it can be ("LEAVE IS TOO !!!"), and starts over from where it then is.
    pub(crate) fn escape_too_long(&mut self, r: usize) {
        let t = self.robots[r].0;
        let robot = &self.robots[r].1;
        if !matches!(robot.behaviour.task(), Task::Leave { .. }) {
            self.construction.ways.leaving.remove(&t);
            return;
        }
        let now = robot.time_ms;
        let idle = robot.wizard.idle(now);
        let since = *self.construction.ways.leaving.entry(t).or_insert(Some(now));
        if idle || since.is_none_or(|s| now - s <= LEAVE_TOO_LONG_MS) {
            return;
        }
        let Some((b, part, node)) = self.standing_on(r) else { return };
        self.put_on_way(r, b, part, node);
        self.construction.ways.leaving.insert(t, Some(now));
        let robot = &mut self.robots[r].1;
        robot.wizard.clear();
        if let Some(task) = robot.behaviour.tasks.last_mut() {
            *task = Task::Leave { goal: None };
        }
    }

    /// A unit's behaviour flag `0x10` switched back on, as the player lets go of it or raises its
    /// auto-driver level from 0 (`Behavior.dll:0x10006c8d`-`0x10006f48`): standing on a building
    /// it is put on the building's hall way and given an escape from that building, first; where
    /// no vertex is on its node it is given nothing.
    pub fn escape_on_release(&mut self, t: usize) {
        let Some(r) = self.robots.iter().position(|(rt, _)| *rt == t) else { return };
        let Some((b, part, node)) = self.standing_on(r) else { return };
        if !self.put_on_way(r, b, part, node) {
            return;
        }
        let id = self.units[b].logical_id;
        let leave = Order { code: orders::LEAVE, parameter: 0, target: orders::Target::LogicId(id) };
        self.robots[r].1.behaviour.insert_order(&leave, orders::INSERT_TO_START);
        self.construction.ways.leaving.insert(t, None);
    }

    /// Whether robot target `t` flies, and so may cross a hall-way link only a flyer crosses.
    fn flies(&self, t: usize) -> bool {
        self.robots.iter().find(|(rt, _)| *rt == t).is_some_and(|(_, r)| r.flyer)
    }

    /// The legs of a walk for robot target `t` at `from` to `goal`: out of the building it
    /// walked into first while it still stands inside that building. Once outside, or up on its
    /// roof, the building is forgotten: the hall way runs under the roof, and from on top of it
    /// the way's first vertex is a point the unit cannot get down to.
    pub fn legs_to(&mut self, t: usize, from: Vec3, goal: Vec3) -> Vec<Vec3> {
        let mut legs = Vec::new();
        if let Some(&b) = self.construction.ways.entered.get(&t) {
            if self.battle.combat.targets.get(b).is_some_and(|x| x.alive) && self.inside(b, from) {
                let flyer = self.flies(t);
                legs = self.way_out(b, from, goal, flyer).unwrap_or_default();
            } else {
                self.construction.ways.entered.remove(&t);
            }
        }
        let start = legs.last().copied().unwrap_or(from);
        let way = self.route(t, start, goal);
        if way.is_empty() {
            return Vec::new();
        }
        let way = self.round_walls(t, start, way);
        legs.extend(way);
        legs
    }

    /// Robot target `t`'s walk `legs` from `from`, with the walls taken out of it: for each
    /// building the walk would run into, the points it holds under the building dropped and the
    /// way round its ground contour ([`Play::way_round`]) put in their place.
    ///
    /// STAND-IN: docs/24-motion.md#what-a-wall-does-to-a-walk--measured-and-a-stand-in -- a
    /// building cuts no areal, so the search's way may run straight across one, and no read step
    /// takes a walker round it: the game's plans again once a point due lies 10 from it. Planned
    /// again from a wall the search gives the same way, and a unit in a corner of the walls stands
    /// there at full speed for as long as its task lasts. So a plan made within a building's
    /// sphere and 10 goes round it, and one whose goal lies behind its walls ends at its contour.
    /// A bridge is left alone, its deck being a way, and so is a building the unit was sent into.
    fn round_walls(&mut self, t: usize, from: Vec3, legs: Vec<Vec3>) -> Vec<Vec3> {
        if self.flies(t) || legs.is_empty() {
            return legs;
        }
        let buildings: Vec<usize> = (0..self.units.len())
            .filter(|&b| {
                self.units[b].kind == KIND_BUILDING
                    && self.units[b].type_word != BUILDING_BRIDGE
                    && self.battle.combat.targets.get(b).is_some_and(|x| x.alive)
                    && self.ground.solids.get(b).is_some_and(|s| s.present)
            })
            .collect();
        let mut legs = legs;
        for b in buildings {
            let Some((centre, radius)) = self.ground.solids.get(b).map(|s| (s.centre, s.radius)) else {
                continue;
            };
            // Only a plan made at the building: a walker comes to a wall by the search's way, as
            // read, and it is planning again from there that gets it nowhere. A way that merely
            // lies across a building far ahead is left to the search's next plan.
            let far = from.truncate().distance(centre.truncate()) >= radius + WALL_MARGIN;
            if far || self.construction.ways.entered.get(&t) == Some(&b) {
                continue;
            }
            // A way that meets no wall of it is left as the search gave it.
            let mut at = from;
            let walled = legs.iter().any(|&p| {
                let hit = self.runs_into(b, at, p);
                at = p;
                hit
            });
            if !walled {
                continue;
            }
            let ring = self.contour(b);
            let under = |p: Vec3| {
                let mut inside = false;
                for i in 0..ring.len() {
                    let (a, c) = (ring[i], ring[(i + 1) % ring.len()]);
                    if (a.y > p.y) != (c.y > p.y) && p.x < a.x + (p.y - a.y) * (c.x - a.x) / (c.y - a.y) {
                        inside = !inside;
                    }
                }
                inside
            };
            // A goal behind the walls is one a walk across the ground never reaches: it ends at
            // the contour's corner nearest it, and holds there.
            let ends_inside = legs.last().filter(|&&p| under(p)).copied();
            let kept: Vec<Vec3> = legs.iter().copied().filter(|&p| !under(p)).collect();
            let mut out = Vec::with_capacity(kept.len());
            let mut at = from;
            for p in kept {
                if self.runs_into(b, at, p) {
                    out.extend(self.round_from(b, at, p));
                }
                out.push(p);
                at = p;
            }
            if let Some(goal) = ends_inside {
                out.extend(self.round_from(b, at, goal));
            }
            legs = out;
        }
        legs
    }

    /// The way round building `b` from `at` to `to`, setting out for the nearest corner of its
    /// contour no wall stands in the way of: in a notch of the walls the very nearest may lie
    /// behind one.
    fn round_from(&mut self, b: usize, at: Vec3, to: Vec3) -> Vec<Vec3> {
        let mut ring = self.contour(b);
        let across = |p: Vec3| p.truncate().distance(at.truncate());
        ring.sort_by(|p, q| across(*p).total_cmp(&across(*q)));
        // From where the unit stands, at its own height: it may be up on a building's skirt.
        let open = |play: &Play, c: Vec3| {
            let waist = Vec3::Z * WALL_WAIST;
            play.ground.solids.get(b).is_none_or(|s| {
                !parkan_sim::solid::blocked(at + waist, c.with_z(at.z) + waist, s)
            }) && !play.runs_into(b, at, c)
        };
        let Some(start) = ring.iter().copied().find(|&c| open(self, c)) else {
            return self.way_round(b, at, to);
        };
        let round = self.way_round(b, start, to);
        if round.is_empty() { vec![start] } else { round }
    }

    /// Whether a walker's leg from `a` to `b_at`, along the ground, meets a shut wall of building
    /// `b`: the leg in pieces a walker's waist over the landscape, since a leg's own ends stand at
    /// an areal's corners, which may lie under the ground it walks on.
    fn runs_into(&self, b: usize, a: Vec3, b_at: Vec3) -> bool {
        let Some(solid) = self.ground.solids.get(b) else { return false };
        let pieces = ((b_at - a).truncate().length() / WALL_PIECE).ceil().max(1.0) as usize;
        let on_land = |p: Vec3| {
            let z = self.ground.landscape_top(p.x, p.y).unwrap_or(p.z);
            p.with_z(z + WALL_WAIST)
        };
        (0..pieces).any(|k| {
            let (p, q) = (
                a.lerp(b_at, k as f32 / pieces as f32),
                a.lerp(b_at, (k + 1) as f32 / pieces as f32),
            );
            p.truncate().distance(solid.centre.truncate()) < solid.radius + WALL_MARGIN
                && parkan_sim::solid::blocked(on_land(p), on_land(q), solid)
        })
    }

    /// The way round building `b` on the ground, from the corner of its contour nearest `from`
    /// to the one nearest its door at `door`: the corners between them, the way about with the
    /// fewer corners off walkable ground and then the shorter, the first corner first. Empty
    /// where the unit already stands by the door's corner, or the building has no contour.
    ///
    /// A walk to a door is planned over the areals, which a building does not cut (docs/24,
    /// "What the links cost"), so a leg to a door on the far side runs straight through the
    /// walls. The contour is the building's own ground plan, the ring a capturing flyer lands
    /// on, and its corners stand outside them.
    fn way_round(&mut self, b: usize, from: Vec3, door: Vec3) -> Vec<Vec3> {
        let ring = self.contour(b);
        if ring.len() < 3 {
            return Vec::new();
        }
        let across = |p: Vec3, q: Vec3| p.truncate().distance(q.truncate());
        let nearest =
            |to: Vec3| (0..ring.len()).min_by(|&i, &j| across(ring[i], to).total_cmp(&across(ring[j], to)));
        let (Some(start), Some(end)) = (nearest(from), nearest(door)) else { return Vec::new() };
        if start == end {
            return Vec::new();
        }
        let usable: Vec<bool> = ring
            .iter()
            .map(|p| match &self.graph {
                Some(graph) => graph.usable(p.x, p.y),
                None => true,
            })
            .collect();
        let round = |step: isize| {
            let mut out = vec![start];
            while *out.last().expect("started with one") != end {
                let last = *out.last().expect("started with one");
                out.push((last as isize + step).rem_euclid(ring.len() as isize) as usize);
            }
            out
        };
        let (up, down) = (round(1), round(-1));
        let blocked = |way: &[usize]| way.iter().filter(|&&i| !usable[i]).count();
        let way = if (blocked(&up), up.len()) <= (blocked(&down), down.len()) { up } else { down };
        way.into_iter().map(|i| ring[i]).collect()
    }

    /// The legs of a walk for robot target `t` at `from` into the building of logic id `id`, to
    /// the place at `goal` — its pod or one of its docks: the search's way over the areals and
    /// the hall way together ([`Play::search_in`]), and with none that search will give, the way
    /// round the building to the door its shortest way in starts at, then that way in; or
    /// straight there.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how a walk to a door gets past the
    /// building's own walls, which cut no areal, is not read: the game's walker goes straight
    /// across its areal to the exit's approach, and plans again once a point it has passed lies
    /// more than 10 from it. Here a walker whose way off the areals would run into a wall goes
    /// round the building's ground contour first ([`Play::way_round`]).
    ///
    /// STAND-IN: docs/31-packages.md#each-tick-slot-7-0x10030300--read -- what the walker does
    /// with the pod handed to it again while it stands there is not read: within 1.5 of it, the
    /// go task's arrival at an object, it holds.
    pub fn legs_inside(&mut self, t: usize, from: Vec3, id: i32, goal: Vec3) -> Vec<Vec3> {
        let Some(b) = self.units.iter().position(|u| u.logical_id == id && u.kind == KIND_BUILDING) else {
            return vec![goal];
        };
        self.construction.ways.entered.insert(t, b);
        if from.distance(goal) <= POD_ARRIVED {
            return Vec::new();
        }
        let flyer = self.flies(t);
        let way = self.way_in(b, from, goal, flyer).unwrap_or_else(|| vec![goal]);
        // A unit that joins the hall way where it stands walks it straight: the global path is
        // over the areals, which lie over the building, and it is under them. The vertex it
        // stands by is behind it, so the walk goes on from the next one.
        if from.distance(way[0]) <= WAY_JOIN {
            return if way.len() > 1 { way[1..].to_vec() } else { way };
        }
        // A flyer goes over the walls; a walker goes round them where its way to the door
        // would run into one, and straight at it where the way is clear.
        let blocked = |play: &Play, a: Vec3, b_at: Vec3| {
            !flyer
                && play
                    .ground
                    .solids
                    .get(b)
                    .is_some_and(|s| s.present && parkan_sim::solid::blocked(a, b_at, s))
        };
        let searched = self.search_in(t, b, from, goal);
        if let Some((outside, inner)) = &searched {
            let mut at = from;
            let clear = outside.iter().all(|&p| {
                let open = !blocked(self, at, p);
                at = p;
                open
            });
            if clear {
                return outside.iter().chain(inner).copied().collect();
            }
        }
        // The search's own way in where it found one, its approach gone round to.
        let (door, way) = match searched {
            Some((outside, inner)) => (outside.last().copied().unwrap_or(inner[0]), inner),
            None => (way[0], way),
        };
        let walled = blocked(self, from, door);
        let round = if walled { self.way_round(b, from, door) } else { Vec::new() };
        let (start, rest) = round.split_first().map_or((door, Vec::new()), |(s, r)| (*s, r.to_vec()));
        let mut legs = self.route(t, from, start);
        if legs.is_empty() {
            return Vec::new();
        }
        legs.extend(rest);
        legs.extend(way);
        legs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_vertex_is_asked_of_the_units_own_joint_in_all_three_axes() {
        let v = |x: f32, joint: u32| parkan_formats::hallway::Vertex { position: [x, 0.0, 0.0], flags: 0, joint };
        let h = HallWay { vertices: vec![v(0.0, 1), v(5.0, 2), v(9.0, 2)], links: Vec::new() };
        let points: Vec<Option<Vec3>> =
            vec![Some(Vec3::new(1.0, 0.0, 0.0)), Some(Vec3::new(5.0, 0.0, 9.0)), Some(Vec3::new(9.0, 0.0, 0.0))];
        let at = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(nearest_on_joint(&h, &points, 1, at), Some(0));
        // The nearer across the ground stands 9 up, so the farther one wins; vertex 0 is off the joint.
        assert_eq!(nearest_on_joint(&h, &points, 2, at), Some(2));
        assert_eq!(nearest_on_joint(&h, &points, 3, at), None, "no vertex on that node");
    }

    #[test]
    fn a_nodes_value_is_its_stage_held_below_its_count() {
        use parkan_sim::damage::Life;
        let mut life = Life::new(&[], vec![None, Some(0)], vec![false, false], 1.0, 1.0).staged(&[1, 3], &[]);
        assert_eq!(node_value(Some(&life), 1), 0, "whole");
        life.nodes[1].stage = 2;
        assert_eq!(node_value(Some(&life), 1), 2, "hurt");
        life.nodes[1].stage = 3;
        assert_eq!(node_value(Some(&life), 1), 2, "gone: held below 3");
        life.nodes[0].stage = 1;
        assert_eq!(node_value(Some(&life), 0), 0, "a node of one stage reads whole, even gone");
        assert_eq!(node_value(None, 0), 0, "no life");
        assert_eq!(node_value(Some(&life), u32::MAX as usize), 0, "the place word's no node");
    }
    use crate::play::RELATION_ALLIED;
    use parkan_formats::hallway::{Link, Vertex};

    #[test]
    fn the_player_gaining_a_building_hears_it_by_the_old_clan_and_losing_one_hears_the_voice_alone() {
        let neutral = announcement(0, 0, Some(1), Some(CLAN_NEUTRAL), Some(RELATION_HOSTILE));
        assert_eq!(neutral, Announcement { text: true, voice: Some(VOICE_NBUILD_CAPTURE) });
        let word_1 = announcement(0, 0, Some(1), Some(2), Some(RELATION_NEUTRAL));
        assert_eq!(word_1.voice, Some(VOICE_NBUILD_CAPTURE));
        let enemy = announcement(0, 0, Some(1), Some(2), Some(RELATION_HOSTILE));
        assert_eq!(enemy.voice, Some(VOICE_EBUILD_CAPTURE));
        let ally = announcement(0, 0, Some(1), Some(1), Some(RELATION_ALLIED));
        assert_eq!(ally, Announcement { text: true, voice: Some(VOICE_BUILD_CAPTURE) });
        let lost = announcement(0, 1, Some(0), Some(1), Some(RELATION_HOSTILE));
        assert_eq!(lost, Announcement { text: false, voice: Some(VOICE_BUILD_CAPTURE) });
        assert_eq!(announcement(0, 1, Some(2), Some(2), None), Announcement { text: false, voice: None });
    }

    #[test]
    fn the_shortest_way_follows_the_links_either_way() {
        let v = |x: f32, flags: u32| Vertex { position: [x, 0.0, 0.0], flags, joint: 0 };
        let h = HallWay {
            vertices: vec![v(0.0, PLACE_POD), v(10.0, 0), v(20.0, PLACE_EXIT), v(5.0, PLACE_EXIT)],
            links: [(1, 0), (2, 1), (3, 0)]
                .map(|(start, end)| Link { start, end, tail: [0xffff_ffff; 8] })
                .to_vec(),
        };
        let points: Vec<Option<Vec3>> =
            h.vertices.iter().map(|v| Some(Vec3::from_array(v.position))).collect();
        let (dist, prev) = shortest(&h, &points, 0, false);
        assert_eq!(dist, vec![0.0, 10.0, 20.0, 5.0]);
        assert_eq!(walk_back(&prev, 2), vec![0, 1, 2]);
    }

    #[test]
    fn a_walker_crosses_no_link_only_a_flyer_may_and_nothing_crosses_a_shut_one() {
        // The Large Factory's shape: the pod 0 is reached from exit 2 along ordinary links and
        // from exit 3 only over a link whose tail's word 1 is zero and word 5 is not, which
        // makes it a flyer's (`ArealMap.dll:0x1000a274`, docs/24, "The hall-way gates").
        let v = |x: f32, flags: u32| Vertex { position: [x, 0.0, 0.0], flags, joint: 0 };
        let open = [0xffff_ffff; 8];
        let mut flyers = open;
        flyers[1] = 0;
        let mut shut = flyers;
        shut[5] = 0;
        let h = HallWay {
            vertices: vec![
                v(0.0, PLACE_POD),
                v(10.0, 0),
                v(30.0, PLACE_EXIT),
                v(-5.0, PLACE_EXIT),
                v(-9.0, 0),
            ],
            links: vec![
                Link { start: 1, end: 0, tail: open },
                Link { start: 2, end: 1, tail: open },
                Link { start: 3, end: 0, tail: flyers },
                Link { start: 4, end: 3, tail: shut },
            ],
        };
        assert!(crosses(&h.links[2], true) && !crosses(&h.links[2], false));
        assert!(!crosses(&h.links[3], true) && !crosses(&h.links[3], false));
        let points: Vec<Option<Vec3>> =
            h.vertices.iter().map(|v| Some(Vec3::from_array(v.position))).collect();
        let (walk, _) = shortest(&h, &points, 0, false);
        assert_eq!(walk[2], 30.0, "the walker's exit is the far one");
        assert!(walk[3].is_infinite() && walk[4].is_infinite(), "the flyers' exit is out of its reach");
        let (fly, _) = shortest(&h, &points, 0, true);
        assert_eq!(fly[3], 5.0, "a flyer takes the near exit");
        assert!(fly[4].is_infinite(), "and nobody the shut link");
    }
}
