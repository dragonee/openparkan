//! Readers for the game's files, one module per format.
//!
//! Each module ports its namesake in `openparkan/` and follows the layout its
//! doc in `docs/` gives. Nothing here opens a window or holds game state; the
//! readers take bytes and return plain data.

pub mod control;
pub mod controls;
pub mod cursor;
pub mod gamedir;
pub mod landmesh;
pub mod materials;
pub mod mesh;
pub mod mission;
pub mod nres;
pub mod objects;
pub mod pose;
pub mod texm;
pub mod wea;

pub use cursor::FormatError;
