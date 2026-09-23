//! The 3D board. The pure parts (geometry, picking, animation, pointer and keyboard input,
//! the 2D view) build and test on the host; `game` is the WebGL2 renderer and only builds
//! for wasm32.

pub mod anim;
pub mod geom;
pub mod input;
pub mod keys;
pub mod pick;
pub mod view;

#[cfg(target_arch = "wasm32")]
pub mod game;

/// The puzzle for `id` (a seed, plus `-easy` or `-hard` for those levels) as JSON: the same
/// puzzle the `generate` CLI prints.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn generate(id: &str) -> String {
    serde_json::to_string(&patches_core::generate(id)).expect("puzzle serialises")
}
