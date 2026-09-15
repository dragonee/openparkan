//! The research panel (`iron3d.dll:0x100898a0`): command mode's page 4, and the screen a
//! player's research centre's pod opens. Its box shows the selected technology with its
//! descendants and its part turning, the batch button, the scroll row, and a row for each
//! technology open to research with its start or stop button and its progress. See
//! `docs/41-commander.md`, "The research page, 4".

use glam::Vec3;

use super::weapons::fill_colour;
use super::{Cockpit, Ink, WHITE, argb, factory, map, messages};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// The page header's title, and the box's words (`0x10088c80`, `0x100890e0`).
pub const STRING_RESEARCH_CENTER: u32 = 5080;
pub const STRING_NO_ITEM: u32 = 5078;
pub const STRING_DESCENDANTS: u32 = 5079;
/// The box, framed and filled as the factory's (`0x10088c80`).
pub const BOX: [f32; 4] = [51.0, 20.0, 369.0, 158.0];
pub const NAME_AT: [f32; 2] = [71.0, 30.0];
pub const DESCENDANTS_AT: [f32; 2] = [71.0, 48.0];
pub const DESCENDANT_X: f32 = 86.0;
pub const DESCENDANTS_TOP: f32 = 60.0;
pub const DESCENDANTS_SHOWN: usize = 4;
pub const DESCENDANT_COLOUR: u32 = 0xffc0_c0ff;
/// The preview: the technology's part turning, 127 × 127.
pub const PREVIEW: [f32; 4] = [226.0, 18.0, 127.0, 127.0];
/// The batch button's frame and its icon, white on the frame after a click and `#80ff80`
/// otherwise.
pub const BATCH_FRAME: [f32; 4] = [170.0, 127.0, 202.0, 148.0];
pub const BATCH_ICON: [f32; 4] = [172.0, 129.0, 200.0, 146.0];
/// The scroll row's pen, its body's width, and its two long buttons.
pub const SCROLL_Y: f32 = 159.0;
pub const SCROLL_BODY: f32 = 208.0;
pub const SCROLL_UP: [f32; 2] = [264.0, 314.0];
pub const SCROLL_DOWN: [f32; 2] = [314.0, 364.0];
/// The rows: 12 shown from (51, 179), 20 apart; a row's button, emitter and bar.
pub const ROWS_TOP: f32 = 179.0;
pub const ROW_STEP: f32 = 20.0;
pub const ROW_HEIGHT: f32 = 19.0;
pub const ROWS_SHOWN: usize = 12;
pub const ROW_BUTTON: [f32; 2] = [56.0, 91.0];
pub const ROW_ICON_X: f32 = 66.0;
pub const ROW_BAR: [f32; 2] = [101.0, 363.0];
pub const QUEUED_COLOUR: u32 = 0xffff_8080;
pub const LABEL_GREY: u32 = 0xffc0_c0c0;
pub const ICON_GREEN: u32 = 0xff80_ff80;

/// What the panel keeps: its rows, the first shown, the selected technology, the batch
/// button lit for a frame, and the preview's part and sphere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panel {
    pub rows: Vec<usize>,
    pub first: usize,
    pub selected: Option<usize>,
    pub flash: bool,
    pub preview: Option<(String, Option<(Vec3, f32)>)>,
    /// The play's count of reported researches when the rows were last made.
    pub reported: u64,
    /// Whether the panel was up at the last update.
    pub shown: bool,
}

/// What a click on the panel did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// The header's exit.
    Exit,
    Taken,
    /// Not the panel's.
    Outside,
}

fn inside([x0, y0, x1, y1]: [f32; 4], [x, y]: [f32; 2]) -> bool {
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
}

fn row_y(k: usize) -> f32 {
    ROWS_TOP + ROW_STEP * k as f32
}

impl Panel {
    /// The refresh (`0x10089fd0`): the rows made again, a scroll past their end back to 0,
    /// and a selection not among them moved to the first row shown.
    pub fn refresh(&mut self, play: &Play) {
        self.rows = play.research_rows();
        self.reported = play.research.reported;
        if self.first >= self.rows.len() {
            self.first = 0;
        }
        if self.selected.is_none_or(|s| !self.rows.contains(&s))
            && let Some(&row) = self.rows.get(self.first)
        {
            self.selected = Some(row);
        }
    }

    /// The panel's area for the commander's click (`0x10089f90`): x 51–369, down to 177 + 20 ×
    /// the row count.
    pub fn bottom(&self) -> f32 {
        177.0 + ROW_STEP * self.rows.len() as f32
    }

    fn can_scroll_up(&self) -> bool {
        self.first > 0
    }

    fn can_scroll_down(&self) -> bool {
        self.rows.len() > ROWS_SHOWN && self.first + ROWS_SHOWN < self.rows.len()
    }

    /// Kept current every frame it is up: refreshed as it opens and after a research is
    /// reported, and the selected technology's part measured for its preview.
    pub fn update(&mut self, play: &mut Play, up: bool) {
        if up && (!self.shown || self.reported != play.research.reported) {
            self.refresh(play);
        }
        self.shown = up;
        if !up {
            return;
        }
        let part = self.selected.and_then(|i| {
            let tree = play.research.clan(play.player_clan)?;
            if tree.state.researched(i) {
                return None;
            }
            tree.tree.items.get(i)?.parts.first().cloned()
        });
        if self.preview.as_ref().map(|p| &p.0) != part.as_ref() {
            self.preview = part.map(|p| {
                let sphere = super::designer::part_sphere(&mut play.assembly, &p);
                (p, sphere)
            });
        }
    }

    /// A left click at `at` on the layout pinned to the top left (`0x10089ce0`), after the
    /// header's exit.
    pub fn click(&mut self, play: &mut Play, at: [f32; 2]) -> Click {
        if inside(factory::EXIT, at) {
            return Click::Exit;
        }
        if !inside([51.0, 0.0, 369.0, self.bottom()], at) {
            return Click::Outside;
        }
        // The batch button, with rows: every queued research cancelled, every row ordered.
        if !self.rows.is_empty() && inside(BATCH_FRAME, at) {
            self.flash = true;
            play.cancel_all_research();
            for &item in &self.rows {
                play.order_research(item);
            }
            if self.selected.is_none() {
                self.selected = self.rows.get(self.first).copied();
            }
            return Click::Taken;
        }
        let row = [SCROLL_Y, SCROLL_Y + ROW_HEIGHT];
        if inside([SCROLL_UP[0], row[0], SCROLL_UP[1], row[1]], at) {
            if self.can_scroll_up() {
                self.first -= 1;
            }
            return Click::Taken;
        }
        if inside([SCROLL_DOWN[0], row[0], SCROLL_DOWN[1], row[1]], at) {
            if self.can_scroll_down() {
                self.first += 1;
            }
            return Click::Taken;
        }
        for (k, &item) in self.rows.iter().skip(self.first).take(ROWS_SHOWN).enumerate() {
            let y = row_y(k);
            if !inside([51.0, y, 369.0, y + ROW_HEIGHT], at) {
                continue;
            }
            if inside([ROW_BUTTON[0], y, ROW_BUTTON[1], y + ROW_HEIGHT], at) {
                if play.research_queued(item) {
                    play.cancel_research(item);
                } else {
                    play.order_research(item);
                }
            }
            self.selected = Some(item);
            return Click::Taken;
        }
        Click::Taken
    }
}

fn put(cockpit: &Cockpit, ink: &mut Ink, name: &str, rect: [f32; 4], colour: u32) {
    if let Some(p) = cockpit.skin.get(name) {
        ink.painter.piece(p, rect, argb(colour));
    }
}

/// A long button (`0x1009a9e0`) from `x0` to `x1` at `y`: its piece for its variant and its
/// 30 × 15 icon at (+10, +2) in the variant's colour.
fn long_button(cockpit: &Cockpit, ink: &mut Ink, [x0, x1]: [f32; 2], y: f32, icon: &str, variant: usize) {
    let piece =
        ["ccres_long_button_off", "ccres_long_button_normal", "ccres_long_button_pressed"][variant.min(2)];
    put(cockpit, ink, piece, [x0, y, x1, y + ROW_HEIGHT], WHITE);
    put(
        cockpit,
        ink,
        icon,
        [x0 + 10.0, y + 2.0, x0 + 40.0, y + 17.0],
        factory::ICON_VARIANTS[variant.min(2)],
    );
}

/// The panel (`0x100898a0`): the box, the scroll row and the rows. Returns the preview.
pub fn panel(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    now_ms: f64,
) -> Vec<super::designer::Preview> {
    let mut previews = Vec::new();
    ink.painter.pin = Pin::TOP_LEFT;
    let panel = cockpit.commander.research.clone();
    let tree = play.research.clan(play.player_clan);
    let [x0, y0, x1, y1] = BOX;
    ink.painter.fill(
        Blend::Alpha,
        [x0 + 5.0, y0 + 5.0, x1 - x0 - 10.0, y1 - y0 - 10.0],
        argb(factory::BOX_FILL),
    );
    messages::frame(cockpit, ink, BOX);
    // STAND-IN: docs/41-commander.md#not-established -- the box's name is drawn in
    // `GAME_FONT`'s colour as whatever drew before left it: white, as the recording reads.
    let shown = panel.selected.zip(tree).filter(|(i, t)| !t.state.researched(*i));
    match shown {
        Some((item, tree)) => {
            let name = tree.tree.items.get(item).map(|i| i.name.clone()).unwrap_or_default();
            ink.text(&name, NAME_AT, WHITE);
            let unlocks = tree.state.unlocks(item);
            if !unlocks.is_empty() {
                let words = cockpit.string(STRING_DESCENDANTS).to_owned();
                ink.text(&words, DESCENDANTS_AT, DESCENDANT_COLOUR);
                let line = ink.font.line_height + 1.0;
                for (k, &u) in unlocks.iter().take(DESCENDANTS_SHOWN).enumerate() {
                    let text = tree.tree.items.get(u).map(label).unwrap_or_default();
                    ink.text(&text, [DESCENDANT_X, DESCENDANTS_TOP + k as f32 * line], DESCENDANT_COLOUR);
                }
                if unlocks.len() > DESCENDANTS_SHOWN {
                    let y = DESCENDANTS_TOP + DESCENDANTS_SHOWN as f32 * line;
                    ink.text("...", [DESCENDANT_X, y], DESCENDANT_COLOUR);
                }
            }
            // STAND-IN: docs/41-commander.md#what-it-draws--read -- the box clips its contents
            // 5 inside; the preview's view is not clipped to it.
            if let Some((part, Some((centre, radius)))) = panel.preview.as_ref() {
                let space = ink.painter.space;
                let [px, py] = space.pixel([PREVIEW[0], PREVIEW[1]], Pin::TOP_LEFT);
                let [sx, sy] = space.scales();
                let viewport = [px, py, PREVIEW[2] * sx, PREVIEW[3] * sy];
                let angle =
                    (now_ms as f32 * super::designer::PREVIEW_TURN_RATE).rem_euclid(std::f32::consts::TAU);
                let (view_proj, model) = super::designer::preview_camera(*centre, *radius, angle, viewport);
                previews.push(super::designer::Preview {
                    key: super::designer::PreviewKey {
                        kind: parkan_formats::mission::KIND_ROCK,
                        path: part.clone(),
                        version: 0,
                    },
                    viewport,
                    view_proj,
                    model,
                    lights: [Vec3::new(-1.0, 0.0, -1.0).normalize(), Vec3::new(1.0, 0.0, -1.0).normalize()],
                    paint: None,
                });
            }
        }
        None => {
            let words = cockpit.string(STRING_NO_ITEM).to_owned();
            ink.text(&words, NAME_AT, WHITE);
        }
    }
    let frame = if panel.flash { "long_button_frame_on" } else { "long_button_frame_off" };
    put(cockpit, ink, frame, BATCH_FRAME, WHITE);
    put(cockpit, ink, "batch_research_icon", BATCH_ICON, if panel.flash { WHITE } else { ICON_GREEN });
    cockpit.commander.research.flash = false;

    // The scroll row from (51, 159).
    let y = SCROLL_Y;
    put(cockpit, ink, "ccres_ending_stub", [51.0, y, 56.0, y + ROW_HEIGHT], WHITE);
    put(cockpit, ink, "ccres_body_stub", [56.0, y, 56.0 + SCROLL_BODY, y + ROW_HEIGHT], WHITE);
    long_button(cockpit, ink, SCROLL_UP, y, "scroll_up_icon", usize::from(panel.can_scroll_up()));
    long_button(cockpit, ink, SCROLL_DOWN, y, "scroll_down_icon", usize::from(panel.can_scroll_down()));
    put(cockpit, ink, "ccres_ending_stub", [369.0, y, 364.0, y + ROW_HEIGHT], WHITE);

    // The rows (`0x100885f0`).
    let text_down = ((ROW_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    for (k, &item) in panel.rows.iter().skip(panel.first).take(ROWS_SHOWN).enumerate() {
        let y = row_y(k);
        let selected = panel.selected == Some(item);
        let look = if selected { "normal" } else { "off" };
        put(cockpit, ink, "ccres_ending_stub", [51.0, y, 56.0, y + ROW_HEIGHT], WHITE);
        put(
            cockpit,
            ink,
            &format!("ccres_short_button_{look}"),
            [ROW_BUTTON[0], y, ROW_BUTTON[1], y + ROW_HEIGHT],
            WHITE,
        );
        let (icon, colour) = if play.research_queued(item) {
            ("resbutton_stop", QUEUED_COLOUR)
        } else {
            ("resbutton_start", WHITE)
        };
        put(cockpit, ink, icon, [ROW_ICON_X, y + 2.0, ROW_ICON_X + 15.0, y + 17.0], colour);
        put(
            cockpit,
            ink,
            &format!("ccres_ray_emitter_{look}"),
            [ROW_BUTTON[1], y, ROW_BAR[0], y + ROW_HEIGHT],
            WHITE,
        );
        let width = ROW_BAR[1] - ROW_BAR[0];
        put(cockpit, ink, "ccres_ray_body", [ROW_BAR[0], y, ROW_BAR[1], y + ROW_HEIGHT], WHITE);
        let progress = tree.and_then(|t| t.state.progress.get(item)).copied().unwrap_or(0.0);
        let percent = (progress * 100.0).round() as i32;
        if percent > 0 {
            let w = width * percent.min(100) as f32 / 100.0;
            ink.painter.fill(
                Blend::Alpha,
                [ROW_BAR[0], y + 3.0, w - 2.0, ROW_HEIGHT - 6.0],
                argb(fill_colour(percent)),
            );
        }
        let text = tree.and_then(|t| t.tree.items.get(item)).map(label).unwrap_or_default();
        ink.centred(&text, ROW_BAR[0], width, y + text_down, if selected { WHITE } else { LABEL_GREY });
        put(cockpit, ink, "ccres_ray_ending", [ROW_BAR[1], y, ROW_BAR[1] + 6.0, y + ROW_HEIGHT], WHITE);
    }
    previews
}

/// A technology's label, `"%s (%s)"` of its name and code, or the name alone with no code
/// (`0x100887f0`).
pub fn label(item: &parkan_formats::research::Item) -> String {
    if item.code.is_empty() { item.name.clone() } else { format!("{} ({})", item.name, item.code) }
}

/// The screen a player's research centre's pod opens (mode 5 with page 4, the column left
/// out): the resource rows, the header, the panel, the message box where mode 5 puts it, and
/// the satellite map.
pub fn screen(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    now_ms: f64,
) -> Vec<super::designer::Preview> {
    factory::resource_rows(cockpit, ink, play, now_ms);
    factory::header(cockpit, ink, STRING_RESEARCH_CENTER);
    let previews = panel(cockpit, ink, play, now_ms);
    ink.painter.pin = Pin::BOTTOM_RIGHT;
    let [left, top, width] = factory::MESSAGES_AT;
    messages::draw_at(cockpit, ink, now_ms, left, top, width);
    ink.painter.pin = Pin::TOP_RIGHT;
    map::draw(cockpit, ink, play, now_ms);
    previews
}
