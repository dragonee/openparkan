//! A building's doors and its control pod: `CBuilding` files its controller's class-12
//! items as doors and its class-13 items as computers, opens a door for a unit standing on
//! it nearby, and fires its capture when a unit stands in the first computer's zone until
//! it has opened. See `docs/24-motion.md`, "Walking into a building", and
//! `docs/27-ownership.md`, "Capture".

use glam::Vec3;
use parkan_formats::control::Controller;
use parkan_formats::mission::{self, Mission};
use parkan_formats::pose::Pose;
use parkan_sim::combat::Part;
use parkan_sim::device::{Item, Motion};

use crate::assembly::Assembly;

/// The item classes `CBuilding` files as doors and as computers (`Terrain.dll:0x100580b0`).
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

/// A child's position (for the pod) and its bounding sphere (for the doors).
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
    /// The controller's channels, which the items name.
    controller: Controller,
}

/// What a building tick asks of the play: the pod fired for a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fired {
    pub target: usize,
    pub child: Child,
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
                node: controller.components[item.component].node.max(0) as usize,
                item,
                phase: Phase::Shut,
                occupant: None,
                fired: false,
                closed_ms: f64::NEG_INFINITY,
            });
            if doors.is_empty() && pod.is_none() {
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
                controller,
            });
        }
        None
    }

    /// The frame node `node` plays from the doors' and the pod's channels, if one drives it.
    fn frame(&self, node: usize) -> Option<f64> {
        let door = self.doors.iter().map(|d| &d.item);
        door.chain(self.pod.iter().map(|p| &p.item))
            .flat_map(|item| item.channels.iter().zip(&item.now))
            .find(|(c, _)| c.node == node as i32)
            .map(|(c, &v)| f64::from(c.frame(v)))
    }

    /// Pose `part`'s nodes where the doors and the pod have their channels now, as the
    /// placement poses a building at rest ([`crate::battle::Battle::load`]).
    pub fn pose(&self, part: &mut Part) {
        let mesh = part.mesh.clone();
        for n in 0..mesh.nodes.len().min(part.nodes.len()) {
            let world = mesh.world_pose_by(n, |k| match self.frame(k) {
                Some(frame) => mesh.pose_at(k, frame),
                None => mesh.local_pose(k),
            });
            let mut local = self.mount.compose(&world);
            local.translation = local.translation.map(|v| v * f64::from(self.scale));
            part.nodes[n] = self.place.compose(&local);
        }
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

    /// The pod node's sphere centre in the world.
    pub fn pod_centre(&self, part: &Part) -> Option<Vec3> {
        Self::sphere(part, self.pod.as_ref()?.node).map(|(c, _)| c)
    }

    /// Whether `at` is in the pod's zone: within 0.8 of the pod part's radius across the
    /// ground of its centre, and within its box in height (`0x10059d80`).
    ///
    /// STAND-IN: docs/24-motion.md#walking-into-a-building--read-and-measured -- which node's
    /// box bounds the zone in height (`0x10058607`) is not read. The pod node's own level-0
    /// box is under a metre tall and above the floor a unit stands on (0.9 on Mission 02's
    /// Large Factory), so the pod node's sphere bounds it in height instead.
    pub fn in_zone(&self, part: &Part, at: Vec3) -> bool {
        let Some(pod) = &self.pod else { return false };
        let Some((centre, radius)) = Self::sphere(part, pod.node) else { return false };
        centre.truncate().distance(at.truncate()) <= POD_ZONE_SHARE * radius
            && (at.z - centre.z).abs() <= radius
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
    pub fn tick(&mut self, now_ms: f64, part: &Part, children: &[Standing]) -> (bool, Option<Fired>) {
        let before: Vec<f32> = self.channel_values();
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
        (self.channel_values() != before, fired)
    }

    fn tick_pod(&mut self, now_ms: f64, part: &Part, children: &[Standing]) -> Option<Fired> {
        let inside: Vec<Child> =
            children.iter().filter(|c| self.in_zone(part, c.position)).map(|c| c.child).collect();
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

    fn channel_values(&self) -> Vec<f32> {
        let doors = self.doors.iter().flat_map(|d| d.item.now.iter().copied());
        doors.chain(self.pod.iter().flat_map(|p| p.item.now.iter().copied())).collect()
    }

    /// The controller the items belong to.
    pub fn controller(&self) -> &Controller {
        &self.controller
    }
}
