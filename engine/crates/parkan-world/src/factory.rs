//! A factory's production: its projects, the build order its screen gives, the build's
//! budgets, and where the finished bot appears. See `docs/36-factory.md`, "Projects" and
//! "Production", and `docs/23-economy.md`, "Construction".

use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, PLACE_CREATION, PLACE_CREATION_OLD};
use parkan_formats::mission::{self, Mission, Value};
use parkan_sim::construct::{self, Construct};

use crate::assembly::Assembly;

/// The most recent projects a factory panel lists (`+0xb674`, five slots).
pub const RECENT_PROJECTS: usize = 5;
/// The mission property that spares a factory's bots their price (`MBehaviour+0x9ec`).
pub const FREE_BOT_NUM: &str = "FreeBotNum";
/// `VOICE_UNIT_READY`, queued for the player's clan when its factory makes a bot
/// (`iron3d.dll:0x10033490`).
pub const VOICE_UNIT_READY: &str = "VOICE_UNIT_READY";
/// A factory's efficiency and a build's power use.
///
/// STAND-IN: docs/23-economy.md#efficiency-is-a-buildings-size -- the engine keeps no power
/// distribution: a factory works at efficiency 1 and draws its build's power at 1 a second.
pub const KPD: f32 = 1.0;
pub const USE_POWER: f32 = 1.0;

/// A design the factory can build: its `.dat` under the path it is registered at, its name,
/// Type and chassis size, and its price.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub path: String,
    pub name: String,
    pub type_word: u32,
    pub chassis_size: u8,
    pub ore: f32,
    pub power: f32,
}

/// A build in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub project: Project,
    pub construct: Construct,
}

/// One factory of the player's clan list.
#[derive(Clone, Debug, PartialEq)]
pub struct Factory {
    pub target: usize,
    pub logic_id: i32,
    /// The building's size: its record's fourth letter (`fr_b_plant` is 4).
    pub size: u8,
    pub free_bots: i32,
    /// The creation vertex (`0x800`, else `0x80`): its point and its joint node.
    pub creation: Option<hallway::Vertex>,
    pub projects: Vec<Project>,
    /// The selected project, or none.
    pub selected: Option<usize>,
    pub build: Option<Build>,
    pub batch: bool,
}

fn number(v: Value) -> i64 {
    match v {
        Value::Int(i) => i64::from(i),
        Value::Float(f) => f as i64,
    }
}

impl Factory {
    /// Mission object `object`, target `target`, if it is a plant (`Type` `0x80000010`).
    pub fn load(assembly: &mut Assembly, mission: &Mission, object: usize, target: usize) -> Option<Factory> {
        let placed = mission.objects.get(object).filter(|o| o.kind == mission::KIND_BUILDING)?;
        let type_word = placed.property("Type").map_or(0, |p| number(p.value)) as u32;
        if type_word != crate::play::BUILDING_PLANT {
            return None;
        }
        let parts = assembly.parts(placed.kind, &placed.path);
        let part = parts.iter().find(|p| p.host == -1)?;
        let size = part.record.as_bytes().get(3).map_or(0, |&b| construct::size_of_letter(b));
        let hall_way = assembly
            .archive(&part.reference.library)
            .and_then(|a| a.read_name(&part.reference.member).ok().map(<[u8]>::to_vec))
            .and_then(|blob| hallway::parse(&blob, &part.reference.member).ok())
            .unwrap_or_default();
        Some(Factory {
            target,
            logic_id: placed.logical_id,
            size,
            free_bots: placed.property(FREE_BOT_NUM).map_or(0, |p| number(p.value)) as i32,
            creation: creation_vertex(&hall_way),
            projects: Vec::new(),
            selected: None,
            build: None,
            batch: false,
        })
    }

    /// Whether the factory is idle: no build running.
    pub fn idle(&self) -> bool {
        self.build.is_none()
    }

    /// The project shown: the one in production, else the selected one.
    pub fn shown(&self) -> Option<&Project> {
        self.build.as_ref().map(|b| &b.project).or_else(|| self.selected.and_then(|i| self.projects.get(i)))
    }

    /// A design accepted in the designer: the newest project, selected (`0x100517d2`).
    pub fn accept(&mut self, project: Project) {
        self.projects.insert(0, project);
        self.projects.truncate(RECENT_PROJECTS);
        self.selected = Some(0);
    }

    /// Start production of the shown project (`0x100986e0`): nothing without a project or a
    /// free mind, or for a chassis bigger than the factory.
    pub fn start(&mut self, batch: bool, free_minds: usize) -> bool {
        let Some(project) = self.shown().cloned() else { return false };
        if !self.idle() || free_minds == 0 || !construct::builds(self.size, project.chassis_size) {
            return false;
        }
        let construct =
            Construct::new(self.size, project.chassis_size, self.free_bots > 0, project.ore, project.power);
        self.build = Some(Build { project, construct });
        self.batch = batch;
        true
    }

    /// Stop production (`0x10098720`): only the button whose kind matches the batch word
    /// acts; the build is dropped and, with no project selected, project 0 is.
    pub fn stop(&mut self, batch: bool) -> bool {
        if self.build.is_none() || batch != self.batch {
            return false;
        }
        self.build = None;
        self.batch = false;
        if self.selected.is_none() && !self.projects.is_empty() {
            self.selected = Some(0);
        }
        true
    }

    /// The build's progress, 0 to 1.
    pub fn progress(&self) -> f32 {
        self.build.as_ref().map_or(0.0, |b| b.construct.progress())
    }

    /// One takt of `dt` seconds; the project that completed, which the factory spends a free
    /// bot on (`0x1002a7f9`). In batch the same design starts again while a mind is free
    /// (`0x100874b0`).
    pub fn takt(&mut self, dt: f32) -> Option<Project> {
        let build = self.build.as_mut()?;
        build.construct.takt(dt, KPD, USE_POWER);
        if !build.construct.done() {
            return None;
        }
        let done = self.build.take()?;
        if self.free_bots > 0 {
            self.free_bots -= 1;
        }
        Some(done.project)
    }
}

/// The first hall-way vertex with `0x800`, else `0x80` (`Behavior.dll:0x100299a0`).
pub fn creation_vertex(hall_way: &HallWay) -> Option<hallway::Vertex> {
    hall_way.first(PLACE_CREATION).or_else(|| hall_way.first(PLACE_CREATION_OLD)).copied()
}

/// A vertex's point in the world, through its joint node's pose (`ArealMap.dll:0x1000a760`).
pub fn vertex_world(vertex: &hallway::Vertex, part: &parkan_sim::combat::Part) -> Option<Vec3> {
    let pose = part.nodes.get(vertex.joint as usize)?;
    let p = pose.apply(vertex.position.map(|v| f64::from(v * part.scale)));
    Some(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
}
