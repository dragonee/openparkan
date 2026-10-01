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

    /// The project shown: the selected recent project, else the one in production. A recent
    /// project's button selects it and the active project's selects none, which shows the unit
    /// in production (`0x100982a0`, `0x100985f0`), so a player looks through the designs while
    /// a build runs on: one of the install's saved pairs shows an S-31 while an M-42t is built
    /// (docs/36, "Projects").
    pub fn shown(&self) -> Option<&Project> {
        self.selected.and_then(|i| self.projects.get(i)).or_else(|| self.build.as_ref().map(|b| &b.project))
    }

    /// Show the unit in production, as the active project's button does, when there is one: what
    /// the panel opens on. The engine's choice; what the game keeps selected across a reopening
    /// is not read (docs/36, "For an engine").
    pub fn show_production(&mut self) {
        if self.build.is_some() {
            self.selected = None;
        }
    }

    /// With recent projects and none shown, the panel selects project 0 (`0x10097a06`).
    pub fn settle(&mut self) {
        if self.shown().is_none() && !self.projects.is_empty() {
            self.selected = Some(0);
        }
    }

    /// The build dropped as the building changes owner, whether or not it was a batch: true
    /// when there was one, whose reservation the old owner then frees.
    pub fn abort(&mut self) -> bool {
        let running = self.build.take().is_some();
        self.batch = false;
        self.settle();
        running
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

    /// Start production of the shown project (`0x100986e0`): nothing without a project, and
    /// then as [`Factory::start_project`] refuses.
    pub fn start(&mut self, batch: bool, free_minds: usize, researched: bool) -> bool {
        let Some(project) = self.shown().cloned() else { return false };
        self.start_project(project, batch, free_minds, researched)
    }

    /// Start production of `project`, which is what a build order naming its own design does:
    /// the player's panel names the shown one, and a clan's AI names one its design store
    /// picked, without touching the factory's recent projects (docs/36, "Production").
    ///
    /// `M_Task_Construct`'s start (`Behavior.dll:0x10029ba0`) refuses four things, in this
    /// order, and none of them is ore or power, which are budgets the build then waits on:
    /// no free mind (`0x10029c30`); a scheme that will not open; a chassis bigger than the
    /// factory, or of no size it knows (`0x1002a3d2`, "Robot SizedType not match"); and, for a
    /// **paid** bot alone, a part its clan's tree has not researched (`0x1002a385`, "Failed to
    /// create … due to technology"). A free bot is never asked its technology: the branch
    /// that prices it (`0x10029f13`) returns before the tree is read. `researched` is whether
    /// every part of `project` is researched in the owner's tree now.
    pub fn start_project(
        &mut self,
        project: Project,
        batch: bool,
        free_minds: usize,
        researched: bool,
    ) -> bool {
        if !self.idle() || free_minds == 0 || !construct::builds(self.size, project.chassis_size) {
            return false;
        }
        if self.free_bots <= 0 && !researched {
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
        self.settle();
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

/// The largest size class `SELECT_SMALLEST` takes: a small chassis, or a tiny one
/// (`ai.dll:0x10010b73`).
pub const SMALLEST_SIZE: u8 = 2;

/// One design in the store, as its 0x124-byte record holds it (`ai.dll:0x10010c30`, which
/// builds a real object from the scheme and reads it): the Type word at `+0x108`, the size
/// class at `+0x10c` (property `0x201`), property 54 at `+0x110`, the guns figure at `+0x114`
/// and the live top speed at `+0x118`; and the byte at `+0x104`, which says the clan may
/// build it.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    pub project: Project,
    /// Every part the scheme names, the chassis first.
    pub parts: Vec<String>,
    /// `+0x104`: every part is researched in the clan's tree ([`Store::refresh`]).
    pub researched: bool,
    pub size_class: u8,
    pub hit_points: f32,
    pub guns: f32,
    pub speed: f32,
}

impl Design {
    /// What `SELECT_BEST_COMBAT` and `SELECT_SMALLEST` rank by (`ai.dll:0x1000fc70`).
    pub fn strength(&self) -> f32 {
        parkan_sim::progression::strength(self.guns, self.hit_points)
    }
}

/// A clan's design store: every `.dat` in `UNITS\UNITS\AI\`, priced and rated against that
/// clan's research tree. See `docs/15-behaviour.md`, "Function 69 sets how sloppy the AI's
/// design pick is" and "What the store holds".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Store {
    pub designs: Vec<Design>,
}

/// Whether a clan whose tree is `catalogue` has every one of `parts`: each found in the tree
/// by its id, in the tree, open and researched (`ai.dll:0x100115f0` and `0x10011530`, which
/// walk the scheme's nodes and ask `IResearch` slots 2 and 3 of each). A part the tree does
/// not list fails it. The factory's own test of a paid bot is the same walk
/// (`Behavior.dll:0x1002a11c`, `0x10029810`), and neither asks `FULL_RESEARCH_TREE`, which is
/// the constructor's part lists' switch.
pub fn researched(catalogue: &crate::designs::Catalogue, parts: &[String]) -> bool {
    parts.iter().all(|p| catalogue.item(p).is_some_and(|i| i.in_tree() && i.available() && i.researched()))
}

impl Store {
    /// Load the store for a clan whose tree is `catalogue`, the factory's size setting the
    /// designer's grade: function 43 (`ai.dll:0x1000d54a`), which fills the records and then
    /// marks the ones the clan may build.
    ///
    /// The records stand in the order the game's own listing of the directory gave them on
    /// this install -- `preload.lda` keeps it -- which is the names' order with their letters
    /// in upper case, `23sfly1e` ahead of `23_cpt1`. Equal scores keep it.
    ///
    /// A record's Type is the file's own class word, which is what the object built from it
    /// answers (`IGameObject` slot 14, `0x10010d67`), not the Type a designer would derive.
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
        names.sort_by_key(|n| n.to_ascii_uppercase());
        let mut designs = Vec::new();
        for name in names {
            let path = format!("{AI_DIR}{name}");
            let Some(unit) = gamedir::resolve(game, &path)
                .and_then(|file| std::fs::read(file).ok())
                .and_then(|data| objects::parse_unit(&data, &path).ok())
            else {
                continue;
            };
            let Some(mut project) = design_project(&mut designer, assembly, &path, &strings) else {
                continue;
            };
            project.type_word = unit.kind;
            let rating = Node::from_unit(&unit).and_then(|design| designer.rate(assembly, &design));
            designs.push(Design {
                parts: unit.components.iter().map(|c| c.reference.member.clone()).collect(),
                researched: false,
                size_class: project.chassis_size,
                hit_points: rating.as_ref().map_or(0.0, |r| r.hit_points),
                guns: rating.as_ref().map_or(0.0, |r| r.guns),
                speed: rating.as_ref().map_or(0.0, |r| r.speed),
                project,
            });
        }
        let mut store = Store { designs };
        store.refresh(&designer.catalogue);
        Ok(store)
    }

    /// Mark the designs the clan may build (`ai.dll:0x10010f90`): each record whose byte at
    /// `+0x104` is still clear has its scheme walked against the clan's tree, and the byte set
    /// when every part is researched. **A set byte is never looked at again**, so a design
    /// once allowed stays allowed. The game runs this when function 43 loads the store, when
    /// the clan is handed its tree (SuperAI slot 21, `0x10001fa0`) and each time function 41
    /// asks for a research (`0x1000d524`) -- not when a design is picked.
    pub fn refresh(&mut self, catalogue: &crate::designs::Catalogue) {
        for d in self.designs.iter_mut().filter(|d| !d.researched) {
            d.researched = researched(catalogue, &d.parts);
        }
    }

    /// The candidates of a pick, ranked: every design whose Type **equals** the order's — not
    /// a mask (`0x10010833`) — that the clan may build (`+0x104`, `0x10010837`) and that does
    /// not carry `CLASS_BUILDING` (`0x1001083d`), scored by `mode` and sorted largest first.
    ///
    /// The six arms of the jump table at `0x10010bbc` are read one at a time, in `mode − 1`
    /// order: `SELECT_BEST_WEAPON` copies the record's `+0x114`, its guns; `SELECT_BEST_ARMOR`
    /// `+0x110`, property 54's hit points; `SELECT_BEST_RANGE` `+0x11c`, which the store's own
    /// fill never writes; `SELECT_FASTEST` `+0x118`, property 145's top speed; and
    /// `SELECT_BEST_COMBAT` and `SELECT_SMALLEST` both call the **strength formula**
    /// (`0x1000fc70`) on `+0x110` and `+0x114`.
    ///
    /// The sort is the game's own (`0x10010a37`): for each place in turn, the first of what
    /// is left that scores strictly more than any before it, exchanged with what stood there.
    /// It is not stable, and the exchange is what orders equal scores.
    ///
    /// STAND-IN: docs/15-behaviour.md#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured
    /// -- `SELECT_BEST_RANGE`'s float is one nothing fills, so it scores every design 0 and the
    /// ranking keeps the store's order; no shipped raise passes it.
    pub fn ranked(&self, type_word: u32, mode: u32) -> Vec<&Design> {
        let score = |d: &Design| match mode {
            planner::SELECT_BEST_WEAPON => d.guns,
            planner::SELECT_BEST_ARMOR => d.hit_points,
            planner::SELECT_FASTEST => d.speed,
            planner::SELECT_BEST_COMBAT | planner::SELECT_SMALLEST => d.strength(),
            _ => 0.0,
        };
        let mut ranked: Vec<&Design> = self
            .designs
            .iter()
            .filter(|d| {
                d.project.type_word == type_word
                    && d.researched
                    && d.project.type_word & parkan_sim::behaviour::BUILDING_BIT == 0
            })
            .collect();
        for place in 0..ranked.len() {
            let mut best = place;
            for i in place..ranked.len() {
                if score(ranked[i]) > score(ranked[best]) {
                    best = i;
                }
            }
            ranked.swap(place, best);
        }
        ranked
    }

    /// The design a build order picks (`ai.dll:0x100107c0`, *read*): of the ranking, not the
    /// best but the one at `draw` — `(rand() + timeGetTime()) % (spread + 1)` in the game,
    /// whose spread function 69 sets from `fDifficulty`. A draw at or past the count falls
    /// back to 0.
    ///
    /// `SELECT_SMALLEST` skips the draw (`0x10010aab`): it takes **the best-ranked design
    /// whose size class is small or tiny** (`0x10010b73`–`0x10010b96`, property `0x201` at
    /// most 2), and only with none the last of the ranking. So "smallest" is the strongest
    /// warbot of a small chassis, not the weakest design in the store.
    ///
    /// The game also works out the clan's largest factory (`0x10006820`, every
    /// `BUILDING_PLANT` on its list, property `0x201`, the maximum) and then **throws the
    /// answer away** — `or eax, 0xffffffff` clobbers it at `0x10010af5` before the size
    /// comparison that would have used it — so the drawn pick applies no size limit, and
    /// neither does this. A design too big for the factory is refused when the build starts
    /// ("Robot SizedType not match"), as it is here.
    pub fn pick(&self, type_word: u32, mode: u32, draw: usize) -> Option<&Project> {
        let ranked = self.ranked(type_word, mode);
        let chosen = if mode == planner::SELECT_SMALLEST {
            ranked.iter().find(|d| d.size_class <= SMALLEST_SIZE).or(ranked.last())
        } else {
            ranked.get(draw).or(ranked.first())
        };
        chosen.map(|d| &d.project)
    }
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
        assert!(!f.start(false, 0, true), "no mind, no build");
        assert!(f.build.is_none(), "and nothing is queued behind it");
        // The project is still shown, so a later order can start it once a mind frees.
        assert!(f.start(false, 1, true));
        assert!(f.build.is_some());
        // A second order while one runs is refused too, and again queues nothing.
        assert!(!f.start(false, 1, true));
    }

    #[test]
    fn a_paid_bot_is_refused_for_a_part_not_researched_and_a_free_one_is_not_asked() {
        // "Failed to create … due to technology" (`Behavior.dll:0x1002a385`): the start walks
        // the scheme against the clan's tree only once it knows the bot is not free.
        let mut f = factory();
        assert!(!f.start_project(project("paid"), false, 1, false), "a part is not researched");
        assert!(f.build.is_none());
        assert!(f.start_project(project("paid"), false, 1, true));
        let mut f = factory();
        f.free_bots = 1;
        assert!(f.start_project(project("free"), false, 1, false), "`FreeBotNum` spares it the test");
        assert!(f.build.as_ref().is_some_and(|b| b.construct.free));
        // And a chassis of no size the factory knows is refused whatever else is true.
        let mut f = factory();
        let mut odd = project("odd");
        odd.chassis_size = 0;
        assert!(!f.start_project(odd, false, 1, true), "Robot SizedType not match");
    }

    #[test]
    fn a_recent_project_shows_while_a_build_runs_and_the_active_one_shows_the_build() {
        let mut f = factory();
        for name in ["p1", "p2"] {
            f.prebuild(project(name));
        }
        assert!(f.start_project(project("made"), false, 1, true));
        // The panel opens on what is being made, the active project's button lit.
        f.show_production();
        assert_eq!((f.selected, f.shown().map(|p| p.name.as_str())), (None, Some("made")));
        // A recent project's button shows that design, and the build runs on.
        f.selected = Some(1);
        assert_eq!(f.shown().map(|p| p.name.as_str()), Some("p1"));
        assert_eq!(f.build.as_ref().map(|b| b.project.name.as_str()), Some("made"));
        // Opening the panel again goes back to the build.
        f.show_production();
        assert_eq!(f.shown().map(|p| p.name.as_str()), Some("made"));
        // Idle, there is nothing to go back to: the selection stays.
        f.selected = Some(1);
        f.build = None;
        f.show_production();
        assert_eq!(f.selected, Some(1));
    }

    #[test]
    fn a_capture_drops_the_build_and_the_panel_shows_project_0() {
        let mut f = factory();
        f.prebuild(project("p1"));
        assert!(f.start_project(project("enemy"), true, 1, true));
        f.show_production();
        assert!(f.abort(), "a build was running");
        assert_eq!((f.build.is_none(), f.batch), (true, false));
        assert_eq!(f.shown().map(|p| p.name.as_str()), Some("p1"));
        assert!(!f.abort(), "nothing left to drop");
    }

    const WARRIOR: u32 = 0x0100_8000;

    fn design(name: &str, size: u8, hit_points: f32, guns: f32, researched: bool) -> Design {
        let mut project = project(name);
        project.type_word = WARRIOR;
        project.chassis_size = size;
        Design { project, parts: Vec::new(), researched, size_class: size, hit_points, guns, speed: 10.0 }
    }

    fn names(ranked: &[&Design]) -> Vec<String> {
        ranked.iter().map(|d| d.project.name.clone()).collect()
    }

    /// The figures are the game's own for five of C03 M02's designs, out of the install's
    /// `preload.lda`.
    fn store() -> Store {
        Store {
            designs: vec![
                design("23_swhl1", 2, 15191.53, 970.0, true),
                design("ai_lt_10", 4, 124_610.52, 5380.0, true),
                design("lwing1", 4, 152_366.39, 201_400.0, false),
                design("m_stopper", 3, 38845.21, 1180.0, true),
                design("speed_c1", 2, 8078.25, 0.0, true),
                design("wswlk22", 2, 15881.57, 470.0, true),
            ],
        }
    }

    #[test]
    fn a_pick_ranks_only_the_designs_the_clan_has_researched() {
        let s = store();
        // `lwing1`, two 100,000-point missiles, would head any ranking; the clan has not
        // researched its parts, and the byte at `+0x104` keeps it out (`ai.dll:0x10010837`).
        assert_eq!(
            names(&s.ranked(WARRIOR, planner::SELECT_BEST_COMBAT)),
            ["ai_lt_10", "m_stopper", "23_swhl1", "wswlk22", "speed_c1"]
        );
        assert_eq!(s.pick(WARRIOR, planner::SELECT_BEST_COMBAT, 0).unwrap().name, "ai_lt_10");
        assert_eq!(s.pick(WARRIOR, planner::SELECT_BEST_COMBAT, 1).unwrap().name, "m_stopper");
        // A draw at or past the count falls back to the best.
        assert_eq!(s.pick(WARRIOR, planner::SELECT_BEST_COMBAT, 5).unwrap().name, "ai_lt_10");
        // The Type is an equality, and a store with nothing researched answers nothing.
        assert!(s.pick(0x0100_2000, planner::SELECT_BEST_COMBAT, 0).is_none());
        let mut none = store();
        none.designs.iter_mut().for_each(|d| d.researched = false);
        assert!(none.pick(WARRIOR, planner::SELECT_BEST_COMBAT, 0).is_none());
    }

    #[test]
    fn select_smallest_takes_the_strongest_design_of_a_small_chassis() {
        let s = store();
        // Not the last of the ranking, `speed_c1`: the first, best first, whose size class is
        // at most 2 (`ai.dll:0x10010b73`). The draw plays no part.
        for draw in [0, 3, 9] {
            assert_eq!(s.pick(WARRIOR, planner::SELECT_SMALLEST, draw).unwrap().name, "23_swhl1");
        }
        // With no small chassis among the candidates it is the last of the ranking.
        let large = Store { designs: s.designs.into_iter().filter(|d| d.size_class > 2).collect() };
        assert_eq!(large.pick(WARRIOR, planner::SELECT_SMALLEST, 0).unwrap().name, "m_stopper");
    }

    #[test]
    fn equal_scores_are_ordered_by_the_sorts_exchanges() {
        // A selection sort that exchanges: `a` and `b` tie below `c`, and putting `c` first
        // sends `a` behind `b`.
        let s = Store {
            designs: vec![
                design("a", 2, 100.0, 1.0, true),
                design("b", 2, 100.0, 1.0, true),
                design("c", 2, 100.0, 9.0, true),
            ],
        };
        assert_eq!(names(&s.ranked(WARRIOR, planner::SELECT_BEST_COMBAT)), ["c", "b", "a"]);
        // An unknown mode scores nothing and keeps the store's order.
        assert_eq!(names(&s.ranked(WARRIOR, planner::SELECT_BEST_RANGE)), ["a", "b", "c"]);
    }
}
