//! Terra, a Minecraft-style voxel sandbox.
//!
//! This is the game binary. It will wire the client (`terra-client`) to an
//! integrated server (`terra-sim`); see `docs/ARCHITECTURE.md`. The version of
//! this crate is the game version players see.

use terra_core::diagnostics;

/// The game version, taken from this crate's `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    diagnostics::init_logging()?;
    diagnostics::install_crash_handler(VERSION, diagnostics::default_crash_dir());

    // The Tracy backend panics on the first scope unless its client is running.
    #[cfg(feature = "profile-tracy")]
    profiling::tracy_client::Client::start();
    profiling::register_thread!("main");

    tracing::info!("Terra {VERSION}");
    println!("Terra {VERSION}");
    Ok(())
}
