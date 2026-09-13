//! `data.tma`, a mission: routes, clans, placed objects. See `docs/04-missions.md`.

use crate::cursor::{Cursor, FormatError, latin1};

/// The second word of every object record.
pub const OBJECT_MARKER: u32 = 0x8000_0002;
/// MSVC's fill for uninitialised memory, found in the tail of string buffers.
pub const DEBUG_FILL: u8 = 0xCD;
/// A string longer than this is not one.
pub const MAX_STRING: u32 = 4096;

pub const KIND_BUILDING: u32 = 0;
pub const KIND_UNIT: u32 = 1;
pub const KIND_VEGETATION: u32 = 2;
pub const KIND_ROCK: u32 = 3;

pub const TYPE_FLOAT: u32 = 0;

#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub id: u32,
    pub points: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub kind: u32,
    pub position: [f32; 3],
    pub inner: f32,
    pub outer: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clan {
    pub name: String,
    pub parent: i32,
    pub base: [f32; 2],
    /// 0 nature, 1 player, 2 enemy, 3 neutral.
    pub kind: u32,
    pub ai_script: String,
    pub zones: Vec<Zone>,
    pub behaviour: String,
    /// How many bots the clan can have at once.
    pub minds: u32,
    /// Towards each other clan, by name, in file order.
    pub relations: Vec<(String, u32)>,
}

/// A property's value and bounds, as floats or ints by its type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Float(f32),
    Int(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub kind: u32,
    pub value: Value,
    pub minimum: Value,
    pub maximum: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub kind: u32,
    pub path: String,
    pub unknown_q: u32,
    pub logical_id: i32,
    pub position: [f32; 3],
    pub pad: [u32; 2],
    /// Radians about z.
    pub rotation: f32,
    pub scale: [f32; 3],
    pub name: String,
    pub tail: (u32, i32, i32, u32),
    pub properties: Vec<Property>,
}

impl Object {
    pub fn property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }

    pub fn clan_id(&self) -> Option<i64> {
        self.property("ClanID").map(|p| match p.value {
            Value::Float(v) => v as i64,
            Value::Int(v) => i64::from(v),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Viewpoint {
    pub position: [f32; 3],
    pub unknown: [u32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mission {
    pub version: u32,
    pub routes: Vec<Route>,
    pub clans: Vec<Clan>,
    pub unknown_pre_objects: u32,
    pub objects: Vec<Object>,
    pub map_path: String,
    pub description: String,
    pub viewpoints: Vec<Viewpoint>,
}

impl Mission {
    /// `DATA\MAPS\Tut_1\land` -> `Tut_1`.
    pub fn map_name(&self) -> &str {
        let parts: Vec<&str> = self.map_path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 { parts[parts.len() - 2] } else { "" }
    }
}

fn string(r: &mut Cursor) -> Result<String, FormatError> {
    let at = r.pos;
    let n = r.u32()?;
    if n > MAX_STRING || n as usize > r.remaining() {
        return Err(FormatError::invalid(r.source(), format!("implausible string length {n} at {at}")));
    }
    let raw = r.bytes(n as usize)?;
    let raw = match raw.iter().position(|&b| b == DEBUG_FILL) {
        Some(cut) => &raw[..cut],
        None => raw,
    };
    let end = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    Ok(latin1(&raw[..end]))
}

fn clan(r: &mut Cursor) -> Result<Clan, FormatError> {
    let name = string(r)?;
    let parent = r.i32()?;
    let base = [r.f32()?, r.f32()?];
    let kind = r.u32()?;
    let ai_script = string(r)?;
    let zones = (0..r.u32()?)
        .map(|_| Ok(Zone { kind: r.u32()?, position: r.vec3()?, inner: r.f32()?, outer: r.f32()? }))
        .collect::<Result<_, FormatError>>()?;
    let behaviour = string(r)?;
    let minds = r.u32()?;
    let mut relations: Vec<(String, u32)> = Vec::new();
    for _ in 0..r.u32()? {
        let other = string(r)?;
        let word = r.u32()?;
        match relations.iter_mut().find(|(n, _)| *n == other) {
            Some(slot) => slot.1 = word,
            None => relations.push((other, word)),
        }
    }
    Ok(Clan { name, parent, base, kind, ai_script, zones, behaviour, minds, relations })
}

fn object(r: &mut Cursor) -> Result<Object, FormatError> {
    let kind = r.u32()?;
    let marker = r.u32()?;
    if marker != OBJECT_MARKER {
        return Err(FormatError::invalid(
            r.source(),
            format!("expected the object marker at {}, found {marker:#010x}", r.pos - 4),
        ));
    }
    let path = string(r)?;
    let unknown_q = r.u32()?;
    let logical_id = r.i32()?;
    let position = r.vec3()?;
    let pad = [r.u32()?, r.u32()?];
    let rotation = r.f32()?;
    let scale = r.vec3()?;
    let name = string(r)?;
    let tail = (r.u32()?, r.i32()?, r.i32()?, r.u32()?);
    let mut properties = Vec::new();
    for _ in 0..r.u32()? {
        let kind = r.u32()?;
        let raw = [r.u32()?, r.u32()?, r.u32()?];
        let name = string(r)?;
        let value = |w: u32| {
            if kind == TYPE_FLOAT { Value::Float(f32::from_bits(w)) } else { Value::Int(w as i32) }
        };
        let property =
            Property { name, kind, value: value(raw[0]), minimum: value(raw[1]), maximum: value(raw[2]) };
        match properties.iter_mut().find(|p: &&mut Property| p.name == property.name) {
            Some(slot) => *slot = property,
            None => properties.push(property),
        }
    }
    Ok(Object { kind, path, unknown_q, logical_id, position, pad, rotation, scale, name, tail, properties })
}

/// Parse a `data.tma`.
pub fn parse(data: &[u8], source: &str) -> Result<Mission, FormatError> {
    let mut r = Cursor::new(data, source);
    let version = r.u32()?;
    let routes = (0..r.u32()?)
        .map(|_| {
            let id = r.u32()?;
            let points = (0..r.u32()?).map(|_| r.vec3()).collect::<Result<_, _>>()?;
            Ok(Route { id, points })
        })
        .collect::<Result<_, FormatError>>()?;
    r.u32()?;
    let clans = (0..r.u32()?).map(|_| clan(&mut r)).collect::<Result<_, _>>()?;
    let unknown_pre_objects = r.u32()?;
    let objects = (0..r.u32()?).map(|_| object(&mut r)).collect::<Result<_, _>>()?;
    let map_path = string(&mut r)?;
    r.u32()?;
    let description = string(&mut r)?;
    r.u32()?;
    let viewpoints = (0..r.u32()?)
        .map(|_| Ok(Viewpoint { position: r.vec3()?, unknown: [r.u32()?, r.u32()?, r.u32()?, r.u32()?] }))
        .collect::<Result<_, FormatError>>()?;
    if r.remaining() != 0 {
        return Err(FormatError::invalid(source, format!("{} bytes left after the trailer", r.remaining())));
    }
    Ok(Mission { version, routes, clans, unknown_pre_objects, objects, map_path, description, viewpoints })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Writer(Vec<u8>);

    impl Writer {
        fn u32(&mut self, v: u32) -> &mut Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn f32(&mut self, v: f32) -> &mut Self {
            self.u32(v.to_bits())
        }
        fn str(&mut self, s: &[u8]) -> &mut Self {
            self.u32(s.len() as u32);
            self.0.extend_from_slice(s);
            self
        }
    }

    fn sample() -> Vec<u8> {
        let mut w = Writer(Vec::new());
        w.u32(1).u32(1).u32(7).u32(2).f32(1.0).f32(2.0).f32(3.0).f32(4.0).f32(5.0).f32(6.0);
        w.u32(6).u32(1);
        w.str(b"Plr").u32(0).f32(10.0).f32(20.0).u32(1).str(b"ai.scr").u32(0).str(b"tree.trf").u32(40);
        w.u32(1).str(b"Plr").u32(1);
        w.u32(10).u32(1);
        w.u32(KIND_UNIT).u32(OBJECT_MARKER).str(b"UNITS\\UNITS\\HERO\\tut1_p.dat").u32(0).u32(5);
        w.f32(433.0).f32(477.1).f32(14.1).u32(0).u32(0).f32(-1.639).f32(1.0).f32(1.0).f32(1.0);
        w.str(b"tut1_p").u32(0).u32(u32::MAX).u32(0).u32(0);
        w.u32(2).u32(1).u32(0).u32(0).u32(u32::MAX).str(b"ClanID");
        w.u32(0).u32(0.5f32.to_bits()).u32(0).u32(1.0f32.to_bits()).str(b"Health");
        w.str(b"DATA\\MAPS\\Tut_1\\land").u32(0).str(b"Line of fire\xcd\xcd\xcdjunk").u32(0);
        w.u32(1).f32(1.0).f32(2.0).f32(3.0).u32(1).u32(2).u32(3).u32(4);
        w.0
    }

    #[test]
    fn a_mission_reads_to_the_end() {
        let m = parse(&sample(), "t").unwrap();
        assert_eq!(m.routes[0].points, vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
        assert_eq!(m.clans[0].name, "Plr");
        assert_eq!(m.clans[0].minds, 40);
        assert_eq!(m.clans[0].relations, vec![("Plr".to_owned(), 1)]);
        let hero = &m.objects[0];
        assert_eq!(hero.name, "tut1_p");
        assert_eq!(hero.position, [433.0, 477.1, 14.1]);
        assert_eq!(hero.clan_id(), Some(0));
        assert_eq!(hero.property("ClanID").unwrap().maximum, Value::Int(-1));
        assert_eq!(hero.property("Health").unwrap().value, Value::Float(0.5));
        assert_eq!(m.map_name(), "Tut_1");
        assert_eq!(m.description, "Line of fire");
        assert_eq!(m.viewpoints[0].unknown, [1, 2, 3, 4]);
    }

    #[test]
    fn a_leftover_byte_or_a_bad_marker_is_refused() {
        let mut data = sample();
        data.push(0);
        assert!(parse(&data, "t").is_err());
        let mut data = sample();
        let at = data.windows(4).position(|w| w == OBJECT_MARKER.to_le_bytes()).unwrap();
        data[at] = 0;
        assert!(parse(&data, "t").is_err());
    }
}
