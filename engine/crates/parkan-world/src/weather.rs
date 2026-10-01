//! A mission's weather: the rain, snow and lightning its atmosphere's keyframes start and
//! stop (`docs/10-sky.md`, "The weather").
//!
//! The atmosphere makes an object as the clock passes its start and deletes it at its stop
//! (`Terrain.dll:0x1006fbb0`); each takt hands every object the fourth float of the two
//! keyframes around the clock, lerped, and rain and snow a colour made from slot 19
//! (`0x10070a20`).

use std::path::Path;

use parkan_formats::gamedir;
use parkan_formats::sky::{self, Atmosphere};
use parkan_sim::sky::{self as atm, Kind};
use parkan_sim::weather::{Fall, Fallen, Speck, Storm, Strike, View, colour, rain_volume};

/// The `sky.wea` slots the snow's and the rain's materials are in: the parameter the
/// start of each carries (`0x1006dc10`'s 7 and 8).
pub const SNOW_SLOT: usize = 7;
pub const RAIN_SLOT: usize = 8;

/// One particle to draw: its quad, the look of its fall's material, and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawn {
    pub look: usize,
    pub speck: Speck,
    pub colour: [f32; 4],
}

#[derive(Clone)]
pub struct Weather {
    pub atmosphere: Atmosphere,
    snow: Fallen,
    rain: Fallen,
    storm: Storm,
    /// The looks the snow and the rain draw with, once the effects' looks are resolved.
    pub looks: [Option<usize>; 2],
}

fn fresh(fall: Fall) -> Fallen {
    Fallen::new(match fall {
        Fall::Snow => 1,
        Fall::Rain => 3,
    })
}

fn kind(fall: Fall) -> Kind {
    match fall {
        Fall::Snow => Kind::Snow,
        Fall::Rain => Kind::Rain,
    }
}

impl Weather {
    /// The weather of the mission in `dir`: its `sky.ske`, and the `sky.wea` beside it for
    /// the two materials. `None` where the atmosphere does not load.
    pub fn load(dir: &Path) -> Option<Weather> {
        let path = gamedir::resolve(dir, "sky.ske")?;
        let mut atmosphere = sky::parse(&std::fs::read(&path).ok()?, &path.display().to_string()).ok()?;
        atmosphere.textures = gamedir::resolve(dir, "sky.wea")
            .and_then(|p| std::fs::read(&p).ok())
            .map(|b| parkan_formats::wea::parse(&b).materials)
            .unwrap_or_default();
        Some(Weather {
            atmosphere,
            snow: fresh(Fall::Snow),
            rain: fresh(Fall::Rain),
            storm: Storm::new(7),
            looks: [None; 2],
        })
    }

    /// The material a fall draws with, where the cycle ever starts one.
    pub fn material(&self, fall: Fall) -> Option<&str> {
        let slot = match fall {
            Fall::Snow => SNOW_SLOT,
            Fall::Rain => RAIN_SLOT,
        };
        let starts = atm::events(&self.atmosphere).iter().any(|e| e.start && e.kind == kind(fall));
        self.atmosphere.textures.get(slot).map(String::as_str).filter(|n| starts && !n.is_empty())
    }

    /// The effects the cycle's lightning starts name.
    pub fn bolt_effects(&self) -> Vec<String> {
        atm::events(&self.atmosphere)
            .iter()
            .filter(|e| e.start && e.kind == Kind::Lightning)
            .filter_map(|e| e.keyframe.effects.first().cloned())
            .collect()
    }

    /// What a kind of weather runs at `now_ms` into the mission, and the keyframe that
    /// started it; `None` while none runs.
    pub fn running(&self, now_ms: f64, kind: Kind) -> Option<(f32, &sky::Keyframe)> {
        let seconds = now_ms / 1000.0;
        let started = atm::running(&self.atmosphere, seconds, kind)?;
        let state = atm::at(&self.atmosphere, atm::position(&self.atmosphere, seconds))?;
        Some((state.weather, started))
    }

    /// How many particles a fall holds.
    pub fn count(&self, fall: Fall) -> usize {
        match fall {
            Fall::Snow => self.snow.world.len(),
            Fall::Rain => self.rain.world.len(),
        }
    }

    /// This frame's particles through `view`, each fall updated first as the game's draw
    /// updates it (`0x10073c10`). A fall whose spell has ended loses its particles with its
    /// object.
    pub fn specks(&mut self, now_ms: f64, view: &View) -> Vec<Drawn> {
        let seconds = now_ms / 1000.0;
        let state = atm::at(&self.atmosphere, atm::position(&self.atmosphere, seconds));
        let mut out = Vec::new();
        for (i, fall) in [Fall::Snow, Fall::Rain].into_iter().enumerate() {
            let on = atm::running(&self.atmosphere, seconds, kind(fall)).is_some();
            let fallen = if fall == Fall::Snow { &mut self.snow } else { &mut self.rain };
            let Some(state) = state.as_ref().filter(|_| on) else {
                if !fallen.world.is_empty() {
                    *fallen = fresh(fall);
                }
                continue;
            };
            fallen.update(fall, now_ms, view, state.weather);
            if let Some(look) = self.looks[i] {
                let colour = colour(state.weather_colour);
                out.extend(fallen.specks(fall, view).into_iter().map(|speck| Drawn { look, speck, colour }));
            }
        }
        out
    }

    /// The lightning's update at `now_ms` over a map from `lo` to `hi`: a strike, and the
    /// effect it plays.
    pub fn strike(&mut self, now_ms: f64, lo: [f32; 2], hi: [f32; 2]) -> Option<(Strike, String)> {
        let Some((intensity, started)) = self.running(now_ms, Kind::Lightning) else {
            self.storm = Storm::new(7);
            return None;
        };
        let effect = started.effects.first().cloned()?;
        let strike = self.storm.update(now_ms, intensity, lo, hi)?;
        Some((strike, effect))
    }

    /// The rain's background sound and its volume in decibels, while it rains.
    pub fn rain_sound(&self, now_ms: f64) -> Option<(String, f32)> {
        let (intensity, started) = self.running(now_ms, Kind::Rain)?;
        Some((started.effects.first()?.clone(), rain_volume(intensity)))
    }
}
