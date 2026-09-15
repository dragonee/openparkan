//! The warbot designer, the factory's robot constructor, as a screen: the source panel of
//! parts on the left, the project in the middle and its own slots in the destination panel
//! on the right, the buttons under the project, and the turning previews. What a design is,
//! what the pages offer and every number shown are [`crate::designs`]'. See
//! `docs/37-designer.md` and `docs/38-designs.md`.

use std::collections::BTreeMap;

use anyhow::Result;
use glam::{Mat4, Vec3};
use parkan_formats::mission::KIND_UNIT;

use super::{Cockpit, Ink, argb};
use crate::assembly::Assembly;
use crate::designs::{BoxLine, Catalogue, Designer, Node, Place, Rating, Tab};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// A second click on the selected row within this long is a double click (`0x10047f10`).
pub const DOUBLE_CLICK_MS: f64 = 200.0;
/// A tab shows at most this many rows, 23 apart from y 65, each 180 × 21.
pub const ROWS_SHOWN: usize = 6;
pub const ROW_TOP: f32 = 65.0;
pub const ROW_STEP: f32 = 23.0;
pub const ROW_WIDTH: f32 = 180.0;
pub const ROW_HEIGHT: f32 = 21.0;
/// The destination panel is the source moved right by this; a panel is 183 wide.
pub const DESTINATION_X: f32 = 457.0;
pub const PANEL_WIDTH: f32 = 183.0;
/// The panels' faint green grounds.
pub const GROUND: u32 = 0x3c32_9632;
/// The titles, the empty tab's title, the part box's missing data, and the prompt.
pub const STRING_SOURCE: u32 = 3049;
pub const STRING_DESTINATION: u32 = 3050;
pub const STRING_NO_ITEMS: u32 = 1115;
pub const STRING_NO_DATA: u32 = 5081;
pub const STRING_SELECT_CHASSIS: u32 = 1107;
pub const STRING_SELECT_TURRET: u32 = 3054;
pub const STRING_SELECT_WEAPON: u32 = 3055;
pub const STRING_TUNE_UP: u32 = 6242;
pub const HINT: [u32; 3] = [3044, 3045, 3048];
pub const YELLOW: u32 = 0xffff_ff00;
pub const GREEN: u32 = 0xff00_ff00;
pub const NO_ITEMS_GREEN: u32 = 0xff00_9600;
pub const RED: u32 = 0xffff_0000;
pub const FIGURE: u32 = 0xffb4_b4ff;
/// *Seen*: every row's text is light grey.
pub const ROW_TEXT: u32 = 0xffc8_c8c8;
pub const LAMP_COLOUR: u32 = 0xffc8_ffc8;
pub const BUTTON_ICON: u32 = 0xff9b_9bff;
pub const BUTTON_ICON_OFF: u32 = 0xff4d_4d7f;
/// A preview's camera stands K × the model's radius from its centre, K = 1 ÷ sin 30°, across
/// a field of 60°; the model is pitched by −0.5 rad and turns at 0.75 rad a second
/// (`0x1009dc10`, `0x1009f300`, `0x1009ef1c`).
pub const PREVIEW_K: f32 = 2.0;
pub const PREVIEW_FIELD: f32 = std::f32::consts::FRAC_PI_3;
pub const PREVIEW_PITCH: f32 = -0.5;
pub const PREVIEW_TURN_RATE: f32 = 0.00075;
/// The scan bands' cycle, and the strips they are drawn from.
pub const BAND_CYCLE_MS: f64 = 4000.0;

/// Tab `i`: its tooltip, frame, icon position and cut on the `icons` page, and its icon's
/// colours off, normal and selected (docs/37, "The tabs").
pub struct TabArt {
    pub tab: Tab,
    pub tooltip: u32,
    pub frame: [f32; 4],
    pub icon: [f32; 2],
    pub cut: [f32; 2],
    pub colours: [u32; 3],
}

const BLUE: [u32; 3] = [0xff40_4064, 0xff96_96e6, 0xffdc_dcff];
const TEAL: [u32; 3] = [0xff40_4b4b, 0xff78_c8c8, 0xffd2_e6e6];

pub const TABS: [TabArt; 6] = [
    TabArt {
        tab: Tab::Chassis,
        tooltip: 6216,
        frame: [8.0, 16.0, 35.0, 53.0],
        icon: [9.0, 25.0],
        cut: [48.0, 72.0],
        colours: BLUE,
    },
    TabArt {
        tab: Tab::Turrets,
        tooltip: 6218,
        frame: [35.0, 16.0, 61.0, 53.0],
        icon: [36.0, 25.0],
        cut: [72.0, 72.0],
        colours: BLUE,
    },
    TabArt {
        tab: Tab::Weapons,
        tooltip: 6219,
        frame: [61.0, 16.0, 87.0, 53.0],
        icon: [62.0, 25.0],
        cut: [96.0, 72.0],
        colours: BLUE,
    },
    TabArt {
        tab: Tab::Armour,
        tooltip: 6217,
        frame: [101.0, 16.0, 128.0, 53.0],
        icon: [102.0, 25.0],
        cut: [193.0, 97.0],
        colours: TEAL,
    },
    TabArt {
        tab: Tab::Internal,
        tooltip: 6221,
        frame: [128.0, 16.0, 154.0, 53.0],
        icon: [129.0, 25.0],
        cut: [120.0, 72.0],
        colours: TEAL,
    },
    TabArt {
        tab: Tab::Ammo,
        tooltip: 6220,
        frame: [154.0, 16.0, 180.0, 53.0],
        icon: [155.0, 25.0],
        cut: [206.0, 192.0],
        colours: TEAL,
    },
];

fn tab_index(tab: Tab) -> usize {
    TABS.iter().position(|t| t.tab == tab).unwrap_or(0)
}

/// The five buttons (docs/37, "The buttons").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Accept,
    Clear,
    Save,
    Load,
    Exit,
}

/// A button's frame, its three frame cuts on page 4 and their size, its icon position, and
/// its icon's page and cut.
pub struct ButtonArt {
    pub button: Button,
    pub tooltip: u32,
    pub frame: [f32; 4],
    pub cuts: [[f32; 2]; 3],
    pub size: [f32; 2],
    pub icon: [f32; 2],
    pub icon_page: &'static str,
    pub icon_cut: [f32; 2],
}

const ACCEPT_CUTS: [[f32; 2]; 3] = [[84.0, 169.0], [159.0, 169.0], [184.0, 44.0]];
const CLEAR_CUTS: [[f32; 2]; 3] = [[119.0, 169.0], [194.0, 169.0], [219.0, 44.0]];

pub const BUTTONS: [ButtonArt; 5] = [
    ButtonArt {
        button: Button::Accept,
        tooltip: 6222,
        frame: [194.0, 437.0, 229.0, 480.0],
        cuts: ACCEPT_CUTS,
        size: [35.0, 43.0],
        icon: [201.0, 450.0],
        icon_page: "page1",
        icon_cut: [176.0, 231.0],
    },
    ButtonArt {
        button: Button::Clear,
        tooltip: 1554,
        frame: [229.0, 437.0, 264.0, 480.0],
        cuts: CLEAR_CUTS,
        size: [35.0, 43.0],
        icon: [231.0, 450.0],
        icon_page: "page1",
        icon_cut: [200.0, 231.0],
    },
    ButtonArt {
        button: Button::Save,
        tooltip: 1555,
        frame: [269.0, 437.0, 304.0, 480.0],
        cuts: ACCEPT_CUTS,
        size: [35.0, 43.0],
        icon: [278.0, 450.0],
        icon_page: "page1",
        icon_cut: [152.0, 231.0],
    },
    ButtonArt {
        button: Button::Load,
        tooltip: 1556,
        frame: [304.0, 437.0, 339.0, 480.0],
        cuts: CLEAR_CUTS,
        size: [35.0, 43.0],
        icon: [308.0, 450.0],
        icon_page: "page1",
        icon_cut: [128.0, 231.0],
    },
    ButtonArt {
        button: Button::Exit,
        tooltip: 1557,
        frame: [405.0, 437.0, 446.0, 480.0],
        cuts: [[141.0, 213.0], [192.0, 213.0], [195.0, 88.0]],
        size: [41.0, 43.0],
        icon: [413.0, 450.0],
        icon_page: "icons",
        icon_cut: [192.0, 72.0],
    },
];

/// Which panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Source,
    Destination,
}

impl Side {
    pub fn x(self) -> f32 {
        match self {
            Side::Source => 0.0,
            Side::Destination => DESTINATION_X,
        }
    }
}

/// A row: its text, and the part or the destination place it stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub text: String,
    pub part: Option<String>,
    pub place: Option<Place>,
}

/// One panel's list: its rows, its first shown row, its selected row, and when that row
/// was last clicked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panel {
    pub rows: Vec<Row>,
    pub first: usize,
    pub selected: Option<usize>,
    pub clicked_ms: Option<f64>,
}

impl Panel {
    /// A click on row `index` at `now_ms`: whether it was a double click, a second click on
    /// the selected row within 0.2 s; a click on another row selects it.
    pub fn click(&mut self, index: usize, now_ms: f64) -> bool {
        if index >= self.rows.len() {
            return false;
        }
        let double =
            self.selected == Some(index) && self.clicked_ms.is_some_and(|t| now_ms - t <= DOUBLE_CLICK_MS);
        self.selected = Some(index);
        self.clicked_ms = (!double).then_some(now_ms);
        double
    }

    /// Scroll by one row up or down, while rows are left.
    pub fn scroll(&mut self, down: bool) {
        if down {
            if self.first + ROWS_SHOWN < self.rows.len() {
                self.first += 1;
            }
        } else {
            self.first = self.first.saturating_sub(1);
        }
    }

    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.selected?)
    }
}

/// What a preview shows: a part's record, or a design registered at a path; and whether the
/// renderer builds it as a unit's assembly or a record's mesh.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PreviewKey {
    pub kind: u32,
    pub path: String,
    /// Bumped each time the design at `path` changes, so the model is built again.
    pub version: u64,
}

/// A model view this frame: what it shows, the viewport in screen pixels, its camera, and
/// the model's matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub key: PreviewKey,
    pub viewport: [f32; 4],
    pub view_proj: Mat4,
    pub model: Mat4,
    pub lights: [Vec3; 2],
}

/// The designer open for one factory.
pub struct Session {
    pub factory: usize,
    pub designer: Designer,
    pub design: Option<Node>,
    pub tab: Tab,
    /// Which tabs can be selected, both panels alike.
    pub enabled: [bool; 6],
    pub source: Panel,
    pub destination: Panel,
    pub rating: Option<Rating>,
    pub name: String,
    pub acceptable: bool,
    pub source_box: Vec<BoxLine>,
    pub destination_box: Vec<BoxLine>,
    /// Where the design is registered for its preview, and its version.
    pub design_path: String,
    pub version: u64,
    /// The socket the destination's selected row names, in the design's frame.
    pub socket: Option<Vec3>,
    /// The design's sphere, and each part preview's.
    pub design_sphere: Option<(Vec3, f32)>,
    pub source_sphere: Option<(Vec3, f32)>,
    pub destination_sphere: Option<(Vec3, f32)>,
    /// The previews' turn, the same angle for each (a newly picked part keeps it), and when
    /// it last moved.
    pub angle: f32,
    pub turned_ms: f64,
    pub opened_ms: f64,
}

/// What the screen does after a click.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    None,
    Close,
    /// Accept: the design's bytes, its Type and name, its chassis's size and its price.
    Accept {
        bytes: Vec<u8>,
        type_word: u32,
        name: String,
        chassis_size: u8,
        ore: f32,
        power: f32,
        lines: Vec<String>,
    },
}

/// The designer screen: a session while it is open.
#[derive(Default)]
pub struct Screen {
    pub session: Option<Session>,
    /// How many designs have been accepted, for their paths.
    pub accepted: usize,
    /// How many sessions have opened, for their previews' paths.
    pub opened: usize,
}

/// The sphere about a set of points: the middle of their box, reaching the farthest.
fn sphere_of(points: impl Iterator<Item = Vec3> + Clone) -> Option<(Vec3, f32)> {
    let (lo, hi) = points.clone().fold(None, |acc: Option<(Vec3, Vec3)>, p| {
        Some(acc.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))))
    })?;
    let centre = (lo + hi) / 2.0;
    let radius = points.map(|p| p.distance(centre)).fold(0.1, f32::max);
    Some((centre, radius))
}

/// The points a part's level-0 slots draw, posed and placed by `pose`.
fn slot_points(mesh: &parkan_formats::mesh::Mesh, pose: &parkan_formats::pose::Pose) -> Vec<Vec3> {
    let posed = mesh.posed_positions();
    let mut out = Vec::new();
    for node in mesh.nodes.iter().filter(|n| !n.is_collision()) {
        let Some(slot) = node.slot_for_lod(0, 0).and_then(|s| mesh.slots.get(usize::from(s))) else {
            continue;
        };
        let first = usize::from(slot.first_triangle);
        for tri in mesh.triangles.iter().skip(first).take(usize::from(slot.triangle_count)) {
            for &v in tri {
                if let Some(p) = posed.get(usize::from(v)) {
                    let at = pose.apply(*p);
                    out.push(Vec3::new(at[0] as f32, at[1] as f32, at[2] as f32));
                }
            }
        }
    }
    out
}

/// The sphere of a record's mesh as drawn.
fn part_sphere(assembly: &mut Assembly, record: &str) -> Option<(Vec3, f32)> {
    let reference = assembly.library.record_mesh(assembly.library.get(record), 0)?;
    let loaded = assembly.mesh(&reference)?;
    let points = slot_points(&loaded.mesh, &parkan_formats::pose::IDENTITY);
    sphere_of(points.into_iter())
}

/// The sphere of an assembly's parts as drawn.
pub fn design_sphere(assembly: &mut Assembly, path: &str) -> Option<(Vec3, f32)> {
    let parts = assembly.parts(KIND_UNIT, path);
    let mut points = Vec::new();
    for part in parts {
        let Some(loaded) = assembly.mesh(&part.reference) else { continue };
        points.extend(slot_points(&loaded.mesh, &part.pose));
    }
    sphere_of(points.into_iter())
}

/// Which slot a tab is: the enabled array's index.
pub fn tab_of(index: usize) -> Tab {
    TABS[index.min(5)].tab
}

impl Session {
    /// The designer for the factory that is target `factory`, graded by its size, offering
    /// the player's clan's research tree (docs/37, "Opening and closing").
    pub fn open(play: &mut Play, factory: usize, path: String, now_ms: f64) -> Result<Session> {
        let game = play.assembly.game.clone();
        let tree = usize::try_from(play.player_clan)
            .ok()
            .and_then(|c| play.clans.get(c))
            .map(|c| c.behaviour.clone())
            .unwrap_or_default();
        let catalogue = Catalogue::open(&game, &tree)?;
        let grade = play.factories.iter().find(|f| f.target == factory).map_or(4, |f| usize::from(f.size));
        Ok(Self::with(Designer::new(&game, catalogue, grade), factory, path, now_ms, &mut play.assembly))
    }

    /// A session on `designer`; only Chassis is enabled, and it is selected.
    pub fn with(
        designer: Designer,
        factory: usize,
        path: String,
        now_ms: f64,
        assembly: &mut Assembly,
    ) -> Session {
        let mut s = Session {
            factory,
            designer,
            design: None,
            tab: Tab::Chassis,
            enabled: [true, false, false, false, false, false],
            source: Panel::default(),
            destination: Panel::default(),
            rating: None,
            name: String::new(),
            acceptable: false,
            source_box: Vec::new(),
            destination_box: Vec::new(),
            design_path: path,
            version: 0,
            socket: None,
            design_sphere: None,
            source_sphere: None,
            destination_sphere: None,
            angle: 0.0,
            turned_ms: now_ms,
            opened_ms: now_ms,
        };
        s.refresh(assembly, &BTreeMap::new());
        s
    }

    /// The rows, the boxes and the design's figures again, after a change: the destination's
    /// rows are the design's places on the tab, the source's what the destination's selected
    /// place offers.
    ///
    /// STAND-IN: docs/37-designer.md#not-established -- which destination row a tab selects as
    /// it turns on is not read: the first, as the recording shows a turret's first socket lit.
    pub fn refresh(&mut self, assembly: &mut Assembly, strings: &BTreeMap<u32, String>) {
        let places = self.designer.places(assembly, self.design.as_ref(), self.tab);
        let rows: Vec<Row> = places
            .into_iter()
            .map(|p| Row {
                text: p.part.as_ref().map(|part| self.designer.catalogue.row(part)).unwrap_or_default(),
                part: p.part.clone(),
                place: Some(p),
            })
            .collect();
        if self.destination.rows.len() != rows.len() {
            self.destination.first = 0;
        }
        self.destination.rows = rows;
        if self.destination.selected.is_none_or(|i| i >= self.destination.rows.len()) {
            self.destination.selected = (!self.destination.rows.is_empty()).then_some(0);
        }
        let offered = self
            .destination
            .selected_row()
            .and_then(|r| r.place.as_ref())
            .map(|p| self.designer.offers(p))
            .unwrap_or_default();
        let source: Vec<Row> = offered
            .into_iter()
            .map(|part| Row { text: self.designer.catalogue.row(&part), part: Some(part), place: None })
            .collect();
        if source.iter().map(|r| &r.part).ne(self.source.rows.iter().map(|r| &r.part)) {
            self.source = Panel { rows: source, ..Panel::default() };
        }
        self.source_box = match self.source.selected_row().and_then(|r| r.part.clone()) {
            Some(part) => {
                self.source_sphere = part_sphere(assembly, &part);
                self.designer.part_box(assembly, &part)
            }
            None => Vec::new(),
        };
        self.destination_box = match self.destination.selected_row().and_then(|r| r.part.clone()) {
            Some(part) => {
                self.destination_sphere = part_sphere(assembly, &part);
                self.designer.part_box(assembly, &part)
            }
            None => Vec::new(),
        };
        match self.design.clone() {
            Some(design) => {
                self.rating = self.designer.rate(assembly, &design);
                self.acceptable = self.designer.acceptable(assembly, &design);
                self.name = self.designer.name(assembly, &design, 0, strings);
                let type_word = self.designer.type_word(&design);
                assembly.register_unit(&self.design_path, self.designer.dat_bytes(&design, type_word));
                self.design_sphere = design_sphere(assembly, &self.design_path);
                self.socket = self.socket_point(assembly, &design);
            }
            None => {
                self.rating = None;
                self.acceptable = false;
                self.name.clear();
                self.design_sphere = None;
                self.socket = None;
            }
        }
        self.version += 1;
    }

    /// Where the destination's selected turret, weapon or ammunition row's socket is on the
    /// design at rest.
    fn socket_point(&self, assembly: &mut Assembly, design: &Node) -> Option<Vec3> {
        let place = self.destination.selected_row()?.place.clone()?;
        let parts = assembly.parts(KIND_UNIT, &self.design_path);
        let at = |assembly: &mut Assembly, part: &crate::assembly::Part, node: i32| -> Option<Vec3> {
            let loaded = assembly.mesh(&part.reference)?;
            let node = usize::try_from(node).ok().filter(|&n| n < loaded.mesh.nodes.len())?;
            let p = part.pose.compose(&loaded.mesh.world_pose(node)).translation;
            Some(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
        };
        match place.tab {
            Tab::Turrets => {
                let chassis = parts.iter().find(|p| p.host == -1)?;
                at(assembly, chassis, place.attach)
            }
            Tab::Weapons => {
                let turret = design.turret()?;
                let part = parts.iter().find(|p| p.record.eq_ignore_ascii_case(&turret.part))?;
                at(assembly, part, place.attach)
            }
            Tab::Ammo => {
                let crate::designs::Host::Gun(socket) = place.host else { return None };
                let part = parts.iter().find(|p| {
                    p.node == socket && p.host >= 0 && p.record.to_ascii_lowercase().starts_with("e_gun_")
                })?;
                let t = part.pose.translation;
                Some(Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32))
            }
            _ => None,
        }
    }

    /// Select tab `tab` in both panels, if it is enabled (`0x10035920`).
    pub fn select_tab(&mut self, tab: Tab, assembly: &mut Assembly, strings: &BTreeMap<u32, String>) -> bool {
        if !self.enabled[tab_index(tab)] || self.tab == tab {
            return false;
        }
        self.tab = tab;
        self.destination = Panel::default();
        self.source = Panel::default();
        self.refresh(assembly, strings);
        true
    }

    /// The tabs a fit enables: a chassis Turrets and Internal systems, and Armour when it
    /// has an armour slot; a turret Weapons and Internal systems; a weapon Ammo.
    ///
    /// STAND-IN: docs/37-designer.md#not-established -- the condition under which a chassis
    /// enables Armour (`0x10052491`) is not read: when the chassis has an armour slot.
    fn enable_after_fit(&mut self, assembly: &mut Assembly, tab: Tab) {
        match tab {
            Tab::Chassis => {
                self.enabled = [true, true, false, false, true, false];
                let armour = self.designer.places(assembly, self.design.as_ref(), Tab::Armour);
                self.enabled[tab_index(Tab::Armour)] = !armour.is_empty();
            }
            Tab::Turrets => {
                self.enabled[tab_index(Tab::Weapons)] = true;
                self.enabled[tab_index(Tab::Internal)] = true;
            }
            Tab::Weapons => self.enabled[tab_index(Tab::Ammo)] = true,
            _ => {}
        }
    }

    /// A double click from `side` on its selected row (`0x100506d0`): from the source the
    /// part is added, a chassis only to an empty project and a turret or a gun only into an
    /// empty slot; from the destination a chassis, turret or gun is removed.
    ///
    /// STAND-IN: docs/37-designer.md#not-established -- which tab the panels turn to after a
    /// fit is not read; *seen*: a chassis turns them to Turrets and a turret to Weapons.
    pub fn double_click(
        &mut self,
        side: Side,
        assembly: &mut Assembly,
        strings: &BTreeMap<u32, String>,
    ) -> bool {
        let tab = self.tab;
        let place = self.destination.selected_row().and_then(|r| r.place.clone());
        match side {
            Side::Source => {
                let Some(part) = self.source.selected_row().and_then(|r| r.part.clone()) else {
                    return false;
                };
                let Some(place) = place else { return false };
                let allowed = match tab {
                    Tab::Chassis => self.design.is_none(),
                    Tab::Turrets | Tab::Weapons => place.part.is_none(),
                    _ => true,
                };
                if !allowed || !self.designer.fit(assembly, &mut self.design, &place, &part) {
                    return false;
                }
                self.enable_after_fit(assembly, tab);
                let next = match tab {
                    Tab::Chassis => Some(Tab::Turrets),
                    Tab::Turrets => Some(Tab::Weapons),
                    _ => None,
                };
                if let Some(next) = next {
                    self.tab = next;
                    self.destination = Panel::default();
                    self.source = Panel::default();
                }
                self.refresh(assembly, strings);
                true
            }
            Side::Destination => {
                let Some(place) = place.filter(|p| p.part.is_some()) else { return false };
                if !matches!(tab, Tab::Chassis | Tab::Turrets | Tab::Weapons) {
                    return false;
                }
                if !Designer::remove(&mut self.design, &place) {
                    return false;
                }
                if self.design.is_none() {
                    self.clear(assembly, strings);
                } else {
                    self.refresh(assembly, strings);
                }
                true
            }
        }
    }

    /// Fit `part` where its name says it goes, as a double click from the source would: a
    /// chassis to the project, a turret to its socket, a gun to the first empty socket that
    /// offers it, any other part to the first place whose rows offer it. Whether it went in.
    pub fn fit_part(&mut self, part: &str, assembly: &mut Assembly, strings: &BTreeMap<u32, String>) -> bool {
        let lower = part.to_ascii_lowercase();
        let tab = if lower.starts_with("r_") {
            Tab::Chassis
        } else if lower.starts_with(crate::designs::TURRET_PREFIX) {
            Tab::Turrets
        } else if lower.starts_with(crate::designs::GUN_PREFIX) {
            Tab::Weapons
        } else if lower.starts_with(crate::designs::ARMOUR_PREFIX) {
            Tab::Armour
        } else if lower.starts_with("i_c") {
            Tab::Ammo
        } else {
            Tab::Internal
        };
        if tab != self.tab && !self.select_tab(tab, assembly, strings) && self.tab != tab {
            return false;
        }
        let empty_only = matches!(tab, Tab::Turrets | Tab::Weapons);
        let Some(place) = self.destination.rows.iter().position(|r| {
            r.place.as_ref().is_some_and(|p| {
                (!empty_only || p.part.is_none())
                    && self.designer.offers(p).iter().any(|o| o.eq_ignore_ascii_case(part))
            })
        }) else {
            return false;
        };
        self.destination.selected = Some(place);
        self.source = Panel::default();
        self.refresh(assembly, strings);
        let Some(row) = self
            .source
            .rows
            .iter()
            .position(|r| r.part.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(part)))
        else {
            return false;
        };
        self.source.selected = Some(row);
        self.double_click(Side::Source, assembly, strings)
    }

    /// Clear the project: only Chassis again (`0x100513c5`).
    pub fn clear(&mut self, assembly: &mut Assembly, strings: &BTreeMap<u32, String>) {
        self.design = None;
        self.enabled = [true, false, false, false, false, false];
        self.tab = Tab::Chassis;
        self.source = Panel::default();
        self.destination = Panel::default();
        self.refresh(assembly, strings);
    }

    /// Whether a button can be clicked: accept with a turret and a design that rates as
    /// acceptable, save with a project, the rest always.
    pub fn enabled_button(&self, button: Button) -> bool {
        match button {
            Button::Accept => self.design.as_ref().is_some_and(|d| d.turret().is_some()) && self.acceptable,
            Button::Save => self.design.is_some(),
            _ => true,
        }
    }

    /// The prompt: with no project Select chassis; then by the tab (docs/37, "The project").
    pub fn prompt(&self) -> Option<u32> {
        if self.design.is_none() {
            return Some(STRING_SELECT_CHASSIS);
        }
        match self.tab {
            Tab::Chassis => None,
            Tab::Turrets => Some(STRING_SELECT_TURRET),
            Tab::Weapons => Some(STRING_SELECT_WEAPON),
            _ => Some(STRING_TUNE_UP),
        }
    }

    /// A click at the layout point `at` at `now_ms` (`0x10055ff0`): the buttons first, then
    /// the source panel, then the destination.
    pub fn click(
        &mut self,
        at: [f32; 2],
        now_ms: f64,
        assembly: &mut Assembly,
        strings: &BTreeMap<u32, String>,
    ) -> Outcome {
        let inside = |[x0, y0, x1, y1]: [f32; 4]| (x0..=x1).contains(&at[0]) && (y0..=y1).contains(&at[1]);
        for b in &BUTTONS {
            let icon = [b.icon[0], b.icon[1], b.icon[0] + 24.0, b.icon[1] + 24.0];
            if !inside(icon) || !self.enabled_button(b.button) {
                continue;
            }
            return match b.button {
                Button::Exit => Outcome::Close,
                Button::Clear => {
                    self.clear(assembly, strings);
                    Outcome::None
                }
                // STAND-IN: docs/37-designer.md#not-established -- save's name field and load's
                // list are not built: the buttons do nothing.
                Button::Save | Button::Load => Outcome::None,
                Button::Accept => self.accept(),
            };
        }
        for side in [Side::Source, Side::Destination] {
            let x0 = side.x();
            // The tabs: a click on a tab's icon.
            for t in &TABS {
                let icon = [x0 + t.icon[0], t.icon[1], x0 + t.icon[0] + 24.0, t.icon[1] + 24.0];
                if inside(icon) {
                    self.select_tab(t.tab, assembly, strings);
                    return Outcome::None;
                }
            }
            let panel = match side {
                Side::Source => &mut self.source,
                Side::Destination => &mut self.destination,
            };
            // The scroll bar's buttons, past six rows.
            if panel.rows.len() > ROWS_SHOWN {
                if inside([x0 + 147.0, 203.0, x0 + 162.0, 224.0]) {
                    panel.scroll(false);
                    return Outcome::None;
                }
                if inside([x0 + 162.0, 203.0, x0 + 180.0, 224.0]) {
                    panel.scroll(true);
                    return Outcome::None;
                }
            }
            let shown = panel.rows.len().saturating_sub(panel.first).min(ROWS_SHOWN);
            let row = (0..shown).find(|&k| {
                let y = ROW_TOP + ROW_STEP * k as f32;
                inside([x0, y, x0 + ROW_WIDTH, y + ROW_HEIGHT])
            });
            // A click in the preview is a click on the first shown row (`0x10049cd6`).
            let row = row.or_else(|| {
                inside([x0 + 8.0, 244.0, x0 + PANEL_WIDTH - 8.0, 396.0]).then_some(0).filter(|_| shown > 0)
            });
            if let Some(k) = row {
                let index = panel.first + k;
                let double = panel.click(index, now_ms);
                if double {
                    self.double_click(side, assembly, strings);
                } else {
                    if side == Side::Destination {
                        self.source = Panel::default();
                    }
                    self.refresh(assembly, strings);
                }
                return Outcome::None;
            }
        }
        Outcome::None
    }

    /// Accept (`0x1005144c`): the design as the factory's project.
    fn accept(&mut self) -> Outcome {
        let Some(design) = self.design.clone() else { return Outcome::None };
        let type_word = self.designer.type_word(&design);
        let bytes = self.designer.dat_bytes(&design, type_word);
        let (ore, power, _) = self.designer.price(&design);
        let lines = self
            .rating
            .map(|r| r.lines(self.designer.offence_range, self.designer.defence_range).to_vec())
            .unwrap_or_default();
        Outcome::Accept {
            bytes,
            type_word,
            name: self.name.clone(),
            chassis_size: crate::robot::chassis_size(&design.part),
            ore,
            power,
            lines,
        }
    }

    /// The previews' turn at `now_ms`: 0.75 rad a second since the last draw.
    pub fn turn(&mut self, now_ms: f64) -> f32 {
        let dt = (now_ms - self.turned_ms).max(0.0) as f32;
        self.turned_ms = now_ms;
        self.angle = (self.angle + dt * PREVIEW_TURN_RATE).rem_euclid(std::f32::consts::TAU);
        self.angle
    }
}

/// A model view's camera and model matrix for a model of sphere (`centre`, `radius`) in a
/// `viewport` (docs/37, "The previews"): the camera `K × radius` back along −y, looking at
/// the origin with z up across a 60° field; the model's centre moved to the origin, turned
/// by `angle` about z and pitched by −0.5 rad.
///
/// STAND-IN: docs/37-designer.md#the-previews--read-and-seen -- which way the camera looks,
/// which axis the pitch turns about and which of the view's sides the field spans are not
/// read: from −y, about x, so the top of the model tips toward the camera, and the field
/// spans the narrower side, where a sphere 2 radii off just fits it; the radius is the one
/// about the drawn level-0 vertices.
pub fn preview_camera(centre: Vec3, radius: f32, angle: f32, viewport: [f32; 4]) -> (Mat4, Mat4) {
    let distance = PREVIEW_K * radius.max(0.1);
    let aspect = (viewport[2] / viewport[3].max(1.0)).max(0.01);
    let eye = Vec3::new(0.0, -distance, 0.0);
    let far = distance + radius * 4.0 + 1.0;
    let near = (distance - radius * 2.0).max(0.05);
    let field_y =
        if aspect < 1.0 { 2.0 * ((PREVIEW_FIELD / 2.0).tan() / aspect).atan() } else { PREVIEW_FIELD };
    let view_proj =
        Mat4::perspective_rh(field_y, aspect, far, near) * Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
    let model = Mat4::from_rotation_x(-PREVIEW_PITCH)
        * Mat4::from_rotation_z(angle)
        * Mat4::from_translation(-centre);
    (view_proj, model)
}

/// Where a layout point projects in a preview: the layout point of `point` in the model's
/// frame, or none behind the camera.
fn project(view_proj: Mat4, model: Mat4, point: Vec3, rect: [f32; 4]) -> Option<[f32; 2]> {
    let clip = view_proj * model * point.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some([rect[0] + (ndc.x + 1.0) / 2.0 * rect[2], rect[1] + (1.0 - ndc.y) / 2.0 * rect[3]])
}

/// The layout rectangles of the three previews: the source part, the destination part and
/// the project (x, y, width, height).
pub const SOURCE_PREVIEW: [f32; 4] = [12.0, 244.0, 160.0, 151.0];
pub const DESTINATION_PREVIEW: [f32; 4] = [DESTINATION_X + 12.0, 244.0, 160.0, 151.0];
pub const PROJECT_PREVIEW: [f32; 4] = [195.0, 10.0, 250.0, 290.0];

/// The designer, drawn in place of the factory screen (`0x10055dc0`), and its previews.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, now_ms: f64) -> Vec<Preview> {
    let Some(mut s) = cockpit.designer.session.take() else { return Vec::new() };
    let previews = draw_session(cockpit, ink, play, &mut s, now_ms);
    cockpit.designer.session = Some(s);
    previews
}

fn draw_session(
    cockpit: &Cockpit,
    ink: &mut Ink,
    _play: &Play,
    s: &mut Session,
    now_ms: f64,
) -> Vec<Preview> {
    // DEPARTURE: docs/35-hud.md#how-the-radar-draws--read -- the screen keeps its 640 × 480
    // shape centred on a wide window, as the HUD keeps its own, with the black ground over
    // the whole window; `--stretch-hud` stretches it as the game's does.
    ink.painter.pin = Pin::CENTRE;
    let space = ink.painter.space;
    let page = |name: &str| cockpit.pages.get(name).copied();
    let [sx, sy] = space.scales();
    let full_w = space.width / sx;
    let full_h = space.height / sy;
    ink.painter.fill(
        Blend::Alpha,
        [320.0 - full_w / 2.0, 240.0 - full_h / 2.0, full_w, full_h],
        argb(0xff00_0000),
    );
    let cut = |ink: &mut Ink, page_name: &str, src: [f32; 4], dst: [f32; 4], colour: u32| {
        if let Some(p) = page(page_name) {
            ink.painter.sprite_to(Blend::Alpha, p, src, dst, argb(colour));
        }
    };
    let pulse = 150.0 + 200.0 * ((now_ms % 250.0) / 1000.0) as f32;
    let circuit = 0xff00_0000 | ((pulse.min(255.0) as u32) << 8);
    let angle = s.turn(now_ms);
    let text_height = ink.font.line_height;

    // The panels' frames and part boxes (`0x10049ef0`).
    for side in [Side::Source, Side::Destination] {
        let x0 = side.x();
        let x1 = x0 + PANEL_WIDTH;
        ink.painter.fill(Blend::Alpha, [x0 + 11.0, 61.0, x1 - x0 - 22.0, 167.0], argb(GROUND));
        cut(ink, "page4", [0.0, 44.0, 183.0, 61.0], [x0, 0.0, 183.0, 61.0], 0xffff_ffff);
        cut(ink, "page4", [121.0, 144.0, 14.0, 16.0], [x0, 228.0, 14.0, 16.0], 0xffff_ffff);
        // Tiles from X₀ + 14 by 13, and the last one from X₁ − 27 to X₁ − 14.
        let mut x = x0 + 14.0;
        while x < x1 - 27.0 {
            cut(ink, "page4", [136.0, 145.0, 12.0, 14.0], [x, 229.0, 13.0, 14.0], 0xffff_ffff);
            x += 13.0;
        }
        cut(ink, "page4", [136.0, 145.0, 12.0, 14.0], [x1 - 27.0, 229.0, 13.0, 14.0], 0xffff_ffff);
        cut(ink, "page4", [150.0, 144.0, 14.0, 16.0], [x1 - 14.0, 228.0, 14.0, 16.0], 0xffff_ffff);
        ink.painter.fill(Blend::Alpha, [x0 + 11.0, 244.0, x1 - x0 - 22.0, 152.0], argb(GROUND));
        cut(
            ink,
            "page7",
            [128.0, 128.0, 128.0, 128.0],
            [x0 + 11.0, 244.0, x1 - x0 - 22.0, 152.0],
            0xff00_6400,
        );
        let (tw, th) = ((x1 - x0 - 22.0) / 2.0, 76.0);
        for (i, j) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            cut(
                ink,
                "page3",
                [172.0, 140.0, 64.0, 64.0],
                [x0 + 11.0 + tw * i, 244.0 + th * j, tw, th],
                circuit,
            );
        }
        cut(ink, "page4", [0.0, 144.0, 20.0, 84.0], [x0, 396.0, 20.0, 84.0], 0xffff_ffff);
        // Tiles from X₀ + 20 by 32, and the last one from X₁ − 52 to X₁ − 20.
        let mut x = x0 + 20.0;
        while x < x1 - 52.0 {
            cut(ink, "page4", [21.0, 145.0, 31.0, 83.0], [x, 397.0, 32.0, 83.0], 0xffff_ffff);
            x += 32.0;
        }
        cut(ink, "page4", [21.0, 145.0, 31.0, 83.0], [x1 - 52.0, 397.0, 32.0, 83.0], 0xffff_ffff);
        cut(ink, "page4", [54.0, 144.0, 20.0, 84.0], [x1 - 20.0, 396.0, 20.0, 84.0], 0xffff_ffff);
        // The part box's rows, or no data.
        let lines = match side {
            Side::Source => &s.source_box,
            Side::Destination => &s.destination_box,
        };
        if lines.is_empty() {
            ink.text(cockpit.string(STRING_NO_DATA), [x0 + 15.0, 415.0], RED);
        } else {
            let step = (text_height + 2.0).round();
            for (i, line) in lines.iter().enumerate() {
                let y = 415.0 + step * i as f32;
                ink.text(&line.label, [x0 + 15.0, y], GREEN);
                let value_right = x0 + 15.0 + 120.0;
                let w = ink.font.advance(&line.value);
                ink.text(&line.value, [value_right - w, y], FIGURE);
                ink.text(&line.unit, [value_right + 3.0, y], GREEN);
            }
        }
    }

    // The tabs and the rows (`0x1004bb00`).
    for side in [Side::Source, Side::Destination] {
        let x0 = side.x();
        let panel = match side {
            Side::Source => &s.source,
            Side::Destination => &s.destination,
        };
        let (title, colour) = if panel.rows.is_empty() {
            (STRING_NO_ITEMS, NO_ITEMS_GREEN)
        } else {
            (if side == Side::Source { STRING_SOURCE } else { STRING_DESTINATION }, YELLOW)
        };
        ink.text(cockpit.string(title), [x0 + 10.0, 6.0], colour);
        for (i, t) in TABS.iter().enumerate() {
            let enabled = s.enabled[i];
            let selected = s.tab == t.tab && enabled;
            let frame_cut = if selected { [27.0, 106.0, 26.0, 37.0] } else { [0.0, 106.0, 26.0, 37.0] };
            let [fx0, fy0, fx1, fy1] = t.frame;
            cut(ink, "page4", frame_cut, [x0 + fx0, fy0, fx1 - fx0, fy1 - fy0], 0xffff_ffff);
            let colour = if !enabled {
                t.colours[0]
            } else if selected {
                t.colours[2]
            } else {
                t.colours[1]
            };
            cut(
                ink,
                "icons",
                [t.cut[0], t.cut[1], 24.0, 24.0],
                [x0 + t.icon[0], t.icon[1], 24.0, 24.0],
                colour,
            );
        }
        let shown = panel.rows.len().saturating_sub(panel.first).min(ROWS_SHOWN);
        for k in 0..shown {
            let index = panel.first + k;
            let row = &panel.rows[index];
            let y = ROW_TOP + ROW_STEP * k as f32;
            cut(ink, "page4", [0.0, 22.0, 180.0, 21.0], [x0, y, ROW_WIDTH, ROW_HEIGHT], 0xffff_ffff);
            let lamp = if panel.selected == Some(index) {
                [195.0, 141.0, 17.0, 21.0]
            } else {
                [178.0, 141.0, 17.0, 21.0]
            };
            cut(ink, "page1", lamp, [x0, y, 17.0, 21.0], LAMP_COLOUR);
            ink.text(&row.text, [x0 + 10.0, y + 7.0], ROW_TEXT);
        }
        if panel.rows.len() > ROWS_SHOWN {
            cut(ink, "page1", [178.0, 141.0, 17.0, 21.0], [x0, 203.0, 17.0, 21.0], LAMP_COLOUR);
            cut(ink, "page4", [0.0, 0.0, 130.0, 21.0], [x0 + 5.0, 203.0, 142.0, 21.0], 0xffff_ffff);
            cut(ink, "page1", [216.0, 78.0, 15.0, 21.0], [x0 + 147.0, 203.0, 15.0, 21.0], 0xffff_ffff);
            cut(ink, "icons", [154.0, 0.0, 11.0, 11.0], [x0 + 148.0, 207.0, 13.0, 13.0], BUTTON_ICON);
            cut(ink, "page1", [231.0, 78.0, 18.0, 21.0], [x0 + 162.0, 203.0, 18.0, 21.0], 0xffff_ffff);
            cut(ink, "icons", [154.0, 11.0, 11.0, 11.0], [x0 + 162.0, 206.0, 13.0, 13.0], BUTTON_ICON);
        }
    }

    // The project's column (`0x1004ec50`).
    ink.painter.fill(Blend::Alpha, [195.0, 10.0, 250.0, 291.0], argb(GROUND));
    let (cx, cy) = (320.0, 155.0);
    for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let x1 = if dx > 0.0 { 445.0 } else { 195.0 };
        let y1 = if dy > 0.0 { 301.0 } else { 10.0 };
        cut(ink, "page7", [0.0, 128.0, 128.0, 128.0], [x1, y1, cx - x1, cy - y1], 0xff00_9b00);
    }
    cut(ink, "page4", [131.0, 0.0, 14.0, 11.0], [185.0, 0.0, 14.0, 11.0], 0xffff_ffff);
    let mut x = 199.0;
    while x < 441.0 {
        cut(ink, "page4", [146.0, 0.0, 8.0, 10.0], [x, 0.0, 8.0f32.min(441.0 - x), 10.0], 0xffff_ffff);
        x += 8.0;
    }
    cut(ink, "page4", [155.0, 0.0, 14.0, 11.0], [441.0, 0.0, 14.0, 11.0], 0xffff_ffff);
    for i in 0..4 {
        for j in 0..3 {
            cut(
                ink,
                "page3",
                [172.0, 140.0, 64.0, 64.0],
                [195.0 + 62.5 * i as f32, 10.0 + 97.0 * j as f32, 62.5, 97.0],
                circuit,
            );
        }
    }
    if let Some(prompt) = s.prompt() {
        let text = cockpit.string(prompt).to_owned();
        ink.centred(&text, 0.0, 640.0, 15.0, GREEN);
    }
    // The box.
    ink.painter.fill(Blend::Alpha, [202.0, 310.0, 236.0, 130.0], argb(GROUND));
    cut(ink, "page4", [74.0, 144.0, 19.0, 16.0], [185.0, 300.0, 19.0, 16.0], 0xffff_ffff);
    let mut x = 204.0;
    while x < 440.0 {
        cut(ink, "page4", [96.0, 145.0, 8.0, 13.0], [x, 301.0, 8.0f32.min(440.0 - x), 13.0], 0xffff_ffff);
        x += 8.0;
    }
    cut(ink, "page4", [105.0, 144.0, 15.0, 15.0], [440.0, 300.0, 15.0, 15.0], 0xffff_ffff);
    cut(ink, "page4", [234.0, 137.0, 9.0, 119.0], [185.0, 315.0, 9.0, 122.0], 0xffff_ffff);
    cut(ink, "page4", [75.0, 169.0, 8.0, 43.0], [185.0, 437.0, 9.0, 43.0], 0xffff_ffff);
    cut(ink, "page4", [154.0, 169.0, 5.0, 43.0], [263.0, 437.0, 6.0, 43.0], 0xffff_ffff);
    cut(ink, "page4", [75.0, 213.0, 67.0, 43.0], [338.0, 437.0, 67.0, 43.0], 0xffff_ffff);
    cut(ink, "page4", [182.0, 213.0, 9.0, 43.0], [446.0, 437.0, 9.0, 43.0], 0xffff_ffff);
    cut(ink, "page3", [41.0, 140.0, 32.0, 115.0], [195.0, 320.0, 32.0, 115.0], 0xffff_ffff);
    let mut x = 227.0;
    while x < 440.0 {
        cut(ink, "page3", [74.0, 140.0, 64.0, 115.0], [x, 320.0, 64.0f32.min(440.0 - x), 115.0], 0xffff_ffff);
        x += 64.0;
    }
    cut(ink, "page3", [139.0, 140.0, 32.0, 115.0], [423.0, 320.0, 32.0, 115.0], 0xffff_ffff);
    match (&s.design, s.rating) {
        (Some(_), Some(rating)) => {
            ink.text(&s.name, [220.0, 331.0], YELLOW);
            let lines = rating.lines(s.designer.offence_range, s.designer.defence_range);
            let (first, step) = ((text_height + 3.0).round(), (text_height + 2.0).round());
            for (i, (label, line)) in crate::designs::BOX_LABELS.iter().zip(lines.iter()).enumerate() {
                let y = 331.0 + first + step * i as f32;
                ink.text(cockpit.string(*label), [220.0, y], GREEN);
                let (value, unit) = line.rsplit_once(' ').unwrap_or((line.as_str(), ""));
                let colour = if i == 0 && rating.full() { RED } else { FIGURE };
                let w = ink.font.advance(value);
                ink.text(value, [352.0 - w, y], colour);
                ink.text(unit, [355.0, y], GREEN);
            }
        }
        _ => {
            let step = (text_height + 1.0).round();
            for (i, id) in HINT.iter().enumerate() {
                let text = cockpit.string(*id).to_owned();
                let w = ink.font.advance(&text);
                ink.text(&text, [(325.0 - w / 2.0).floor(), 341.0 + step * i as f32], GREEN);
            }
        }
    }
    // The buttons.
    for b in &BUTTONS {
        let enabled = s.enabled_button(b.button);
        let [bx0, by0, bx1, by1] = b.frame;
        let c = b.cuts[0];
        cut(ink, "page4", [c[0], c[1], b.size[0], b.size[1]], [bx0, by0, bx1 - bx0, by1 - by0], 0xffff_ffff);
        let colour = if enabled { BUTTON_ICON } else { BUTTON_ICON_OFF };
        cut(
            ink,
            b.icon_page,
            [b.icon_cut[0], b.icon_cut[1], 24.0, 24.0],
            [b.icon[0], b.icon[1], 24.0, 24.0],
            colour,
        );
    }

    // The previews.
    let lights = [Vec3::new(-1.0, 0.0, -1.0).normalize(), Vec3::new(1.0, 0.0, -1.0).normalize()];
    let mut previews = Vec::new();
    let viewport = |rect: [f32; 4]| {
        let [px, py] = space.pixel([rect[0], rect[1]], Pin::CENTRE);
        [px, py, rect[2] * sx, rect[3] * sy]
    };
    let mut project_camera = None;
    if let (Some(_), Some((centre, radius))) = (&s.design, s.design_sphere) {
        let vp = viewport(PROJECT_PREVIEW);
        let (view_proj, model) = preview_camera(centre, radius, angle, vp);
        project_camera = Some((view_proj, model));
        previews.push(Preview {
            key: PreviewKey { kind: KIND_UNIT, path: s.design_path.clone(), version: s.version },
            viewport: vp,
            view_proj,
            model,
            lights,
        });
    }
    for (side, rect, sphere) in [
        (Side::Source, SOURCE_PREVIEW, s.source_sphere),
        (Side::Destination, DESTINATION_PREVIEW, s.destination_sphere),
    ] {
        let panel = match side {
            Side::Source => &s.source,
            Side::Destination => &s.destination,
        };
        let (Some(part), Some((centre, radius))) =
            (panel.selected_row().and_then(|r| r.part.clone()), sphere)
        else {
            continue;
        };
        let vp = viewport(rect);
        let (view_proj, model) = preview_camera(centre, radius, angle, vp);
        previews.push(Preview {
            key: PreviewKey { kind: parkan_formats::mission::KIND_ROCK, path: part, version: 0 },
            viewport: vp,
            view_proj,
            model,
            lights,
        });
    }

    // The callout from the destination's selected row to its socket (`0x1004f3ed`).
    if let (Some((view_proj, model)), Some(socket), Some(index)) =
        (project_camera, s.socket, s.destination.selected)
    {
        let k = index.saturating_sub(s.destination.first);
        if matches!(s.tab, Tab::Turrets | Tab::Weapons | Tab::Ammo)
            && index >= s.destination.first
            && k < ROWS_SHOWN
            && let Some([px, py]) = project(view_proj, model, socket, PROJECT_PREVIEW)
        {
            let r = ROW_TOP + ROW_STEP * k as f32 + 10.0;
            let py = py.clamp(10.0, 301.0);
            let px = px.clamp(195.0, 445.0);
            let green = argb(GREEN);
            ink.painter.over = true;
            ink.painter.line(Blend::Alpha, [DESTINATION_X, r], [445.0, r], 1.0, green);
            ink.painter.line(Blend::Alpha, [427.0, r.clamp(10.0, 301.0)], [427.0, py], 1.0, green);
            ink.painter.line(Blend::Alpha, [(px + 16.0).min(445.0), py], [427.0, py], 1.0, green);
            for (a, b) in [
                ([px - 15.0, py - 15.0], [px + 15.0, py - 15.0]),
                ([px + 15.0, py - 15.0], [px + 15.0, py + 15.0]),
                ([px + 15.0, py + 15.0], [px - 15.0, py + 15.0]),
                ([px - 15.0, py + 15.0], [px - 15.0, py - 15.0]),
            ] {
                ink.painter.line(Blend::Alpha, a, b, 1.0, green);
            }
            ink.painter.over = false;
        }
    }

    // The scan bands, over the views (`0x1009f750`).
    let cycle = (now_ms - s.opened_ms).rem_euclid(BAND_CYCLE_MS);
    ink.painter.over = true;
    for (rect, start) in [
        ([185.0, 10.0, 455.0, 304.0], 0.0),
        ([1.0, 241.0, 182.0, 396.0], 2000.0),
        ([458.0, 241.0, 639.0, 396.0], 2000.0),
    ] {
        let t = ((cycle - start) / 1000.0) as f32;
        if !(0.0..1.0).contains(&t) {
            continue;
        }
        let [x0, y0, x1, y1] = rect;
        let h = y1 - y0;
        let b = h * 5.0 / 57.0;
        let top = (y0 + (h + b) * t - b).max(y0);
        let bottom = (y0 + (h + b) * t).min(y1);
        if bottom <= top {
            continue;
        }
        let g = 254.0 * (1.0 - 2.0 * (t - 0.5).abs());
        let strip = [0.0, 202.0 + 17.0 * (((now_ms / 16.0) as u64 % 3) as f32), 163.0, 16.0];
        // STAND-IN: docs/37-designer.md#the-scan-bands--read-and-seen -- the band's green
        // specular is taken as an added colour of (⅔g, g, ⅔g).
        if let Some(p) = page("page5") {
            ink.painter.sprite_to(
                Blend::Alpha,
                p,
                strip,
                [x0, top, x1 - x0, bottom - top],
                argb(0xff00_9b00),
            );
            let add = [g * 2.0 / 3.0 / 255.0, g / 255.0, g * 2.0 / 3.0 / 255.0, 1.0];
            ink.painter.sprite_to(Blend::Add, p, strip, [x0, top, x1 - x0, bottom - top], add);
        }
    }
    ink.painter.over = false;
    previews
}

/// The layout point a window pixel is at, as the designer lays itself out.
pub fn layout_point(space: crate::hud::Space, pixel: [f32; 2]) -> [f32; 2] {
    space.layout(pixel, Pin::CENTRE)
}

impl Screen {
    /// Whether the designer is up.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// Open the designer for the factory that is target `factory`, unless it is open
    /// (`0x10055c70`).
    pub fn open(&mut self, play: &mut Play, factory: usize, strings: &BTreeMap<u32, String>) -> Result<()> {
        if self.session.is_some() {
            return Ok(());
        }
        self.opened += 1;
        let path = format!("UNITS\\designer_{}.dat", self.opened);
        let now = play.hero.time_ms;
        let mut session = Session::open(play, factory, path, now)?;
        session.refresh(&mut play.assembly, strings);
        self.session = Some(session);
        Ok(())
    }

    /// Close it (exit, Esc or accept): the factory screen comes back.
    pub fn close(&mut self) {
        self.session = None;
    }

    /// A click at the layout point `at`: a button, a tab, a row. Accept hands the factory a
    /// project, registered in memory under its own path, and closes (docs/37, "What each
    /// does").
    pub fn click(&mut self, play: &mut Play, at: [f32; 2], strings: &BTreeMap<u32, String>) {
        let now = play.hero.time_ms;
        let Some(session) = self.session.as_mut() else { return };
        match session.click(at, now, &mut play.assembly, strings) {
            Outcome::None => {}
            Outcome::Close => self.close(),
            Outcome::Accept { bytes, type_word, name, chassis_size, ore, power, lines } => {
                let factory = session.factory;
                self.accepted += 1;
                let logic = play.factories.iter().find(|f| f.target == factory).map_or(0, |f| f.logic_id);
                let path = format!("UNITS\\bld_unit_{logic}_{}.dat", self.accepted);
                play.assembly.register_unit(&path, bytes);
                let sphere = design_sphere(&mut play.assembly, &path).map(|(c, r)| (c.to_array(), r));
                if let Some(f) = play.factories.iter_mut().find(|f| f.target == factory) {
                    f.accept(crate::factory::Project {
                        path,
                        name,
                        type_word,
                        chassis_size,
                        ore,
                        power,
                        lines,
                        sphere,
                    });
                }
                self.close();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(n: usize) -> Panel {
        Panel {
            rows: (0..n)
                .map(|i| Row { text: format!("row {i}"), part: Some(format!("p{i}")), place: None })
                .collect(),
            ..Panel::default()
        }
    }

    #[test]
    fn a_second_click_on_the_selected_row_within_a_fifth_of_a_second_is_a_double_click() {
        let mut p = rows(3);
        assert!(!p.click(1, 1000.0), "the first click selects");
        assert_eq!(p.selected, Some(1));
        assert!(p.click(1, 1150.0), "again within 0.2 s");
        assert!(!p.click(1, 1200.0), "a double click does not start another");
        assert!(!p.click(1, 1500.0) && !p.click(2, 1600.0), "too late, or another row");
        assert!(!p.click(2, 1900.0), "0.3 s later");
        assert!(!p.click(7, 1950.0) && p.selected == Some(2), "no such row");
    }

    #[test]
    fn a_list_scrolls_one_row_at_a_time_while_rows_are_left() {
        let mut p = rows(8);
        p.scroll(false);
        assert_eq!(p.first, 0);
        p.scroll(true);
        p.scroll(true);
        p.scroll(true);
        assert_eq!(p.first, 2, "six shown of eight");
    }

    #[test]
    fn a_previews_sphere_fits_its_view_whatever_its_turn() {
        let (centre, radius) = (Vec3::new(3.0, -2.0, 1.0), 5.0);
        for viewport in [[0.0, 0.0, 250.0, 290.0], [0.0, 0.0, 160.0, 151.0]] {
            for angle in [0.0, 1.0, 2.5] {
                let (view_proj, model) = preview_camera(centre, radius, angle, viewport);
                let world = model * centre.extend(1.0);
                assert!(world.truncate().length() < 1e-4, "the centre is moved to the origin");
                for d in [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::X, -Vec3::Y, -Vec3::Z] {
                    // A point on the sphere, turned as the model is, stays inside the view.
                    let clip = view_proj * (world.truncate() + d * radius * 0.99).extend(1.0);
                    let ndc = clip.truncate() / clip.w;
                    assert!(
                        clip.w > 0.0 && ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0,
                        "{viewport:?} {d} {ndc}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_callout_projects_through_the_project_views_camera() {
        let (view_proj, model) = preview_camera(Vec3::ZERO, 4.0, 0.0, [0.0, 0.0, 250.0, 290.0]);
        let [x, y] = project(view_proj, model, Vec3::ZERO, PROJECT_PREVIEW).unwrap();
        assert!(
            (x - 320.0).abs() < 1e-3 && (y - 155.0).abs() < 1e-3,
            "the centre at the view's middle: {x} {y}"
        );
    }
}
