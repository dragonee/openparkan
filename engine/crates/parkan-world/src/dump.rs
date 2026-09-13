//! Canonical JSON of what the readers make of a file.
//!
//! The twin of `openparkan/dump.py`: `openparkan golden` runs both and compares
//! them, so every key here must match a key there.

use std::path::Path;

use anyhow::Result;
use parkan_formats::mission::{self, Value as PropertyValue};
use parkan_formats::nres::Archive;
use parkan_formats::{landmesh, materials, texm};
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

/// Dump `path` as `kind`; `names` narrows a `texm` dump to those textures.
pub fn dump(kind: &str, path: &Path, names: &[String]) -> Result<Value> {
    match kind {
        "nres" => nres(path),
        "mission" if path.is_dir() => mission_file(&path.join("data.tma")),
        "mission" => mission_file(path),
        "texm" => texm(path, names),
        "materials" => material_library(path),
        "landmesh" => land_mesh(path),
        other => anyhow::bail!("unknown kind {other:?}; expected nres, mission, texm, materials or landmesh"),
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
