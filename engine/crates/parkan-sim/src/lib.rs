//! The game's rules as plain Rust, ported from `docs/` and `openparkan/`.
//!
//! Nothing here draws or opens files: the readers in `parkan-formats` supply the
//! data, and the caller supplies the time and the input. Where the game's
//! behaviour is not established the code says `STAND-IN` and names the doc
//! section, and `engine/README.md` lists it.

pub mod ground;
pub mod input;
pub mod machine;
pub mod motion;
pub mod turret;
