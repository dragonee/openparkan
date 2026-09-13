//! Readers for the game's files, one module per format.
//!
//! Each module ports its namesake in `openparkan/` and follows the layout its
//! doc in `docs/` gives. Nothing here opens a window or holds game state; the
//! readers take bytes and return plain data.

pub mod cursor;
pub mod gamedir;
pub mod mission;
pub mod nres;

pub use cursor::FormatError;
