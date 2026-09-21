//! A factory's production: its projects, the build order its screen gives, the build's
//! budgets, and where the finished bot appears. See `docs/36-factory.md`, "Projects" and
//! "Production", and `docs/23-economy.md`, "Construction".

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, PLACE_CREATION, PLACE_CREATION_OLD};
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::{gamedir, objects};
use parkan_sim::construct::{self, Construct};
use parkan_sim::planner;

use crate::assembly::Assembly;
use crate::designs::{Designer, Node};

/// The most recent projects a factory panel lists (`+0xb674`, five slots).
pub const RECENT_PROJECTS: usize = 5;
/// The `mission.cfg` object naming the designs a mission starts the recent projects with,
/// and where they are loaded from (`iron3d.dll:0x1004ddc9`, `0x1004df52`).
pub const PREBUILD: &str = "prebuild";
pub const PREBUILD_DIR: &str = "units\\units\\prebld\\";
/// The mission property that spares a factory's bots their price (`MBehaviour+0x9ec`).
pub const FREE_BOT_NUM: &str = "FreeBotNum";
/// `VOICE_UNIT_READY`, queued for the player's clan when its factory makes a bot
/// (`iron3d.dll:0x10033490`).
pub const VOICE_UNIT_READY: &str = "VOICE_UNIT_READY";

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
    /// The unit box's five lines as the designer rated it, and the design's sphere for its
    /// preview; empty and none for a design the designer did not make.
    pub lines: Vec<String>,
    pub sphere: Option<([f32; 3], f32)>,
}

/// A build in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub project: Project,
    pub construct: Construct,
    /// The ore its last takt asked for: the factory's ore property's most until the next.
    pub request: f32,
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
    /// Its class-26 efficiency and the level its batteries serve it at, whose product is its
    /// KPD, and its profile's `Use_Power` (docs/23, "Efficiency is a building's size").
    pub efficiency: f32,
    pub level: f32,
    pub use_power: f32,
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
            efficiency: 1.0,
            level: 1.0,
            use_power: 0.0,
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

    /// A design `mission.cfg`'s `prebuild` object names (`iron3d.dll:0x1004ded7`): the recent
    /// projects shift down and it loads into slot 0. With none selected, project 0 is, as the
    /// panel selects it once it has recent projects and shows none (`0x10097a06`).
    pub fn prebuild(&mut self, project: Project) {
        self.projects.insert(0, project);
        self.projects.truncate(RECENT_PROJECTS);
        self.selected.get_or_insert(0);
    }

    /// Start production of the shown project (`0x100986e0`): nothing without a project or a
    /// free mind, or for a chassis bigger than the factory.
    pub fn start(&mut self, batch: bool, free_minds: usize) -> bool {
        let Some(project) = self.shown().cloned() else { return false };
        self.start_project(project, batch, free_minds)
    }

    /// Start production of `project`, which is what a build order naming its own design does:
    /// the player's panel names the shown one, and a clan's AI names one its design store
    /// picked, without touching the factory's recent projects (docs/36, "Production").
    pub fn start_project(&mut self, project: Project, batch: bool, free_minds: usize) -> bool {
        if !self.idle() || free_minds == 0 || !construct::builds(self.size, project.chassis_size) {
            return false;
        }
        // A factory's ore cost is divided by its KPD as a build starts (`0x1002a2a7`).
        let ore = project.ore / self.kpd().max(f32::EPSILON);
        let construct =
            Construct::new(self.size, project.chassis_size, self.free_bots > 0, ore, project.power);
        self.build = Some(Build { project, construct, request: 0.0 });
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

    /// The factory's efficiency now: KPD.
    pub fn kpd(&self) -> f32 {
        self.efficiency * self.level
    }

    /// Whether its build is collecting power, and so draws `Use_Power`.
    pub fn drawing_power(&self) -> bool {
        self.build.as_ref().is_some_and(|b| b.construct.drawing_power())
    }

    /// One takt of `dt` seconds with `held` the ore the distribution brought: the project that
    /// completed, which the factory spends a free bot on (`0x1002a7f9`). In batch the same
    /// design starts again while a mind is free (`0x100874b0`).
    pub fn takt(&mut self, dt: f32, held: &mut f32) -> Option<Project> {
        let (kpd, use_power) = (self.kpd(), self.use_power);
        let build = self.build.as_mut()?;
        build.request = build.construct.takt(dt, kpd, use_power, held);
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

/// The design at `path` as the constructor rates, names and prices it from `designer`'s
/// research tree: what accept hands a factory (`0x1005144c`).
pub fn design_project(
    designer: &mut Designer,
    assembly: &mut Assembly,
    path: &str,
    strings: &BTreeMap<u32, String>,
) -> Option<Project> {
    let file = gamedir::resolve(&assembly.game, path)?;
    let unit = objects::parse_unit(&std::fs::read(file).ok()?, path).ok()?;
    let design = Node::from_unit(&unit)?;
    let (ore, power, _) = designer.price(&design);
    let lines = designer
        .rate(assembly, &design)
        .map(|r| r.lines(designer.offence_range, designer.defence_range).to_vec())
        .unwrap_or_default();
    Some(Project {
        path: path.to_owned(),
        name: designer.name(assembly, &design, 0, strings),
        type_word: designer.type_word(&design),
        chassis_size: crate::robot::chassis_size(&design.part),
        ore,
        power,
        lines,
        sphere: crate::cockpit::designer::design_sphere(assembly, path).map(|(c, r)| (c.to_array(), r)),
    })
}

/// `mission.cfg`'s `prebuild` object (`iron3d.dll:0x1004ddc9`): each design it names, from
/// `units\units\prebld\`, in turn into every factory's recent projects, so the last named is
/// project 0 (docs/23, "What `prebuild` does"). The player clan's research tree prices them.
/// Returns how many loaded.
pub fn prebuild(play: &mut crate::play::Play, game: &Path, mission_dir: &Path) -> Result<usize> {
    let Some(cfg) = gamedir::resolve(mission_dir, "mission.cfg") else { return Ok(0) };
    let blocks = crate::resources::read_cfg(&cfg)?;
    let Some(names) = blocks.get(PREBUILD) else { return Ok(0) };
    let names: Vec<String> = names.properties.iter().map(|(_, v)| v.clone()).collect();
    if names.is_empty() || play.factories.is_empty() {
        return Ok(0);
    }
    let catalogue = play.catalogue().context("the player's clan has no research tree")?;
    let strings = crate::resources::game_strings(game)?;
    let grade = play.factories.first().map_or(4, |f| usize::from(f.size));
    let mut designer = Designer::new(game, catalogue, grade);
    let mut loaded = 0;
    for name in names {
        let path = format!("{PREBUILD_DIR}{name}");
        let Some(project) = design_project(&mut designer, &mut play.assembly, &path, &strings) else {
            continue;
        };
        for f in &mut play.factories {
            f.prebuild(project.clone());
        }
        loaded += 1;
    }
    Ok(loaded)
}

/// The AI's design store: the directory function 43 loads into the SuperAI's `+0x40c`
/// (`ai.dll:0x1000d561`).
pub const AI_DIR: &str = "units\\units\\ai\\";

/// One design in the store, with the three floats of its 0x124-byte record that a
/// `SELECT_*` ranks by (`ai.dll:0x10010c30`): property 54 at `+0x110`, the guns figure at
/// `+0x114` and the live top speed at `+0x118`.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    pub project: Project,
    pub hit_points: f32,
    pub guns: f32,
    pub speed: f32,
}

/// A clan's design store: every `.dat` in `UNITS\UNITS\AI\`, priced and rated against that
/// clan's research tree. See `docs/15-behaviour.md`, "Function 69 sets how sloppy the AI's
/// design pick is".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Store {
    pub designs: Vec<Design>,
}

impl Store {
    /// Load the store for a clan whose tree is `catalogue`, the factory's size setting the
    /// designer's grade.
    pub fn load(
        game: &Path,
        assembly: &mut Assembly,
        catalogue: crate::designs::Catalogue,
        grade: usize,
    ) -> Result<Store> {
        let strings = crate::resources::game_strings(game).unwrap_or_default();
        let mut designer = Designer::new(game, catalogue, grade);
        let dir = gamedir::resolve(game, AI_DIR).context("no UNITS\\UNITS\\AI")?;
        let mut names: Vec<String> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .filter(|n| n.to_ascii_lowercase().ends_with(".dat"))
            .collect();
        names.sort();
        let mut designs = Vec::new();
        for name in names {
            let path = format!("{AI_DIR}{name}");
            let Some(project) = design_project(&mut designer, assembly, &path, &strings) else {
                continue;
            };
            let rating = gamedir::resolve(game, &path)
                .and_then(|file| std::fs::read(file).ok())
                .and_then(|data| objects::parse_unit(&data, &path).ok())
                .and_then(|unit| Node::from_unit(&unit))
                .and_then(|design| designer.rate(assembly, &design));
            designs.push(Design {
                hit_points: design_hit_points(assembly, &project.path),
                guns: rating.as_ref().map_or(0.0, |r| r.guns),
                speed: rating.as_ref().map_or(0.0, |r| r.speed),
                project,
            });
        }
        Ok(Store { designs })
    }

    /// The design a build order picks (`ai.dll:0x100107c0`, *read*). Every design of the
    /// wanted robot type is scored by `mode`, the scores sorted largest first, and the one at
    /// `draw` taken rather than the best — `draw` being `(rand() + timeGetTime()) % (spread +
    /// 1)`, whose spread function 69 sets from `fDifficulty`. A draw at or past the count
    /// falls back to 0, and `SELECT_SMALLEST` skips the draw and takes the **last** of the
    /// ranking, the weakest.
    ///
    /// The six arms of the jump table at `0x10010bbc` are read one at a time, in `mode − 1`
    /// order: `SELECT_BEST_WEAPON` copies the record's `+0x114`, its guns; `SELECT_BEST_ARMOR`
    /// `+0x110`, property 54's hit points; `SELECT_BEST_RANGE` `+0x11c`, which the store's own
    /// fill never writes; `SELECT_FASTEST` `+0x118`, property 145's top speed; and
    /// `SELECT_BEST_COMBAT` and `SELECT_SMALLEST` both call the **strength formula**
    /// (`0x1000fc70`) on `+0x110` and `+0x114` — so "best combat" is guns over armour, not
    /// armour, and "smallest" is the weakest by that same figure rather than the least chassis.
    ///
    /// The candidate list is every design whose `+0x108` Type **equals** the order's — not a
    /// mask — that does not carry `CLASS_BUILDING`.
    ///
    /// STAND-IN: docs/15-behaviour.md#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured
    /// -- `SELECT_BEST_RANGE`'s float is one nothing fills, so it scores every design 0 and the
    /// ranking keeps the store's order; no shipped raise passes it. The game also works out the
    /// clan's largest factory (`0x10006820`, every `BUILDING_PLANT` on its list, property
    /// `0x201`, the maximum) and then **throws the answer away** — `or eax, 0xffffffff`
    /// clobbers it at `0x10010af5` before the size comparison that would have used it — so the
    /// pick applies no size limit, and neither does this. A design too big for the factory is
    /// refused when the build starts ("Robot SizedType not match"), as it is here.
    pub fn pick(&self, type_word: u32, mode: u32, draw: usize) -> Option<&Project> {
        let strength = |d: &Design| parkan_sim::progression::strength(d.guns, d.hit_points);
        let score = |d: &Design| match mode {
            planner::SELECT_BEST_WEAPON => d.guns,
            planner::SELECT_BEST_ARMOR => d.hit_points,
            planner::SELECT_BEST_RANGE => 0.0,
            planner::SELECT_FASTEST => d.speed,
            _ => strength(d),
        };
        let mut ranked: Vec<&Design> = self
            .designs
            .iter()
            .filter(|d| {
                d.project.type_word == type_word
                    && d.project.type_word & parkan_sim::behaviour::BUILDING_BIT == 0
            })
            .collect();
        ranked.sort_by(|a, b| score(b).total_cmp(&score(a)));
        let at = if mode == planner::SELECT_SMALLEST {
            ranked.len().checked_sub(1)?
        } else if draw < ranked.len() {
            draw
        } else {
            0
        };
        ranked.get(at).map(|d| &d.project)
    }
}

/// A design's hit points at full, property 54: its parts' `.ndp` durabilities summed, which
/// is what the control system accumulates as it builds the object (docs/15, "What a strength
/// is").
pub fn design_hit_points(assembly: &mut Assembly, path: &str) -> f32 {
    let parts = assembly.parts(mission::KIND_UNIT, path);
    let mut total = 0.0;
    for part in parts {
        let Some(loaded) = assembly.mesh(&part.reference) else { continue };
        let mesh = loaded.mesh.clone();
        let (life, _) = crate::battle::part_damage(assembly, &part, &mesh, 1.0, 1.0, false);
        total += life.map_or(0.0, |l| l.full());
    }
    total
}

/// A vertex's point in the world, through its joint node's pose (`ArealMap.dll:0x1000a760`).
pub fn vertex_world(vertex: &hallway::Vertex, part: &parkan_sim::combat::Part) -> Option<Vec3> {
    let pose = part.nodes.get(vertex.joint as usize)?;
    let p = pose.apply(vertex.position.map(|v| f64::from(v * part.scale)));
    Some(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> Project {
        Project {
            path: format!("units\\units\\prebld\\{name}.dat"),
            name: name.to_owned(),
            type_word: 0,
            chassis_size: 2,
            ore: 0.0,
            power: 0.0,
            lines: Vec::new(),
            sphere: None,
        }
    }

    fn factory() -> Factory {
        Factory {
            target: 0,
            logic_id: 0,
            size: 4,
            free_bots: 0,
            creation: None,
            projects: Vec::new(),
            selected: None,
            build: None,
            batch: false,
            efficiency: 1.0,
            level: 1.0,
            use_power: 0.0,
        }
    }

    #[test]
    fn prebuilt_designs_shift_down_so_the_last_named_is_project_0_and_it_is_selected() {
        let mut f = factory();
        for name in ["p1", "p2"] {
            f.prebuild(project(name));
        }
        assert_eq!(f.projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["p2", "p1"]);
        assert_eq!(f.selected, Some(0));
        for name in ["a", "b", "c", "d"] {
            f.prebuild(project(name));
        }
        assert_eq!(
            f.projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["d", "c", "b", "a", "p2"]
        );
    }

    #[test]
    fn a_build_with_no_free_mind_is_dropped_and_nothing_waits_for_one() {
        // "No Free mind... cannot start constructing": the task's start fails, the stack
        // removes the order and nothing in the factory waits (docs/23, "The bot limit is
        // the clan's mind count"). The AI's own handlers do the same one level up -- a
        // refused `ORDER_BUILDING_CONSTRUCT` marks the problem solved.
        let mut f = factory();
        f.prebuild(project("p1"));
        assert!(!f.start(false, 0), "no mind, no build");
        assert!(f.build.is_none(), "and nothing is queued behind it");
        // The project is still shown, so a later order can start it once a mind frees.
        assert!(f.start(false, 1));
        assert!(f.build.is_some());
        // A second order while one runs is refused too, and again queues nothing.
        assert!(!f.start(false, 1));
    }
}
