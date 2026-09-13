//! Canonical JSON of what the readers make of a file.
//!
//! The twin of `openparkan/dump.py`: `openparkan golden` runs both and compares
//! them, so every key here must match a key there.

use std::path::Path;

use anyhow::Result;
use parkan_formats::mission::{self, Value as PropertyValue};
use parkan_formats::nres::Archive;
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

/// Dump `path` as `kind`: `nres` or `mission` (a `data.tma`, or its directory).
pub fn dump(kind: &str, path: &Path) -> Result<Value> {
    match kind {
        "nres" => nres(path),
        "mission" if path.is_dir() => mission_file(&path.join("data.tma")),
        "mission" => mission_file(path),
        other => anyhow::bail!("unknown kind {other:?}; expected nres or mission"),
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
