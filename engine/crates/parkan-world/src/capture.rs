//! A small warbot's capture, the world's side: the places a capture search goes to — a
//! building's contour and its pod — the ways into a building and out of it along its hall way,
//! and what the player hears when a building changes owner. See `docs/31-packages.md`, "The
//! capture, tick by tick", and `docs/27-ownership.md`, "Capture".

use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, PLACE_POD};
use parkan_formats::mission::KIND_BUILDING;
use parkan_sim::behaviour::Places;
use parkan_sim::path::{Refusal, Way};

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

/// The buildings' hall ways by path, and which building each unit walked into.
#[derive(Clone, Debug, Default)]
pub struct Ways {
    hall_ways: HashMap<String, Rc<HallWay>>,
    /// A unit's target: the building target it was last sent into along the hall way.
    pub entered: HashMap<usize, usize>,
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

/// The shortest ways along a hall way's links from `from` to every vertex: each vertex's
/// distance and the vertex before it, `positions` giving the vertices in the world.
fn shortest(hall_way: &HallWay, positions: &[Option<Vec3>], from: usize) -> (Vec<f32>, Vec<Option<usize>>) {
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
        for link in &hall_way.links {
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
    fn hall_way_points(&mut self, t: usize) -> Option<(Rc<HallWay>, Vec<Option<Vec3>>)> {
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
    /// way shortest, counting the way to it straight; that vertex first.
    ///
    /// A door the walk outside cannot reach is passed over: one standing more than
    /// [`WAY_DOOR_STEP`] over the ground under it is on an upper storey, up the building's own
    /// ramps, which the areal map does not carry, so the unit goes round to a door at ground
    /// level instead. With no such door left it takes the nearest anyway, as it did before.
    ///
    /// STAND-IN: docs/31-packages.md#not-established -- how the walker joins the hall way is
    /// not read: straight to that vertex, then along the links.
    pub fn way_in(&mut self, t: usize, from: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let (h, points) = self.hall_way_points(t)?;
        let near = |i: &usize| points[*i].map_or(f32::INFINITY, |p| p.distance(goal));
        let end = (0..h.vertices.len()).min_by(|a, b| near(a).total_cmp(&near(b)))?;
        let (dist, prev) = shortest(&h, &points, end);
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
    /// hall-way vertex along the links to the exit that makes the way to `to` shortest,
    /// counting the rest straight.
    ///
    /// STAND-IN: docs/31-packages.md#the-escape--read -- the building's own paths an escape is
    /// routed out by ("LEAVE IS TOO !!!") are not read: the hall way's.
    pub fn way_out(&mut self, t: usize, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let (h, points) = self.hall_way_points(t)?;
        let near = |i: &usize| points[*i].map_or(f32::INFINITY, |p| p.distance(from));
        let start = (0..h.vertices.len()).min_by(|a, b| near(a).total_cmp(&near(b)))?;
        let (dist, prev) = shortest(&h, &points, start);
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
            .filter_map(|t| {
                let (h, points) = self.hall_way_points(t)?;
                let kept: Vec<usize> = (0..points.len()).filter(|&i| points[i].is_some()).collect();
                let index = |i: u32| kept.iter().position(|&k| k == i as usize);
                let way = Way {
                    points: kept.iter().filter_map(|&i| points[i]).collect(),
                    flags: kept.iter().map(|&i| h.vertices[i].flags).collect(),
                    links: h.links.iter().filter_map(|l| Some((index(l.start)?, index(l.end)?))).collect(),
                };
                Some((t, way))
            })
            .collect()
    }

    /// The points robot target `t` walks from `from` to `goal` across the ground: the walker's
    /// global path for a unit that does not fly, straight for a flyer or on a map with no areal
    /// map (docs/24, "The global path"). A goal the walker refuses, or one it finds no way to,
    /// gives none, and the unit holds; a unit on an areal no link leaves makes for the nearest
    /// walkable ground first.
    ///
    /// STAND-IN: docs/24-motion.md#the-global-path--read -- how the walker goes to the point it
    /// finds off a non-walkable areal is not read: straight. What the walker does when its search
    /// fails is not read: it holds, its queues emptied as `SetTarget` empties them first. The
    /// game's `rand()` is not followed: a 32-bit xorshift of the play's own.
    pub fn route(&mut self, t: usize, from: Vec3, goal: Vec3) -> Vec<Vec3> {
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return vec![goal] };
        if robot.flyer || self.graph.is_none() {
            return vec![goal];
        }
        let (clearance, standing) =
            (robot.collision.1, robot.walker.ground.and_then(|h| h.solid).map(|s| s.0));
        let ways = self.bridge_ways();
        let on = |b: Option<usize>| b.and_then(|b| ways.iter().position(|(w, _)| *w == b));
        let aboard = on(standing);
        let ways: Vec<Way> = ways.into_iter().map(|(_, w)| w).collect();
        let Play { graph, walk_seed, .. } = self;
        let graph = graph.as_ref().expect("tested above");
        let mut random = || {
            let mut x = *walk_seed;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *walk_seed = x;
            (x >> 8) as f32 / (1u32 << 24) as f32
        };
        match graph.route(from, goal, &ways, aboard, clearance, &mut random) {
            Ok(legs) => legs,
            Err(Refusal::Stranded) => graph.escape(from, &mut random).into_iter().collect(),
            Err(Refusal::Goal | Refusal::NoWay) => Vec::new(),
        }
    }

    /// The legs of a walk for robot target `t` at `from` to `goal`: out of the building it
    /// walked into first while it still stands inside that building's outer ring. Once outside,
    /// the building is forgotten.
    pub fn legs_to(&mut self, t: usize, from: Vec3, goal: Vec3) -> Vec<Vec3> {
        let mut legs = Vec::new();
        if let Some(&b) = self.construction.ways.entered.get(&t) {
            if self.battle.combat.targets.get(b).is_some_and(|x| x.alive) && self.within_contour(b, from) {
                legs = self.way_out(b, from, goal).unwrap_or_default();
            } else {
                self.construction.ways.entered.remove(&t);
            }
        }
        let start = legs.last().copied().unwrap_or(from);
        let way = self.route(t, start, goal);
        if way.is_empty() {
            return Vec::new();
        }
        legs.extend(way);
        legs
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
    /// the place at `goal` — its pod or one of its docks: the way round the building to the door
    /// its way in starts at, then that way in; or straight there.
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
        let way = self.way_in(b, from, goal).unwrap_or_else(|| vec![goal]);
        // A unit that joins the hall way where it stands walks it straight: the global path is
        // over the areals, which lie over the building, and it is under them. The vertex it
        // stands by is behind it, so the walk goes on from the next one.
        if from.distance(way[0]) <= WAY_JOIN {
            return if way.len() > 1 { way[1..].to_vec() } else { way };
        }
        // A flyer goes over the walls; a walker goes round them where its way to the door
        // would run into one, and straight at it where the way is clear.
        let flyer = self.robots.iter().find(|(rt, _)| *rt == t).is_some_and(|(_, r)| r.flyer);
        let walled = !flyer
            && self
                .ground
                .solids
                .get(b)
                .is_some_and(|s| s.present && parkan_sim::solid::blocked(from, way[0], s));
        let round = if walled { self.way_round(b, from, way[0]) } else { Vec::new() };
        let (start, rest) = round.split_first().map_or((way[0], Vec::new()), |(s, r)| (*s, r.to_vec()));
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
            links: vec![Link { start: 1, end: 0 }, Link { start: 2, end: 1 }, Link { start: 3, end: 0 }],
        };
        let points: Vec<Option<Vec3>> =
            h.vertices.iter().map(|v| Some(Vec3::from_array(v.position))).collect();
        let (dist, prev) = shortest(&h, &points, 0);
        assert_eq!(dist, vec![0.0, 10.0, 20.0, 5.0]);
        assert_eq!(walk_back(&prev, 2), vec![0, 1, 2]);
    }
}
