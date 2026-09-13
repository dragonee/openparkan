//! Canonical JSON of what the readers make of a file.
//!
//! The twin of `openparkan/dump.py`: `openparkan golden` runs both and compares
//! them, so every key here must match a key there.

use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::mission::{self, Value as PropertyValue};
use parkan_formats::nres::Archive;
use parkan_formats::pose::Pose;
use parkan_formats::{control, controls, cpt, exp, fxid, landmesh, materials, mesh, ndp, sky, texm, wea};

use crate::assembly;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// A float JSON can hold, or the name of one it cannot.
pub fn number(v: f32) -> Value {
    if v.is_nan() {
        json!("NaN")
    } else if v.is_infinite() {
        json!(if v > 0.0 { "Infinity" } else { "-Infinity" })
    } else {
        json!(f64::from(v))
    }
}

pub fn vector(values: &[f32]) -> Value {
    Value::Array(values.iter().copied().map(number).collect())
}

/// An archive's version and directory, one SHA-256 per member.
pub fn nres(path: &Path) -> Result<Value> {
    let archive = Archive::open(path)?;
    let entries = archive
        .entries
        .iter()
        .map(|e| {
            let digest = Sha256::digest(archive.read(e)?);
            let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
            Ok(json!({
                "name": e.name,
                "tag": e.tag(),
                "type_id": e.type_id(),
                "offset": e.offset,
                "size": e.size,
                "index": e.index,
                "element_count": e.element_count,
                "link_count": e.link_count,
                "sha256": hex,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({ "kind": "nres", "version": archive.version, "entries": entries }))
}

fn property_value(v: PropertyValue) -> Value {
    match v {
        PropertyValue::Float(f) => number(f),
        PropertyValue::Int(i) => json!(i),
    }
}

/// A `data.tma` as the reader holds it, unknown fields included.
pub fn mission_file(path: &Path) -> Result<Value> {
    let data = std::fs::read(path)?;
    let m = mission::parse(&data, &path.display().to_string())?;
    let routes: Vec<Value> = m
        .routes
        .iter()
        .map(|r| json!({ "id": r.id, "points": r.points.iter().map(|p| vector(p)).collect::<Vec<_>>() }))
        .collect();
    let clans: Vec<Value> = m
        .clans
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "parent": c.parent,
                "base": vector(&c.base),
                "kind": c.kind,
                "ai_script": c.ai_script,
                "zones": c.zones.iter().map(|z| json!({
                    "kind": z.kind,
                    "position": vector(&z.position),
                    "inner": number(z.inner),
                    "outer": number(z.outer),
                })).collect::<Vec<_>>(),
                "behaviour": c.behaviour,
                "minds": c.minds,
                "relations": c.relations.iter().map(|(n, w)| json!([n, w])).collect::<Vec<_>>(),
            })
        })
        .collect();
    let objects: Vec<Value> = m
        .objects
        .iter()
        .map(|o| {
            json!({
                "kind": o.kind,
                "path": o.path,
                "unknown_q": o.unknown_q,
                "logical_id": o.logical_id,
                "position": vector(&o.position),
                "pad": o.pad,
                "rotation": number(o.rotation),
                "scale": vector(&o.scale),
                "name": o.name,
                "tail": [o.tail.0, o.tail.1, o.tail.2, o.tail.3],
                "properties": o.properties.iter().map(|p| json!({
                    "name": p.name,
                    "kind": p.kind,
                    "value": property_value(p.value),
                    "minimum": property_value(p.minimum),
                    "maximum": property_value(p.maximum),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({
        "kind": "mission",
        "version": m.version,
        "routes": routes,
        "clans": clans,
        "unknown_pre_objects": m.unknown_pre_objects,
        "objects": objects,
        "map_path": m.map_path,
        "description": m.description,
        "viewpoints": m.viewpoints.iter().map(|v| json!({
            "position": vector(&v.position),
            "unknown": v.unknown,
        })).collect::<Vec<_>>(),
    }))
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Textures of an archive, the `names` given or all, decoded at level 0.
pub fn texm(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let key = |n: &str| n.split('.').next().unwrap_or(n).to_ascii_uppercase();
    let wanted: Vec<String> = names.iter().map(|n| key(n)).collect();
    let mut out = Vec::new();
    for e in archive.entries.iter().filter(|e| e.tag() == "Texm") {
        if !wanted.is_empty() && !wanted.contains(&key(&e.name)) {
            continue;
        }
        let t = texm::decode(archive.read(e)?, &e.name, None)?;
        out.push(json!({
            "name": e.name,
            "width": t.width,
            "height": t.height,
            "format": t.format,
            "mips": t.mips,
            "flags": t.flags,
            "flags14": t.flags14,
            "alpha": t.has_alpha(),
            "pages": t.pages.iter().map(|p| json!(p)).collect::<Vec<_>>(),
            "rgba_sha256": hex(&t.levels[0]),
        }));
    }
    Ok(json!({ "kind": "texm", "textures": out }))
}

/// Every `MAT0` record, entries and tracks included.
pub fn material_library(path: &Path) -> Result<Value> {
    let lib = materials::Library::open(path)?;
    let out: Vec<Value> = lib
        .materials
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "blend": m.blend,
                "blend_mode": m.blend_mode(),
                "surface": m.surface,
                "speed_factor": number(m.speed_factor),
                "damage_rate": number(m.damage_rate),
                "frames": m.frames(),
                "entries": m.entries.iter().map(|e| json!({
                    "texture": e.texture,
                    "cell": e.cell,
                    "ambient": e.ambient,
                    "diffuse": e.diffuse,
                    "specular": e.specular,
                    "emissive": e.emissive,
                    "alphas": e.alphas.map(|a| f64::from(a) / 100.0),
                    "power": e.power,
                })).collect::<Vec<_>>(),
                "tracks": m.tracks.iter().map(|t| json!({
                    "kind": t.word & 7,
                    "param": t.word >> 3,
                    "keys": t.keys.iter().map(|k| json!([k.entry, k.time, k.unread])).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({ "kind": "materials", "materials": out }))
}

/// How many height samples a side the land mesh dump takes.
pub const HEIGHT_SAMPLES: usize = 17;

/// A `Land.msh`: every stream as parsed, and the elevation on a grid.
pub fn land_mesh(path: &Path) -> Result<Value> {
    let land = landmesh::load(path)?;
    let (lo, hi) = land.bounds();
    let step = |a: usize, i: usize| {
        f64::from(lo[a]) + (f64::from(hi[a]) - f64::from(lo[a])) * (i as f64 + 0.5) / HEIGHT_SAMPLES as f64
    };
    let mut grid = Vec::new();
    for j in 0..HEIGHT_SAMPLES {
        for i in 0..HEIGHT_SAMPLES {
            grid.push(land.height_at_f64(step(0, i), step(1, j)).map_or(Value::Null, |z| json!(z)));
        }
    }
    let arrays = |v: &[[f32; 3]]| v.iter().map(|p| vector(p)).collect::<Vec<_>>();
    let pairs = |v: &[[f32; 2]]| v.iter().map(|p| vector(p)).collect::<Vec<_>>();
    Ok(json!({
        "kind": "landmesh",
        "bounds": [vector(&lo), vector(&hi)],
        "lod_split": land.lod_split(),
        "water_level": land.water_level().map_or(Value::Null, number),
        "layer1": land.layer1,
        "layer2": land.layer2,
        "positions": arrays(&land.positions),
        "normals": arrays(&land.normals),
        "uv1": pairs(&land.uv1),
        "uv2": pairs(&land.uv2),
        "blend": vector(&land.blend),
        "faces": land.faces.iter().map(|f| json!([
            f.vertices, f.adjacency, f.flags, f.surface, f.tex1, f.tex2, vector(&f.normal), f.edge_twins,
        ])).collect::<Vec<_>>(),
        "cells": land.cells.iter().map(|c| json!([c.first, c.count])).collect::<Vec<_>>(),
        "height_grid": grid,
    }))
}

fn pose(p: &Pose) -> Value {
    json!([p.translation, p.rotation])
}

/// The frames a mesh dump poses every animated node at: whole, halves, past the end.
fn sample_frames(frame_count: u32) -> [f64; 7] {
    let fc = f64::from(frame_count);
    [0.0, 0.5, 1.0, 2.25, fc * 0.5 + 0.3, fc - 1.0, fc + 1.0]
}

/// One `MESH` member of an archive, `names[0]`, as the reader holds it.
pub fn object_mesh(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let member = names.first().map_or("", String::as_str);
    let stem = member.rsplit_once('.').map_or(member, |(s, _)| s);
    let wear = archive.read_name(&format!("{stem}.wea")).map(wea::parse).unwrap_or_default();
    let m = mesh::parse(archive.read_name(member)?, member)?;
    Ok(json!({
        "kind": "mesh",
        "name": member,
        "wear": { "materials": wear.materials, "lightmaps": wear.lightmaps },
        "nodes": m.nodes.iter().enumerate().map(|(i, n)| json!({
            "name": n.name,
            "flags": n.flags,
            "parent": n.parent,
            "anim_start": n.anim_start,
            "fallback_key": n.fallback_key,
            "slot_index": n.slot_index,
            "world_pose": pose(&m.world_pose(i)),
        })).collect::<Vec<_>>(),
        "slots": m.slots.iter().map(|s| json!([
            s.first_triangle, s.triangle_count, s.first_batch, s.batch_count,
            vector(&s.aabb_min), vector(&s.aabb_max), vector(&s.sphere), number(s.area), number(s.volume),
        ])).collect::<Vec<_>>(),
        "batches": m.batches.iter().map(|b| json!([
            b.material, b.flag, b.first_index, b.index_count, b.first_vertex, b.vertex_count,
        ])).collect::<Vec<_>>(),
        "triangles": m.triangles,
        "positions": m.posed_positions(),
        "normals": m.normals,
        "uv": m.uv,
        "lightmap_uv": m.lightmap_uv,
        "keys": m.keys.iter().map(|k| json!([vector(&k.translation), number(k.time), k.rotation])).collect::<Vec<_>>(),
        "face_flags": m.face_flags,
        "face_normals": m.face_normals.iter().map(|n| vector(n)).collect::<Vec<_>>(),
        "frame_map": m.frame_map,
        "frame_count": m.frame_count,
        "root_pose": pose(&m.root_pose()),
        "node_of_vertex": m.node_of_vertex(),
        "sphere": m.sphere.map_or(Value::Null, |(c, r)| json!([vector(&c), number(r)])),
        "samples": m.nodes.iter().enumerate().filter(|(_, n)| n.is_animated()).map(|(i, _)| {
            let fc = f64::from(m.frame_count);
            json!({
                "node": i,
                "at": sample_frames(m.frame_count).iter().map(|&f| pose(&m.pose_at(i, f))).collect::<Vec<_>>(),
                "blended": pose(&m.blended_pose(i, 0.5, fc * 0.6, 0.3)),
            })
        }).collect::<Vec<_>>(),
    }))
}

/// Every object of a mission as the parts it is drawn from.
pub fn mission_assembly(path: &Path) -> Result<Value> {
    let tma = if path.is_dir() { path.join("data.tma") } else { path.to_path_buf() };
    let data = std::fs::read(&tma)?;
    let m = mission::parse(&data, &tma.display().to_string())?;
    let game =
        tma.ancestors().find(|d| d.join("objects.rlb").exists()).context("no install above the mission")?;
    let mut built = assembly::Assembly::new(game)?;
    let objects: Vec<Value> = m
        .objects
        .iter()
        .map(|o| {
            let parts = built.parts(o.kind, &o.path);
            json!({
                "kind": o.kind,
                "path": o.path,
                "parts": parts.iter().map(|p| json!({
                    "library": p.reference.library,
                    "member": p.reference.member,
                    "pose": pose(&p.pose),
                    "host": p.host,
                    "node": p.node,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({ "kind": "assembly", "objects": objects }))
}

fn bounds_box(b: &([f32; 3], [f32; 3])) -> Value {
    json!([vector(&b.0), vector(&b.1)])
}

/// The `.ctl` members of an archive, all of them unless `names` picks some.
pub fn controllers(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let members: Vec<String> = if names.is_empty() {
        archive.entries.iter().filter(|e| e.tag() == control::CTL_TAG).map(|e| e.name.clone()).collect()
    } else {
        names.to_vec()
    };
    let mut out = Vec::new();
    for member in &members {
        let c = control::parse(archive.read_name(member)?, member)?;
        out.push(json!({
            "name": member,
            "counts": c.counts,
            "triples": c.triples.iter().map(|t| vector(t)).collect::<Vec<_>>(),
            "scale": c.scale,
            "pair": vector(&c.pair),
            "mode": c.mode,
            "bounds": vector(&c.bounds),
            "cone": number(c.cone),
            "flags": c.flags,
            "payload": number(c.payload),
            "bare": c.bare,
            "states": c.states.iter().map(|s| json!({
                "flags": s.flags,
                "mode": s.mode,
                "pair_a": vector(&s.pair_a),
                "pair_b": vector(&s.pair_b),
                "blend": number(s.blend),
                "length": number(s.length),
                "velocity": bounds_box(&s.velocity),
                "spin": bounds_box(&s.spin),
                "engine": number(s.engine),
                "actions": s.actions,
                "request": s.request,
            })).collect::<Vec<_>>(),
            "costs": vector(&c.costs),
            "live_costs": vector(&c.live_costs()),
            "channels": c.channels.iter().map(|ch| json!([
                ch.node, number(ch.first), number(ch.last), number(ch.initial), ch.origin,
                ch.point, number(ch.rate), number(ch.span), ch.flags,
            ])).collect::<Vec<_>>(),
            "components": c.components.iter().map(|k| json!({
                "type_id": k.type_id,
                "library": k.resource.library,
                "member": k.resource.member,
                "index": k.index,
                "entries": k.entries,
                "label": k.label,
                "values": vector(&k.values),
                "power": number(k.power),
                "node": k.node,
                "mass": number(k.mass),
                "flags": k.flags,
                "group": k.group,
            })).collect::<Vec<_>>(),
            "groups": c.groups,
            "references": c.references.iter().map(|r| json!({
                "library": r.resource.library,
                "member": r.resource.member,
                "values": r.values,
                "group": r.group,
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "kind": "control", "controllers": out }))
}

/// The `.cpt` members of an archive, all of them unless `names` picks some.
pub fn control_points(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let members: Vec<String> = if names.is_empty() {
        archive.entries.iter().filter(|e| e.tag() == cpt::CTPT_TAG).map(|e| e.name.clone()).collect()
    } else {
        names.to_vec()
    };
    let mut out = Vec::new();
    for member in &members {
        let points = cpt::parse(archive.read_name(member)?, member)?;
        out.push(json!({
            "name": member,
            "points": points.iter().map(|p| json!({
                "name": p.name,
                "a": vector(&p.a),
                "nodes": [p.nodes().0, p.nodes().1],
                "position": vector(&p.position),
                "direction": vector(&p.direction),
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "kind": "cpt", "members": out }))
}

/// The members of an archive carrying `tag`, all of them unless `names` picks some.
fn members(archive: &Archive, tag: &str, names: &[String]) -> Vec<String> {
    if names.is_empty() {
        archive.entries.iter().filter(|e| e.tag() == tag).map(|e| e.name.clone()).collect()
    } else {
        names.to_vec()
    }
}

/// The `.ndp` members of an archive.
pub fn damage_tables(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let mut out = Vec::new();
    for member in members(&archive, ndp::NDP_TAG, names) {
        let nodes = ndp::parse(archive.read_name(&member)?, &member)?;
        out.push(json!({
            "name": member,
            "nodes": nodes.iter().map(|d| json!([
                d.flags, number(d.durability), number(d.density),
                [d.explosion.library, d.explosion.member],
            ])).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "kind": "ndp", "members": out }))
}

/// The `.exp` members of an archive.
pub fn explosions(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let mut out = Vec::new();
    for member in members(&archive, exp::EXP_TAG, names) {
        let e = exp::parse(archive.read_name(&member)?, &member)?;
        out.push(json!({
            "name": member,
            "kind": e.kind,
            "damage": number(e.damage),
            "radius": number(e.radius),
            "values": vector(&e.values),
            "placement": e.placement,
            "slots": e.slots.iter().map(|s| json!([s.library, s.member])).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "kind": "exp", "members": out }))
}

/// The `FXID` members of an archive: header fields, and each emitter's live floats.
pub fn fx_effects(path: &Path, names: &[String]) -> Result<Value> {
    let archive = Archive::open(path)?;
    let mut out = Vec::new();
    for member in members(&archive, fxid::FXID_TAG, names) {
        let e = fxid::parse(archive.read_name(&member)?, &member)?;
        let h = &e.header;
        out.push(json!({
            "name": member,
            "header": {
                "count": h.count,
                "mode": h.mode,
                "duration": number(h.duration),
                "jitter": number(h.jitter),
                "flags": h.flags,
                "gate": h.gate,
                "offset": vector(&h.offset),
                "point": vector(&h.point),
                "scale": vector(&h.scale),
            },
            "emitters": e.emitters.iter().map(|m| json!({
                "kind": m.kind,
                "word": m.word,
                "resource": [m.resource.library, m.resource.member],
                "window": m.window().map_or(Value::Null, |(lo, hi)| json!([number(lo), number(hi)])),
                "live": m.live_floats().iter().map(|&(at, v)| json!([at, number(v)])).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "kind": "fxid", "members": out }))
}

/// A `sky.ske`: its sections and their headers, every keyframe as stored, and the
/// time the clock starts at.
pub fn atmosphere(path: &Path) -> Result<Value> {
    let a = sky::parse(&std::fs::read(path)?, &path.display().to_string())?;
    Ok(json!({
        "kind": "sky",
        "sections": a.sections,
        "day_seconds": a.day_seconds() as i64,
        "section_headers": a.section_headers.iter().map(|s| json!({
            "index": s.index,
            "version": s.version,
            "count": s.count,
            "end": s.end.0,
            "day": s.day.0,
        })).collect::<Vec<_>>(),
        "keyframes": a.keyframes.iter().map(|k| json!({
            "version": k.version,
            "time": k.time.0,
            "opcode": k.opcode,
            "hour": k.hour,
            "minute": k.minute,
            "section": k.section,
            "name": k.name,
            "names": k.names,
            "effects": k.effects,
            "slots": k.slots,
            "intensity": vector(&k.intensity),
        })).collect::<Vec<_>>(),
        "start": a.start.0,
        "trailer_word": a.trailer_word,
        "sky_flag": a.sky_flag,
    }))
}

/// A `.tbl`: every row, and the numbers the engine resolves its names to.
pub fn input_table(path: &Path) -> Result<Value> {
    let rows = controls::load(path)?;
    Ok(json!({
        "kind": "controls",
        "rows": rows.iter().map(|a| json!({
            "device": a.device,
            "modifier": a.modifier,
            "key": a.key,
            "pressed": a.pressed,
            "target": a.target,
            "command": a.command,
            "value": number(a.value),
            "index": a.index,
            "state": a.state,
            "ramp": number(a.ramp),
            "ramp_time": a.ramp_time,
            "note": a.note,
            "code": a.code(),
            "class_id": a.class_id(),
            "bits": a.bits(),
        })).collect::<Vec<_>>(),
    }))
}

/// Dump `path` as `kind`; `names` narrows a `texm` dump to those textures.
pub fn dump(kind: &str, path: &Path, names: &[String]) -> Result<Value> {
    match kind {
        "nres" => nres(path),
        "mission" if path.is_dir() => mission_file(&path.join("data.tma")),
        "mission" => mission_file(path),
        "texm" => texm(path, names),
        "materials" => material_library(path),
        "landmesh" => land_mesh(path),
        "mesh" => object_mesh(path, names),
        "assembly" => mission_assembly(path),
        "control" => controllers(path, names),
        "controls" => input_table(path),
        "cpt" => control_points(path, names),
        "ndp" => damage_tables(path, names),
        "exp" => explosions(path, names),
        "fxid" => fx_effects(path, names),
        "sky" => atmosphere(path),
        other => anyhow::bail!(
            "unknown kind {other:?}; expected nres, mission, texm, materials, landmesh, mesh, assembly, control, controls, cpt, ndp, exp, fxid or sky"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_json_cannot_hold_is_named() {
        assert_eq!(number(f32::NAN), json!("NaN"));
        assert_eq!(number(f32::NEG_INFINITY), json!("-Infinity"));
        assert_eq!(number(0.5), json!(0.5));
    }
}
