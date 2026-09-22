//! The 3D board. The pure parts (geometry, animation, input) build and test on the host;
//! `game` is the WebGL2 renderer and only builds for wasm32.

pub mod anim;
pub mod geom;
pub mod input;

#[cfg(target_arch = "wasm32")]
pub mod game;
