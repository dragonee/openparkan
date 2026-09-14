//! The cockpit HUD's art and the geometry it is drawn with: the pages
//! `ui/game_resources.cfg` names, and sprites cut from them placed on the game's
//! 640 × 480 layout. See `docs/35-hud.md`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::resources::{GAME_RESOURCES, locate, read_cfg};

/// The layout the HUD's figures are given in.
pub const LAYOUT: [f32; 2] = [640.0, 480.0];

/// One page of HUD art: RGBA8 rows, top first.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The name the mission's minimap is kept under among the pages.
pub const MINIMAP: &str = "minimap";

/// The pages of the `textures` resource, by their resource index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pages {
    pub pages: Vec<Page>,
    /// The resource's names (`page9`, `icons`, …) and the index of the page each is.
    pub names: BTreeMap<String, usize>,
}

impl Pages {
    /// The `textures` resource of `ui/game_resources.cfg`: each name an entry index of its
    /// `library`, a `Texm` (docs/35-hud.md).
    pub fn open(game: &Path) -> Result<Pages> {
        let cfg = locate(game, GAME_RESOURCES).with_context(|| format!("no {GAME_RESOURCES}"))?;
        let blocks = read_cfg(&cfg)?;
        let textures = blocks.get("textures").context("no textures resource")?;
        let unquote = |v: &str| v.trim().trim_matches('"').to_owned();
        let library = textures.get("library").map(unquote).context("the textures name no library")?;
        let path = locate(game, &library).with_context(|| format!("no {library}"))?;
        let archive = parkan_formats::nres::Archive::open(&path)?;
        let mut out = Pages::default();
        for entry in &archive.entries {
            let texture = parkan_formats::texm::decode(archive.read(entry)?, &entry.name, None)?;
            out.pages.push(Page {
                name: entry.name.clone(),
                width: texture.width,
                height: texture.height,
                rgba: texture.levels.into_iter().next().unwrap_or_default(),
            });
        }
        for (key, value) in &textures.properties {
            if let Ok(index) = unquote(value).parse::<usize>()
                && key != "type"
                && index < out.pages.len()
            {
                out.names.insert(key.clone(), index);
            }
        }
        Ok(out)
    }

    /// Add the mission's minimap, the texture `mission.cfg`'s `minimap` resource names
    /// (`iron3d.dll:0x100736e7`), as the page [`MINIMAP`] names; whether it loaded.
    pub fn add_minimap(&mut self, game: &Path, mission_dir: &Path) -> Result<bool> {
        let Some(cfg) = parkan_formats::gamedir::resolve(mission_dir, "mission.cfg") else {
            return Ok(false);
        };
        let descriptors = parkan_formats::resources::descriptors(&read_cfg(&cfg)?);
        let Some(d) = descriptors.iter().find(|d| d.role == MINIMAP) else { return Ok(false) };
        let (Some(member), Some(path)) = (d.get(MINIMAP), locate(game, &d.library)) else { return Ok(false) };
        let archive = parkan_formats::nres::Archive::open(&path)?;
        let Some(entry) = archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case(member)) else {
            return Ok(false);
        };
        let texture = parkan_formats::texm::decode(archive.read(entry)?, &entry.name, None)?;
        self.names.insert(MINIMAP.to_owned(), self.pages.len());
        self.pages.push(Page {
            name: entry.name.clone(),
            width: texture.width,
            height: texture.height,
            rgba: texture.levels.into_iter().next().unwrap_or_default(),
        });
        Ok(true)
    }

    /// The page a resource name draws from.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }
}

/// A sprite cut from a page (`iron3d.dll:0x1008f830`): its page, its rectangle, and how many
/// quarter turns clockwise its corners take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub page: u16,
    pub rect: Rect,
    pub turns: u8,
}

impl Piece {
    /// Its rectangle's width and height.
    pub fn size(&self) -> [f32; 2] {
        [self.rect[2], self.rect[3]]
    }
}

/// The interface's named sprites: `ui/compaund.cfg`'s compound-control pieces and
/// `ui/hq.cfg`'s, each a texture name, an offset, a size and a rotation (docs/35-hud.md).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skin {
    /// By lower-case name.
    pub pieces: BTreeMap<String, Piece>,
}

/// The files the skin is read from.
pub const SKIN_FILES: [&str; 2] = ["ui/compaund.cfg", "ui/hq.cfg"];

impl Skin {
    /// Every object of the skin files naming a texture of `pages`; keys in any case.
    pub fn open(game: &Path, pages: &Pages) -> Result<Skin> {
        let mut out = Skin::default();
        for file in SKIN_FILES {
            let path = locate(game, file).with_context(|| format!("no {file}"))?;
            for block in read_cfg(&path)?.0 {
                let get = |key: &str| {
                    block
                        .properties
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(key))
                        .map(|(_, v)| v.trim().trim_matches('"').to_owned())
                };
                let number = |key: &str| get(key).and_then(|v| v.parse::<f32>().ok());
                let (Some(texture), Some(x), Some(y), Some(w), Some(h)) = (
                    get("texture"),
                    number("offset_x"),
                    number("offset_y"),
                    number("width"),
                    number("height"),
                ) else {
                    continue;
                };
                let Some(page) = pages.index(&texture) else { continue };
                let turns = (number("rotate").unwrap_or(0.0) / 90.0).round().rem_euclid(4.0) as u8;
                out.pieces.insert(
                    block.name.to_ascii_lowercase(),
                    Piece { page: page as u16, rect: [x, y, w, h], turns },
                );
            }
        }
        Ok(out)
    }

    pub fn get(&self, name: &str) -> Option<Piece> {
        self.pieces.get(&name.to_ascii_lowercase()).copied()
    }
}

/// Where an element stays when the screen is not the layout's shape: the fraction of the
/// layout (0 left or top, 1 right or bottom) that sits on the same fraction of the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin(pub f32, pub f32);

impl Pin {
    pub const TOP_LEFT: Pin = Pin(0.0, 0.0);
    pub const TOP: Pin = Pin(0.5, 0.0);
    pub const TOP_RIGHT: Pin = Pin(1.0, 0.0);
    pub const CENTRE: Pin = Pin(0.5, 0.5);
    pub const BOTTOM_LEFT: Pin = Pin(0.0, 1.0);
    pub const BOTTOM: Pin = Pin(0.5, 1.0);
    pub const BOTTOM_RIGHT: Pin = Pin(1.0, 1.0);
}

/// A screen the layout is drawn on.
///
/// The game scales the layout by the screen's width over 640 across and its height over 480
/// down (`services.dll:0x10004a80`, `0x10004a90`, docs/35-hud.md, "How the radar draws"), so
/// on a screen of another shape the HUD stretches; `stretch` does so.
///
/// DEPARTURE: docs/35-hud.md#how-the-radar-draws--read -- the game's screens were all the
/// layout's shape. Without `stretch` the layout scales by the height alone, and each
/// element keeps its pin to the screen's edges, so a wide window keeps the radar and the
/// reticle round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    pub width: f32,
    pub height: f32,
    pub stretch: bool,
}

impl Space {
    pub fn new(width: f32, height: f32) -> Space {
        Space { width, height, stretch: false }
    }

    /// The scale text is drawn at: the height's.
    pub fn scale(&self) -> f32 {
        self.height / LAYOUT[1]
    }

    /// The scale across and down.
    pub fn scales(&self) -> [f32; 2] {
        if self.stretch { [self.width / LAYOUT[0], self.scale()] } else { [self.scale(); 2] }
    }

    /// A layout point, pinned, in screen pixels (y down).
    pub fn pixel(&self, [x, y]: [f32; 2], Pin(px, py): Pin) -> [f32; 2] {
        let [sx, sy] = self.scales();
        [px * self.width + (x - px * LAYOUT[0]) * sx, py * self.height + (y - py * LAYOUT[1]) * sy]
    }

    /// The layout point, pinned, a screen pixel is at.
    pub fn layout(&self, [x, y]: [f32; 2], Pin(px, py): Pin) -> [f32; 2] {
        let [sx, sy] = self.scales();
        [(x - px * self.width) / sx + px * LAYOUT[0], (y - py * self.height) / sy + py * LAYOUT[1]]
    }

    /// A layout point, pinned, in normalised device coordinates (y up).
    pub fn ndc(&self, at: [f32; 2], pin: Pin) -> [f32; 2] {
        let [x, y] = self.pixel(at, pin);
        [x / self.width * 2.0 - 1.0, 1.0 - y / self.height * 2.0]
    }
}

/// How a batch meets what is under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Blend {
    /// Over, by the art's alpha.
    #[default]
    Alpha,
    /// Added, by the art's alpha.
    Add,
}

/// A HUD vertex: in screen pixels (y down), at a page's pixel, or flat colour with none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub page: Option<u16>,
    pub colour: [f32; 4],
}

/// A run of triangles drawn the same way, in order: under the views of units the HUD
/// shows, or over them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    pub blend: Blend,
    pub over: bool,
    pub vertices: Vec<Vertex>,
}

/// A rectangle of the layout or of a page: x, y, width, height.
pub type Rect = [f32; 4];

/// The HUD's triangles for one frame, given in layout pixels.
#[derive(Clone, Debug)]
pub struct Painter {
    pub space: Space,
    /// Where what is pushed from now on is pinned.
    pub pin: Pin,
    /// Whether what is pushed from now on draws over the views of units.
    pub over: bool,
    pub batches: Vec<Batch>,
}

impl Painter {
    pub fn new(space: Space) -> Painter {
        Painter { space, pin: Pin::CENTRE, over: false, batches: Vec::new() }
    }

    fn triangles(&mut self, blend: Blend) -> &mut Vec<Vertex> {
        let over = self.over;
        if self.batches.last().is_none_or(|b| b.blend != blend || b.over != over) {
            self.batches.push(Batch { blend, over, vertices: Vec::new() });
        }
        &mut self.batches.last_mut().expect("pushed above").vertices
    }

    /// A quad from its four corners in order around it, each with its page pixel.
    pub fn quad(
        &mut self,
        blend: Blend,
        corners: [[f32; 2]; 4],
        uvs: [[f32; 2]; 4],
        page: Option<u16>,
        colour: [f32; 4],
    ) {
        let (space, pin) = (self.space, self.pin);
        let v = |i: usize| Vertex { position: space.pixel(corners[i], pin), uv: uvs[i], page, colour };
        let out = self.triangles(blend);
        out.extend([0, 1, 2, 0, 2, 3].map(v));
    }

    /// `src` of `page` drawn at its own size with its top left at `at`.
    pub fn sprite(&mut self, blend: Blend, page: u16, src: Rect, at: [f32; 2], colour: [f32; 4]) {
        self.sprite_to(blend, page, src, [at[0], at[1], src[2], src[3]], colour);
    }

    /// `src` of `page` stretched over `dst`; a negative width or height mirrors it.
    pub fn sprite_to(&mut self, blend: Blend, page: u16, src: Rect, dst: Rect, colour: [f32; 4]) {
        let [sx, sy, sw, sh] = src;
        let [dx, dy, dw, dh] = dst;
        let (x0, x1, y0, y1) = (dx, dx + dw, dy, dy + dh);
        self.quad(
            blend,
            [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
            [[sx, sy], [sx + sw, sy], [sx + sw, sy + sh], [sx, sy + sh]],
            Some(page),
            colour,
        );
    }

    /// `src` of `page` at its own size, centred on `centre` and turned by `angle` radians
    /// clockwise on the screen.
    pub fn sprite_turned(
        &mut self,
        blend: Blend,
        page: u16,
        src: Rect,
        centre: [f32; 2],
        angle: f32,
        colour: [f32; 4],
    ) {
        let [sx, sy, sw, sh] = src;
        let (s, c) = angle.sin_cos();
        let at = |dx: f32, dy: f32| [centre[0] + dx * c - dy * s, centre[1] + dx * s + dy * c];
        let (hw, hh) = (sw / 2.0, sh / 2.0);
        self.quad(
            blend,
            [at(-hw, -hh), at(hw, -hh), at(hw, hh), at(-hw, hh)],
            [[sx, sy], [sx + sw, sy], [sx + sw, sy + sh], [sx, sy + sh]],
            Some(page),
            colour,
        );
    }

    /// `piece` over the quad (x₀, y₀)–(x₁, y₁), alpha-blended, the piece's top left at the
    /// first corner, so x₁ < x₀ mirrors it across and y₁ < y₀ mirrors it down
    /// (`iron3d.dll:0x1008f970`).
    pub fn piece(&mut self, piece: Piece, [x0, y0, x1, y1]: [f32; 4], colour: [f32; 4]) {
        let [sx, sy, sw, sh] = piece.rect;
        let uvs = [[sx, sy], [sx + sw, sy], [sx + sw, sy + sh], [sx, sy + sh]];
        let turns = usize::from(piece.turns % 4);
        self.quad(
            Blend::Alpha,
            [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
            std::array::from_fn(|i| uvs[(i + 4 - turns) % 4]),
            Some(piece.page),
            colour,
        );
    }

    /// A flat rectangle.
    pub fn fill(&mut self, blend: Blend, [x, y, w, h]: Rect, colour: [f32; 4]) {
        self.quad(blend, [[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [[0.0; 2]; 4], None, colour);
    }

    /// A triangle of `page`, each corner at its page pixel.
    pub fn textured_triangle(
        &mut self,
        blend: Blend,
        corners: [[f32; 2]; 3],
        uvs: [[f32; 2]; 3],
        page: u16,
        colour: [f32; 4],
    ) {
        let (space, pin) = (self.space, self.pin);
        let v = |i: usize| Vertex {
            position: space.pixel(corners[i], pin),
            uv: uvs[i],
            page: Some(page),
            colour,
        };
        let out = self.triangles(blend);
        out.extend([0, 1, 2].map(v));
    }

    /// A line from `a` to `b`, `width` layout units wide.
    pub fn line(&mut self, blend: Blend, a: [f32; 2], b: [f32; 2], width: f32, colour: [f32; 4]) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy).max(1e-6);
        let (nx, ny) = (-dy / length * width / 2.0, dx / length * width / 2.0);
        self.quad(
            blend,
            [[a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny], [b[0] - nx, b[1] - ny], [a[0] - nx, a[1] - ny]],
            [[0.0; 2]; 4],
            None,
            colour,
        );
    }

    /// A flat triangle.
    pub fn triangle(&mut self, blend: Blend, corners: [[f32; 2]; 3], colour: [f32; 4]) {
        let (space, pin) = (self.space, self.pin);
        let v =
            |i: usize| Vertex { position: space.pixel(corners[i], pin), uv: [0.0; 2], page: None, colour };
        let out = self.triangles(blend);
        out.extend([0, 1, 2].map(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_scales_by_height_and_each_element_keeps_its_pin() {
        let square = Space::new(1280.0, 960.0);
        assert_eq!(square.pixel([640.0, 480.0], Pin::TOP_LEFT), [1280.0, 960.0]);
        assert_eq!(square.pixel([0.0, 0.0], Pin::BOTTOM_RIGHT), [0.0, 0.0]);
        let wide = Space::new(1920.0, 960.0);
        assert_eq!(wide.pixel([640.0, 480.0], Pin::BOTTOM_RIGHT), [1920.0, 960.0]);
        assert_eq!(wide.pixel([0.0, 306.0], Pin::BOTTOM_LEFT), [0.0, 612.0]);
        assert_eq!(wide.pixel([320.0, 240.0], Pin::CENTRE), [960.0, 480.0]);
        assert_eq!(wide.ndc([320.0, 0.0], Pin::TOP), [0.0, 1.0]);
        assert_eq!(wide.layout([0.0, 612.0], Pin::BOTTOM_LEFT), [0.0, 306.0]);
        let stretched = Space { stretch: true, ..wide };
        assert_eq!(
            stretched.pixel([0.0, 306.0], Pin::CENTRE),
            [0.0, 612.0],
            "the game's stretch takes no pin"
        );
        assert_eq!(stretched.pixel([320.0, 240.0], Pin::BOTTOM_RIGHT), [960.0, 480.0]);
    }

    #[test]
    fn a_sprite_is_two_triangles_and_a_negative_width_mirrors_it() {
        let mut p = Painter::new(Space::new(640.0, 480.0));
        p.pin = Pin::TOP_LEFT;
        p.sprite(Blend::Alpha, 7, [55.0, 0.0, 14.0, 28.0], [10.0, 20.0], [1.0; 4]);
        p.sprite_to(Blend::Alpha, 7, [55.0, 0.0, 14.0, 28.0], [100.0, 20.0, -14.0, 28.0], [1.0; 4]);
        p.fill(Blend::Add, [0.0, 0.0, 4.0, 4.0], [1.0; 4]);
        assert_eq!(p.batches.len(), 2, "a change of blend starts a batch");
        let v = &p.batches[0].vertices;
        assert_eq!(v.len(), 12);
        assert_eq!((v[0].position, v[0].uv), ([10.0, 20.0], [55.0, 0.0]));
        assert_eq!((v[2].position, v[2].uv), ([24.0, 48.0], [69.0, 28.0]));
        assert_eq!(
            (v[6].position, v[6].uv),
            ([100.0, 20.0], [55.0, 0.0]),
            "mirrored: the art's left on the right"
        );
        assert_eq!(v[7].position, [86.0, 20.0]);
    }

    #[test]
    fn a_piece_turned_a_quarter_shows_its_bottom_left_at_the_top_left() {
        let mut p = Painter::new(Space::new(640.0, 480.0));
        p.pin = Pin::TOP_LEFT;
        let piece = Piece { page: 10, rect: [1.0, 238.0, 8.0, 8.0], turns: 1 };
        p.piece(piece, [100.0, 0.0, 108.0, 8.0], [1.0; 4]);
        let v = &p.batches[0].vertices;
        assert_eq!((v[0].position, v[0].uv), ([100.0, 0.0], [1.0, 246.0]));
        assert_eq!((v[1].position, v[1].uv), ([108.0, 0.0], [1.0, 238.0]));
    }

    #[test]
    fn a_turned_sprite_turns_clockwise_about_its_centre() {
        let mut p = Painter::new(Space::new(640.0, 480.0));
        p.sprite_turned(
            Blend::Alpha,
            0,
            [0.0, 0.0, 10.0, 2.0],
            [100.0, 100.0],
            std::f32::consts::FRAC_PI_2,
            [1.0; 4],
        );
        let top_left = p.batches[0].vertices[0].position;
        assert!((top_left[0] - 101.0).abs() < 1e-4 && (top_left[1] - 95.0).abs() < 1e-4, "{top_left:?}");
    }
}
