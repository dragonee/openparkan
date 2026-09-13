//! `parkan-dump KIND PATH`: what the engine's readers make of a file, as JSON.
//!
//! `openparkan golden` runs this beside the Python readers and compares.

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, path] = args.as_slice() else {
        anyhow::bail!("usage: parkan-dump nres|mission PATH");
    };
    let value = parkan_world::dump::dump(kind, Path::new(path))?;
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
