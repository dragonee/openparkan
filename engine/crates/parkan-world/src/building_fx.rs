//! A building's load group: the effects its controller creates as it is placed — console
//! screens, signs and lamps on its control points, chimney smoke, dock glows, and its doors'
//! sounds on their nodes — run as the hero's turret's load group is. See `docs/13-control.md`,
//! "A building's load group".

use glam::Vec3;
use parkan_formats::control::{
    ACT_EFFECT_NODE, ACT_EFFECT_POINTS, ACT_START_EFFECT, CONDITIONS, Controller, ENTRY_LOAD, run_group,
};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::{self, Mission};
use parkan_formats::pose::rotate;
use parkan_sim::combat::Part;
use parkan_sim::effects::Frame;

use crate::assembly::Assembly;

/// The time modes that read an owner node's animation value as it only rises, and as it only
/// falls (`Effect.dll:0x10005c60`, docs/11, "Effect time t").
pub const MODE_RISING: u32 = 16;
pub const MODE_FALLING: u32 = 17;

/// Where a load-group effect hangs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    /// Action 4: the centroid frame of three control points.
    Points([usize; 3]),
    /// Action 3: a node of the building's mesh.
    Node(usize),
}

/// One effect the load group creates, under its record's id.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub name: String,
    pub on: On,
    pub id: i32,
}

/// A placed building's load group, on the part whose controller it is.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingEffects {
    pub target: usize,
    pub part: usize,
    pub points: Vec<ControlPoint>,
    pub effects: Vec<Placed>,
    /// Action 10's starts: an id and the time mode it is started in.
    pub starts: Vec<(i32, u32)>,
}

fn read(assembly: &mut Assembly, library: &str, member: &str) -> Option<Vec<u8>> {
    assembly.archive(library)?.read_name(member).ok().map(<[u8]>::to_vec)
}

/// The value an owner node's animation shows through a rising or falling mode: mode 16 follows
/// the value up and to 0, holding it while it falls; mode 17 follows it down and to 1, holding
/// it while it rises (docs/11, "Effect time t").
pub fn monotone(mode: u32, last: f32, value: f32) -> f32 {
    match mode {
        MODE_RISING if value < last && value != 0.0 => last,
        MODE_FALLING if value > last && value != 1.0 => last,
        _ => value,
    }
}

impl BuildingEffects {
    /// Mission object `object`, target `target`: a building whose own record's controller,
    /// reached through its FORT record, has a load group with effects (`Control.dll:0x10009408`).
    pub fn load(
        assembly: &mut Assembly,
        mission: &Mission,
        object: usize,
        target: usize,
    ) -> Option<BuildingEffects> {
        let placed = mission.objects.get(object).filter(|o| o.kind == mission::KIND_BUILDING)?;
        let parts = assembly.parts(placed.kind, &placed.path);
        let (p, part) = parts.iter().enumerate().find(|(_, p)| p.host == -1)?;
        let ctl = assembly.library.record_slot(assembly.library.get(&part.record), "ctl", 0)?;
        let data = read(assembly, &ctl.library, &ctl.member)?;
        let controller: Controller = parkan_formats::control::parse(&data, &ctl.member).ok()?;
        let points = assembly
            .library
            .record_slot(assembly.library.get(&part.record), "cpt", 0)
            .and_then(|slot| read(assembly, &slot.library, &slot.member).map(|b| (b, slot)))
            .and_then(|(b, slot)| cpt::parse(&b, &slot.member).ok())
            .unwrap_or_default();
        let group = controller.group(ENTRY_LOAD);
        let mut effects = Vec::new();
        let mut starts = Vec::new();
        for r in run_group(&group, &[false; CONDITIONS]) {
            let [.., v4, v5, v6, v7, _] = r.values;
            match r.action() {
                ACT_EFFECT_POINTS if !r.resource.member.is_empty() => effects.push(Placed {
                    name: r.resource.member.clone(),
                    on: On::Points([v4, v5, v6].map(|v| usize::try_from(v).unwrap_or(0))),
                    id: v7,
                }),
                ACT_EFFECT_NODE if !r.resource.member.is_empty() => {
                    if let Ok(node) = usize::try_from(v4) {
                        effects.push(Placed { name: r.resource.member.clone(), on: On::Node(node), id: v7 });
                    }
                }
                ACT_START_EFFECT => starts.push((v4, u32::try_from(v5).unwrap_or(0))),
                _ => {}
            }
        }
        (!effects.is_empty()).then_some(BuildingEffects { target, part: p, points, effects, starts })
    }

    /// Control point `i` in the world: where it is on its node, as the node is posed now, and
    /// its direction.
    pub fn point(&self, part: &Part, i: usize) -> Option<(Vec3, Vec3)> {
        let p = self.points.get(i)?;
        let node = usize::try_from(p.nodes().0).ok()?;
        let pose = part.nodes.get(node)?;
        let at = pose.apply(p.position.map(|v| f64::from(v * part.scale)));
        let dir = rotate(pose.rotation, p.direction.map(f64::from));
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        Some((f(at), f(dir)))
    }

    /// The frame an effect hangs on now. An action-3 effect's is its node's world matrix: the
    /// instance asks `IAnimation` (interface `0xb`) slot 4 for the node with 2
    /// (`Effect.dll:0x1000625a`), which hands back the node record's `+0x20`
    /// (`AniMesh.dll:0x10005320`), the matrix the pose walk builds as the object's world matrix
    /// times the node's chain of keyed poses (`0x10008b30`, `0x1000919a`). So it stands at the
    /// node's own origin as the node is posed now, wherever the geometry the node draws lies:
    /// on 68 of `fortif.rlb`'s 112 door sounds more than 10 m from the door, 30.8 m on the three
    /// factories' side doors (docs/13, "A building's load group").
    pub fn frame(&self, part: &Part, on: On) -> Option<Frame> {
        match on {
            On::Points(points) => {
                let [a, b, c] = points.map(|i| self.point(part, i));
                Some(Frame::from_points([a?, b?, c?]))
            }
            On::Node(node) => {
                let pose = part.nodes.get(node)?;
                let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
                let at = f(pose.translation);
                let y = f(rotate(pose.rotation, [0.0, 1.0, 0.0]));
                Some(Frame::along(at, y, 1.0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rising_mode_holds_while_its_value_falls_and_a_falling_mode_while_it_rises() {
        // A door opening, then closing: the open sound follows the rise, the close sound the fall.
        let values = [0.0, 0.4, 1.0, 0.6, 0.0, 0.5];
        let run = |mode| {
            let mut last = 0.0;
            values
                .iter()
                .map(|&v| {
                    last = monotone(mode, last, v);
                    last
                })
                .collect::<Vec<f32>>()
        };
        assert_eq!(run(MODE_RISING), vec![0.0, 0.4, 1.0, 1.0, 0.0, 0.5]);
        assert_eq!(run(MODE_FALLING), vec![0.0, 0.0, 1.0, 0.6, 0.0, 0.0]);
        assert_eq!(monotone(1, 0.7, 0.2), 0.2, "other modes take the value");
    }
}
