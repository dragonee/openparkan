//! The game's rules as plain Rust, ported from `docs/` and `openparkan/`.
//!
//! Nothing here draws or opens files: the readers in `parkan-formats` supply the
//! data, and the caller supplies the time and the input. Where the game's
//! behaviour is not established the code says `STAND-IN` and names the doc
//! section, and `engine/README.md` lists it.

pub mod behaviour;
pub mod briefing;
pub mod combat;
pub mod damage;
pub mod device;
pub mod effects;
pub mod ground;
pub mod guns;
pub mod hit;
pub mod input;
pub mod machine;
pub mod motion;
pub mod orders;
pub mod progression;
pub mod script;
pub mod sky;
pub mod solid;
pub mod targeting;
pub mod turret;
pub mod wizard;
