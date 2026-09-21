//! A building's doors and its control pod: `CBuilding` files its controller's class-12
//! items as doors and its class-13 items as computers, opens a door for a unit standing on
//! it nearby, and fires its capture when a unit stands in the first computer's zone until
//! it has opened. See `docs/24-motion.md`, "Walking into a building", and
//! `docs/27-ownership.md`, "Capture".
//!
//! Every other component that names channels is an item too, and nothing files those: the
//! controller's own time driver steps them from the switch word the record carries
//! (`docs/28-chassis.md`, "Every component is stepped, not only a device"). That is what
//! turns a mine's rotors, the Main Teleport's rings, the energy bridge's hub and, once, a
//! tower's mast.

use glam::Vec3;
use parkan_formats::control::Controller;
use parkan_formats::mission::{self, Mission};
use parkan_formats::pose::Pose;
use parkan_sim::combat::{Part, Target};
use parkan_sim::device::{Item, Motion};

use crate::assembly::Assembly;

/// The item classes `CBuilding` files as doors and as computers (`Terrain.dll:0x100580b0`).
/// Those two are the only classes it looks for; every other component the controller holds
/// is left to the time driver, which steps whatever names a channel.
pub const DOOR_TYPE: i32 = 12;
pub const COMPUTER_TYPE: i32 = 13;
/// An open door nothing holds closes this long after it opened (`CBuilding::SendMsg`,
/// `0x10057550`).
pub const DOOR_CLOSE_MS: f64 = 5000.0;
/// A pod that has closed can fire again this long after (docs/27, "Capture").
pub const POD_REARM_MS: f64 = 5000.0;
/// The pod's zone across the ground: this share of its part's bounding radius
/// (`0x10059d80`).
pub const POD_ZONE_SHARE: f32 = 0.8;

/// Where a door or a pod is in its stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Shut,
    Opening,
    Open,
    Closing,
}

/// A unit standing on a building: the hero, or the robot that is target `t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Child {
    Hero,
    Robot(usize),
}

/// A child's position, and its bounding sphere: the doors measure the sphere, and the pod
/// its centre (`Terrain.dll:0x10059ff1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standing {
    pub child: Child,
    pub position: Vec3,
    pub centre: Vec3,
    pub radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Door {
    pub item: Item,
    /// The nodes its channels play.
    pub nodes: Vec<usize>,
    pub phase: Phase,
    pub opened_ms: f64,
    pub held: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pod {
    pub item: Item,
    /// The first node its item plays (`Terrain.dll:0x100583a2`), whose sphere the zone is
    /// measured across the ground by.
    pub node: usize,
    pub phase: Phase,
    /// Who switched it on, and whether its capture has fired for them.
    pub occupant: Option<Child>,
    pub fired: bool,
    pub closed_ms: f64,
}

/// One building's doors and pod, on the part whose controller carries them.
#[derive(Clone, Debug, PartialEq)]
pub struct Building {
    pub target: usize,
    pub part: usize,
    /// The placement, the part's mount and the object's scale its nodes are posed from.
    pub place: Pose,
    pub mount: Pose,
    pub scale: f32,
    pub doors: Vec<Door>,
    pub pod: Option<Pod>,
    /// Every other component that names channels, stepped from the switch word its record
    /// carries: the nine class-26 and two class-29 records the install ships
    /// ([`running_items`]).
    pub running: Vec<Item>,
    /// The pod's zone: its node's sphere at rest, as placed.
    pub zone: Option<(Vec3, f32)>,
    /// The controller's channels, which the items name.
    controller: Controller,
}

/// What a building tick asks of the play: the pod fired for a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fired {
    pub target: usize,
    pub child: Child,
}

/// How many of a part's nodes both its mesh and its pose list hold.
fn mesh_nodes(part: &Part) -> usize {
    part.mesh.nodes.len().min(part.nodes.len())
}

/// A controller's items of `class`, switched off at their start.
///
/// STAND-IN: docs/24-motion.md#walking-into-a-building--read-and-measured -- what `CBuilding`
/// does to an item's switch word as it files it is not read; the records leave it at the
/// constructor's 5, which would wrap for ever and never let a pod report open, so a door
/// and a pod start shut, their word 0 and their progress 0.
fn items(controller: &Controller, class: i32) -> Vec<Item> {
    controller
        .components
        .iter()
        .enumerate()
        .filter(|(_, c)| c.type_id == class)
        .map(|(i, c)| {
            let mut item = Item::new(i, c, &controller.channels);
            item.state = 0;
            item
        })
        .collect()
}

/// An item as it starts, with the one switch word in the install this does not play as read.
///
/// STAND-IN: docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured
/// -- an item whose word **bounces** is started as one that opens and stops. The word 9 is
/// *read* from the record's `+0x18` (`0x10021d86`) and bit 8 is *read* to hold the progress at
/// an end and swap the low bits (`0x10020ae4`), so as the file stands both towers raise their
/// gun mast over five seconds and stow it over the next five, for ever. Nothing found switches
/// the word off: the component factory files every class in the controller's timed list
/// (`Control.dll:0x1002d70a`), `CBuilding` looks for classes 12 and 13 and no other
/// (`Terrain.dll:0x100583a2`), and neither record names a section-5 group at `+0x10` or
/// `+0x14`. *Measured*: the two class-29 records are the only ones of the install's 1066 whose
/// word bounces. In the game a tower's mast comes up as it is built and stays up.
pub fn started(mut item: Item) -> Item {
    if item.state & parkan_sim::device::BOUNCE != 0 {
        item.switch(item.state & parkan_sim::device::CLOSING == 0);
    }
    item
}

/// The controller's items that run by themselves: every component that names a channel and
/// is neither a door nor a control pod, in the switch word its own record gives it
/// (`Control.dll:0x10021d86`, else the constructor's 5).
///
/// The component factory (`0x1002d4b0`) files **every** class it builds in the controller's
/// timed list, and the time driver (`0x1002d260`) runs each one's update, so a class the
/// owner never looks at still steps its channels. *Measured*: across the 1066 component
/// records the install ships, the classes that name channels and that nothing else here
/// drives are **26** -- nine records, the three mines' rotors, the Main Teleport's twenty-eight
/// rings and `fr_e_brige`'s hub -- and **29**, two records, the Small and Large Towers' masts.
/// Class 26 is a plain base item (case 11 of the factory) and class 29 falls to its default
/// case, which is the base item too.
pub fn running_items(controller: &Controller) -> Vec<Item> {
    controller
        .components
        .iter()
        .enumerate()
        .filter(|(_, c)| !matches!(c.type_id, DOOR_TYPE | COMPUTER_TYPE) && !c.entries.is_empty())
        .map(|(i, c)| started(Item::new(i, c, &controller.channels)))
        .collect()
}

impl Building {
    /// Mission object `object`, target `target`, if one of its parts' controllers has a door
    /// or a computer. Doors are every class-12 item; the pod is the file's first class-13
    /// item (`Terrain.dll:0x100583a2`).
    pub fn load(
        assembly: &mut Assembly,
        mission: &Mission,
        object: usize,
        target: usize,
    ) -> Option<Building> {
        let placed = mission.objects.get(object).filter(|o| o.kind == mission::KIND_BUILDING)?;
        let parts = assembly.parts(placed.kind, &placed.path);
        for (p, part) in parts.iter().enumerate() {
            let Some(slot) = assembly.library.record_slot(assembly.library.get(&part.record), "ctl", 0)
            else {
                continue;
            };
            let Some(controller) = assembly
                .archive(&slot.library)
                .and_then(|a| a.read_name(&slot.member).ok())
                .and_then(|data| parkan_formats::control::parse(data, &slot.member).ok())
            else {
                continue;
            };
            let doors: Vec<Door> = items(&controller, DOOR_TYPE)
                .into_iter()
                .map(|item| Door {
                    nodes: item.channels.iter().filter_map(|c| usize::try_from(c.node).ok()).collect(),
                    item,
                    phase: Phase::Shut,
                    opened_ms: 0.0,
                    held: false,
                })
                .collect();
            let pod = items(&controller, COMPUTER_TYPE).into_iter().next().map(|item| Pod {
                node: item
                    .channels
                    .iter()
                    .find_map(|c| usize::try_from(c.node).ok())
                    .unwrap_or(controller.components[item.component].node.max(0) as usize),
                item,
                phase: Phase::Shut,
                occupant: None,
                fired: false,
                closed_ms: f64::NEG_INFINITY,
            });
            let running = running_items(&controller);
            if doors.is_empty() && pod.is_none() && running.is_empty() {
                continue;
            }
            let half = f64::from(placed.rotation) / 2.0;
            return Some(Building {
                target,
                part: p,
                place: Pose {
                    translation: placed.position.map(f64::from),
                    rotation: [half.cos(), 0.0, 0.0, half.sin()],
                },
                mount: part.pose,
                scale: placed.placed_scale(),
                doors,
                pod,
                running,
                zone: None,
                controller,
            });
        }
        None
    }

    /// The frame node `node` plays from the items' channels, if one drives it.
    fn frame(&self, node: usize) -> Option<f64> {
        let door = self.doors.iter().map(|d| &d.item);
        door.chain(self.pod.iter().map(|p| &p.item))
            .chain(&self.running)
            .flat_map(|item| item.channels.iter().zip(&item.now))
            .find(|(c, _)| c.node == node as i32)
            .map(|(c, &v)| f64::from(c.frame(v)))
    }

    /// Pose the building's own part where its items have their channels now, as the placement
    /// poses a building at rest ([`crate::battle::Battle::load`]), and carry every part that
    /// hangs on one of its nodes along with it.
    pub fn pose(&self, parts: &mut [Part], posed_elsewhere: impl Fn(usize) -> bool) {
        let Some(own) = parts.get(self.part) else { return };
        let (mesh, count) = (own.mesh.clone(), mesh_nodes(own));
        for n in 0..count {
            let world = mesh.world_pose_by(n, |k| match self.frame(k) {
                Some(frame) => mesh.pose_at(k, frame),
                None => mesh.local_pose(k),
            });
            let mut local = self.mount.compose(&world);
            local.translation = local.translation.map(|v| v * f64::from(self.scale));
            parts[self.part].nodes[n] = self.place.compose(&local);
        }
        self.carry(parts, &posed_elsewhere);
    }

    /// Repose every part that hangs on a node this building plays, and everything hanging on
    /// those in turn: a tower's turret, its guns, its radar and its deflector all stand on
    /// nodes of the mast its class-29 item raises, and a part left where the assembly mounted
    /// it stays behind in the ground.
    ///
    /// A hosted part's nodes are its host's socket, less the part's own root pose, times the
    /// node's place in its mesh -- the chain [`crate::robot::Robot::part_pose`] walks for a
    /// warbot's turret and guns.
    fn carry(&self, parts: &mut [Part], posed_elsewhere: &impl Fn(usize) -> bool) {
        for p in 0..parts.len() {
            let Some((host, socket)) = parts[p].host else { continue };
            if host >= p || posed_elsewhere(p) || (host != self.part && !self.carried(parts, host)) {
                continue;
            }
            let Some(&mount) = parts[host].nodes.get(socket) else { continue };
            let mesh = parts[p].mesh.clone();
            let root = mesh.root_pose().invert();
            let scale = f64::from(parts[p].scale);
            for n in 0..mesh_nodes(&parts[p]) {
                let mut own = root.compose(&mesh.world_pose(n));
                own.translation = own.translation.map(|v| v * scale);
                parts[p].nodes[n] = mount.compose(&own);
            }
        }
    }

    /// Whether part `p` hangs, through however many hosts, on the building's own part.
    fn carried(&self, parts: &[Part], p: usize) -> bool {
        let mut at = p;
        for _ in 0..parts.len() {
            match parts[at].host {
                Some((host, _)) if host < at => at = host,
                _ => return at == self.part,
            }
            if at == self.part {
                return true;
            }
        }
        false
    }

    /// Whether any of its items plays a node of its mesh, so the part has to be drawn node
    /// by node rather than as one model at rest: the energy bridge takes no damage, and
    /// without this its three turning arms stand still while the rays hung on them sweep.
    pub fn plays_nodes(&self) -> bool {
        let door = self.doors.iter().map(|d| &d.item);
        door.chain(self.pod.iter().map(|p| &p.item))
            .chain(&self.running)
            .any(|item| !item.channels.is_empty())
    }

    /// Whether node `node` is a door's that is open, whose faces let units through
    /// (`IBuilding` slot 17, `AniMesh.dll:0x1000dd13`).
    pub fn open_door_node(&self, node: usize) -> bool {
        self.doors.iter().any(|d| d.phase == Phase::Open && d.nodes.contains(&node))
    }

    /// A node's level-0 slot sphere in the world, from `part`'s poses.
    fn sphere(part: &Part, node: usize) -> Option<(Vec3, f32)> {
        let slot = part.mesh.slots.get(usize::from(part.mesh.nodes.get(node)?.slot_index[0]))?;
        let pose = part.nodes.get(node)?;
        let [cx, cy, cz, r] = slot.sphere;
        let c = pose.apply([cx, cy, cz].map(|v| f64::from(v * part.scale)));
        Some((Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32), r * part.scale))
    }

    /// Keep the pod's zone from `part`'s nodes as placed, before anything plays them.
    pub fn place_zone(&mut self, part: &Part) {
        self.zone = self.pod.as_ref().and_then(|p| Self::sphere(part, p.node));
    }

    /// The pod's zone's centre in the world.
    pub fn pod_centre(&self, _part: &Part) -> Option<Vec3> {
        self.zone.map(|z| z.0)
    }

    /// The heights the pod's zone lies between: the z of the first and last corners of its
    /// node's parent's level-0 box, the current damage variant's, in the world
    /// (`Terrain.dll:0x10059ec8`, `IJointMesh` slot 4).
    pub fn zone_heights(&self, part: &Part) -> Option<(f32, f32)> {
        let pod = self.pod.as_ref()?;
        let parent = usize::from(part.mesh.nodes.get(pod.node)?.parent);
        let node = part.mesh.nodes.get(parent)?;
        let slot = part
            .slot(parent)
            .or_else(|| Some(node.slot_index[0]).filter(|&s| s != parkan_formats::mesh::NO_SLOT))?;
        let slot = part.mesh.slots.get(usize::from(slot))?;
        let pose = part.nodes.get(parent)?;
        let z = |corner: [f32; 3]| pose.apply(corner.map(|v| f64::from(v * part.scale)))[2] as f32;
        let (a, b) = (z(slot.aabb_min), z(slot.aabb_max));
        Some((a.min(b), a.max(b)))
    }

    /// Whether a child whose bounding sphere is centred at `at` is in the pod's zone
    /// (`Terrain.dll:0x10059d80`): across the ground within 0.8 of the radius of the pod
    /// node's level-0 sphere, and in height between the corners of its parent's box
    /// (docs/27, "The zone's height is the pod node's parent's box").
    pub fn in_zone(&self, part: &Part, at: Vec3) -> bool {
        let Some((centre, radius)) = self.pod.as_ref().and_then(|p| Self::sphere(part, p.node)) else {
            return false;
        };
        let Some((low, high)) = self.zone_heights(part) else { return false };
        centre.truncate().distance(at.truncate()) <= POD_ZONE_SHARE * radius && (low..=high).contains(&at.z)
    }

    /// One tick at `now_ms` with the units standing on the building: the doors open for a
    /// child near them and close once free; the pod opens for a child in its zone and fires
    /// when it has opened with that child still there. Returns whether any channel moved, and
    /// the pod's firing.
    ///
    /// STAND-IN: docs/24-motion.md#walking-into-a-building--read-and-measured -- the capsule
    /// a door's part is measured against (`Terrain.dll:0x1005a27f`) is not read: the door
    /// node's level-0 slot sphere stands in for it. Holds are worked out afresh from every
    /// child each tick rather than on each child's move.
    /// Returns whether the doors' or the pod's channels moved, whether a running item's did,
    /// and a pod that fired.
    pub fn tick(&mut self, now_ms: f64, part: &Part, children: &[Standing]) -> (bool, bool, Option<Fired>) {
        let before: Vec<f32> = self.channel_values();
        let before_running: Vec<f32> = self.running_values();
        for item in &mut self.running {
            item.tick(now_ms, &Motion::default(), true);
        }
        for d in &mut self.doors {
            d.held = children.iter().any(|c| {
                d.nodes
                    .iter()
                    .filter_map(|&n| Self::sphere(part, n))
                    .any(|(centre, r)| centre.distance(c.centre) <= c.radius + r)
            });
            if d.held && matches!(d.phase, Phase::Shut | Phase::Closing) {
                d.item.switch(true);
                d.phase = Phase::Opening;
            }
        }
        let motion = Motion::default();
        for d in &mut self.doors {
            d.item.tick(now_ms, &motion, true);
            match d.phase {
                Phase::Opening if d.item.stopped() => {
                    d.phase = Phase::Open;
                    d.opened_ms = now_ms;
                }
                Phase::Open if !d.held && now_ms - d.opened_ms >= DOOR_CLOSE_MS => {
                    d.item.switch(false);
                    d.phase = Phase::Closing;
                }
                Phase::Closing if d.item.stopped() => d.phase = Phase::Shut,
                _ => {}
            }
        }
        let fired = self.tick_pod(now_ms, part, children);
        (self.channel_values() != before, self.running_values() != before_running, fired)
    }

    fn tick_pod(&mut self, now_ms: f64, part: &Part, children: &[Standing]) -> Option<Fired> {
        let inside: Vec<Child> =
            children.iter().filter(|c| self.in_zone(part, c.centre)).map(|c| c.child).collect();
        let target = self.target;
        let pod = self.pod.as_mut()?;
        pod.item.tick(now_ms, &Motion::default(), true);
        let mut fired = None;
        match pod.phase {
            Phase::Shut if now_ms - pod.closed_ms >= POD_REARM_MS => {
                if let Some(&first) = inside.first() {
                    pod.occupant = Some(first);
                    pod.fired = false;
                    pod.item.switch(true);
                    pod.phase = Phase::Opening;
                }
            }
            Phase::Opening if pod.item.stopped() => {
                pod.phase = Phase::Open;
                if let Some(child) = pod.occupant.filter(|c| inside.contains(c)) {
                    pod.fired = true;
                    fired = Some(Fired { target, child });
                }
            }
            Phase::Open if !pod.occupant.is_some_and(|c| inside.contains(&c)) => {
                pod.item.switch(false);
                pod.phase = Phase::Closing;
            }
            Phase::Closing if pod.item.stopped() => {
                pod.phase = Phase::Shut;
                pod.occupant = None;
                pod.closed_ms = now_ms;
            }
            _ => {}
        }
        fired
    }

    /// A hit naming node `node` (`Control.dll:0x1000ebc0`): the components are walked from the
    /// last to the first, and the first door whose node it is opens through `IBuilding` slot
    /// 14 (`Terrain.dll:0x1005b480`) if it is shut or closing; an opening or open door is left
    /// as it is. Nothing holds it: it closes 5 s after it has opened unless a child near it
    /// holds it. Returns whether it started opening.
    pub fn shot(&mut self, node: usize) -> bool {
        let components = &self.controller.components;
        let door = self.doors.iter_mut().rev().find(|d| {
            components.get(d.item.component).and_then(|c| usize::try_from(c.node).ok()) == Some(node)
        });
        match door {
            Some(d) if matches!(d.phase, Phase::Shut | Phase::Closing) => {
                d.item.switch(true);
                d.phase = Phase::Opening;
                true
            }
            _ => false,
        }
    }

    fn channel_values(&self) -> Vec<f32> {
        let doors = self.doors.iter().flat_map(|d| d.item.now.iter().copied());
        doors.chain(self.pod.iter().flat_map(|p| p.item.now.iter().copied())).collect()
    }

    fn running_values(&self) -> Vec<f32> {
        self.running.iter().flat_map(|i| i.now.iter().copied()).collect()
    }

    /// The controller the items belong to.
    pub fn controller(&self) -> &Controller {
        &self.controller
    }
}

/// A round struck part `part`'s node `node` of target `struck` at `point` (docs/24, "A shot
/// opens a door"). The hit opens the struck building's door on that node before its kind is
/// looked at, whoever fired. An area hit, whose radius is `reach`, carries the same node to
/// every building whose sphere it reaches, and only the number is compared. Returns the
/// targets whose door started opening.
pub fn open_shot_doors(
    buildings: &mut [Building],
    targets: &[Target],
    (struck, part, node): (usize, usize, usize),
    point: Vec3,
    reach: Option<f32>,
) -> Vec<usize> {
    let mut opened = Vec::new();
    for b in buildings.iter_mut() {
        let hit = if b.target == struck {
            b.part == part
        } else {
            reach.is_some_and(|r| {
                targets.get(b.target).is_some_and(|t| t.alive && (t.centre - point).length() < t.radius + r)
            })
        };
        if hit && b.shot(node) {
            opened.push(b.target);
        }
    }
    opened
}
