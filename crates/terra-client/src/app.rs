//! The winit application: window lifecycle, events and the frame loop.

use std::sync::Arc;
use std::time::{Duration, Instant};

use terra_render::clear::{self, SKY_BLUE};
use terra_render::{Acquired, Gpu, GpuError, WindowSurface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::ClientError;
use crate::input::InputState;
use crate::timing::{FrameTimer, STATS_WINDOW};

/// The window's initial inner size in logical pixels.
const INITIAL_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);

/// Toggles vsync. A temporary debug binding until the settings menu (M4).
const TOGGLE_VSYNC: KeyCode = KeyCode::F9;

/// Releases the grabbed cursor. (Clicking into the window grabs it again.)
const RELEASE_CURSOR: KeyCode = KeyCode::Escape;

/// Vsync at startup.
const DEFAULT_VSYNC: bool = true;

/// The window's app id (Wayland) and class (X11), for compositor rules and taskbars.
const APP_ID: &str = "terra";

/// How long to wait before trying to draw again when there's nothing to draw into
/// (minimized or hidden window). Short enough that a window shown again redraws without a
/// visible delay (Wayland sends no event for it), long enough to cost no measurable CPU.
const HIDDEN_RETRY: Duration = Duration::from_millis(100);

/// The client application, driven by winit's event loop.
pub(crate) struct App {
    title: String,
    /// `None` until `resumed` creates the window.
    running: Option<Running>,
    /// The first fatal error; the event loop exits when it is set.
    error: Option<ClientError>,
}

/// When to draw the next frame.
enum NextRedraw {
    /// Right away.
    Now,
    /// After [`HIDDEN_RETRY`], or sooner if the window changes.
    Later,
}

/// Everything that exists while the window is open.
struct Running {
    // Declared (and so dropped) before the window it presents to.
    surface: WindowSurface,
    gpu: Gpu,
    window: Arc<Window>,
    timer: FrameTimer,
    input: InputState,
    /// Whether the cursor is grabbed (locked or confined, and hidden). Mouse movement
    /// only counts while it is, so moving the mouse to click into the window doesn't
    /// turn the camera.
    cursor_grabbed: bool,
    /// When the frame statistics are logged next.
    next_stats_log: Instant,
}

impl App {
    pub(crate) fn new(title: String) -> Self {
        Self {
            title,
            running: None,
            error: None,
        }
    }

    /// The outcome once the event loop has returned.
    pub(crate) fn into_result(self) -> Result<(), ClientError> {
        self.error.map_or(Ok(()), Err)
    }

    fn start(&self, event_loop: &ActiveEventLoop) -> Result<Running, ClientError> {
        let attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(INITIAL_SIZE)
            .with_resizable(true);
        let window = Arc::new(event_loop.create_window(with_app_id(attributes))?);
        let size = window.inner_size();
        let (gpu, surface) = pollster::block_on(Gpu::for_window(
            Box::new(event_loop.owned_display_handle()),
            Arc::clone(&window) as Arc<dyn terra_render::wgpu::DisplayAndWindowHandle>,
            (size.width, size.height),
            DEFAULT_VSYNC,
        ))?;
        window.request_redraw();
        Ok(Running {
            surface,
            gpu,
            window,
            timer: FrameTimer::new(),
            input: InputState::new(),
            cursor_grabbed: false,
            next_stats_log: Instant::now() + STATS_WINDOW,
        })
    }
}

/// Sets the window's Wayland app id and X11 class to [`APP_ID`]. winit leaves both
/// empty otherwise.
#[cfg(target_os = "linux")]
fn with_app_id(attributes: WindowAttributes) -> WindowAttributes {
    // The Wayland and X11 extension traits set the same field, so one call covers both.
    use winit::platform::wayland::WindowAttributesExtWayland;

    attributes.with_name(APP_ID, APP_ID)
}

#[cfg(not(target_os = "linux"))]
fn with_app_id(attributes: WindowAttributes) -> WindowAttributes {
    attributes
}

impl Running {
    /// Runs one frame: update, then render. Input that arrived since the previous frame
    /// is visible to the update and cleared afterwards.
    fn frame(&mut self) -> Result<NextRedraw, GpuError> {
        let now = Instant::now();
        let dt = self.timer.tick(now);
        {
            profiling::scope!("update");
            self.update(now, dt);
        }
        let next = {
            profiling::scope!("render");
            self.render()
        };
        self.input.end_frame();
        if let Ok(NextRedraw::Later) = next {
            // The wait until the next frame isn't a frame time.
            self.timer.pause();
        }
        next
    }

    /// Reacts to this frame's input. `_dt` is the time to simulate; nothing moves yet.
    fn update(&mut self, now: Instant, _dt: Duration) {
        if self.input.pressed(TOGGLE_VSYNC) {
            let vsync = !self.surface.vsync();
            self.surface.set_vsync(&self.gpu, vsync);
        }
        if self.input.pressed(RELEASE_CURSOR) {
            self.release_cursor();
        }
        // Until the debug overlay shows them.
        if now >= self.next_stats_log {
            self.next_stats_log = now + STATS_WINDOW;
            if let Some(stats) = self.timer.stats() {
                tracing::debug!(
                    "{:.1} fps, frame time avg {:.2} ms, max {:.2} ms",
                    stats.fps,
                    stats.avg.as_secs_f64() * 1e3,
                    stats.max.as_secs_f64() * 1e3,
                );
            }
        }
    }

    /// Grabs and hides the cursor, so the mouse turns the camera instead of leaving the
    /// window. Locks it in place where the platform can (Wayland), else confines it to
    /// the window (X11 has no lock).
    fn grab_cursor(&mut self) {
        let window = &self.window;
        let grabbed = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .map(|()| CursorGrabMode::Locked)
            .or_else(|_| {
                window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .map(|()| CursorGrabMode::Confined)
            });
        match grabbed {
            Ok(mode) => {
                window.set_cursor_visible(false);
                self.cursor_grabbed = true;
                tracing::debug!("cursor grabbed ({mode:?})");
            }
            Err(error) => tracing::warn!("can't grab the cursor: {error}"),
        }
    }

    /// Shows the cursor and lets it move freely again.
    fn release_cursor(&mut self) {
        if !self.cursor_grabbed {
            return;
        }
        if let Err(error) = self.window.set_cursor_grab(CursorGrabMode::None) {
            tracing::warn!("can't release the cursor: {error}");
        }
        self.window.set_cursor_visible(true);
        self.cursor_grabbed = false;
        tracing::debug!("cursor released");
    }

    /// Draws and presents a frame, if the surface has one to draw into.
    fn render(&mut self) -> Result<NextRedraw, GpuError> {
        let window = &self.window;
        let window_size = || {
            let size = window.inner_size();
            (size.width, size.height)
        };
        let frame = match self.surface.acquire(&self.gpu, window_size)? {
            Acquired::Frame(frame) => frame,
            Acquired::Retry => return Ok(NextRedraw::Now),
            Acquired::Wait => return Ok(NextRedraw::Later),
        };
        let view = frame
            .texture
            .create_view(&terra_render::wgpu::TextureViewDescriptor::default());
        clear::clear(&self.gpu, &view, SKY_BLUE);
        // On Wayland this ties the next `RedrawRequested` to the compositor's frame
        // callback. With vsync off that would cap the frame rate, so skip it then.
        if self.surface.vsync() {
            self.window.pre_present_notify();
        }
        self.gpu.queue.present(frame);
        Ok(NextRedraw::Now)
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // The `HIDDEN_RETRY` wait is over: try drawing again.
        if let StartCause::ResumeTimeReached { .. } = cause {
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(running) = &self.running {
                running.window.request_redraw();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Desktop platforms resume once. (Android would resume again after a suspend.)
        if self.running.is_some() {
            return;
        }
        match self.start(event_loop) {
            Ok(running) => self.running = Some(running),
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                running
                    .surface
                    .resize(&running.gpu, (size.width, size.height));
                running.window.request_redraw();
            }
            WindowEvent::Occluded(false) => running.window.request_redraw(),
            WindowEvent::Focused(false) => {
                running.input.focus_lost();
                running.release_cursor();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => running.input.key(key, state),
            WindowEvent::MouseInput { state, button, .. } => {
                // The click that grabs the cursor only grabs it; the game doesn't see it.
                if state == ElementState::Pressed && !running.cursor_grabbed {
                    running.grab_cursor();
                    if running.cursor_grabbed {
                        return;
                    }
                }
                running.input.mouse_button(button, state);
            }
            WindowEvent::RedrawRequested => {
                let next = running.frame();
                profiling::finish_frame!();
                match next {
                    Ok(NextRedraw::Now) => running.window.request_redraw(),
                    // A timer rather than an event, since no event is sure to come: winit
                    // reports no `Occluded` on Wayland.
                    Ok(NextRedraw::Later) => event_loop
                        .set_control_flow(ControlFlow::WaitUntil(Instant::now() + HIDDEN_RETRY)),
                    Err(error) => {
                        self.error = Some(error.into());
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        if let DeviceEvent::MouseMotion { delta } = event
            && running.cursor_grabbed
        {
            running.input.mouse_motion(delta);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Release the GPU and the window before the event loop tears down the display.
        self.running = None;
    }
}
