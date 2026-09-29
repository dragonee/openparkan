//! The warbot designer, the factory's robot constructor, as a screen: the source panel of
//! parts on the left, the project in the middle and its own slots in the destination panel
//! on the right, the buttons under the project, and the turning previews. What a design is,
//! what the pages offer and every number shown are [`crate::designs`]'. See
//! `docs/37-designer.md` and `docs/38-designs.md`.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use glam::{Mat4, Vec3};
use parkan_formats::mission::KIND_UNIT;
use parkan_formats::objects;

use super::{Cockpit, Ink, argb};
use crate::assembly::Assembly;
use crate::designs::{
    BoxLine, CLASS_ARMOUR, CLASS_CLIP, CLASS_GUN, CLASS_INTERNAL, CLASS_TURRET, Designer, Host, Node, Place,
    Rating, Tab,
};
use crate::hud::{Blend, Layer, Pin};
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
/// A row's text colour.
///
/// STAND-IN: docs/37-designer.md#the-rows--read-and-seen -- the game hands the font a
/// gradient down the glyph, `0xffc8c8c8`, white, `0xffc8c8c8` on a selected row and
/// `0xff323296`, white, `0xff9696fa` on the rest (`iron3d.dll:0x10046c5e`); a text run here
/// takes one colour, the light grey the recording reads.
pub const ROW_TEXT: u32 = 0xffc8_c8c8;
/// Where a part box row's value ends and its unit starts, from the row's x (`0x1006ebe5`,
/// `0x1006ec01`).
pub const PART_BOX_VALUE_RIGHT: f32 = 125.0;
pub const PART_BOX_UNIT: f32 = 128.0;
pub const LAMP_COLOUR: u32 = 0xffc8_ffc8;
pub const BUTTON_ICON: u32 = 0xff9b_9bff;
pub const BUTTON_ICON_OFF: u32 = 0xff4d_4d7f;
/// A preview's camera stands K × the model's radius from the origin, K = 1 ÷ sin 30°, across
/// a field of 60°; the model is pitched by −0.5 rad about y and turns clockwise, seen from
/// above, at 0.75 rad a second (`0x1009dc10`, `0x1009ec9d`, `0x1009ef1c`).
pub const PREVIEW_K: f32 = 2.0;
pub const PREVIEW_FIELD: f32 = std::f32::consts::FRAC_PI_3;
pub const PREVIEW_PITCH: f32 = -0.5;
pub const PREVIEW_TURN_RATE: f32 = 0.00075;
/// A preview's two lights, in the model's own frame: the view sets both directions every
/// draw through the model's placement (`0x1009f087`–`0x1009f114`, space 2), so they turn
/// with it; and their colour, (2, 2, 2) each (`0x1009e888`).
pub const PREVIEW_LIGHTS: [[f32; 3]; 2] = [[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0]];
pub const PREVIEW_LIGHT_COLOUR: f32 = 2.0;
/// Save's name field (`+0x1444`, laid out at `0x1004eaad`): its rectangle, the most
/// characters it holds, and the room a character needs to be taken: the text so far must be
/// narrower than the field less 30 (`0x100459a4`–`0x100459af`).
pub const FIELD: [f32; 4] = [270.0, 410.0, 370.0, 430.0];
pub const FIELD_MAX: usize = 16;
pub const FIELD_ROOM: f32 = FIELD[2] - FIELD[0] - 30.0;
/// "Type the name of the designed warbot...", centred on x 320 at the view's foot plus 25
/// (`0x1005020e`–`0x10050332`).
pub const STRING_TYPE_NAME: u32 = 3056;
pub const TYPE_NAME_Y: f32 = 326.0;
/// The field's text: magenta while it takes characters, else pale blue between dark blue
/// (`0x10045597`–`0x10045639`); the caret, a 5-wide bar after the text for the first half of
/// every second (`0x10045721`–`0x10045812`).
pub const FIELD_ACTIVE: u32 = 0xffff_00ff;
pub const FIELD_IDLE: u32 = 0xffc8_c8ff;
pub const CARET: u32 = 0xff96_00ff;
/// Load's rows (`0x10045bd0`, placed at `0x100512e3`): 242 × 21 from x 200, the first at the
/// box's top plus 20 and each 22 below, 4 shown at a time (`0x100503a0`).
pub const LIST_ROW: [f32; 2] = [200.0, 320.0];
pub const LIST_ROW_SIZE: [f32; 2] = [242.0, 21.0];
pub const LIST_ROW_STEP: f32 = 22.0;
pub const LIST_SHOWN: usize = 4;
/// Load's scroll control (`0x10046080` at y 410): a lamp at x 230, its bar from 235 to 377,
/// and the up and down buttons whose 13 × 13 icons take the click (`0x10035a30`).
pub const LIST_SCROLL_Y: f32 = 410.0;
pub const LIST_UP: [f32; 4] = [378.0, 413.0, 391.0, 426.0];
pub const LIST_DOWN: [f32; 4] = [392.0, 413.0, 405.0, 426.0];
/// A load row's text: a gradient down the glyph from `0xff326496` to `0xff96c8fa`, halfway
/// `0xff9696c8` (`0x10045fac`–`0x10045fcb`).
///
/// STAND-IN: docs/37-designer.md#the-rows--read-and-seen -- a text run here takes one colour:
/// the gradient's middle.
pub const LIST_TEXT: u32 = 0xff96_96c8;
/// The files load leaves out: any whose name holds one of these (`strstr`,
/// `0x100511b3`–`0x100511f5`).
pub const LEFT_OUT: [&str; 3] = ["bld_unit_", "view_unit_", "temp_unit"];

/// Save's name field.
#[derive(Clone, Debug, PartialEq)]
pub struct NameField {
    pub text: String,
    /// Taking characters (`+0x14`). Enter clears it, and a click on the field turns it over
    /// (`0x10045820`); once it is clear the next takt saves and the field goes.
    pub active: bool,
    /// When the caret's blink last started (`+0x10`).
    pub stamp_ms: f64,
}

impl NameField {
    /// A character typed into the field (`0x100458f0`), `width` measuring text as the game
    /// font sets it: backspace takes the last character off, Enter stops the typing, and any
    /// other character the holder passes on — the C library's printable ones
    /// (`0x10055fbd`) — is added while fewer than 16 are there and the text so far fits.
    /// Whether the typing stopped.
    pub fn key(&mut self, c: char, width: impl Fn(&str) -> f32) -> bool {
        match c {
            '\u{8}' => {
                self.text.pop();
            }
            '\r' | '\n' => {
                self.active = false;
                return true;
            }
            c if (' '..='~').contains(&c)
                && self.text.chars().count() < FIELD_MAX
                && width(&self.text) < FIELD_ROOM =>
            {
                self.text.push(c);
            }
            _ => {}
        }
        false
    }
}

/// Load's list of saved designs, by the name they were saved under.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadList {
    pub names: Vec<String>,
    /// The first shown (`+0xad6c`).
    pub first: usize,
}

impl LoadList {
    /// How many rows show: four, or what is left from the first.
    pub fn shown(&self) -> usize {
        self.names.len().saturating_sub(self.first).min(LIST_SHOWN)
    }

    /// Up one row, or down one while more than four are left (`0x10050f7a`, `0x10051007`).
    pub fn scroll(&mut self, down: bool) {
        if down {
            if self.first + LIST_SHOWN < self.names.len() {
                self.first += 1;
            }
        } else {
            self.first = self.first.saturating_sub(1);
        }
    }

    /// The layout rectangle of shown row `k`.
    pub fn row(k: usize) -> [f32; 4] {
        let y = LIST_ROW[1] + LIST_ROW_STEP * k as f32;
        [LIST_ROW[0], y, LIST_ROW[0] + LIST_ROW_SIZE[0], y + LIST_ROW_SIZE[1]]
    }
}

/// The scan bands' cycle, and the strips they are drawn from.
pub const BAND_CYCLE_MS: f64 = 4000.0;
/// The colour of every texel of the three `ui_tex5` strips the bands draw, (255, 221, 255):
/// their pattern is in their alpha alone (*measured*, `tests/test_designs.py`).
pub const BAND_STRIP: [f32; 3] = [1.0, 221.0 / 255.0, 1.0];

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
    /// The two lights' directions in the model's own frame, which the draw turns through
    /// `model` ([`preview_lights`]).
    pub lights: [Vec3; 2],
    /// Drawn flat in this colour, with no lights: the placement ghost (docs/32).
    pub paint: Option<[f32; 3]>,
}

/// The designer open for one factory.
pub struct Session {
    pub factory: usize,
    pub designer: Designer,
    pub design: Option<Node>,
    pub tab: Tab,
    /// Which tabs can be selected, both panels alike.
    pub enabled: [bool; 6],
    /// Each tab's own selected destination row, as the destination panel keeps one per tab
    /// (`+0xcec8` + k × `0xcea4`); the tab on shows its row in `destination`.
    pub remembered: [Option<usize>; 6],
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
    /// Save's name field (`+0xad7e`) and load's list (`+0xad7d`), while either is up.
    pub field: Option<NameField>,
    pub list: Option<LoadList>,
}

/// What the screen does after a click.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    None,
    Close,
    /// Save: the design written as `units/<name>.dat` (`0x10050c28`).
    Save {
        name: String,
        bytes: Vec<u8>,
    },
    /// Load was clicked: the screen lists the saved designs ([`Session::show_list`]).
    List,
    /// A load row was clicked: the design saved under this name (`0x10050c86`).
    Load(String),
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
    /// The game's `units/`, whose designs load lists (docs/37, "The buttons").
    pub units: Option<std::path::PathBuf>,
    /// Where save writes `<name>.dat`, which load lists too. With none the designs saved are
    /// only kept here, as they always also are.
    ///
    /// DEPARTURE: docs/37-designer.md#the-buttons--read-and-seen -- the game writes a design
    /// into its own `units/`, which an install need not let the player write; the engine
    /// writes it to a folder of the player's own, and `--save-to-game` to `units/`.
    pub saves: Option<std::path::PathBuf>,
    /// The designs saved while the engine runs, by name.
    pub saved: Vec<(String, Vec<u8>)>,
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
pub fn part_sphere(assembly: &mut Assembly, record: &str) -> Option<(Vec3, f32)> {
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
        let catalogue = play.catalogue().context("the player's clan has no research tree")?;
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
            remembered: [None; 6],
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
            field: None,
            list: None,
        };
        s.refresh(assembly, &BTreeMap::new());
        s
    }

    /// The rows, the boxes and the design's figures again, after a change: the destination's
    /// rows are the design's places on the tab, the source's what the destination's selected
    /// place offers. A tab with no row selected yet starts at its first, as the panel's builder
    /// sets it (`0x1004c9fe`).
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

    /// Select tab `tab` in both panels, if it is enabled (`0x10035920`). Turning a tab on
    /// touches no row (`0x10049e50`): the destination shows the row that tab had selected last.
    pub fn select_tab(&mut self, tab: Tab, assembly: &mut Assembly, strings: &BTreeMap<u32, String>) -> bool {
        if !self.enabled[tab_index(tab)] || self.tab == tab {
            return false;
        }
        self.remembered[tab_index(self.tab)] = self.destination.selected;
        self.tab = tab;
        self.destination = Panel { selected: self.remembered[tab_index(tab)], ..Panel::default() };
        self.source = Panel::default();
        self.refresh(assembly, strings);
        self.show_selected();
        true
    }

    /// Whether the design gives tab `tab` any destination rows.
    fn has_rows(&mut self, assembly: &mut Assembly, tab: Tab) -> bool {
        !self.designer.places(assembly, self.design.as_ref(), tab).is_empty()
    }

    /// Turn tab `tab` on in both panels at its first row, when the design gives it rows.
    fn open_tab(&mut self, assembly: &mut Assembly, tab: Tab) -> bool {
        let rows = self.has_rows(assembly, tab);
        if rows {
            self.enabled[tab_index(tab)] = true;
            self.remembered[tab_index(tab)] = Some(0);
        }
        rows
    }

    /// What a fit into tab `tab` does to the tabs (docs/37, "Which tab and row a fit
    /// leaves"), and the tab it turns both panels to, if any:
    /// - a chassis (`0x10051f3a`–`0x100524ae`) turns Turrets, Internal systems and Armour on,
    ///   each only when the chassis gives it a row, at its first row, and turns the panels to
    ///   Turrets when it is on;
    /// - a turret (`0x10052912`–`0x100529ac`, `0x10052c24`) turns Internal systems and, when
    ///   it has a gun socket, Weapons on at their first rows, and the panels to Weapons;
    /// - a gun turns Ammo on at its first row (`0x10053407`–`0x10053425`).
    fn after_fit(&mut self, assembly: &mut Assembly, tab: Tab) -> Option<Tab> {
        match tab {
            Tab::Chassis => {
                self.enabled = [true, false, false, false, false, false];
                self.remembered = [None; 6];
                let turrets = self.open_tab(assembly, Tab::Turrets);
                self.open_tab(assembly, Tab::Internal);
                self.open_tab(assembly, Tab::Armour);
                turrets.then_some(Tab::Turrets)
            }
            Tab::Turrets => {
                self.open_tab(assembly, Tab::Internal);
                self.open_tab(assembly, Tab::Weapons).then_some(Tab::Weapons)
            }
            Tab::Weapons => {
                self.enabled[tab_index(Tab::Ammo)] = true;
                self.remembered[tab_index(Tab::Ammo)] = Some(0);
                None
            }
            _ => None,
        }
    }

    /// What taking a turret or a gun off leaves of the tabs its parts had rows on: each such
    /// tab left with no rows is turned off in both panels, and one that keeps rows starts again
    /// at its first (`0x10053f0e`–`0x100540ea`, `0x1005432d`–`0x100543a5`).
    fn after_removal(&mut self, assembly: &mut Assembly, tab: Tab) {
        let touched: &[Tab] = match tab {
            Tab::Turrets => &[Tab::Internal, Tab::Weapons, Tab::Ammo],
            Tab::Weapons => &[Tab::Ammo],
            _ => &[],
        };
        for &t in touched {
            if self.has_rows(assembly, t) {
                self.remembered[tab_index(t)] = Some(0);
            } else {
                self.enabled[tab_index(t)] = false;
                self.remembered[tab_index(t)] = None;
            }
        }
    }

    /// A double click from `side` on its selected row (`0x100506d0`): from the source the
    /// part is added, a chassis only to an empty project and a turret or a gun only into an
    /// empty slot; from the destination a chassis, turret or gun is removed. Every fit and
    /// every removal steps its own tab's selection to the next row, wrapping round
    /// (`0x10052754`, `0x10052ec6`, `0x100530e9`, `0x100536c6`, `0x10053966`; `0x10054170`,
    /// `0x10054414`).
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
                // The tab the fit was made on steps to its next row, so the sockets of a
                // turret, the armour, the systems and the clips fill one after the other and
                // the source panel offers the next slot's parts without another click.
                self.step_destination();
                if let Some(next) = self.after_fit(assembly, tab) {
                    self.remembered[tab_index(tab)] = self.destination.selected;
                    self.tab = next;
                    self.destination =
                        Panel { selected: self.remembered[tab_index(next)], ..Panel::default() };
                    self.source = Panel::default();
                }
                self.refresh(assembly, strings);
                self.show_selected();
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
                    self.after_removal(assembly, tab);
                    self.step_destination();
                    self.refresh(assembly, strings);
                    self.show_selected();
                }
                true
            }
        }
    }

    /// The destination's list scrolled so that its selected row shows.
    fn show_selected(&mut self) {
        if let Some(row) = self.destination.selected {
            self.destination.first = self.destination.first.clamp(row.saturating_sub(ROWS_SHOWN - 1), row);
        }
    }

    /// The destination's selection moved on to the next row, wrapping round at the last, with
    /// the list scrolled to keep it in view.
    fn step_destination(&mut self) {
        let rows = self.destination.rows.len();
        if rows == 0 {
            return;
        }
        let next = self.destination.selected.map_or(0, |row| (row + 1) % rows);
        self.destination.selected = Some(next);
        self.destination.clicked_ms = None;
        self.destination.first = self.destination.first.clamp(next.saturating_sub(ROWS_SHOWN - 1), next);
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
        self.remembered = [None; 6];
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

    /// Whether a button is drawn bright: as [`Session::enabled_button`], but with a project and
    /// the name field or the load list up, accept and save are drawn bright whatever the
    /// design, the numbers' routine that would say otherwise not being run (`0x1004f2ea`–
    /// `0x1004f306`, `0x10050409`).
    pub fn drawn_enabled(&self, button: Button) -> bool {
        let over = self.design.is_some() && (self.field.is_some() || self.list.is_some());
        (over && matches!(button, Button::Accept | Button::Save)) || self.enabled_button(button)
    }

    /// A click at the layout point `at` at `now_ms` (`0x10055ff0`, `0x100504d0`). While the
    /// name field is up a click on it turns its typing over, and while the list is up a click
    /// takes a shown row or a scroll button; neither lets a click reach the buttons, and one
    /// that falls elsewhere goes on to the panels. Otherwise the buttons come first, then the
    /// source panel, then the destination.
    pub fn click(
        &mut self,
        at: [f32; 2],
        now_ms: f64,
        assembly: &mut Assembly,
        strings: &BTreeMap<u32, String>,
    ) -> Outcome {
        let inside = |[x0, y0, x1, y1]: [f32; 4]| (x0..=x1).contains(&at[0]) && (y0..=y1).contains(&at[1]);
        if let Some(field) = self.field.as_mut() {
            // Strictly inside (`0x100504e5`–`0x10050512`).
            let [x0, y0, x1, y1] = FIELD;
            if at[0] > x0 && at[0] < x1 && at[1] > y0 && at[1] < y1 {
                field.active = !field.active;
                field.stamp_ms = now_ms;
                return if field.active { Outcome::None } else { self.save() };
            }
        } else if let Some(list) = self.list.as_mut() {
            // The control test is the cursor in [x0, x1) × [y0, y1) (`0x10035920`).
            let within = |[x0, y0, x1, y1]: [f32; 4]| at[0] >= x0 && at[0] < x1 && at[1] >= y0 && at[1] < y1;
            if let Some(k) = (0..list.shown()).find(|&k| within(LoadList::row(k))) {
                return Outcome::Load(list.names[list.first + k].clone());
            }
            if within(LIST_UP) || within(LIST_DOWN) {
                list.scroll(within(LIST_DOWN));
                return Outcome::None;
            }
        } else {
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
                    // Save opens the name field, empty and typing (`0x100510b4`–`0x1005112d`).
                    Button::Save => {
                        self.field = Some(NameField { text: String::new(), active: true, stamp_ms: now_ms });
                        Outcome::None
                    }
                    Button::Load => Outcome::List,
                    Button::Accept => self.accept(),
                };
            }
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

    /// The name field done with: the design written under the field's text, the same writer
    /// accept uses (`0x10050c28`), and the field gone (`0x10050c2d`).
    fn save(&mut self) -> Outcome {
        let Some(field) = self.field.take() else { return Outcome::None };
        let Some(design) = self.design.clone() else { return Outcome::None };
        let type_word = self.designer.type_word(&design);
        Outcome::Save { name: field.text, bytes: self.designer.dat_bytes(&design, type_word) }
    }

    /// A character while the name field is up (`0x10055fa0`); Enter saves.
    pub fn key(&mut self, c: char, width: impl Fn(&str) -> f32) -> Outcome {
        let done = self.field.as_mut().is_some_and(|field| field.key(c, width));
        if done { self.save() } else { Outcome::None }
    }

    /// Esc (`0x10055e80`): it drops the load list or the name field, unsaved, while one is up,
    /// and closes the designer otherwise. Whether the designer stays.
    pub fn escape(&mut self) -> bool {
        if self.list.take().is_some() {
            return true;
        }
        self.field.take().is_some()
    }

    /// Put up the load list from the designs saved under `files`' names, when any of them
    /// qualifies (`0x10051153`–`0x1005139a`): a name holding `bld_unit_`, `view_unit_` or
    /// `temp_unit` is left out, and `World3D.dll`'s `stdGetValidRobots` keeps a file that
    /// reads as a design (`0x10014e5d`–`0x10014f07`) whose chassis is no bigger than the
    /// factory builds (`0x10014f19`–`0x10014fc2`) and whose every part is in the tree and
    /// researched (`0x1001505b`–`0x1001511c`).
    pub fn show_list(&mut self, files: &[(String, objects::Unit)]) {
        let sizes = crate::designs::chassis_prefixes(self.designer.grade);
        let names: Vec<String> = files
            .iter()
            .filter(|(name, _)| {
                let low = name.to_ascii_lowercase();
                !LEFT_OUT.iter().any(|w| low.contains(w))
            })
            .filter(|(_, unit)| {
                Node::from_unit(unit).is_some_and(|tree| {
                    let chassis = tree.part.to_ascii_lowercase();
                    sizes.iter().any(|p| chassis.starts_with(&p.to_ascii_lowercase()))
                        && self.designer.price(&tree).2
                })
            })
            .map(|(name, _)| name.clone())
            .collect();
        self.list = (!names.is_empty()).then_some(LoadList { names, first: 0 });
    }

    /// Load a saved design (`0x10050cef`–`0x10050efc`, `0x10055190`): the list goes, the
    /// project is cleared, and the file's parts are fitted again in its own order, each through
    /// the same add a double click makes, into the row the loader picks for it — the chassis;
    /// a turret into the socket it names; the chassis's armour and every internal system into
    /// the next row of their tab, counted over the file; a gun into its socket; a clip into
    /// the first ammunition row of its gun from the last one used. A part not in the tree is
    /// passed over (`0x100552c8`). Both panels then turn to Chassis (`0x10050ecb`–`0x10050ee4`).
    pub fn load(
        &mut self,
        unit: &objects::Unit,
        assembly: &mut Assembly,
        strings: &BTreeMap<u32, String>,
    ) -> bool {
        self.list = None;
        self.clear(assembly, strings);
        let Some(tree) = Node::from_unit(unit) else { return false };
        if !self.replay(Tab::Chassis, &|_, _| true, &tree.part, assembly, strings) {
            return false;
        }
        let (mut armour, mut internal, mut clip) = (0, 0, 0);
        for child in &tree.children {
            match child.class {
                CLASS_TURRET => {
                    let at = child.attach;
                    self.replay(Tab::Turrets, &|_, p| p.attach == at, &child.part, assembly, strings);
                    for part in &child.children {
                        match part.class {
                            CLASS_GUN => {
                                let at = part.attach;
                                self.replay(
                                    Tab::Weapons,
                                    &|_, p| p.attach == at,
                                    &part.part,
                                    assembly,
                                    strings,
                                );
                                for c in part.children.iter().filter(|c| c.class == CLASS_CLIP) {
                                    let (gun, from) = (Host::Gun(at), clip);
                                    let wanted = |i: usize, p: &Place| i >= from && p.host == gun;
                                    let rows =
                                        self.designer.places(assembly, self.design.as_ref(), Tab::Ammo);
                                    if let Some(i) = rows.iter().enumerate().position(|(i, p)| wanted(i, p)) {
                                        clip = i;
                                    }
                                    self.replay(Tab::Ammo, &wanted, &c.part, assembly, strings);
                                }
                            }
                            CLASS_INTERNAL => {
                                let k = internal;
                                self.replay(Tab::Internal, &|i, _| i == k, &part.part, assembly, strings);
                                internal += 1;
                            }
                            _ => {}
                        }
                    }
                }
                CLASS_ARMOUR => {
                    let k = armour;
                    self.replay(Tab::Armour, &|i, _| i == k, &child.part, assembly, strings);
                    armour += 1;
                }
                CLASS_INTERNAL => {
                    let k = internal;
                    self.replay(Tab::Internal, &|i, _| i == k, &child.part, assembly, strings);
                    internal += 1;
                }
                _ => {}
            }
        }
        self.select_tab(Tab::Chassis, assembly, strings);
        true
    }

    /// Fit `part` into the row of `tab` that `wanted` picks, as the loader's add does: the
    /// tab's row set, the part picked in the source, and a double click from there.
    fn replay(
        &mut self,
        tab: Tab,
        wanted: &dyn Fn(usize, &Place) -> bool,
        part: &str,
        assembly: &mut Assembly,
        strings: &BTreeMap<u32, String>,
    ) -> bool {
        if self.designer.catalogue.item(part).is_none() || !self.enabled[tab_index(tab)] {
            return false;
        }
        if self.tab != tab {
            self.remembered[tab_index(self.tab)] = self.destination.selected;
            self.tab = tab;
            self.destination = Panel { selected: self.remembered[tab_index(tab)], ..Panel::default() };
        }
        let rows = self.designer.places(assembly, self.design.as_ref(), tab);
        let Some(row) = rows.iter().enumerate().position(|(i, p)| wanted(i, p)) else { return false };
        self.destination.selected = Some(row);
        self.source = Panel::default();
        self.refresh(assembly, strings);
        let Some(offered) = self
            .source
            .rows
            .iter()
            .position(|r| r.part.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(part)))
        else {
            return false;
        };
        self.source.selected = Some(offered);
        self.double_click(Side::Source, assembly, strings)
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
/// `viewport` (docs/37, "The previews").
///
/// The camera is a `CCamera` placed once at (−K × radius, 0, 0) with no turn
/// (`iron3d.dll:0x1009ec03`–`0x1009ec61`), so it looks along +x with z up, and its 60° is the
/// view's field, which spans the view's width (`Terrain.dll:0x100848a0` → the view's slot
/// 10; docs/10, "The sun and the moon are drawn").
///
/// The placement each draw sets (`0x1009efea`–`0x1009f060`) is P · T(c) · R · T(−c): R the
/// turn, a quaternion of `angle` about +z made into a matrix that turns the model **clockwise
/// seen from above** (`Ngi32.dll:0x10014540`), about its own centre; then P, the frame the
/// camera routine built (`0x1009ec66`–`0x1009ed6c`), the pitch of −0.5 rad about y with −c in
/// its translation column. So the turn is the model's own, under the pitch, and the centre
/// lands not at the origin but at (P's rotation − I) · c. The radius is the one about the
/// drawn level-0 vertices.
pub fn preview_camera(centre: Vec3, radius: f32, angle: f32, viewport: [f32; 4]) -> (Mat4, Mat4) {
    let distance = PREVIEW_K * radius.max(0.1);
    let aspect = (viewport[2] / viewport[3].max(1.0)).max(0.01);
    let eye = Vec3::new(-distance, 0.0, 0.0);
    let far = distance + radius * 4.0 + 1.0;
    let near = (distance - radius * 2.0).max(0.05);
    let field_y = 2.0 * ((PREVIEW_FIELD / 2.0).tan() / aspect).atan();
    let view_proj =
        Mat4::perspective_rh(field_y, aspect, far, near) * Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
    let pitch = Mat4::from_translation(-centre) * Mat4::from_rotation_y(PREVIEW_PITCH);
    let turn =
        Mat4::from_translation(centre) * Mat4::from_rotation_z(-angle) * Mat4::from_translation(-centre);
    (view_proj, pitch * turn)
}

/// The tint that draws a scan band's strip as the device lit it, `g` being the specular's
/// green, 0 to 254: the strip times the diffuse `0xff009b00` plus the specular (⅔g, g, ⅔g),
/// held to 1, over the strip's own colour [`BAND_STRIP`].
pub fn band_tint(g: f32) -> [f32; 4] {
    let diffuse = argb(0xff00_9b00);
    let specular = [g * 2.0 / 3.0 / 255.0, g / 255.0, g * 2.0 / 3.0 / 255.0];
    let [r, gg, b] = [0, 1, 2].map(|i| (BAND_STRIP[i] * diffuse[i] + specular[i]).min(1.0) / BAND_STRIP[i]);
    [r, gg, b, 1.0]
}

/// The directions a preview's two lights travel this draw: [`PREVIEW_LIGHTS`] through the
/// model's matrix, normalised.
pub fn preview_lights(model: Mat4) -> [Vec3; 2] {
    PREVIEW_LIGHTS.map(|d| model.transform_vector3(Vec3::from_array(d)).normalize_or(Vec3::NEG_Z))
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
            // The row draw (`0x1006ea50`): the label at x, the value right-aligned to end at
            // x + 90 + 35, the unit at x + 128, x being X₀ + 15.
            let step = (text_height + 2.0).round();
            for (i, line) in lines.iter().enumerate() {
                let y = 415.0 + step * i as f32;
                let x = x0 + 15.0;
                ink.text(&line.label, [x, y], GREEN);
                let value_right = x + PART_BOX_VALUE_RIGHT;
                let w = ink.font.advance(&line.value);
                ink.text(&line.value, [(value_right - w).round(), y], FIGURE);
                ink.text(&line.unit, [x + PART_BOX_UNIT, y], GREEN);
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
        // A tab under the cursor arms its tooltip (`0x10047c10`); a button's the same
        // (`0x10035a60`, `0x10035af9`), each on the square a click must land on.
        let cursor = cockpit.cursor_at(space, Pin::CENTRE);
        for (i, t) in TABS.iter().enumerate() {
            let [ix, iy] = [x0 + t.icon[0], t.icon[1]];
            cockpit.tip.hand([ix, iy, ix + 24.0, iy + 24.0], cursor, t.tooltip);
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
    // The black inner panel and the numbers only while neither the name field nor the load
    // list is up (`0x1004f1db`, `0x1004f2ea`), and the hint while the list is not
    // (`0x1004faba`).
    let over = s.field.is_some() || s.list.is_some();
    if !over {
        cut(ink, "page3", [41.0, 140.0, 32.0, 115.0], [195.0, 320.0, 32.0, 115.0], 0xffff_ffff);
        let mut x = 227.0;
        while x < 440.0 {
            cut(
                ink,
                "page3",
                [74.0, 140.0, 64.0, 115.0],
                [x, 320.0, 64.0f32.min(440.0 - x), 115.0],
                0xffff_ffff,
            );
            x += 64.0;
        }
        cut(ink, "page3", [139.0, 140.0, 32.0, 115.0], [423.0, 320.0, 32.0, 115.0], 0xffff_ffff);
    }
    match (&s.design, s.rating) {
        _ if s.list.is_some() || (s.design.is_some() && over) => {}
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
    // A load row, and the empty one behind the name field (`+0x146c`): page 1's long bar
    // and the dark lamp (`0x10045e60`). The lamp's lit states come and go with a click, which
    // loads at once here.
    let row_art = |ink: &mut Ink, y: f32| {
        let [w, h] = LIST_ROW_SIZE;
        cut(ink, "page1", [14.0, 57.0, 242.0, 21.0], [LIST_ROW[0], y, w, h], 0xffff_ffff);
        cut(ink, "page1", [178.0, 141.0, 17.0, 21.0], [LIST_ROW[0], y, 17.0, h], 0xffff_ffff);
    };
    if let Some(field) = &s.field {
        // The name field (`0x10050202`–`0x10050349`): the prompt, the row, and the field's
        // text centred in it, drawn with no frame (`0x100454e0` handed 0), and the caret.
        let prompt = cockpit.string(STRING_TYPE_NAME).to_owned();
        ink.centred(&prompt, 0.0, 640.0, TYPE_NAME_Y, GREEN);
        row_art(ink, FIELD[1]);
        let [x0, y0, x1, y1] = FIELD;
        let w = ink.font.advance(&field.text);
        let a = ((y1 - y0 - text_height) / 2.0).floor();
        let x = x0 + ((x1 - x0 - w) / 2.0).floor();
        // STAND-IN: docs/37-designer.md#the-rows--read-and-seen -- idle, the field's text is
        // a gradient from `0xff323264` through `0xffc8c8ff` halfway; a text run takes the
        // middle.
        let colour = if field.active { FIELD_ACTIVE } else { FIELD_IDLE };
        ink.text(&field.text, [x, y0 + a], colour);
        if field.active && (now_ms - field.stamp_ms).rem_euclid(1000.0) < 500.0 {
            let end = x0 + ((x1 - x0) / 2.0).floor() + (w / 2.0).floor();
            ink.painter.fill(Blend::Alpha, [end + 1.0, y0 + a, 5.0, y1 - y0 - 2.0 * a], argb(CARET));
        }
    } else if let Some(list) = &s.list {
        // The load list (`0x100503a0`–`0x10050404`): the shown rows, each name centred on
        // x 320, and the scroll control (`0x100464c0`).
        for k in 0..list.shown() {
            let [_, y0, _, y1] = LoadList::row(k);
            row_art(ink, y0);
            let name = &list.names[list.first + k];
            let w = ink.font.advance(name);
            ink.text(
                name,
                [(320.0 - w / 2.0).floor(), y0 + ((y1 - y0 - text_height) / 2.0).floor()],
                LIST_TEXT,
            );
        }
        let y = LIST_SCROLL_Y;
        cut(ink, "page1", [178.0, 141.0, 17.0, 21.0], [230.0, y, 17.0, 21.0], 0xffff_ffff);
        cut(ink, "page4", [0.0, 0.0, 130.0, 21.0], [235.0, y, 142.0, 21.0], 0xffff_ffff);
        cut(ink, "page1", [216.0, 78.0, 15.0, 21.0], [377.0, y, 15.0, 21.0], 0xffff_ffff);
        cut(ink, "icons", [154.0, 0.0, 11.0, 11.0], [LIST_UP[0], LIST_UP[1], 13.0, 13.0], BUTTON_ICON);
        cut(ink, "page1", [231.0, 78.0, 18.0, 21.0], [392.0, y, 18.0, 21.0], 0xffff_ffff);
        cut(ink, "icons", [154.0, 11.0, 11.0, 11.0], [LIST_DOWN[0], LIST_DOWN[1], 13.0, 13.0], BUTTON_ICON);
    }

    // The buttons.
    let cursor = cockpit.cursor_at(space, Pin::CENTRE);
    for b in &BUTTONS {
        cockpit.tip.hand([b.icon[0], b.icon[1], b.icon[0] + 24.0, b.icon[1] + 24.0], cursor, b.tooltip);
        let enabled = s.drawn_enabled(b.button);
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
    let lights = PREVIEW_LIGHTS.map(Vec3::from_array);
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
            paint: None,
        });
    }
    // STAND-IN: docs/37-designer.md#the-rows--read-and-seen -- which model the destination's
    // preview keeps is read in part: a picked row with no part leaves the last model up
    // (`0x1004d0dd`), a tab turned on at an empty row drops it (`0x1004ce12`), and the fits
    // write the previews too, which is not traced; here each preview shows its selected
    // row's part.
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
            paint: None,
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
            ink.painter.layer = Layer::OverViews;
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
            ink.painter.layer = Layer::UnderViews;
        }
    }

    // The scan bands, over the views (`0x1009f750`).
    let cycle = (now_ms - s.opened_ms).rem_euclid(BAND_CYCLE_MS);
    ink.painter.layer = Layer::OverViews;
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
        // The quad goes out in phase 1, the strip times its diffuse `0xff009b00`, and the device
        // adds its specular (⅔g, g, ⅔g) to that, held to 1, before blending by the strip's
        // alpha (docs/37, "The scan bands"). Every texel of the three strips is BAND_STRIP, so
        // that sum is one colour, which the painter's tint gives the strip exactly.
        if let Some(p) = page("page5") {
            ink.painter.sprite_to(Blend::Alpha, p, strip, [x0, top, x1 - x0, bottom - top], band_tint(g));
        }
    }
    ink.painter.layer = Layer::UnderViews;
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
        // Opening writes 9 into the flags of the view the world is drawn from (`0x10055c9b`):
        // 8 keeps the world from being drawn behind the designer, and the whole word is
        // replaced, the infrared's `0x20` with it. On a building's screen that view is the
        // hero's camera, so its night sight is off once the designer closes (docs/36, "The
        // designer does not pause the world"). In command mode it is the command camera's.
        if matches!(play.mode(), crate::play::Mode::Factory(_)) {
            play.hero.pilot.switches.infrared = false;
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

    /// Whether the designer takes the mouse and Esc: while it is up and the mission is being
    /// played, the game's state word 4 (`0x10055ea6`, `0x10056004`; docs/34, "After the
    /// outcome"). Once it is won or lost they pass on, and the outcome panel is what shows.
    pub fn takes_input(&self, play: &Play) -> bool {
        self.is_open() && play.progression.as_ref().is_none_or(|p| p.progress.outcome.is_none())
    }

    /// Esc: the load list or the name field goes, or the designer closes (`0x10055e80`).
    pub fn escape(&mut self) {
        if !self.session.as_mut().is_some_and(Session::escape) {
            self.close();
        }
    }

    /// A character typed while the name field is up, measured by the game font.
    pub fn key(&mut self, c: char, font: &crate::text::GameFont) {
        let Some(session) = self.session.as_mut() else { return };
        let outcome = session.key(c, |t| font.advance(t));
        self.keep(outcome);
    }

    /// A design saved, kept by name, and written to the saves' folder when there is one. A
    /// name the field took with a path separator in it is kept here only.
    fn keep(&mut self, outcome: Outcome) {
        let Outcome::Save { name, bytes } = outcome else { return };
        if let Some(dir) = self.saves.as_ref().filter(|_| !name.contains(['/', '\\'])) {
            let path = dir.join(format!("{name}.dat"));
            if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, &bytes)) {
                eprintln!("cannot save {}: {e}", path.display());
            }
        }
        self.saved.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
        self.saved.push((name, bytes));
    }

    /// The saved designs by name: the `.dat` files in `units/` and then in the saves' folder
    /// that read as designs, and the ones kept here, in name order.
    pub fn saved_designs(&self) -> Vec<(String, objects::Unit)> {
        let mut out: Vec<(String, objects::Unit)> = Vec::new();
        let mut add = |name: String, bytes: &[u8]| {
            if let Ok(unit) = objects::parse_unit(bytes, &name) {
                out.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
                out.push((name, unit));
            }
        };
        let dirs = [&self.units, &self.saves];
        for entries in dirs.into_iter().flatten().filter_map(|d| std::fs::read_dir(d).ok()) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_suffix(".dat").or_else(|| n.strip_suffix(".DAT")))
                else {
                    continue;
                };
                if let Ok(bytes) = std::fs::read(&path) {
                    add(name.to_owned(), &bytes);
                }
            }
        }
        for (name, bytes) in &self.saved {
            add(name.clone(), bytes);
        }
        out.sort_by_key(|(n, _)| n.to_ascii_uppercase());
        out
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
            outcome @ Outcome::Save { .. } => self.keep(outcome),
            Outcome::List => {
                let files = self.saved_designs();
                if let Some(session) = self.session.as_mut() {
                    session.show_list(&files);
                }
            }
            Outcome::Load(name) => {
                let files = self.saved_designs();
                let unit = files.into_iter().find(|(n, _)| *n == name).map(|(_, u)| u);
                if let (Some(session), Some(unit)) = (self.session.as_mut(), unit) {
                    session.load(&unit, &mut play.assembly, strings);
                }
            }
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
    fn the_name_field_takes_sixteen_printable_characters_while_they_fit_and_enter_ends_it() {
        let mut f = NameField { text: String::new(), active: true, stamp_ms: 0.0 };
        // Each character 4 wide: the text so far must stay under 70 (`0x100459aa`).
        let width = |t: &str| 4.0 * t.chars().count() as f32;
        for c in "Warbot Mk.2\u{7}é".chars() {
            assert!(!f.key(c, width));
        }
        assert_eq!(f.text, "Warbot Mk.2", "a bell and a letter past ASCII are not printable");
        f.key('\u{8}', width);
        assert_eq!(f.text, "Warbot Mk.");
        for _ in 0..20 {
            f.key('x', width);
        }
        assert_eq!(f.text.len(), FIELD_MAX, "sixteen at most");
        // Wide characters stop sooner: the 18th unit of width no longer fits.
        let mut g = NameField { text: String::new(), active: true, stamp_ms: 0.0 };
        for _ in 0..20 {
            g.key('W', |t: &str| 7.5 * t.chars().count() as f32);
        }
        assert_eq!(g.text.len(), 10, "7.5 × 10 = 75 is past the 70 of room");
        assert!(f.key('\r', width) && !f.active, "Enter stops the typing");
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
    fn a_previews_model_turns_clockwise_about_its_centre_under_a_pitch_that_leaves_the_centre_off_the_origin()
    {
        let viewport = [0.0, 0.0, 160.0, 151.0];
        // The turn is the model's own, clockwise seen from above (`Ngi32.dll:0x10014540`):
        // a quarter turn takes its left side, +y, to +x, away from the camera; the pitch then
        // tips that point up by the 0.5 rad the frame leans (`0x1009ec9d`).
        let (_, model) = preview_camera(Vec3::ZERO, 3.0, std::f32::consts::FRAC_PI_2, viewport);
        let left = model.transform_point3(Vec3::Y);
        assert!(left.distance(Vec3::new(0.5f32.cos(), 0.0, 0.5f32.sin())) < 1e-5, "{left}");
        // A small turn moves the left side away from the camera, at −x.
        let (_, model) = preview_camera(Vec3::ZERO, 3.0, 0.1, viewport);
        assert!(model.transform_point3(Vec3::Y).x > 0.0);
        // The pitch's frame keeps −c in its translation column (`0x1009ec66`–`0x1009ec91`), so
        // the centre, turned about itself and put back, lands at (pitch − I) · c whatever the
        // turn (`0x1009efea`–`0x1009f04d`): a centre 2 m above the origin sits 0.96 m nearer the
        // camera and 0.24 m lower than the origin the camera looks at.
        let centre = Vec3::new(0.0, 0.0, 2.0);
        for angle in [0.0, 1.0, 2.5, 4.0] {
            let (_, model) = preview_camera(centre, 5.0, angle, viewport);
            let at = model.transform_point3(centre);
            let want = Vec3::new(-2.0 * 0.5f32.sin(), 0.0, 2.0 * (0.5f32.cos() - 1.0));
            assert!(at.distance(want) < 1e-5, "{angle}: {at} against {want}");
        }
    }

    #[test]
    fn a_previews_camera_looks_along_x_its_field_spans_the_width_and_the_top_tips_toward_it() {
        let radius = 3.0;
        let distance = PREVIEW_K * radius;
        for viewport in [[0.0, 0.0, 250.0, 290.0], [0.0, 0.0, 160.0, 151.0]] {
            let (view_proj, model) = preview_camera(Vec3::ZERO, radius, 0.0, viewport);
            let ndc = |p: Vec3| {
                let clip = view_proj * p.extend(1.0);
                clip.truncate() / clip.w
            };
            // Placed at (−K r, 0, 0) with no turn: +y, its left, is the view's left edge
            // where the half field of 30° meets it, across the width whatever the height.
            let edge = distance * (PREVIEW_FIELD / 2.0).tan();
            let left = ndc(Vec3::new(0.0, edge, 0.0));
            assert!((left.x + 1.0).abs() < 1e-4 && left.y.abs() < 1e-4, "{viewport:?} {left}");
            let aspect = viewport[2] / viewport[3];
            let top = ndc(Vec3::new(0.0, 0.0, edge / aspect));
            assert!((top.y - 1.0).abs() < 1e-4, "{viewport:?} {top}");
            // The pitch is about y: the model's up leans toward the camera, at −x.
            let up = model.transform_vector3(Vec3::Z);
            assert!(up.x < -0.47 && up.y.abs() < 1e-6 && (up.z - 0.5f32.cos()).abs() < 1e-5, "{up}");
        }
    }

    #[test]
    fn a_previews_lights_turn_with_the_model() {
        for angle in [0.0, 0.7, 2.0, 4.5] {
            let (_, model) = preview_camera(Vec3::new(1.0, 2.0, 3.0), 2.0, angle, [0.0, 0.0, 160.0, 151.0]);
            let lights = preview_lights(model);
            for (light, own) in lights.iter().zip(PREVIEW_LIGHTS) {
                let back = model.inverse().transform_vector3(*light).normalize();
                assert!(back.distance(Vec3::from_array(own).normalize()) < 1e-5, "{angle} {back}");
            }
        }
    }

    #[test]
    fn a_scan_band_is_its_strip_times_green_plus_the_specular_held_to_one() {
        let lit = |g: f32| {
            let t = band_tint(g);
            [0, 1, 2].map(|i| t[i] * BAND_STRIP[i])
        };
        let dark = lit(0.0);
        assert!(dark[0] == 0.0 && dark[2] == 0.0 && (dark[1] - BAND_STRIP[1] * 155.0 / 255.0).abs() < 1e-6);
        let mid = lit(254.0);
        assert!((mid[0] - 254.0 * 2.0 / 3.0 / 255.0).abs() < 1e-6 && mid[1] == 1.0, "{mid:?}");
        assert_eq!(mid[0], mid[2], "red and blue alike");
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
