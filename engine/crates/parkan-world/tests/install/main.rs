//! Against the install: `cargo test -- --ignored`.
//!
//! Each file here checks one thing the game does; `common` holds the fixtures they share.
//! One file's tests run on their own: `cargo test --test install weapons:: -- --ignored`.

mod audio;
mod base;
mod campaign;
mod common;
mod mission_01;
mod mission_02;
mod mission_03;
mod mission_04;
mod motion;
mod scene;
mod squad;
mod view;
mod weapons;
