//! Terra, a Minecraft-style voxel sandbox.
//!
//! This is the game binary. It will wire the client (`terra-client`) to an
//! integrated server (`terra-sim`); see `docs/ARCHITECTURE.md`. The version of
//! this crate is the game version players see.

/// The game version, taken from this crate's `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("Terra {VERSION}");
}
