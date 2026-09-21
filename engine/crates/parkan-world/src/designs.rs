//! A warbot design, as the robot constructor offers, fits, rates, names, writes and prices
//! one. The port of `openparkan/designs.py`; see `docs/38-designs.md`. The screen that shows
//! it is `docs/37-designer.md`'s.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::control::{self, Controller};
use parkan_formats::objects::{self, DAT_MAGIC, NAME_FIELD, Record};
use parkan_formats::research::{self, Tree};
use parkan_formats::{exp, mesh, ndp, profiles};

use crate::assembly::Assembly;

/// What a slot's default part adds to its label (`iron3d.dll:0x100520bc`).
pub const DEFAULT_SUFFIX: &str = "_df";
pub const BRAIN_PREFIX: &str = "i_brn_";
pub const ARMOUR_PREFIX: &str = "i_arm_";
pub const TURRET_PREFIX: &str = "e_tur_";
pub const GUN_PREFIX: &str = "e_gun_";
/// The chassis page's prefixes, one more size a grade (`0x10048b38`).
pub const CHASSIS_PREFIXES: [&str; 4] = ["r_t", "r_l", "r_m", "r_b"];
/// A page holds at most this many rows (`0x1004878b`).
pub const PAGE_ROWS: usize = 64;
/// `Iron_3D.ini`'s section and keys the unit box's percentages are taken over (`0x1006f6d0`).
pub const TEMP_SECTION: &str = "TEMP";
pub const OFFENCE_RANGE: (f32, f32) = (0.0, 6550.0);
pub const DEFENCE_RANGE: (f32, f32) = (0.0, 24400.0);
/// The unit box's colours: labels green, figures lavender, a full payload red.
pub const LABEL_COLOUR: u32 = 0xff00_ff00;
pub const FIGURE_COLOUR: u32 = 0xffb4_b4ff;
pub const FULL_COLOUR: u32 = 0xffff_0000;
/// The unit box's labels and units (`iron3d.dll`'s strings 5069–5073, 6206, 6176).
pub const BOX_LABELS: [u32; 5] = [5069, 5070, 5071, 5072, 5073];
/// The archive every part of a written design is named in.
pub const LIBRARY: &str = "objects.rlb";
/// The classes a written design's components carry.
pub const CLASS_CHASSIS: u32 = 0;
pub const CLASS_TURRET: u32 = 1;
pub const CLASS_ARMOUR: u32 = 2;
pub const CLASS_INTERNAL: u32 = 3;
pub const CLASS_GUN: u32 = 4;
pub const CLASS_CLIP: u32 = 5;
/// The record tags of a part that hangs outside and of one fitted into a slot.
pub const EXTERNAL_TAG: &str = "EXTO";
pub const INTERNAL_TAG: &str = "INTO";
/// A unit's class words (`iron3d.dll:0x10076490`): strings 6200–6204, else 6205.
pub const UNKNOWN_WORD: u32 = 6205;
/// What an unbuilt design numbers itself (`0x100765e0`).
pub const UNBUILT: &str = "X";
/// The armour component class (docs/26-damage.md, "Armour").
pub const ARMOUR_TYPE: i32 = 27;
/// A gun's firing values: energy a shot and interval (docs/29-weapons.md).
pub const GUN_SHOT_ENERGY: usize = 2;
pub const GUN_INTERVAL: usize = 3;

/// The six tabs of both panels (docs/37-designer.md, "The tabs").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Chassis,
    Turrets,
    Weapons,
    Armour,
    Internal,
    Ammo,
}

/// One node of a design: the part, where it attaches, what hangs under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The part id, as chosen.
    pub part: String,
    /// A node of the host's mesh for a turret or gun; the host controller's slot for an
    /// internal part or clip; −1 on the chassis.
    pub attach: i32,
    pub class: u32,
    pub children: Vec<Node>,
}

impl Node {
    fn new(part: &str, attach: i32, class: u32, children: Vec<Node>) -> Node {
        Node { part: part.to_owned(), attach, class, children }
    }

    /// The node and every node under it, depth first.
    pub fn walk(&self) -> Vec<&Node> {
        let mut out = vec![self];
        for c in &self.children {
            out.extend(c.walk());
        }
        out
    }

    /// The turret, if one is fitted.
    pub fn turret(&self) -> Option<&Node> {
        self.children.iter().find(|c| c.class == CLASS_TURRET)
    }

    fn turret_mut(&mut self) -> Option<&mut Node> {
        self.children.iter_mut().find(|c| c.class == CLASS_TURRET)
    }

    /// A written design read back into its tree.
    pub fn from_unit(unit: &objects::Unit) -> Option<Node> {
        let parents = unit.parents().ok()?;
        let mut nodes: Vec<Node> = unit
            .components
            .iter()
            .map(|c| Node::new(&c.reference.member, c.attach_node, c.class_id, Vec::new()))
            .collect();
        for i in (1..nodes.len()).rev() {
            let parent = usize::try_from(parents[i]).ok()?;
            let child = nodes[i].clone();
            nodes[parent].children.insert(0, child);
        }
        nodes.into_iter().next()
    }
}

/// Where on a design a destination row puts a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Chassis,
    Turret,
    /// The gun on this turret socket.
    Gun(i32),
}

/// A destination row: the chassis itself, or a socket or slot of a part in the design, with
/// its label and the part it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub tab: Tab,
    pub host: Host,
    /// The socket node or the slot; −1 for the chassis.
    pub attach: i32,
    pub label: String,
    pub part: Option<String>,
}

/// The unit box's figures (`iron3d.dll:0x1006fc00`), in the game's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rating {
    /// Total mass and spare payload (properties 124 and 137), kg.
    pub mass: f32,
    pub spare: f32,
    /// The live top speed (property 145), m/s.
    pub speed: f32,
    /// Defence and offence before `[TEMP]`'s normalisation (177, 178).
    pub defence: f32,
    pub offence: f32,
    /// The gun total a strength is multiplied by, `IGameObject` variable `0x204`'s `+4`
    /// (docs/15, "What a strength is"), which the AI's design store reads off a real object
    /// built from the scheme. See [`Designer::rate`] for the stand-in behind it.
    pub guns: f32,
    /// The fitted radar's range (property 0x50), 0 with none.
    pub sensor_range: f32,
}

/// A figure over one of `[TEMP]`'s ranges, as the unit box prints it (`0x100700e3`).
pub fn percent(value: f32, (low, high): (f32, f32)) -> i32 {
    if value.partial_cmp(&low) != Some(std::cmp::Ordering::Greater) {
        return 0;
    }
    if value.partial_cmp(&high) != Some(std::cmp::Ordering::Less) {
        return 100;
    }
    (f64::from(100.0 / (high - low)) * f64::from(value - low)).round_ties_even() as i32
}

/// `"%-.f"`: a figure rounded to a whole number, a half to even.
pub fn whole(v: f64) -> String {
    format!("{}", v.round_ties_even() as i64)
}

impl Rating {
    /// Red, and no accepting: the spare payload is not above 0.
    pub fn full(&self) -> bool {
        self.spare.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
    }

    /// The five figures as printed, units included: weight, top speed, defence, offence,
    /// sensor range.
    pub fn lines(&self, offence: (f32, f32), defence: (f32, f32)) -> [String; 5] {
        [
            format!("{} / {} t", whole(f64::from(self.mass) * 0.001), whole(f64::from(self.spare) * 0.001)),
            format!("{} kph", whole(f64::from(self.speed) * 3.6)),
            format!("{} %", percent(self.defence, defence)),
            format!("{} %", percent(self.offence, offence)),
            format!("{} m", whole(f64::from(self.sensor_range))),
        ]
    }
}

/// One row of a part's box: its label, its value as printed, its unit.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxLine {
    pub label: String,
    pub value: String,
    pub unit: String,
}

/// The gun page a turret socket offers: `e_gun_` and the label's last two letters, cannons
/// then launchers when the last is `r` (`0x10048338`).
pub fn gun_prefixes(label: &str) -> Vec<String> {
    let l = label.to_ascii_lowercase();
    let n = l.len();
    if n < 2 {
        return Vec::new();
    }
    let (size, kind) = (&l[n - 2..n - 1], &l[n - 1..]);
    if kind == "r" {
        vec![format!("{GUN_PREFIX}{size}c"), format!("{GUN_PREFIX}{size}l")]
    } else {
        vec![format!("{GUN_PREFIX}{size}{kind}")]
    }
}

/// The chassis page for a factory of size class `grade`, 1 to 4.
pub fn chassis_prefixes(grade: usize) -> Vec<String> {
    CHASSIS_PREFIXES[..grade.min(CHASSIS_PREFIXES.len())].iter().map(|s| (*s).to_owned()).collect()
}

/// What one clan's research tree lets the constructor offer (`iron3d.dll:0x1008a780`).
#[derive(Clone, Debug)]
pub struct Catalogue {
    pub tree: Tree,
    /// `[CS] FULL_RESEARCH_TREE`: every part of a prefix is offered.
    pub full: bool,
    items: HashMap<String, usize>,
    spelling: HashMap<String, String>,
}

impl Catalogue {
    pub fn new(tree: Tree, full: bool) -> Catalogue {
        let items =
            tree.part_ids.iter().zip(&tree.part_items).map(|(p, &i)| (p.to_ascii_lowercase(), i)).collect();
        let spelling = tree.part_ids.iter().map(|p| (p.to_ascii_lowercase(), p.clone())).collect();
        Catalogue { tree, full, items, spelling }
    }

    /// The tree at `path` under the install, a clan record's `behaviour`
    /// (`MISSIONS\SCRIPTS\tut2_pl.trf`), with `FULL_RESEARCH_TREE` from `Iron_3D.ini`.
    pub fn open(game: &Path, path: &str) -> Result<Catalogue> {
        let file = parkan_formats::gamedir::resolve(game, path).with_context(|| format!("no {path}"))?;
        let tree = research::parse(&std::fs::read(&file)?, path)?;
        let full = crate::settings::value(game, "CS", "FULL_RESEARCH_TREE")
            .is_some_and(|v| v.parse::<i64>().ok().is_some_and(|n| n != 0));
        Ok(Catalogue::new(tree, full))
    }

    /// A part id as the tree spells it, `R_B_02` for `r_b_02`.
    pub fn spelling(&self, part: &str) -> String {
        self.spelling.get(&part.to_ascii_lowercase()).cloned().unwrap_or_else(|| part.to_owned())
    }

    pub fn item(&self, part: &str) -> Option<&research::Item> {
        self.items.get(&part.to_ascii_lowercase()).and_then(|&i| self.tree.items.get(i))
    }

    /// Whether a part is offered: its item in the tree and researched, or any with `full`.
    pub fn offered(&self, part: &str) -> bool {
        self.item(part).is_some_and(|i| self.full || (i.in_tree() && i.researched()))
    }

    /// The parts whose ids start with any of `prefixes`, in `TRFB` order, at most 64.
    pub fn page(&self, prefixes: &[String]) -> Vec<String> {
        self.tree
            .part_ids
            .iter()
            .filter(|p| {
                let low = p.to_ascii_lowercase();
                prefixes.iter().any(|x| low.starts_with(&x.to_ascii_lowercase()))
            })
            .filter(|p| self.offered(p))
            .take(PAGE_ROWS)
            .cloned()
            .collect()
    }

    /// The part a slot labelled `label` is filled with, when it is offered.
    pub fn default(&self, label: &str) -> Option<String> {
        let part = format!("{}{DEFAULT_SUFFIX}", label.to_ascii_lowercase());
        self.offered(&part).then_some(part)
    }

    /// A row's text: "name (code)".
    pub fn row(&self, part: &str) -> String {
        match self.item(part) {
            Some(i) => format!("{} ({})", i.name, i.code),
            None => part.to_owned(),
        }
    }
}

/// The record a part id names, and what it hangs as.
struct PartData {
    record: Option<Record>,
    controller: Option<Controller>,
    labels: Vec<String>,
}

/// Designs out of one installation, offered from one catalogue.
pub struct Designer {
    pub catalogue: Catalogue,
    /// The factory's size class the chassis page is graded by.
    pub grade: usize,
    pub offence_range: (f32, f32),
    pub defence_range: (f32, f32),
    weigher: Weigher,
    rounds: HashMap<String, Option<Round>>,
}

/// Puts units together as the engine loads them, their parts' records read once.
#[derive(Default)]
pub struct Weigher {
    parts: HashMap<String, PartData>,
}

fn read(assembly: &mut Assembly, reference: &objects::ResourceRef) -> Option<Vec<u8>> {
    assembly.archive(&reference.library)?.read_name(&reference.member).ok().map(<[u8]>::to_vec)
}

impl Designer {
    /// A designer for `catalogue`, graded for a factory of size class `grade`, with the unit
    /// box's `[TEMP]` ranges from `Iron_3D.ini`.
    ///
    /// STAND-IN: docs/38-designs.md#not-established -- that the factory record's `+0x30`, the
    /// grade, is the building's size class is derived from its readers, not its writer.
    pub fn new(game: &Path, catalogue: Catalogue, grade: usize) -> Designer {
        let temp = |key: &str, default: f32| {
            crate::settings::value(game, TEMP_SECTION, key).and_then(|v| v.parse().ok()).unwrap_or(default)
        };
        let normalise = crate::settings::value(game, TEMP_SECTION, "NORMALIZE").is_none_or(|v| v != "0");
        let (offence_range, defence_range) = if normalise {
            (
                (temp("OFFENCE_MIN", OFFENCE_RANGE.0), temp("OFFENCE_MAX", OFFENCE_RANGE.1)),
                (temp("DEFENCE_MIN", DEFENCE_RANGE.0), temp("DEFENCE_MAX", DEFENCE_RANGE.1)),
            )
        } else {
            ((0.0, 100.0), (0.0, 100.0))
        };
        Designer {
            catalogue,
            grade,
            offence_range,
            defence_range,
            weigher: Weigher::default(),
            rounds: HashMap::new(),
        }
    }

    fn data(&mut self, assembly: &mut Assembly, part: &str) -> &PartData {
        self.weigher.data(assembly, part)
    }

    /// A part's socket labels, node by node (mesh stream 10).
    pub fn labels(&mut self, assembly: &mut Assembly, part: &str) -> Vec<String> {
        self.data(assembly, part).labels.clone()
    }

    /// Every labelled slot of `part` but a brain's, filled with its `_df` part when offered
    /// (`0x10051fdb`).
    pub fn defaults(&mut self, assembly: &mut Assembly, part: &str) -> Vec<Node> {
        let gun = part.to_ascii_lowercase().starts_with(GUN_PREFIX);
        let labels: Vec<String> = self
            .data(assembly, part)
            .controller
            .as_ref()
            .map(|c| c.components.iter().map(|k| k.label.to_ascii_lowercase()).collect())
            .unwrap_or_default();
        labels
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_empty() && !l.starts_with(BRAIN_PREFIX))
            .filter_map(|(slot, label)| {
                let chosen = self.catalogue.default(label)?;
                let class = if label.starts_with(ARMOUR_PREFIX) {
                    CLASS_ARMOUR
                } else if gun {
                    CLASS_CLIP
                } else {
                    CLASS_INTERNAL
                };
                Some(Node::new(&chosen, slot as i32, class, Vec::new()))
            })
            .collect()
    }

    /// A new design from a chassis, its slots filled (`0x10051bb0`).
    pub fn chassis(&mut self, assembly: &mut Assembly, part: &str) -> Node {
        let children = self.defaults(assembly, part);
        Node::new(part, -1, CLASS_CHASSIS, children)
    }

    /// The turret on the chassis node labelled `e_tur_`, its slots filled, replacing any
    /// turret and what hangs on it.
    ///
    /// STAND-IN: docs/38-designs.md#not-established -- what the turret fit does to guns on a
    /// replaced turret is not read: they go with it.
    pub fn fit_turret(&mut self, assembly: &mut Assembly, design: &mut Node, part: &str) -> bool {
        let labels = self.labels(assembly, &design.part);
        let Some(node) = labels.iter().position(|l| l.to_ascii_lowercase().starts_with(TURRET_PREFIX)) else {
            return false;
        };
        let children = self.defaults(assembly, part);
        design.children.retain(|c| c.class != CLASS_TURRET);
        design.children.push(Node::new(part, node as i32, CLASS_TURRET, children));
        true
    }

    /// A gun on turret node `socket`, its clip slot filled, replacing the gun there.
    ///
    /// STAND-IN: docs/38-designs.md#not-established -- what swapping a gun does to its clip is
    /// not read: the old gun and its clip go.
    pub fn fit_gun(&mut self, assembly: &mut Assembly, design: &mut Node, part: &str, socket: i32) -> bool {
        let children = self.defaults(assembly, part);
        let Some(turret) = design.turret_mut() else { return false };
        turret.children.retain(|c| !(c.class == CLASS_GUN && c.attach == socket));
        turret.children.push(Node::new(part, socket, CLASS_GUN, children));
        true
    }

    /// The destination rows of `tab` for a design (docs/37, "The tabs"; docs/38, "Fitting").
    pub fn places(&mut self, assembly: &mut Assembly, design: Option<&Node>, tab: Tab) -> Vec<Place> {
        let Some(design) = design else {
            return if tab == Tab::Chassis {
                vec![Place { tab, host: Host::Chassis, attach: -1, label: String::new(), part: None }]
            } else {
                Vec::new()
            };
        };
        let slot_places = |me: &mut Designer,
                           assembly: &mut Assembly,
                           node: &Node,
                           host: Host,
                           wanted: &dyn Fn(&str) -> bool| {
            let labels: Vec<String> = me
                .data(assembly, &node.part)
                .controller
                .as_ref()
                .map(|c| c.components.iter().map(|k| k.label.to_ascii_lowercase()).collect())
                .unwrap_or_default();
            labels
                .into_iter()
                .enumerate()
                .filter(|(_, l)| !l.is_empty() && !l.starts_with(BRAIN_PREFIX) && wanted(l))
                .map(|(slot, label)| Place {
                    tab,
                    host,
                    attach: slot as i32,
                    part: node
                        .children
                        .iter()
                        .find(|c| c.attach == slot as i32 && !matches!(c.class, CLASS_TURRET | CLASS_GUN))
                        .map(|c| c.part.clone()),
                    label,
                })
                .collect::<Vec<_>>()
        };
        match tab {
            Tab::Chassis => {
                vec![Place {
                    tab,
                    host: Host::Chassis,
                    attach: -1,
                    label: String::new(),
                    part: Some(design.part.clone()),
                }]
            }
            Tab::Turrets => {
                let labels = self.labels(assembly, &design.part);
                labels
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.to_ascii_lowercase().starts_with(TURRET_PREFIX))
                    .map(|(node, label)| Place {
                        tab,
                        host: Host::Chassis,
                        attach: node as i32,
                        label: label.to_ascii_lowercase(),
                        part: design.turret().map(|t| t.part.clone()),
                    })
                    .collect()
            }
            Tab::Weapons => {
                let Some(turret) = design.turret() else { return Vec::new() };
                let labels = self.labels(assembly, &turret.part);
                labels
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| {
                        let l = l.to_ascii_lowercase();
                        l.starts_with("central_") || l.starts_with("universal_")
                    })
                    .map(|(node, label)| Place {
                        tab,
                        host: Host::Turret,
                        attach: node as i32,
                        label: label.to_ascii_lowercase(),
                        part: turret
                            .children
                            .iter()
                            .find(|c| c.class == CLASS_GUN && c.attach == node as i32)
                            .map(|c| c.part.clone()),
                    })
                    .collect()
            }
            Tab::Armour => {
                slot_places(self, assembly, design, Host::Chassis, &|l| l.starts_with(ARMOUR_PREFIX))
            }
            Tab::Internal => {
                let mut out =
                    slot_places(self, assembly, design, Host::Chassis, &|l| !l.starts_with(ARMOUR_PREFIX));
                if let Some(turret) = design.turret() {
                    out.extend(slot_places(self, assembly, turret, Host::Turret, &|l| {
                        !l.starts_with(ARMOUR_PREFIX)
                    }));
                }
                out
            }
            Tab::Ammo => {
                let Some(turret) = design.turret() else { return Vec::new() };
                let guns: Vec<Node> =
                    turret.children.iter().filter(|c| c.class == CLASS_GUN).cloned().collect();
                guns.iter()
                    .flat_map(|g| slot_places(self, assembly, g, Host::Gun(g.attach), &|_| true))
                    .collect()
            }
        }
    }

    /// The source rows a place offers (docs/38, "The catalogue").
    pub fn offers(&self, place: &Place) -> Vec<String> {
        let prefixes = match (place.tab, place.host) {
            (Tab::Chassis, _) => chassis_prefixes(self.grade),
            (Tab::Weapons, _) => gun_prefixes(&place.label),
            _ => vec![place.label.clone()],
        };
        self.catalogue.page(&prefixes)
    }

    /// Put `part` into `place` of `design`, starting the design from a chassis. False when
    /// the place takes nothing.
    pub fn fit(
        &mut self,
        assembly: &mut Assembly,
        design: &mut Option<Node>,
        place: &Place,
        part: &str,
    ) -> bool {
        match place.tab {
            Tab::Chassis => {
                *design = Some(self.chassis(assembly, part));
                return true;
            }
            Tab::Turrets => return design.as_mut().is_some_and(|d| self.fit_turret(assembly, d, part)),
            Tab::Weapons => {
                return design.as_mut().is_some_and(|d| self.fit_gun(assembly, d, part, place.attach));
            }
            _ => {}
        }
        let Some(root) = design.as_mut() else { return false };
        let host = match place.host {
            Host::Chassis => Some(root),
            Host::Turret => root.turret_mut(),
            Host::Gun(socket) => root
                .turret_mut()
                .and_then(|t| t.children.iter_mut().find(|c| c.class == CLASS_GUN && c.attach == socket)),
        };
        let Some(host) = host else { return false };
        let class = match place.tab {
            Tab::Armour => CLASS_ARMOUR,
            Tab::Ammo => CLASS_CLIP,
            _ => CLASS_INTERNAL,
        };
        host.children.retain(|c| !(c.attach == place.attach && !matches!(c.class, CLASS_TURRET | CLASS_GUN)));
        host.children.push(Node::new(part, place.attach, class, Vec::new()));
        true
    }

    /// Take the chassis, turret or gun at `place` out of the design (`0x10053a50`); the other
    /// tabs remove nothing.
    pub fn remove(design: &mut Option<Node>, place: &Place) -> bool {
        match place.tab {
            Tab::Chassis => design.take().is_some(),
            Tab::Turrets => design.as_mut().is_some_and(|d| {
                let before = d.children.len();
                d.children.retain(|c| c.class != CLASS_TURRET);
                d.children.len() != before
            }),
            Tab::Weapons => design.as_mut().and_then(Node::turret_mut).is_some_and(|t| {
                let before = t.children.len();
                t.children.retain(|c| !(c.class == CLASS_GUN && c.attach == place.attach));
                t.children.len() != before
            }),
            _ => false,
        }
    }

    /// The design's `Type`: its turret's role, an animal for an `a` chassis, else 0.
    pub fn type_word(&self, design: &Node) -> u32 {
        if design.part.to_ascii_lowercase().starts_with('a') {
            return research::TYPE_ANIMAL;
        }
        design.turret().and_then(|t| self.catalogue.item(&t.part)).map_or(0, research::Item::object_type)
    }

    /// The design as the components the constructor writes: every node's slots, then its
    /// sockets, ascending; labels "name (code)"; flags 1 (docs/38, "The files").
    pub fn components(&self, design: &Node) -> Vec<objects::Component> {
        fn put(me: &Designer, node: &Node, out: &mut Vec<objects::Component>) {
            let mut children: Vec<&Node> = node.children.iter().collect();
            children.sort_by_key(|c| (matches!(c.class, CLASS_TURRET | CLASS_GUN), c.attach));
            out.push(objects::Component {
                reference: objects::ResourceRef {
                    library: LIBRARY.to_owned(),
                    member: me.catalogue.spelling(&node.part),
                },
                label: me.catalogue.row(&node.part),
                flags: 1,
                attach_node: node.attach,
                class_id: node.class,
                child_count: children.len() as i32,
            });
            for c in children {
                put(me, c, out);
            }
        }
        let mut out = Vec::new();
        put(self, design, &mut out);
        out
    }

    /// The design as the `.dat` the constructor and the factory write.
    pub fn dat_bytes(&self, design: &Node, type_word: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&DAT_MAGIC.to_le_bytes());
        out.extend_from_slice(&type_word.to_le_bytes());
        let field = |out: &mut Vec<u8>, text: &str| {
            let mut bytes: Vec<u8> = text.chars().map(|c| u32::from(c).min(255) as u8).collect();
            bytes.truncate(NAME_FIELD - 1);
            bytes.resize(NAME_FIELD, 0);
            out.extend_from_slice(&bytes);
        };
        for c in self.components(design) {
            field(&mut out, &c.reference.library);
            field(&mut out, &c.reference.member);
            out.extend_from_slice(&c.flags.to_le_bytes());
            out.extend_from_slice(&c.attach_node.to_le_bytes());
            field(&mut out, &c.label);
            out.extend_from_slice(&c.class_id.to_le_bytes());
            out.extend_from_slice(&c.child_count.to_le_bytes());
        }
        out
    }

    /// The design put together as the engine loads it ([`Weigher::assemble`]).
    pub fn assemble(
        &mut self,
        assembly: &mut Assembly,
        components: &[objects::Component],
    ) -> Option<Assembled> {
        self.weigher.assemble(assembly, components)
    }

    /// A round's damage, range and blast: its nodes' hit points and their explosions' damage,
    /// its controller's first bound, and its first node's area blast radius
    /// (docs/29-weapons.md).
    fn round(&mut self, assembly: &mut Assembly, member: &str) -> Option<Round> {
        let key = member.to_ascii_lowercase();
        if let Some(found) = self.rounds.get(&key) {
            return *found;
        }
        let found = (|| {
            let (record, controller) = {
                let data = self.data(assembly, &key);
                (data.record.clone()?, data.controller.clone()?)
            };
            let slot = record.slot_with_suffix("ndp")?.clone();
            let rows = ndp::parse(&read(assembly, &slot)?, &slot.member).ok()?;
            let mut damage: f32 = rows.iter().map(|r| r.durability).sum();
            let mut blast = 0.0;
            for (i, row) in rows.iter().enumerate().filter(|(_, r)| r.explosion.is_set()) {
                if let Some(e) =
                    read(assembly, &row.explosion).and_then(|b| exp::parse(&b, &row.explosion.member).ok())
                {
                    damage += e.damage;
                    if i == 0 && e.kind == exp::HIT_AREA {
                        blast = e.radius;
                    }
                }
            }
            Some(Round { damage, range: controller.bounds[0], blast })
        })();
        self.rounds.insert(key, found);
        found
    }

    /// The unit box's figures for `design` (`iron3d.dll:0x1006fc00`, docs/38, "The unit box").
    pub fn rate(&mut self, assembly: &mut Assembly, design: &Node) -> Option<Rating> {
        let components = self.components(design);
        let a = self.assemble(assembly, &components)?;
        let (mass, body) = a.load();
        let spare = (a.payload + body - mass).max(0.0);
        let engines: f32 = a.of_type(control::ENGINE_TYPE).map(|d| d.values[0]).sum();
        let ratio = if a.payload != 0.0 { spare / a.payload } else { 0.0 };
        let speed = a.top_speed.min(a.top_speed * engines * (1.0 + ratio) / 2.0);
        let mut offence = 0.0;
        let guns: Vec<(String, f32)> = a
            .of_type(control::GUN_TYPE)
            .filter(|g| !g.resource.member.is_empty())
            .map(|g| (g.resource.member.clone(), g.values[GUN_INTERVAL]))
            .collect();
        for (member, interval) in guns {
            if let Some(round) = self.round(assembly, &member) {
                offence += round.damage * 1000.0 / interval.max(1.0);
            }
        }
        // STAND-IN: docs/15-behaviour.md#what-a-strength-is--read-and-measured -- the gun
        // total is `sum(a ÷ b × rounds)` over two authored figures that are not read; the
        // rounds a gun holds over its interval in seconds, as the play prices a live one.
        let guns: f32 = a
            .of_type(control::GUN_TYPE)
            .map(|g| {
                let held = g.values[parkan_sim::guns::MAGAZINE];
                let held = if held < 0.0 { 1.0 } else { held };
                held * 1000.0 / g.values[GUN_INTERVAL].max(1.0)
            })
            .sum();
        Some(Rating {
            mass,
            spare,
            speed,
            defence: a.defence(),
            offence,
            guns,
            sensor_range: a
                .of_type(control::RADAR_TYPE)
                .next()
                .map_or(0.0, |d| d.values[control::RADAR_RANGE]),
        })
    }

    /// Whether accept is allowed: a turret fitted and spare payload above 0 (`0x10050409`).
    pub fn acceptable(&mut self, assembly: &mut Assembly, design: &Node) -> bool {
        design.turret().is_some() && self.rate(assembly, design).is_some_and(|r| !r.full())
    }

    /// The three letters: size from the chassis name, chassis from its profile's
    /// `ChassisType`, class from the `Type`; `?` for any not known (`0x10076270`).
    pub fn letters(&mut self, assembly: &mut Assembly, design: &Node) -> String {
        let size = match crate::robot::chassis_size(&design.part) {
            1 => 'T',
            2 => 'S',
            3 => 'M',
            4 => 'L',
            _ => '?',
        };
        let profile = self
            .data(assembly, &design.part)
            .record
            .as_ref()
            .and_then(|r| r.slot_with_suffix("var").cloned());
        let chassis_type = profile
            .and_then(|slot| {
                assembly.archive(profiles::ARCHIVE)?.read_name(&slot.member).ok().map(<[u8]>::to_vec)
            })
            .and_then(|b| profiles::parse(&b, "var").ok())
            .and_then(|vars| vars.into_iter().find(|v| v.name == profiles::CHASSIS_TYPE))
            .map_or(0, |v| v.value as i64);
        let chassis = match chassis_type {
            1 => 'F',
            2 => 'S',
            3 => 'W',
            4 => 'T',
            5 => 'A',
            6 => 'U',
            _ => '?',
        };
        let class = match self.type_word(design) {
            research::TYPE_BUILDER => 'B',
            research::TYPE_TRANSPORT => 'T',
            research::TYPE_WARRIOR => 'W',
            research::TYPE_HQ => 'C',
            research::TYPE_HERO => 'H',
            _ => '?',
        };
        [size, chassis, class].iter().collect()
    }

    /// The design's name: "LFW-X Warrior" in the constructor (`number` 0), "LFW-2 Warrior" once
    /// built with its clan's number; `strings` is `iron3d.dll`'s string table.
    pub fn name(
        &mut self,
        assembly: &mut Assembly,
        design: &Node,
        number: u32,
        strings: &BTreeMap<u32, String>,
    ) -> String {
        let word = match self.type_word(design) {
            research::TYPE_TRANSPORT => 6200,
            research::TYPE_BUILDER => 6201,
            research::TYPE_WARRIOR => 6202,
            research::TYPE_HQ => 6203,
            research::TYPE_HERO => 6204,
            _ => UNKNOWN_WORD,
        };
        let letters = self.letters(assembly, design);
        let number = if number == 0 { UNBUILT.to_owned() } else { number.to_string() };
        format!("{letters}-{number} {}", strings.get(&word).map_or("", String::as_str))
    }

    /// What building the design costs: ore, energy, and whether every part is researched
    /// (`Behavior.dll:0x10029810`). The factory divides the ore by its efficiency.
    ///
    /// A part's two build figures are its own — `objects.dlb`'s costs, mirrored into every
    /// clan's `.trf` — so they are summed whatever the clan has researched, and the research
    /// state only says whether the design may be *offered*. For the player that is the same
    /// sum either way, because the panel never offers an unresearched part; for the AI's
    /// design store it is the difference between a build that is paid for and one that is
    /// free. The store is not research-gated: neither its fill (`ai.dll:0x10010c30`) nor
    /// `M_Task_Construct::Start` (`Behavior.dll:0x100299a0`) makes a technology query.
    pub fn price(&self, design: &Node) -> (f32, f32, bool) {
        let (mut ore, mut energy, mut ok) = (0.0, 0.0, true);
        for node in design.walk() {
            match self.catalogue.item(&node.part) {
                Some(i) => {
                    let (e, o) = i.build_cost();
                    energy += e;
                    ore += o;
                    ok &= i.in_tree() && i.researched();
                }
                None => ok = false,
            }
        }
        (ore, energy, ok)
    }

    /// A part's box: its `TRFA` rows, each value by its field, one decimal (docs/38, "A part's
    /// box"). A quoted field prints as written.
    ///
    /// STAND-IN: docs/38-designs.md#not-established -- `Epower` is not traced to a record
    /// value and prints 0.0; the properties behind `regener` (0x77), `capacity` (113),
    /// `throughput` (164), `shotnum` (0x55) and `blast` (0x76) are taken as the record values
    /// the recording's figures fit: a shield's second value and a repair unit's first, a
    /// battery's first ÷ 1000 and its power figure, a gun's or clip's magazine, and its round's
    /// first area blast radius; `Adfactor` prints 0.0; and the formatter that prints one
    /// decimal whatever the template asks is not read.
    pub fn part_box(&mut self, assembly: &mut Assembly, part: &str) -> Vec<BoxLine> {
        let Some(template) = self.catalogue.item(part).map(|i| i.template.clone()) else { return Vec::new() };
        let component = objects::Component {
            reference: objects::ResourceRef { library: LIBRARY.to_owned(), member: part.to_owned() },
            label: String::new(),
            flags: 1,
            attach_node: -1,
            class_id: 0,
            child_count: 0,
        };
        let assembled = self.assemble(assembly, std::slice::from_ref(&component));
        let first = assembled.as_ref().and_then(|a| a.devices.first().cloned());
        let controller = self.data(assembly, part).controller.clone();
        let member = first.as_ref().map(|d| d.resource.member.clone()).unwrap_or_default();
        let round = if member.is_empty() { None } else { self.round(assembly, &member) };
        let of = |class: i32| first.as_ref().filter(|d| d.type_id == class);
        let mut out = Vec::new();
        for row in parse_template(&template) {
            let value = match row.field.as_str() {
                f if f.starts_with('"') => {
                    out.push(BoxLine {
                        label: row.label,
                        value: f.trim_matches('"').to_owned(),
                        unit: row.unit,
                    });
                    continue;
                }
                "shotnum" => {
                    let shots = of(control::GUN_TYPE).map_or(0.0, |d| d.values[0]);
                    out.push(BoxLine { label: row.label, value: whole(f64::from(shots)), unit: row.unit });
                    continue;
                }
                "weight" => assembled.as_ref().map_or(0.0, |a| a.load().0 * 0.001),
                "payload" => controller.as_ref().map_or(0.0, |c| c.payload * 0.001),
                "maxspeed" => {
                    controller.as_ref().map_or(0.0, |c| c.triples[control::TRIPLE_TOP_SPEED][1] * 3.6)
                }
                "wattage" => first.as_ref().map_or(0.0, |d| {
                    if d.type_id == control::GUN_TYPE { d.values[GUN_SHOT_ENERGY] } else { d.power }
                }),
                "Frate" => of(control::GUN_TYPE).map_or(0.0, |d| 1000.0 / d.values[GUN_INTERVAL].max(1.0)),
                "range" => round.map_or(0.0, |r| r.range),
                "damage" => round.map_or(0.0, |r| r.damage),
                "blast" => round.map_or(0.0, |r| r.blast),
                "sensrange" => of(control::RADAR_TYPE).map_or(0.0, |d| d.values[control::RADAR_RANGE]),
                "density" => of(ARMOUR_TYPE).map_or(0.0, |d| d.values[0]),
                "effic" => of(control::DEFLECTOR_TYPE).map_or(0.0, |d| d.values[0] * 100.0),
                "Spower" => of(control::FIGHT_SHIELD_TYPE).map_or(0.0, |d| d.values[0]),
                "regener" => of(control::FIGHT_SHIELD_TYPE)
                    .map(|d| d.values[1])
                    .or_else(|| of(control::REPAIR_TYPE).map(|d| d.values[0]))
                    .unwrap_or(0.0),
                "capacity" => of(control::BATTERY_TYPE).map_or(0.0, |d| d.values[0] * 0.001),
                "throughput" => of(control::BATTERY_TYPE).map_or(0.0, |d| d.power),
                _ => 0.0,
            };
            out.push(BoxLine { label: row.label, value: format!("{value:.1}"), unit: row.unit });
        }
        out
    }
}

/// What a round does, as the part box and the offence read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
    pub damage: f32,
    pub range: f32,
    pub blast: f32,
}

/// One node's mass and life, as `Control.dll:0x1000fac0` sums them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeMass {
    /// Its `.ndp` flags: `0x20` the left running gear, `0x40` the right.
    pub flags: i32,
    pub density: f32,
    pub volume: f32,
    pub area: f32,
    pub life: f32,
    /// It came with the root, the chassis's own body.
    pub root: bool,
}

/// An assembled design: the root's payload and forward top speed, its merged devices and
/// nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembled {
    pub payload: f32,
    pub top_speed: f32,
    pub devices: Vec<control::Component>,
    pub nodes: Vec<NodeMass>,
}

impl Assembled {
    pub fn of_type(&self, type_id: i32) -> impl Iterator<Item = &control::Component> {
        self.devices.iter().filter(move |d| d.type_id == type_id)
    }

    /// Total mass and the root's own body mass (`Control.dll:0x1000fac0`): every node's
    /// density × volume plus armour's weight over its area, and every device's mass.
    pub fn load(&self) -> (f32, f32) {
        let per_area = self.of_type(ARMOUR_TYPE).last().map_or(0.0, |d| d.values[0]);
        let total: f32 = self.nodes.iter().map(|n| n.density * n.volume + per_area * n.area).sum::<f32>()
            + self.devices.iter().map(|d| d.mass).sum::<f32>();
        let body = self.nodes.iter().filter(|n| n.root).map(|n| n.density * n.volume).sum();
        (total, body)
    }

    /// Property 177 (`Control.dll:0x10013940`): the life of the first node with any area
    /// over armour's share kept, plus the deflector's first coefficient times the shield
    /// generator's sector maximum when the unit has both.
    pub fn defence(&self) -> f32 {
        let life = self.nodes.iter().find(|n| n.area != 0.0).map_or(0.0, |n| n.life);
        let kept = self.of_type(ARMOUR_TYPE).last().map_or(1.0, |d| d.values[1]);
        let mut value = if kept != 0.0 { life / kept } else { f32::INFINITY };
        if let (Some(shield), Some(deflector)) =
            (self.of_type(control::FIGHT_SHIELD_TYPE).next(), self.of_type(control::DEFLECTOR_TYPE).next())
        {
            value += deflector.values[0] * shield.values[0];
        }
        value
    }
}

impl Weigher {
    fn data(&mut self, assembly: &mut Assembly, part: &str) -> &PartData {
        let key = part.to_ascii_lowercase();
        if !self.parts.contains_key(&key) {
            let record = assembly.library.get(&key).cloned();
            let controller = record
                .as_ref()
                .and_then(|r| r.slot_with_suffix("ctl").cloned())
                .and_then(|slot| read(assembly, &slot).and_then(|b| control::parse(&b, &slot.member).ok()));
            let labels = record
                .as_ref()
                .and_then(|r| r.mesh().cloned())
                .and_then(|m| read(assembly, &m).and_then(|b| mesh::labels(&b, &m.member).ok()))
                .unwrap_or_default();
            self.parts.insert(key.clone(), PartData { record, controller, labels });
        }
        &self.parts[&key]
    }

    /// Each node's density, level-0 volume and area, and life, for a part's record.
    pub fn node_masses(&mut self, assembly: &mut Assembly, part: &str, root: bool) -> Vec<NodeMass> {
        let Some(record) = self.data(assembly, part).record.clone() else { return Vec::new() };
        let Some(mesh_ref) = record.mesh().cloned() else { return Vec::new() };
        let Some(loaded) = assembly.mesh(&mesh_ref) else { return Vec::new() };
        let rows = record
            .slot_with_suffix("ndp")
            .cloned()
            .and_then(|slot| read(assembly, &slot).and_then(|b| ndp::parse(&b, &slot.member).ok()))
            .unwrap_or_default();
        loaded
            .mesh
            .nodes
            .iter()
            .zip(rows)
            .map(|(node, row)| {
                let slot = loaded.mesh.slots.get(usize::from(node.slot_index[0]));
                NodeMass {
                    flags: row.flags,
                    density: row.density,
                    volume: slot.map_or(0.0, |s| s.volume),
                    area: slot.map_or(0.0, |s| s.area),
                    life: row.durability,
                    root,
                }
            })
            .collect()
    }

    /// The design put together as `AniMesh.dll` and `Control.dll` load it: the root brings
    /// every node and record, a turret or gun its nodes but its node 0 and its records, and an
    /// internal part or clip replaces the record at its slot (docs/28-chassis.md). None
    /// without a root controller.
    pub fn assemble(
        &mut self,
        assembly: &mut Assembly,
        components: &[objects::Component],
    ) -> Option<Assembled> {
        let unit = objects::Unit { kind: 0, components: components.to_vec() };
        let parents = unit.parents().ok()?;
        let mut out: Option<Assembled> = None;
        let mut first_device: HashMap<usize, usize> = HashMap::new();
        for (i, c) in components.iter().enumerate() {
            let member = c.reference.member.to_ascii_lowercase();
            let (tag, controller) = {
                let data = self.data(assembly, &member);
                (data.record.as_ref().map(|r| r.tag.clone()), data.controller.clone())
            };
            let (Some(tag), Some(controller)) = (tag, controller) else { continue };
            if i == 0 {
                out = Some(Assembled {
                    payload: controller.payload,
                    top_speed: controller.triples[control::TRIPLE_TOP_SPEED][1],
                    devices: Vec::new(),
                    nodes: Vec::new(),
                });
            }
            if out.is_none() {
                continue;
            }
            if i == 0 || tag == EXTERNAL_TAG {
                let masses = self.node_masses(assembly, &member, i == 0);
                let a = out.as_mut()?;
                first_device.insert(i, a.devices.len());
                a.devices.extend(controller.components.iter().cloned());
                a.nodes.extend(if i == 0 { masses } else { masses.into_iter().skip(1).collect() });
            } else if tag == INTERNAL_TAG
                && let Some(first) = controller.components.first()
                && let Some(&base) = usize::try_from(parents[i]).ok().and_then(|p| first_device.get(&p))
            {
                let a = out.as_mut()?;
                let slot = base as i64 + i64::from(c.attach_node);
                if let Some(d) = usize::try_from(slot).ok().and_then(|s| a.devices.get_mut(s)) {
                    *d = first.clone();
                }
            }
        }
        out
    }
}

/// One stat template row: `@G@<label>@B,<field>,G,<unit>,<width>,<decimals>@`.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateRow {
    pub label: String,
    pub field: String,
    pub unit: String,
}

/// A `TRFA` template's rows (docs/19-descriptions.md).
pub fn parse_template(text: &str) -> Vec<TemplateRow> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("@G@") {
        rest = &rest[start + 3..];
        let Some(b) = rest.find("@B,").or_else(|| rest.find("@b,")) else { break };
        let label = rest[..b].trim_end().to_owned();
        rest = &rest[b + 3..];
        let Some(end) = rest.find('@') else { break };
        let fields: Vec<&str> = rest[..end].split(',').collect();
        rest = &rest[end + 1..];
        out.push(TemplateRow {
            label,
            field: fields.first().copied().unwrap_or_default().to_owned(),
            unit: fields.get(2).copied().unwrap_or_default().to_owned(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pages_prefixes_follow_the_socket_labels_and_the_grade() {
        assert_eq!(gun_prefixes("universal_bl"), vec!["e_gun_bl"]);
        assert_eq!(gun_prefixes("universal_br"), vec!["e_gun_bc", "e_gun_bl"]);
        assert_eq!(chassis_prefixes(2), vec!["r_t", "r_l"]);
        assert_eq!(chassis_prefixes(9).len(), 4);
    }

    #[test]
    fn the_percentages_are_rounded_over_their_range_and_held_to_it() {
        assert_eq!(percent(0.0, DEFENCE_RANGE), 0);
        assert_eq!(percent(2957.0, DEFENCE_RANGE), 12);
        assert_eq!(percent(30_000.0, DEFENCE_RANGE), 100);
        assert_eq!(whole(0.5), "0");
        assert_eq!(whole(25.95), "26");
    }

    #[test]
    fn a_template_gives_its_rows_labels_fields_and_units() {
        let rows = parse_template("@G@Weight       @B,weight,G,t,5,1@\n@G@Rate of fire @B,Frate,G,1/s,5,@");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].label.as_str(), rows[0].field.as_str(), rows[0].unit.as_str()),
            ("Weight", "weight", "t")
        );
        assert_eq!((rows[1].label.as_str(), rows[1].unit.as_str()), ("Rate of fire", "1/s"));
    }

    #[test]
    fn a_written_design_reads_back_into_its_tree() {
        let comp = |member: &str, attach: i32, class: u32, children: i32| objects::Component {
            reference: objects::ResourceRef { library: LIBRARY.into(), member: member.into() },
            label: String::new(),
            flags: 1,
            attach_node: attach,
            class_id: class,
            child_count: children,
        };
        let unit = objects::Unit {
            kind: 0,
            components: vec![
                comp("r_b_02", -1, CLASS_CHASSIS, 2),
                comp("i_eng_b_df", 0, CLASS_INTERNAL, 0),
                comp("e_tur_bb_01", 7, CLASS_TURRET, 1),
                comp("e_gun_bl_15", 3, CLASS_GUN, 0),
            ],
        };
        let tree = Node::from_unit(&unit).unwrap();
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.turret().unwrap().children[0].part, "e_gun_bl_15");
        assert_eq!(tree.walk().len(), 4);
    }
}
