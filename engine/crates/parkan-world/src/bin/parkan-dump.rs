//! `parkan-dump KIND PATH`: what the engine's readers make of a file, as JSON.
//!
//! `openparkan golden` runs this beside the Python readers and compares.

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, path, names @ ..] = args.as_slice() else {
        anyhow::bail!("usage: parkan-dump nres|mission|texm|materials|landmesh PATH [NAME...]");
    };
    let value = parkan_world::dump::dump(kind, Path::new(path), names)?;
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
