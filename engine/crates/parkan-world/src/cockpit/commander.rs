//! The commander panel of command mode (`CState` mode 4): the resource rows, the icon column
//! down the left with its lock, the page a column button opens — the battle units, builders
//! and transports with their unit box, rows and order menu; the factory; the towers, bunkers
//! and other buildings — the commander's satellite map and the message box. See
//! `docs/41-commander.md` and `docs/40-command-mode.md`, "What command mode draws".

use parkan_sim::hq;

use super::panels::{life_share, order_status};
use super::weapons::fill_colour;
use super::{Cockpit, Ink, WHITE, argb, factory, map, messages};
use crate::hud::{Blend, Layer, Pin};
use crate::play::Play;
use crate::selection::{
    BATTLE_UNITS, BUILDERS, BUNKERS, FACTORY, OTHER_BUILDINGS, RESEARCH_CENTRE, TOWERS, TRANSPORTS,
};

/// A column button's widget, its icon's inset and size (`0x1009c030`, `0x1009bec0`).
pub const BUTTON: [f32; 2] = [46.0, 29.0];
pub const ICON_INSET: [f32; 2] = [16.0, 3.0];
pub const ICON: f32 = 23.0;
/// The lock's widget and its icon's inset and size.
pub const LOCK: [f32; 2] = [46.0, 24.0];
pub const LOCK_ICON_INSET: [f32; 2] = [18.0, 7.0];
pub const LOCK_ICON: f32 = 13.0;
pub const LOCK_OPEN_Y: f32 = 457.0;
pub const LOCK_SHUT_Y: f32 = 12.0;
/// The column's items slide one a step, every 0.05 s, over 16 steps (`0x10083e1d`).
pub const SLIDE_STEPS: u8 = 16;
pub const SLIDE_MS: f64 = 50.0;
/// A hovered enabled button flips between its looks every 0.1 s (`0x1009bf61`).
pub const HOVER_FLIP_MS: f64 = 100.0;
/// A button's icon colour: disabled, enabled, on.
pub const ICON_COLOURS: [u32; 3] = [0xff64_6464, 0xff80_ff80, WHITE];

/// What a column button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Strategic control off: back to the hero.
    Hero,
    Page(u8),
    Chat,
    Map,
    GameMenu,
}

/// One item of the column: a button, or a separator (`objpanel_separator`, 46 × 24).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Button { y: f32, icon: &'static str, tooltip: u32, control: Control },
    Separator { y: f32 },
}

/// The column's items under its head, in the order the slide draws them (`0x10082550`).
pub const COLUMN: [Item; 16] = [
    Item::Button { y: 12.0, icon: "objpanel_icon_hero", tooltip: 1612, control: Control::Hero },
    Item::Separator { y: 41.0 },
    Item::Button { y: 65.0, icon: "objpanel_icon_warbots", tooltip: 1600, control: Control::Page(1) },
    Item::Button { y: 94.0, icon: "objpanel_icon_builder", tooltip: 1602, control: Control::Page(3) },
    Item::Button { y: 123.0, icon: "objpanel_icon_cargo", tooltip: 1601, control: Control::Page(2) },
    Item::Separator { y: 152.0 },
    Item::Button { y: 176.0, icon: "objpanel_icon_restree", tooltip: 5080, control: Control::Page(4) },
    Item::Separator { y: 205.0 },
    Item::Button { y: 229.0, icon: "objpanel_icon_factory", tooltip: 1607, control: Control::Page(5) },
    Item::Button { y: 258.0, icon: "objpanel_icon_tower", tooltip: 1614, control: Control::Page(6) },
    Item::Button { y: 287.0, icon: "objpanel_icon_bunker", tooltip: 1608, control: Control::Page(7) },
    Item::Button { y: 316.0, icon: "objpanel_icon_build", tooltip: 1610, control: Control::Page(8) },
    Item::Separator { y: 345.0 },
    Item::Button { y: 369.0, icon: "objpanel_icon_chat", tooltip: 5038, control: Control::Chat },
    Item::Button { y: 399.0, icon: "objpanel_icon_map", tooltip: 1501, control: Control::Map },
    Item::Button { y: 428.0, icon: "objpanel_icon_system", tooltip: 1302, control: Control::GameMenu },
];

/// Each page's header title (`0x10083a20`, table `0x10083c00`), and 6205 for any other.
pub fn title(page: u8) -> u32 {
    match page {
        1 => 1600,
        2 => 1601,
        3 => 1602,
        4 => 5080,
        5 => 1607,
        6 => 1614,
        7 => 1608,
        8 => 1610,
        _ => 6205,
    }
}

/// The unit Types a unit page lists, and the building Types a building page lists.
pub fn page_units(page: u8) -> Option<u32> {
    match page {
        1 => Some(BATTLE_UNITS),
        2 => Some(TRANSPORTS),
        3 => Some(BUILDERS),
        _ => None,
    }
}

pub fn page_buildings(page: u8) -> Option<u32> {
    match page {
        5 => Some(FACTORY),
        6 => Some(TOWERS),
        7 => Some(BUNKERS),
        8 => Some(OTHER_BUILDINGS),
        _ => None,
    }
}

/// The unit box (`0x10085530`), its fill, and what it says with no unit or several.
pub const UNIT_BOX: [f32; 4] = [51.0, 20.0, 369.0, 117.0];
pub const STRING_NO_BOTS: u32 = 5076;
pub const STRING_BOTS_SELECTED: u32 = 5077;
pub const GREEN: u32 = 0xff00_ff00;
pub const YELLOW: u32 = 0xffff_ff00;
pub const GREY: u32 = 0xff80_8080;
/// The unit box's two icons, its name, its lines' pen, and its buttons (`0x10085890`).
pub const UNIT_ICONS: [[f32; 2]; 2] = [[60.0, 27.0], [77.0, 27.0]];
pub const UNIT_NAME: [f32; 2] = [101.0, 30.0];
pub const UNIT_LINES: [f32; 2] = [122.0, 27.0];
pub const DRIVE_BUTTONS: [[f32; 2]; 3] = [[60.0, 88.0], [84.0, 88.0], [108.0, 88.0]];
pub const STRATEGIC_BUTTON: [f32; 2] = [138.0, 88.0];
/// The `Type` whose unit box shows Strategic control (`0x10085b5e`).
pub const HQ_TYPE: u32 = 0x0101_0000;
pub const EXPLODE_BUTTON: [f32; 4] = [324.0, 88.0, 356.0, 109.0];
/// The rows: 20 apart from (51, 118) under the unit box, from (51, 171) under the factory
/// panel, from (51, 20) on the other building pages.
pub const ROW_STEP: f32 = 20.0;
pub const ROW_HEIGHT: f32 = 19.0;
pub const UNIT_ROWS_TOP: f32 = 118.0;
pub const FACTORY_ROWS_TOP: f32 = 171.0;
pub const BUILDING_ROWS_TOP: f32 = 20.0;
pub const ROWS_LEFT: f32 = 51.0;
/// A unit row's bar (`0x10095b80`) and a building row's end (`0x100961f0`).
pub const UNIT_BAR: f32 = 248.0;
pub const BUILDING_BAR_END: f32 = 354.0;
/// The order menu: its x, its strip's text box, its rows' width and bar, how far down its
/// rows may reach and how tall one counts (`0x1007b1a0`, `0x1007b1e0`, `0x1007b0bf`).
pub const MENU_X: f32 = 100.0;
pub const MENU_TEXT: f32 = 158.0;
pub const MENU_ROW: f32 = 269.0;
pub const MENU_BAR: f32 = 247.0;
pub const MENU_BOTTOM: f32 = 454.0;
pub const MENU_ROW_COUNT_STEP: f32 = 21.0;
pub const STRING_ORDERS: u32 = 5084;
/// A pressed row fills its bar for a second; the menu rebuilds every 2 s (`0x1009c89a`,
/// `0x1007b4a0`).
pub const PRESSED_MS: f64 = 1000.0;
pub const REBUILD_MS: f64 = 2000.0;
/// The resource rows swallow a click (`0x1008d690`).
pub const RESOURCE_ROWS: [f32; 4] = [374.0, 0.0, 640.0, 42.0];
/// The commander's satellite map and its title bar (docs/35, "The object").
pub const MAP_PANEL: [f32; 4] = [374.0, 63.0, 640.0, 329.0];
pub const MAP_TITLE_Y: f32 = 43.0;
pub const STRING_SATELLITE_MAP: u32 = 5074;
/// The sounds a click plays (`ui/game_resources.cfg`).
pub const BUTTON_CLICK: &str = "BUTTON_CLICK";
pub const BAR_OPEN: &str = "BAR_OPEN";

/// What the panel keeps from frame to frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub page: u8,
    /// How far the column has slid out, 0 shut to 16 open, and which way it moves.
    pub steps: u8,
    pub sliding: i8,
    pub slid_ms: f64,
    /// Each column button's look this frame: enabled, and on.
    pub enabled: [bool; 16],
    pub on: [bool; 16],
    /// The order menu's rows, when they were built, and the row pressed and when.
    pub menu: Vec<u8>,
    pub menu_ms: Option<f64>,
    pub pressed: Option<(u8, f64)>,
    /// Where the cursor is on the layout pinned to the top left, while over the window.
    pub cursor: Option<[f32; 2]>,
    /// Command mode was up at the last update.
    pub entered: bool,
    /// The cursor's state this frame (`0x10104148`), and the band's corners on the layout
    /// pinned to the top left while one is up.
    pub cursor_state: u8,
    pub band: Option<[[f32; 2]; 2]>,
    /// The unit or building under the cursor, which gets a marker.
    pub hovered: Option<usize>,
    /// The research panel page 4 draws (`+0x7b8`).
    pub research: super::research::Panel,
}

impl Default for Panel {
    fn default() -> Self {
        Panel {
            page: 0,
            steps: SLIDE_STEPS,
            sliding: 0,
            slid_ms: 0.0,
            enabled: [false; 16],
            on: [false; 16],
            menu: Vec::new(),
            menu_ms: None,
            pressed: None,
            cursor: None,
            entered: false,
            cursor_state: 1,
            band: None,
            hovered: None,
            research: super::research::Panel::default(),
        }
    }
}

/// What a click on the panel asks the play for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Click {
    /// Taken by the panel, nothing more to do.
    Taken,
    /// Not the panel's: the world's.
    World,
    /// An order row that opens a pick: Route, Guard, a Build.
    Pick(hq::Act),
    /// A control of the factory panel on page 5.
    Factory(usize, factory::Click),
}

fn inside([x0, y0, x1, y1]: [f32; 4], [x, y]: [f32; 2]) -> bool {
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
}

fn button_rect(y: f32) -> [f32; 4] {
    [0.0, y, BUTTON[0], y + BUTTON[1]]
}

fn lock_rect(steps: u8) -> [f32; 4] {
    let y = if steps == 0 { LOCK_SHUT_Y } else { LOCK_OPEN_Y };
    [0.0, y, LOCK[0], y + LOCK[1]]
}

/// The buildings of `mask` a page shows.
fn buildings_of(play: &Play, mask: u32) -> Vec<usize> {
    play.own_buildings_within(mask)
        .into_iter()
        .filter(|&t| mask != FACTORY && mask != RESEARCH_CENTRE || play.units[t].type_word == mask)
        .collect()
}

impl Panel {
    /// Turn to page `page` (`0x10084d80`), selecting for it (`0x10084e60`).
    pub fn turn(&mut self, play: &mut Play, page: u8, now_ms: f64) {
        self.page = page;
        if let Some(mask) = page_units(page) {
            let selected = play.selected_units();
            let keep = selected.len() == 1 && hq::within(play.units[selected[0]].type_word, mask);
            if !keep && let Some(&first) = play.own_units_within(mask).first() {
                play.select_unit_alone(first);
            }
            self.rebuild(play, now_ms);
        } else if let Some(mask) = page_buildings(page) {
            let keep = play.selected.len() == 1 && hq::within(play.units[play.selected[0]].type_word, mask);
            if !keep && let Some(&first) = buildings_of(play, mask).first() {
                play.select_building(first);
            }
        } else if page == 4 {
            // Turning to the research page refreshes its panel (`0x10084dda`).
            self.research.refresh(play);
        }
    }

    /// The order menu opens again for the selection (`0x1007a8e0`, `0x1007aaa0`).
    pub fn rebuild(&mut self, play: &mut Play, now_ms: f64) {
        self.menu = play.hq_rows();
        self.menu_ms = Some(now_ms);
    }

    /// The column's update every frame in command mode (`0x10083c60`): the buttons' looks,
    /// the slide, the lock under a resting mouse, and the page kept current (`0x10084045`).
    pub fn update(&mut self, play: &mut Play, now_ms: f64) {
        let units = |mask| !play.own_units_within(mask).is_empty();
        let buildings = |mask: u32| !buildings_of(play, mask).is_empty();
        for (i, item) in COLUMN.iter().enumerate() {
            let Item::Button { control, .. } = *item else { continue };
            self.enabled[i] = match control {
                Control::Hero | Control::Map | Control::GameMenu => true,
                Control::Chat => false,
                Control::Page(p @ 1..=3) => units(page_units(p).unwrap_or(0)),
                Control::Page(4) => buildings(RESEARCH_CENTRE),
                Control::Page(p) => page_buildings(p).is_some_and(buildings),
            };
            self.on[i] = matches!(control, Control::Page(p) if p == self.page);
        }
        // The slide steps once every 0.05 s.
        if self.sliding != 0 {
            while now_ms - self.slid_ms >= SLIDE_MS {
                self.slid_ms += SLIDE_MS;
                let steps = self.steps as i8 + self.sliding;
                self.steps = steps.clamp(0, SLIDE_STEPS as i8) as u8;
                if self.steps == 0 || self.steps == SLIDE_STEPS {
                    self.sliding = 0;
                    break;
                }
            }
        } else if self.steps == 0 && self.cursor.is_some_and(|c| inside(lock_rect(0), c)) {
            // The mouse merely resting on the shut lock slides the column out (`0x10083d41`).
            self.slide(1, now_ms, play);
        }
        // The page kept current.
        if let Some(mask) = page_units(self.page) {
            let list = play.own_units_within(mask);
            if list.is_empty() {
                self.page = 0;
            } else if !play.selected_units().iter().any(|t| list.contains(t)) {
                play.select_unit_alone(list[0]);
                self.rebuild(play, now_ms);
            } else if self.pressed.is_none_or(|(_, at)| now_ms - at >= PRESSED_MS)
                && self.menu_ms.is_none_or(|at| now_ms - at >= REBUILD_MS)
            {
                self.rebuild(play, now_ms);
            }
        } else if (4..=8).contains(&self.page) {
            let index = COLUMN
                .iter()
                .position(|i| matches!(i, Item::Button { control: Control::Page(p), .. } if *p == self.page));
            if index.is_some_and(|i| !self.enabled[i]) {
                self.page = 0;
            }
        }
    }

    fn slide(&mut self, way: i8, now_ms: f64, play: &mut Play) {
        self.sliding = way;
        self.slid_ms = now_ms;
        play.ui_sound(BAR_OPEN);
    }

    /// A left click at `left` on the layout pinned to the top left and `right` pinned to the
    /// top right (`0x1008d690`, `0x100841a0`).
    pub fn click(
        &mut self,
        play: &mut Play,
        map: &mut map::SatelliteMap,
        left: [f32; 2],
        right: [f32; 2],
        now_ms: f64,
    ) -> Click {
        if inside(RESOURCE_ROWS, right) {
            return Click::Taken;
        }
        // The lock.
        if self.sliding == 0 && self.steps == SLIDE_STEPS && inside(lock_rect(self.steps), left) {
            self.slide(-1, now_ms, play);
            return Click::Taken;
        }
        // The map's exit, its `+0x230` rectangle, while it is open: the map closes
        // (`0x10084343`-`0x1008437d`), whatever the column and the page.
        if map.open && inside(map::EXIT, right) {
            map.toggle();
            return Click::Taken;
        }
        // The column's buttons, while it is out.
        if self.steps == SLIDE_STEPS {
            for (i, item) in COLUMN.iter().enumerate() {
                let Item::Button { y, control, .. } = *item else { continue };
                if !self.enabled[i] || !inside(button_rect(y), left) {
                    continue;
                }
                match control {
                    // Back to the hero, a mode at a time (`0x10062ce0` with 0).
                    Control::Hero => {
                        play.roll_back_to_foot();
                        self.page = 0;
                    }
                    Control::Page(p) if p == self.page => self.page = 0,
                    Control::Page(p) => self.turn(play, p, now_ms),
                    Control::Map => map.toggle(),
                    // STAND-IN: docs/41-commander.md#what-a-click-on-the-column-does -- the chat
                    // overlay and the game menu's screen (mode 7) are not built: a click on
                    // either is taken and does nothing.
                    Control::Chat | Control::GameMenu => {}
                }
                return Click::Taken;
            }
        }
        if self.page == 0 || self.steps != SLIDE_STEPS {
            return Click::World;
        }
        // The header's exit.
        if inside(factory::EXIT, left) {
            self.page = 0;
            return Click::Taken;
        }
        if let Some(mask) = page_units(self.page) {
            return self.click_unit_page(play, mask, left, now_ms);
        }
        if self.page == 4 {
            return match self.research.click(play, left) {
                super::research::Click::Outside => Click::World,
                _ => Click::Taken,
            };
        }
        if let Some(mask) = page_buildings(self.page) {
            let top = if self.page == 5 { FACTORY_ROWS_TOP } else { BUILDING_ROWS_TOP };
            if self.page == 5
                && let Some(&factory) = play.selected.first().filter(|&&t| play.units[t].type_word == FACTORY)
                && let Some(c) = factory::click(play, factory, left)
            {
                return Click::Factory(factory, c);
            }
            let rows = buildings_of(play, mask);
            for (i, &t) in rows.iter().enumerate() {
                let y = top + ROW_STEP * i as f32;
                if !inside([ROWS_LEFT, y, 369.0, y + ROW_HEIGHT], left) {
                    continue;
                }
                play.select_building(t);
                if play.units[t].type_word == FACTORY {
                    self.page = 5;
                }
                // Strategic control, on a bunker's row (`0x100862d6`).
                let strategic = [BUILDING_BUTTONS_X, y, BUILDING_BUTTONS_X + BUILDING_BUTTON, y + ROW_HEIGHT];
                if hq::within(play.units[t].type_word, BUNKERS) && inside(strategic, left) {
                    play.enter_command(t);
                }
                return Click::Taken;
            }
        }
        Click::World
    }

    fn click_unit_page(&mut self, play: &mut Play, mask: u32, at: [f32; 2], now_ms: f64) -> Click {
        let list = play.own_units_within(mask);
        for (i, &t) in list.iter().enumerate() {
            let y = UNIT_ROWS_TOP + ROW_STEP * i as f32;
            if inside([ROWS_LEFT, y, 369.0, y + ROW_HEIGHT], at) {
                play.select_unit_alone(t);
                self.rebuild(play, now_ms);
                return Click::Taken;
            }
        }
        // The unit box's drive buttons, their icons grown by 2 (`0x1008470d`): telepresence at
        // levels 0, 1 and 2.
        if let [t] = play.selected_units()[..] {
            for (level, corner) in DRIVE_BUTTONS.iter().enumerate() {
                let rect = [corner[0] + 1.0, corner[1] + 1.0, corner[0] + 20.0, corner[1] + 20.0];
                if inside(rect, at) {
                    play.telepresence(t, level as u8);
                    self.page = 0;
                    return Click::Taken;
                }
            }
            // Strategic control on an HQ (`0x100847d5`): its command view, page 0.
            let [x, y] = STRATEGIC_BUTTON;
            if play.units[t].type_word == HQ_TYPE && inside([x + 1.0, y + 1.0, x + 20.0, y + 20.0], at) {
                play.enter_hq_command(t);
                self.page = 0;
                return Click::Taken;
            }
            // Explode! (`0x10084827`–`0x10084861`), unless one is pending.
            let [x, y, ..] = EXPLODE_BUTTON;
            if inside([x, y, x + 34.0, y + 19.0], at) {
                play.explode(t);
                return Click::Taken;
            }
        }
        let top = UNIT_ROWS_TOP + ROW_STEP * list.len() as f32;
        for (j, &command) in self.visible_rows(top).iter().enumerate() {
            let y = top + ROW_STEP * (j + 1) as f32;
            if !inside([MENU_X, y, MENU_X + MENU_ROW, y + ROW_HEIGHT], at) {
                continue;
            }
            self.pressed = Some((command, now_ms));
            play.ui_sound(BUTTON_CLICK);
            return match play.hq_command(command) {
                Some(hq::Act::Order(_)) | None => Click::Taken,
                Some(act) => Click::Pick(act),
            };
        }
        if inside(UNIT_BOX, at) { Click::Taken } else { Click::World }
    }

    /// Whether the panel answers for the cursor at `left` and `right` (`0x10084b80`), so the
    /// world takes no pick there: the resource rows, the column, the page's box and rows.
    pub fn hit(&self, play: &Play, map_open: bool, left: [f32; 2], right: [f32; 2]) -> bool {
        if inside(RESOURCE_ROWS, right)
            || (map_open && inside([374.0, MAP_TITLE_Y, 640.0, MAP_PANEL[1]], right))
        {
            return true;
        }
        if inside(lock_rect(self.steps), left)
            || (self.steps > 0 && inside([0.0, 0.0, BUTTON[0], 481.0], left))
        {
            return true;
        }
        if self.page == 0 || self.steps != SLIDE_STEPS {
            return false;
        }
        let bottom = if let Some(mask) = page_units(self.page) {
            let top = UNIT_ROWS_TOP + ROW_STEP * play.own_units_within(mask).len() as f32;
            top + ROW_STEP * (self.visible_rows(top).len() + 1) as f32
        } else if let Some(mask) = page_buildings(self.page) {
            let top = if self.page == 5 { FACTORY_ROWS_TOP } else { BUILDING_ROWS_TOP };
            top + ROW_STEP * buildings_of(play, mask).len() as f32
        } else if self.page == 4 {
            self.research.bottom()
        } else {
            UNIT_BOX[3]
        };
        inside([ROWS_LEFT, 0.0, 369.0, bottom], left)
    }

    /// The order rows that fit above 454 with the menu at `top` (`0x1007b1a0`).
    fn visible_rows(&self, top: f32) -> &[u8] {
        let fit = ((MENU_BOTTOM - top) / MENU_ROW_COUNT_STEP).max(0.0) as usize;
        &self.menu[..self.menu.len().min(fit)]
    }
}

/// A building row's buttons: at this x, each this wide.
///
/// STAND-IN: docs/41-commander.md#the-building-pages-5-to-8--read-and-seen -- the width of
/// the piece a row's icon stands in is not read: 20, as the recording's rows measure, so the
/// buttons start at x 86.
pub const BUILDING_BUTTONS_X: f32 = 86.0;
pub const BUILDING_BUTTON: f32 = 35.0;
/// A unit row's icon pieces (`0x1009a7a0`): each a `body_text` square as wide as the piece is
/// high, 19, its icon inset by 2 (docs/31, "The wingman menu from first person").
pub const ICON_PIECE: f32 = 19.0;

/// The panel in command mode, after the world: the resource rows, the column and the page, the
/// commander's map and the message box (`0x1008d51c`).
pub fn draw(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    now_ms: f64,
    view_proj: glam::Mat4,
) -> Vec<super::designer::Preview> {
    let mut previews = Vec::new();
    // The building ghost, drawn flat over the world in its colour (docs/32).
    if let Some(g) = play.commander.ghost.as_ref().filter(|g| g.placed) {
        let c = argb(if g.valid { crate::pick::GHOST_GOOD } else { crate::pick::GHOST_BAD });
        let space = ink.painter.space;
        previews.push(super::designer::Preview {
            key: super::designer::PreviewKey {
                kind: parkan_formats::mission::KIND_BUILDING,
                path: g.path.clone(),
                version: 0,
            },
            viewport: [0.0, 0.0, space.width, space.height],
            view_proj,
            model: glam::Mat4::from_translation(g.at) * glam::Mat4::from_rotation_z(g.yaw),
            lights: [glam::Vec3::NEG_Z; 2],
            paint: Some([c[0], c[1], c[2]]),
        });
    }
    // The markers over the selected units and the one under the cursor (`0x1007d5e0`).
    super::markers::draw(cockpit, ink, play, view_proj, cockpit.commander.hovered);
    factory::resource_rows(cockpit, ink, play, now_ms);
    // While the ghost is up (cursor state 8) only the resource rows draw (`0x1008d326`).
    let placing = cockpit.commander.cursor_state == 8;
    ink.painter.pin = Pin::TOP_LEFT;
    if !placing {
        column(cockpit, ink, now_ms);
    }
    let page = cockpit.commander.page;
    if !placing && page != 0 && cockpit.commander.steps == SLIDE_STEPS {
        factory::header(cockpit, ink, title(page));
        if let Some(mask) = page_units(page) {
            unit_page(cockpit, ink, play, mask, now_ms);
        } else if page == 4 {
            previews = super::research::panel(cockpit, ink, play, now_ms);
        } else if let Some(mask) = page_buildings(page) {
            let top = if page == 5 { FACTORY_ROWS_TOP } else { BUILDING_ROWS_TOP };
            // Page 5 draws the selected factory's panel above its rows.
            if page == 5
                && let Some(&t) = play.selected.first().filter(|&&t| play.units[t].type_word == FACTORY)
            {
                previews = factory::panel(cockpit, ink, play, t, now_ms);
                ink.painter.pin = Pin::TOP_LEFT;
            }
            building_rows(cockpit, ink, play, mask, top);
        }
    }
    map::draw_in(cockpit, ink, play, now_ms, MAP_PANEL, true);
    ink.painter.pin = Pin::BOTTOM_RIGHT;
    let [left, top, width] = factory::MESSAGES_AT;
    messages::draw_at(cockpit, ink, now_ms, left, top, width);
    ink.painter.pin = Pin::TOP_LEFT;
    band_and_cursor(cockpit, ink, now_ms);
    previews
}

/// The band in state 7, and the software cursor of the state (`0x100585b0`, `0x10057060`):
/// four 16 × 16 phases of `new_ui1` stepping every 150 ms, its extent the hot spot. The
/// cursor draws in the top layer: the HUD's text is laid out into its own pass, drawn after
/// all of the art, so the rows and the order menu under the pointer painted over it, where
/// the interface drawing the cursor last leaves nothing on top of it.
fn band_and_cursor(cockpit: &Cockpit, ink: &mut Ink, now_ms: f64) {
    let panel = &cockpit.commander;
    if let Some([[x0, y0], [x1, y1]]) = panel.band {
        let colour = argb(crate::pick::BAND_COLOUR);
        let w = 1.0 / ink.painter.space.scale();
        for (a, b) in [([x0, y0], [x1, y0]), ([x1, y0], [x1, y1]), ([x1, y1], [x0, y1]), ([x0, y1], [x0, y0])]
        {
            ink.painter.line(Blend::Alpha, a, b, w, colour);
        }
    }
    cursor(cockpit, ink, panel.cursor_state, now_ms);
}

/// The software cursor of cursor state `state` at the commander's cursor (`0x100585b0`,
/// `0x10057060`), in the top layer, pinned top left as the cursor is laid out. A building's
/// screen and the warbot designer show it in state 1, `ARROW`: the frame runs the chooser
/// wherever the cursor is shown (`0x10060c95`), and the pick answers kind 0 in view state 1
/// and while the designer is up (`0x1008daa4`–`0x1008dae5`), docs/36, "The cursor in mode 5".
///
/// STAND-IN: docs/42-selection.md#the-cursor-shows-a-state--read-and-measured -- whether the
/// display's slot 12 answers, which picks the system's cursor, is not read: the software
/// cursor is drawn, the system's hidden.
pub(super) fn cursor(cockpit: &Cockpit, ink: &mut Ink, state: u8, now_ms: f64) {
    let (Some(at), Some((offset, hot)), Some(&page)) =
        (cockpit.commander.cursor, crate::pick::cursor_object(state), cockpit.pages.get("new_ui1"))
    else {
        return;
    };
    ink.painter.pin = Pin::TOP_LEFT;
    let side = crate::pick::CURSOR_SIDE;
    let phase = ((now_ms / crate::pick::CURSOR_PHASE_MS).floor() as i64).rem_euclid(4) as f32;
    let layer = std::mem::replace(&mut ink.painter.layer, Layer::OverText);
    ink.painter.sprite(
        Blend::Alpha,
        page,
        [offset[0] + side * phase, offset[1], side, side],
        [at[0] - hot[0], at[1] - hot[1]],
        [1.0; 4],
    );
    ink.painter.layer = layer;
}

pub(super) fn put(cockpit: &Cockpit, ink: &mut Ink, name: &str, rect: [f32; 4], colour: u32) {
    if let Some(p) = cockpit.skin.get(name) {
        ink.painter.piece(p, rect, argb(colour));
    }
}

/// A 15 × 15 icon of `ui_menu` at its cell.
pub(super) fn icon(cockpit: &Cockpit, ink: &mut Ink, cell: [f32; 2], at: [f32; 2], colour: u32) {
    if let Some(&page) = cockpit.pages.get("ui_menu") {
        ink.painter.sprite(Blend::Alpha, page, [cell[0], cell[1], 15.0, 15.0], at, argb(colour));
    }
}

fn column(cockpit: &Cockpit, ink: &mut Ink, now_ms: f64) {
    let panel = &cockpit.commander;
    put(cockpit, ink, "objpanel_head", [0.0, 0.0, 46.0, 12.0], WHITE);
    let flip = ((now_ms / HOVER_FLIP_MS).floor() as i64) % 2 == 1;
    for (i, item) in COLUMN.iter().enumerate().take(usize::from(panel.steps)) {
        match *item {
            Item::Separator { y } => put(cockpit, ink, "objpanel_separator", [0.0, y, 46.0, y + 24.0], WHITE),
            Item::Button { y, icon, .. } => {
                let mut state = if !panel.enabled[i] {
                    0
                } else if panel.on[i] {
                    2
                } else {
                    1
                };
                if panel.enabled[i] && flip && panel.cursor.is_some_and(|c| inside(button_rect(y), c)) {
                    state = if state == 2 { 1 } else { 2 };
                }
                let frame = ["objpanel_button1", "objpanel_button2", "objpanel_button3"][state];
                put(cockpit, ink, frame, button_rect(y), WHITE);
                let [ix, iy] = [ICON_INSET[0], y + ICON_INSET[1]];
                put(cockpit, ink, icon, [ix, iy, ix + ICON, iy + ICON], ICON_COLOURS[state]);
            }
        }
    }
    // The lock, hidden while the column slides.
    if panel.sliding == 0 {
        let [x0, y0, x1, y1] = lock_rect(panel.steps);
        put(cockpit, ink, "objpanel_lock2", [x0, y0, x1, y1], WHITE);
        let [ix, iy] = [x0 + LOCK_ICON_INSET[0], y0 + LOCK_ICON_INSET[1]];
        put(cockpit, ink, "objpanel_lock_icon", [ix, iy, ix + LOCK_ICON, iy + LOCK_ICON], ICON_COLOURS[1]);
    }
}

/// A unit's two icons' cells (`0x10077120`) and their tint: [`unit_cells`] by its Type and its
/// property `0x207`, [`unit_tint`] by its record's `+0x30`.
///
/// STAND-IN: docs/41-commander.md#the-box -- a *Tiny Tower*, a walking warrior whose record's
/// `+0x64` answers its query 2 with 0 or less, shows the towers' cell (81, 126) for its first
/// icon and a blank second (`0x10077342`-`0x100773d1`); which robots answer so is not read,
/// and none is told apart.
pub(super) fn unit_icons(play: &Play, t: usize) -> ([Option<[f32; 2]>; 2], u32) {
    let u = &play.units[t];
    (unit_cells(u.type_word, u.designation.chassis_type), unit_tint(play.record_class(t)))
}

/// A unit's two icons' cells on `ui_menu` (`0x10077120`). The first by its Type, compared
/// whole: a builder's (65, 110), a transport's (81, 110), a warrior's, an HQ's or the hero's
/// (49, 110), and none for any other. The second by its property `0x207`, its chassis
/// profile's `ChassisType` (table `0x100773fc`): flying (97, 94), walking (49, 94), wheeled
/// (65, 94), tracked (81, 94), and none past 4 (docs/41, "The box").
pub fn unit_cells(type_word: u32, chassis_type: u8) -> [Option<[f32; 2]>; 2] {
    use super::panels::{TYPE_BUILDER, TYPE_HQ, TYPE_TRANSPORT, TYPE_WARRIOR};
    let first = match type_word {
        TYPE_BUILDER => Some([65.0, 110.0]),
        TYPE_TRANSPORT => Some([81.0, 110.0]),
        TYPE_WARRIOR | TYPE_HQ | crate::play::ROBOT_HERO => Some([49.0, 110.0]),
        _ => None,
    };
    let second = match chassis_type {
        1 => Some([97.0, 94.0]),
        2 => Some([49.0, 94.0]),
        3 => Some([65.0, 94.0]),
        4 => Some([81.0, 94.0]),
        _ => None,
    };
    [first, second]
}

/// A unit's icons' tint by its record's `+0x30` (table `0x100773ec`), the size class its bind
/// stores from property `0x201` (`0x1007e538`-`0x1007e549`): tiny a pale pink, small red,
/// medium green, large blue. Any other leaves the caller's colour, which no shipped unit
/// reaches -- all 296 placed ones are 1 to 4 (docs/41, "The box"); white here.
pub fn unit_tint(size_class: u8) -> u32 {
    match size_class {
        1 => 0xffff_e7ff,
        2 => 0xffff_8080,
        3 => 0xff80_ff80,
        4 => 0xff80_80ff,
        _ => WHITE,
    }
}

/// A building's icon's tint by its record's `+0x30` (`0x100344fc`, table `0x100347dc`), the
/// size class its root's fourth letter gives: 1 white, small red, medium green, large and
/// enhanced blue. Any other leaves the caller's colour, which no shipped building reaches --
/// every root's fourth letter is `l`, `m`, `b` or `e` (docs/41, "The building pages"); white
/// here.
pub fn building_tint(size_class: u32) -> u32 {
    match size_class {
        2 => 0xffff_0000,
        3 => 0xff00_ff00,
        4 | 5 => 0xff00_00ff,
        _ => WHITE,
    }
}

/// Building target `t`'s size class, its record's `+0x30`: the fourth letter of its root
/// record ([`crate::selection::building_size`]).
fn building_class(play: &Play, t: usize) -> u32 {
    let root = play.assembly.records(play.commander.paths.get(t).map_or("", String::as_str));
    root.first().map_or(0, |r| crate::selection::building_size(r))
}

/// A unit's name and status, `"%s [%s]"` (docs/31, "The orders").
fn name_status(cockpit: &Cockpit, play: &Play, t: usize) -> String {
    let name = cockpit.panels.names.get(t).cloned().unwrap_or_default();
    format!("{name} [{}]", cockpit.string(order_status(super::panels::status_order(play, t))))
}

fn unit_page(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, mask: u32, now_ms: f64) {
    let [x0, y0, x1, y1] = UNIT_BOX;
    ink.painter.fill(
        Blend::Alpha,
        [x0 + 5.0, y0 + 5.0, x1 - x0 - 10.0, y1 - y0 - 10.0],
        argb(factory::BOX_FILL),
    );
    messages::frame(cockpit, ink, UNIT_BOX);
    let selected = play.selected_units();
    match selected[..] {
        [] => {
            let text = cockpit.string(STRING_NO_BOTS).to_owned();
            ink.text(&text, UNIT_NAME, GREEN);
        }
        [t] => unit_box(cockpit, ink, play, t),
        _ => {
            let text = cockpit.string(STRING_BOTS_SELECTED).replace("%d", &selected.len().to_string());
            ink.text(&text, UNIT_NAME, GREEN);
        }
    }
    let list = play.own_units_within(mask);
    let text_down = ((ROW_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    for (i, &t) in list.iter().enumerate() {
        let y = UNIT_ROWS_TOP + ROW_STEP * i as f32;
        let lit = selected.contains(&t);
        let look = if lit { "normal" } else { "off" };
        let mut pen = ROWS_LEFT;
        put(
            cockpit,
            ink,
            &format!("ccres_lamp_text_ending_{look}"),
            [pen, y, pen + 10.0, y + ROW_HEIGHT],
            WHITE,
        );
        pen += 10.0;
        let (cells, tint) = unit_icons(play, t);
        for cell in cells {
            put(cockpit, ink, "ccres_body_text", [pen, y, pen + ICON_PIECE, y + ROW_HEIGHT], WHITE);
            if let Some(cell) = cell {
                icon(cockpit, ink, cell, [(pen + 2.0).round(), y + 2.0], tint);
            }
            pen += ICON_PIECE;
        }
        put(cockpit, ink, "ccres_separator_left_text", [pen, y, pen + 5.0, y + ROW_HEIGHT], WHITE);
        pen += 5.0;
        put(cockpit, ink, &format!("ccres_ray_emitter_{look}"), [pen, y, pen + 10.0, y + ROW_HEIGHT], WHITE);
        pen += 10.0;
        life_bar(cockpit, ink, play, t, [pen, y, pen + UNIT_BAR]);
        let text = name_status(cockpit, play, t);
        ink.centred(&text, pen, UNIT_BAR, y + text_down, if lit { WHITE } else { GREY });
        pen += UNIT_BAR;
        put(cockpit, ink, "ccres_ray_ending", [pen, y, pen + 6.0, y + ROW_HEIGHT], WHITE);
    }
    // The order menu under the list.
    let top = UNIT_ROWS_TOP + ROW_STEP * list.len() as f32;
    let mut pen = MENU_X;
    put(cockpit, ink, "ccres_ending_text", [pen, top, pen + 5.0, top + ROW_HEIGHT], WHITE);
    pen += 5.0;
    put(cockpit, ink, "ccres_body_text", [pen, top, pen + MENU_TEXT, top + ROW_HEIGHT], WHITE);
    let orders = cockpit.string(STRING_ORDERS).to_owned();
    ink.centred(&orders, pen, MENU_TEXT, top + text_down, WHITE);
    pen += MENU_TEXT;
    for arrow in ["scroll_up_icon", "scroll_down_icon"] {
        put(cockpit, ink, "ccres_long_button_normal", [pen, top, pen + 50.0, top + ROW_HEIGHT], WHITE);
        put(cockpit, ink, arrow, [pen + 10.0, top + 2.0, pen + 40.0, top + 17.0], factory::ICON_VARIANTS[1]);
        pen += 50.0;
    }
    put(cockpit, ink, "ccres_ending_text", [pen + 5.0, top, pen, top + ROW_HEIGHT], WHITE);
    let panel = &cockpit.commander;
    let rows: Vec<u8> = panel.visible_rows(top).to_vec();
    let pressed = panel.pressed.filter(|(_, at)| now_ms - at < PRESSED_MS).map(|(c, _)| c);
    for (j, command) in rows.into_iter().enumerate() {
        let y = top + ROW_STEP * (j + 1) as f32;
        let mut pen = MENU_X;
        put(cockpit, ink, "ccres_ending_text", [pen, y, pen + 5.0, y + ROW_HEIGHT], WHITE);
        pen += 5.0;
        put(cockpit, ink, "ccres_ray_emitter_normal", [pen, y, pen + 10.0, y + ROW_HEIGHT], WHITE);
        pen += 10.0;
        put(cockpit, ink, "ccres_ray_body", [pen, y, pen + MENU_BAR, y + ROW_HEIGHT], WHITE);
        if pressed == Some(command) {
            ink.painter.fill(
                Blend::Alpha,
                [pen, y + 3.0, MENU_BAR - 2.0, ROW_HEIGHT - 6.0],
                argb(fill_colour(100)),
            );
        }
        let text = hq::row(command).map(|r| cockpit.string(r.string).to_owned()).unwrap_or_default();
        ink.centred(&text, pen, MENU_BAR, y + text_down, WHITE);
        pen += MENU_BAR;
        put(cockpit, ink, "ccres_ray_ending", [pen, y, pen + 6.0, y + ROW_HEIGHT], WHITE);
    }
}

/// One selected unit in the box (`0x10085890`).
fn unit_box(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, t: usize) {
    let (cells, tint) = unit_icons(play, t);
    for (cell, at) in cells.into_iter().zip(UNIT_ICONS) {
        if let Some(cell) = cell {
            icon(cockpit, ink, cell, at, tint);
        }
    }
    let text = name_status(cockpit, play, t);
    ink.text(&text, UNIT_NAME, YELLOW);
    if let Some(lines) = play.unit_lines(t) {
        let height = ink.font.line_height;
        let (first, step) = ((height + 3.0).round(), (height + 2.0).round());
        let [x, y0] = UNIT_LINES;
        for (i, (label, line)) in crate::designs::BOX_LABELS.iter().zip(lines.iter()).enumerate() {
            let y = y0 + first + step * i as f32;
            let label = cockpit.string(*label).to_owned();
            ink.text(&label, [x, y], super::designer::GREEN);
            let (value, unit) = line.rsplit_once(' ').unwrap_or((line.as_str(), ""));
            let w = ink.font.advance(value);
            ink.text(value, [x + 132.0 - w, y], super::designer::FIGURE);
            ink.text(unit, [x + 135.0, y], super::designer::GREEN);
        }
    }
    // The buttons along the bottom: telepresence at levels 0 to 2, strategic control for an HQ,
    // and Explode!.
    let boardable = play.can_take(t);
    let lit = if boardable { WHITE } else { GREY };
    for (at, name) in DRIVE_BUTTONS.iter().zip([
        "buildscreen_hq_icon",
        "botscreen_autogunner_icon",
        "botscreen_autodriver_icon",
    ]) {
        put(cockpit, ink, "short_button_frame_off", [at[0], at[1], at[0] + 21.0, at[1] + 21.0], WHITE);
        put(cockpit, ink, name, [at[0] + 3.0, at[1] + 3.0, at[0] + 18.0, at[1] + 18.0], lit);
    }
    // Strategic control, for a unit of the HQ type, lit when its turret makes it an HQ
    // (`0x10085b5e`).
    if play.units.get(t).is_some_and(|u| u.type_word == HQ_TYPE) {
        let at = STRATEGIC_BUTTON;
        let lit = if play.is_hq(t) { WHITE } else { GREY };
        put(cockpit, ink, "short_button_frame_off", [at[0], at[1], at[0] + 21.0, at[1] + 21.0], WHITE);
        put(
            cockpit,
            ink,
            "buildscreen_direct_icon",
            [at[0] + 3.0, at[1] + 3.0, at[0] + 18.0, at[1] + 18.0],
            lit,
        );
    }
    // Explode!, `_on` with its icon grey while one is pending.
    let pending = play.explode_pending(t);
    let [ex0, ey0, ex1, ey1] = EXPLODE_BUTTON;
    let frame = if pending { "long_button_frame_on" } else { "long_button_frame_off" };
    put(cockpit, ink, frame, [ex0, ey0, ex1, ey1], WHITE);
    let lit = if pending { GREY } else { WHITE };
    put(cockpit, ink, "self_destruction_icon", [ex0 + 2.0, ey0 + 2.0, ex0 + 32.0, ey0 + 17.0], lit);
}

/// A row's bar over its unit's or building's life (`0x1009a380`, `0x1007e980`).
pub(super) fn life_bar(cockpit: &Cockpit, ink: &mut Ink, play: &Play, t: usize, [x0, y, x1]: [f32; 3]) {
    put(cockpit, ink, "ccres_ray_body", [x0, y, x1, y + ROW_HEIGHT], WHITE);
    let share = play
        .battle
        .combat
        .targets
        .get(t)
        .map_or(0.0, |target| life_share(target.parts.iter().filter_map(|p| p.life.as_ref())));
    let percent = (share * 100.0).round() as i32;
    if percent > 0 {
        let w = (x1 - x0) * percent.min(100) as f32 / 100.0;
        ink.painter.fill(Blend::Alpha, [x0, y + 3.0, w - 2.0, ROW_HEIGHT - 6.0], argb(fill_colour(percent)));
    }
}

/// A building's icon cell (`0x100344e0`) and its tint by its record `+0x30`.
fn building_icon(type_word: u32) -> [f32; 2] {
    match type_word {
        0x8000_0008 => [81.0, 110.0],
        0x8000_0004 => [129.0, 126.0],
        0x8000_0002 => [113.0, 126.0],
        0x8000_0010 => [129.0, 94.0],
        0x8000_0040 => [97.0, 126.0],
        0x8000_0400 => [49.0, 126.0],
        t if hq::within(t, BUNKERS) => [65.0, 126.0],
        _ => [81.0, 126.0],
    }
}

fn building_rows(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, mask: u32, top: f32) {
    let text_down = ((ROW_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    for (i, t) in buildings_of(play, mask).into_iter().enumerate() {
        let y = top + ROW_STEP * i as f32;
        let lit = play.selected.contains(&t);
        let look = if lit { "normal" } else { "off" };
        let type_word = play.units[t].type_word;
        let mut pen = ROWS_LEFT;
        put(
            cockpit,
            ink,
            &format!("ccres_lamp_text_ending_{look}"),
            [pen, y, pen + 10.0, y + ROW_HEIGHT],
            WHITE,
        );
        pen += 10.0;
        put(cockpit, ink, "ccres_body_text", [pen, y, pen + 20.0, y + ROW_HEIGHT], WHITE);
        let tint = building_tint(building_class(play, t));
        icon(cockpit, ink, building_icon(type_word), [pen + 2.0, y + 2.0], tint);
        pen += 20.0;
        put(cockpit, ink, "ccres_separator_left_text", [pen, y, pen + 5.0, y + ROW_HEIGHT], WHITE);
        pen += 5.0;
        let bunker = hq::within(type_word, BUNKERS);
        if bunker {
            put(
                cockpit,
                ink,
                "ccres_short_button_normal",
                [pen, y, pen + BUILDING_BUTTON, y + ROW_HEIGHT],
                WHITE,
            );
            put(
                cockpit,
                ink,
                "buildscreen_direct_icon",
                [pen + 10.0, y + 2.0, pen + 25.0, y + 17.0],
                factory::ICON_VARIANTS[1],
            );
            pen += BUILDING_BUTTON;
        }
        if bunker || hq::within(type_word, TOWERS) {
            put(
                cockpit,
                ink,
                "ccres_short_button_normal",
                [pen, y, pen + BUILDING_BUTTON, y + ROW_HEIGHT],
                WHITE,
            );
            put(
                cockpit,
                ink,
                "buildscreen_hq_icon",
                [pen + 10.0, y + 2.0, pen + 25.0, y + 17.0],
                factory::ICON_VARIANTS[1],
            );
            pen += BUILDING_BUTTON;
        }
        put(cockpit, ink, &format!("ccres_ray_emitter_{look}"), [pen, y, pen + 10.0, y + ROW_HEIGHT], WHITE);
        pen += 10.0;
        life_bar(cockpit, ink, play, t, [pen, y, BUILDING_BAR_END]);
        let name = play.building_name(t, &cockpit.strings);
        ink.centred(&name, pen, BUILDING_BAR_END - pen, y + text_down, if lit { WHITE } else { GREY });
        put(
            cockpit,
            ink,
            "ccres_ray_ending",
            [BUILDING_BAR_END, y, BUILDING_BAR_END + 6.0, y + ROW_HEIGHT],
            WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_units_icons_are_picked_by_its_type_and_its_chassis_type_and_tinted_by_its_size() {
        // `0x10077120`: the first by the Type compared whole, the second by property `0x207`
        // through the table at `0x100773fc`, the tint by `+0x30` through `0x100773ec`.
        let builder = unit_cells(0x0100_4000, 3);
        assert_eq!(builder, [Some([65.0, 110.0]), Some([65.0, 94.0])]);
        assert_eq!(unit_cells(0x0100_2000, 4), [Some([81.0, 110.0]), Some([81.0, 94.0])]);
        for warrior in [0x0100_8000, 0x0101_0000, 0x0102_0000] {
            assert_eq!(unit_cells(warrior, 1), [Some([49.0, 110.0]), Some([97.0, 94.0])]);
        }
        assert_eq!(unit_cells(0x0100_8000, 2)[1], Some([49.0, 94.0]));
        assert_eq!(unit_cells(0x2000_0000, 5), [None, None], "an animal's Type and a ChassisType past 4");
        let tints: Vec<u32> = (1..=4).map(unit_tint).collect();
        assert_eq!(tints, [0xffff_e7ff, 0xffff_8080, 0xff80_ff80, 0xff80_80ff]);
    }

    #[test]
    fn a_buildings_icon_is_tinted_by_its_size_class() {
        // `0x100344fc`, table `0x100347dc`: 1 white, 2 red, 3 green, 4 and 5 blue.
        let tints: Vec<u32> = (1..=5).map(building_tint).collect();
        assert_eq!(tints, [WHITE, 0xffff_0000, 0xff00_ff00, 0xff00_00ff, 0xff00_00ff]);
    }
}
