//! The winit application: window lifecycle, events and the frame loop.

use std::sync::Arc;

use terra_render::clear::{self, SKY_BLUE};
use terra_render::{Gpu, WindowSurface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::ClientError;

/// The window's initial inner size in logical pixels.
const INITIAL_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 720.0);

/// Toggles vsync. A temporary debug binding until the settings menu (M4).
const TOGGLE_VSYNC: KeyCode = KeyCode::F9;

/// Vsync at startup.
const DEFAULT_VSYNC: bool = true;

/// The window's app id (Wayland) and class (X11), for compositor rules and taskbars.
const APP_ID: &str = "terra";

/// The client application, driven by winit's event loop.
pub(crate) struct App {
    title: String,
    /// `None` until `resumed` creates the window.
    running: Option<Running>,
    /// The first fatal error; the event loop exits when it is set.
    error: Option<ClientError>,
}

/// Everything that exists while the window is open.
struct Running {
    // Declared (and so dropped) before the window it presents to.
    surface: WindowSurface,
    gpu: Gpu,
    window: Arc<Window>,
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
    fn redraw(&mut self) {
        let Some(frame) = self.surface.acquire(&self.gpu) else {
            return;
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
    }
}

impl ApplicationHandler for App {
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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(TOGGLE_VSYNC),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                let vsync = !running.surface.vsync();
                running.surface.set_vsync(&running.gpu, vsync);
            }
            WindowEvent::RedrawRequested => {
                running.redraw();
                running.window.request_redraw();
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Release the GPU and the window before the event loop tears down the display.
        self.running = None;
    }
}
