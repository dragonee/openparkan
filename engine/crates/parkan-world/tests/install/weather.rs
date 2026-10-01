//! The weather a mission's atmosphere runs: `docs/10-sky.md`, "The weather".

use crate::common::*;
use glam::Vec3;
use parkan_formats::gamedir;
use parkan_sim::sky::Kind;
use parkan_sim::weather::{ALPHA, BOLT_HEIGHT, COLOUR_FLOOR, DRAWN_MOST, FAR, Fall, NEAR, View, count};
use parkan_world::fx::Owner;
use parkan_world::play::{Lit, Play};

const C03_M02: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.03/Mission.02";
const TICK: f64 = 1000.0 / 60.0;

fn with_weather(path: &str) -> Play {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut play = campaign_play(path);
    play.load_weather(&gamedir::resolve(&game, path).unwrap());
    play
}

fn view(play: &Play) -> View {
    let e = play.eye();
    View::new(e.position, e.forward, e.up, e.fov_x, (1400.0, 1050.0))
}

#[test]
#[ignore = "needs the game install"]
fn c03_m02_snows_dust_from_its_first_tick() {
    let mut play = with_weather(C03_M02);
    {
        let w = play.weather.as_mut().expect("a sky.ske");
        // Its snow slot holds the dust, and it never rains there, so the rain's material is
        // not asked for.
        assert_eq!(w.material(Fall::Snow), Some("DUST_ADD"));
        assert_eq!(w.material(Fall::Rain), None);
        // The clock starts 18 s into a 900 s day: the snow's start at 00:00 and the
        // lightning's at 00:01 are both behind it, and both run at 1.
        assert_eq!(w.running(0.0, Kind::Snow).map(|r| r.0), Some(1.0));
        let (strength, started) = w.running(0.0, Kind::Lightning).expect("lightning from 00:01");
        assert_eq!((strength, started.effects[0].as_str()), (1.0, "env_lightning"));
        assert!(w.running(0.0, Kind::Rain).is_none());
        // The looks resolve as the world's textures load; any index will do to count quads.
        w.looks = [Some(0), None];
    }
    play.tick(TICK, [0.0; 2]);
    let v = view(&play);
    let drawn = play.weather_specks(&v);
    let w = play.weather.as_ref().unwrap();
    // A thousand at the hero's own field of view, or its box's share of them.
    assert_eq!(w.count(Fall::Snow), count(1.0, &v.reach()));
    assert!(w.count(Fall::Snow) > 600, "{}", w.count(Fall::Snow));
    assert_eq!(w.count(Fall::Rain), 0);
    // A third or so of the box is in view, the box being as wide near the eye as it is far.
    assert!((200..=DRAWN_MOST).contains(&drawn.len()), "{}", drawn.len());
    for d in &drawn {
        let (pixel, depth) = v.pixel(d.speck.anchors[0]);
        assert!((NEAR..=FAR).contains(&depth), "{depth}");
        assert!((0.0..=1400.0).contains(&pixel.x) && (0.0..=1050.0).contains(&pixel.y), "{pixel}");
    }
    // Slot 19 is black until the sun's keyframe at 00:30, so the dust is the floor's grey.
    assert_eq!(drawn[0].colour, [COLOUR_FLOOR, COLOUR_FLOOR, COLOUR_FLOOR, ALPHA]);
    // A minute on the sun is up and slot 19 red: the dust is red, its green and blue held
    // at the floor.
    for _ in 0..3600 {
        play.tick(TICK, [0.0; 2]);
    }
    let v = view(&play);
    let drawn = play.weather_specks(&v);
    assert!(drawn.len() > 200);
    assert_eq!(drawn[0].colour, [1.0, COLOUR_FLOOR, COLOUR_FLOOR, ALPHA]);
    assert!(play.rain_sound().is_none());
}

#[test]
#[ignore = "needs the game install"]
fn c03_m02s_lightning_strikes_the_map_every_six_to_nine_seconds() {
    let mut play = with_weather(C03_M02);
    // The bolt's effect was loaded with the weather, so its material's look resolves with the
    // rest.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut store = parkan_world::textures::TextureStore::open(&game).unwrap();
    play.fx.resolve_looks(&mut store).unwrap();
    let (lo, hi) = play.ground.bounds();
    let mut strikes: Vec<(f64, Vec3)> = Vec::new();
    for _ in 0..(60 * 60) {
        play.tick(TICK, [0.0; 2]);
        let now = play.hero.time_ms;
        let bolt = play.fx.instances.iter().find(|(o, _)| *o == Owner::Lightning).map(|(_, i)| i);
        let Some(bolt) = bolt.filter(|b| strikes.last().is_none_or(|s| s.0 != b.start_ms)) else { continue };
        strikes.push((bolt.start_ms, bolt.frame.origin));
        let at = bolt.frame.origin;
        assert!((lo[0]..=hi[0]).contains(&at.x) && (lo[1]..=hi[1]).contains(&at.y), "{at}");
        // The effect stands half its 600 above the ground, 40 wide and 600 tall.
        let ground = play.ground.landscape_top(at.x, at.y).unwrap_or(play.ground.world_box().0.z);
        assert!((at.z - ground - BOLT_HEIGHT / 2.0).abs() < 1e-3, "{at} over {ground}");
        assert_eq!(bolt.frame.axes.map(|a| a.length()), [40.0, 40.0, 600.0]);
        assert_eq!((bolt.effect.header.mode, bolt.effect.header.duration), (1, 0.75));
        // Its light starts at (7, 7, 10) and a range of 100 times the frame's 40, and both
        // run down to nothing over the bolt's 0.75 s; the header's jitter moves where the
        // first tick finds them, and the light's own moves its colour and its range.
        let mut lights = Vec::new();
        bolt.lights(now, &mut lights);
        assert_eq!(lights.len(), 1);
        let light = lights[0];
        assert!((3000.0..=4250.0).contains(&light.range) && light.position == at, "{light:?}");
        assert!((4.5..=7.6).contains(&light.colour.x) && light.colour.z > light.colour.x, "{light:?}");
        // It carries `0x20000000`, which keeps it out of the landscape's emulated discs and
        // out of nothing else: the shade's lighter takes it, for every vertex in range, with
        // the block's (0, 0, 1) -- the square of what is left of the range.
        assert_eq!(light.flags, parkan_sim::effects::LIGHT_NOT_EMULATED);
        let lit: Vec<_> = play.vertex_lights().into_iter().filter(|v| v.light.position == at).collect();
        assert_eq!(lit.len(), 1, "the bolt's light among the frame's");
        assert_eq!((lit[0].lit, lit[0].light.attenuation), (Lit::Everything, [0.0, 0.0, 1.0]));
        // A quarter of the range under it, a face turned up to it takes 0.56 of its colour.
        let l = lit[0].light;
        let got = l.diffuse_at(at - Vec3::Z * (l.range * 0.25), Vec3::Z);
        assert!((got - l.colour * 0.5625).length() < 1e-3, "{got} of {}", l.colour);
        // And its one sprite is the bolt, a mode-1 streak along its block's direction, the
        // frame's third axis: its picture's u runs 600 up the world, its v 40 across, and no
        // fog reaches it.
        let sprites = play.sprites(play.eye().position);
        let bolts: Vec<_> = sprites.iter().filter(|(_, s)| s.material == "env_lightning").collect();
        assert_eq!(bolts.len(), 1, "{sprites:?}");
        let s = &bolts[0].1;
        let [u, v, _] = s.matrix.expect("drawn by its mode");
        assert!((u - Vec3::Z * 600.0).length() < 1e-2, "{u}");
        assert!((v.length() - 40.0).abs() < 1e-2 && v.z.abs() < 1e-3, "{v}");
        assert!(s.unfogged && s.centre == at);
    }
    // Six seconds' rest and up to three more at full intensity.
    assert!(strikes.len() >= 6, "{strikes:?}");
    for pair in strikes.windows(2) {
        let gap = pair[1].0 - pair[0].0;
        assert!((6000.0..9100.0).contains(&gap), "{gap}");
    }
    // A bolt hurts nothing: the hero stands as it stood.
    assert!(!play.hero.dead());
}

#[test]
#[ignore = "needs the game install"]
fn single_01_rains_on_its_first_morning_and_the_rain_is_heard() {
    let mut play = with_weather("MISSIONS/Single.01");
    let e = play.eye();
    let w = play.weather.as_mut().expect("a sky.ske");
    assert_eq!(w.material(Fall::Rain), Some("RAIN_DROP"));
    assert_eq!(w.material(Fall::Snow), None);
    // The clock starts at 01:30, 56 s into a 900 s day; the rain starts at 05:40, 212 s in,
    // at no intensity, and is at 0.6 by 07:00, 262 s in.
    assert!(w.running(100_000.0, Kind::Rain).is_none());
    assert!(w.rain_sound(100_000.0).is_none());
    let at = (262.0 - 56.0) * 1000.0 + 500.0;
    let (strength, _) = w.running(at, Kind::Rain).expect("rain from 05:40");
    assert!((strength - 0.6).abs() < 0.01, "{strength}");
    let (sound, decibels) = w.rain_sound(at).expect("its sound");
    assert_eq!(sound, "atm_rain1.wav");
    assert!((decibels - -3.1).abs() < 0.1, "{decibels}");
    // The drops are kept and drawn as the snow is, 600 of the box's 1000 at 0.6.
    w.looks = [None, Some(0)];
    let v = View::new(e.position, e.forward, e.up, 1.3, (800.0, 600.0));
    w.specks(at, &v);
    let drawn = w.specks(at + TICK, &v);
    assert_eq!(w.count(Fall::Rain), 600);
    assert!(drawn.len() > 100, "{}", drawn.len());
    // A drop is stretched down the screen from where it was a frame before.
    let long = drawn.iter().filter(|d| d.speck.anchors[0] != d.speck.anchors[1]).count();
    assert!(long * 10 > drawn.len() * 8, "{long} of {}", drawn.len());
    // The rain stops at 11:00, 412 s in.
    assert!(w.running((413.0 - 56.0) * 1000.0, Kind::Rain).is_none());
}
