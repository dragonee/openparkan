//! Readers for the game's files, one module per format.
//!
//! Each module ports its namesake in `openparkan/` and follows the layout its
//! doc in `docs/` gives. Nothing here opens a window or holds game state; the
//! readers take bytes and return plain data.

pub mod arealmap;
pub mod basement;
pub mod cfg;
pub mod control;
pub mod controls;
pub mod cpt;
pub mod cursor;
pub mod exp;
pub mod font;
pub mod fxid;
pub mod gamedir;
pub mod hallway;
pub mod landmesh;
pub mod materials;
pub mod mesh;
pub mod mission;
pub mod ndp;
pub mod nres;
pub mod objects;
pub mod pose;
pub mod profiles;
pub mod research;
pub mod resources;
pub mod rsli;
pub mod scr;
pub mod sky;
pub mod texm;
pub mod userdir;
pub mod wea;

pub use cursor::FormatError;
