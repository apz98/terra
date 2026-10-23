//! Terra's game client: the window, the event loop and (later) input, camera and UI.
//!
//! [`run`] opens the window and drives the frame loop on the calling thread, which must
//! be the main thread. Rendering goes through `terra-render`; this crate owns no GPU
//! code (`docs/adr/0003-crate-layout.md`).

mod app;

use winit::event_loop::EventLoop;

/// Why the client stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The event loop couldn't start or failed while running.
    #[error("event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    /// The window couldn't be created.
    #[error("can't create the window: {0}")]
    Window(#[from] winit::error::OsError),
    /// The GPU couldn't be set up.
    #[error(transparent)]
    Gpu(#[from] terra_render::GpuError),
}

/// Opens the game window titled `Terra <version>` and runs until it is closed.
///
/// Must be called on the main thread. Uses Wayland when available, X11 otherwise
/// (winit's choice; `WAYLAND_DISPLAY`/`DISPLAY` decide).
pub fn run(version: &str) -> Result<(), ClientError> {
    let event_loop = EventLoop::new()?;
    let mut app = app::App::new(format!("Terra {version}"));
    event_loop.run_app(&mut app)?;
    app.into_result()
}
