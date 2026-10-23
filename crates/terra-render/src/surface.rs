//! The window's surface (swapchain): format, present mode, resize and recovery.

use std::sync::Arc;

use wgpu::{CurrentSurfaceTexture, PresentMode, TextureFormat};

use crate::gpu::{Gpu, GpuError};

/// Picks the present mode for a vsync setting from the modes the surface supports.
///
/// Vsync on is `Fifo`, which every surface supports. Vsync off prefers `Mailbox`
/// (no tearing, newest frame wins), then `Immediate` (may tear), and falls back to
/// `Fifo` if neither is supported.
pub fn choose_present_mode(vsync: bool, supported: &[PresentMode]) -> PresentMode {
    if !vsync {
        for mode in [PresentMode::Mailbox, PresentMode::Immediate] {
            if supported.contains(&mode) {
                return mode;
            }
        }
    }
    PresentMode::Fifo
}

/// Picks the surface format: the first sRGB format the surface supports, so shaders
/// write linear colour and the hardware encodes it; otherwise the first format at all.
/// `None` if the list is empty.
pub fn choose_surface_format(supported: &[TextureFormat]) -> Option<TextureFormat> {
    supported
        .iter()
        .copied()
        .find(TextureFormat::is_srgb)
        .or_else(|| supported.first().copied())
}

/// Clamps a surface size to the largest texture the device supports in each dimension
/// (`configure` rejects anything larger). The compositor scales the smaller image up.
fn clamp_size((width, height): (u32, u32), max_dimension: u32) -> (u32, u32) {
    (width.min(max_dimension), height.min(max_dimension))
}

/// What to do after acquiring a frame failed `failures_in_a_row` times in a row (this
/// failure included). The first failure has already been fixed by reconfiguring or
/// recreating the surface, so retry. If it fails again, retrying at once would spin.
fn after_failure(failures_in_a_row: u32) -> Acquired {
    if failures_in_a_row <= 1 {
        Acquired::Retry
    } else {
        Acquired::Wait
    }
}

/// The result of [`WindowSurface::acquire`].
#[derive(Debug)]
pub enum Acquired {
    /// Draw into this texture, submit, then present it with `gpu.queue.present`.
    Frame(wgpu::SurfaceTexture),
    /// No frame this time, but the surface is ready again: redraw right away.
    Retry,
    /// No frame, and none is likely soon: the window is minimized or hidden, or acquiring
    /// keeps failing. Wait a little (or for the window to change) before redrawing.
    /// Redrawing at once would spin.
    Wait,
}

/// A window's surface, configured for the window's size and the vsync setting.
pub struct WindowSurface {
    /// Kept to recreate the surface when it is lost.
    window: Arc<dyn wgpu::DisplayAndWindowHandle>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    present_modes: Vec<PresentMode>,
    vsync: bool,
    /// The last frame was suboptimal: reconfigure before acquiring the next one.
    reconfigure: bool,
    /// The surface was already reconfigured for a suboptimal frame at this size. If it
    /// stays suboptimal, reconfiguring again won't help.
    suboptimal_retried: bool,
    /// Acquires that failed since the last frame.
    failures: u32,
}

impl WindowSurface {
    pub(crate) fn new(
        gpu: &Gpu,
        window: Arc<dyn wgpu::DisplayAndWindowHandle>,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        vsync: bool,
    ) -> Result<Self, GpuError> {
        let mut surface = Self {
            window,
            surface,
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                // The format, present mode and size are set below.
                format: TextureFormat::Rgba8UnormSrgb,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: 0,
                height: 0,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: Vec::new(),
            },
            present_modes: Vec::new(),
            vsync,
            reconfigure: false,
            suboptimal_retried: false,
            failures: 0,
        };
        surface.apply_capabilities(gpu)?;
        surface.set_size(gpu, size);
        surface.configure(gpu);
        Ok(surface)
    }

    /// The surface's texture format. It can change when a lost surface is recreated.
    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    /// The surface size in physical pixels: the window's size, clamped to the device's
    /// largest texture (0×0 while the window is minimized).
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Whether vsync is on.
    pub fn vsync(&self) -> bool {
        self.vsync
    }

    /// The present mode in use.
    pub fn present_mode(&self) -> PresentMode {
        self.config.present_mode
    }

    /// Resizes the surface to the window's new inner size in physical pixels. A 0×0
    /// size (minimized) is remembered but not configured; frames are skipped until the
    /// window has a size again.
    pub fn resize(&mut self, gpu: &Gpu, size: (u32, u32)) {
        if self.set_size(gpu, size) {
            self.configure(gpu);
        }
    }

    /// Turns vsync on or off, picking the present mode with [`choose_present_mode`].
    pub fn set_vsync(&mut self, gpu: &Gpu, vsync: bool) {
        self.vsync = vsync;
        self.config.present_mode = choose_present_mode(vsync, &self.present_modes);
        tracing::info!(vsync, present_mode = ?self.config.present_mode, "vsync changed");
        self.configure(gpu);
    }

    /// Acquires the texture to draw the next frame into, or says why there is none and
    /// when to try again (see [`Acquired`]). An outdated surface is reconfigured and a
    /// lost one recreated here, ready for the next try.
    ///
    /// `window_size` returns the window's current inner size in physical pixels. It is
    /// only called when the surface needs reconfiguring, since on X11 it asks the server.
    ///
    /// # Errors
    ///
    /// [`GpuError::DeviceLost`] once the device is lost, and [`GpuError::CreateSurface`]
    /// or [`GpuError::NoSurfaceFormat`] if a lost surface can't be recreated. The window
    /// can't show anything after either.
    pub fn acquire(
        &mut self,
        gpu: &Gpu,
        window_size: impl Fn() -> (u32, u32),
    ) -> Result<Acquired, GpuError> {
        if self.reconfigure {
            self.set_size(gpu, window_size());
            self.configure(gpu);
        }
        if self.is_zero_sized() {
            return Ok(Acquired::Wait);
        }
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => {
                self.failures = 0;
                self.suboptimal_retried = false;
                Ok(Acquired::Frame(frame))
            }
            CurrentSurfaceTexture::Suboptimal(frame) => {
                self.failures = 0;
                // Configuring while a frame is alive panics, so do it before the next one.
                // Only once per size: some drivers stay suboptimal, and reconfiguring every
                // frame would rebuild the swapchain every frame.
                if !self.suboptimal_retried {
                    tracing::debug!("surface suboptimal; reconfiguring before the next frame");
                    self.reconfigure = true;
                    self.suboptimal_retried = true;
                }
                Ok(Acquired::Frame(frame))
            }
            CurrentSurfaceTexture::Timeout => {
                tracing::debug!("surface timed out; skipping a frame");
                Ok(self.failed())
            }
            CurrentSurfaceTexture::Occluded => Ok(Acquired::Wait),
            CurrentSurfaceTexture::Outdated => {
                tracing::debug!("surface outdated; reconfiguring");
                // The cached size is what's outdated (e.g. an X11 resize whose event hasn't
                // arrived yet), so ask the window.
                self.set_size(gpu, window_size());
                self.configure(gpu);
                Ok(self.failed())
            }
            CurrentSurfaceTexture::Lost => {
                tracing::warn!("surface lost; recreating it");
                self.recreate(gpu)?;
                Ok(self.failed())
            }
            // wgpu's default error handler panics on validation errors (see `Gpu`), so
            // wgpu only returns this when it dropped the error because the device is lost.
            CurrentSurfaceTexture::Validation => Err(gpu.device_lost_error()),
        }
    }

    /// Counts a failed acquire and says when to try again.
    fn failed(&mut self) -> Acquired {
        self.failures = self.failures.saturating_add(1);
        after_failure(self.failures)
    }

    fn is_zero_sized(&self) -> bool {
        self.config.width == 0 || self.config.height == 0
    }

    /// Sets the size to configure next, clamped to the device's largest texture. Returns
    /// whether it changed.
    fn set_size(&mut self, gpu: &Gpu, size: (u32, u32)) -> bool {
        let max_dimension = gpu.device.limits().max_texture_dimension_2d;
        let clamped = clamp_size(size, max_dimension);
        if clamped == self.size() {
            return false;
        }
        if clamped != size {
            tracing::warn!(
                ?size,
                max_dimension,
                "window is larger than the GPU's largest surface; drawing it smaller"
            );
        }
        (self.config.width, self.config.height) = clamped;
        self.suboptimal_retried = false;
        true
    }

    /// Picks the format and present mode from the surface's capabilities on the adapter.
    fn apply_capabilities(&mut self, gpu: &Gpu) -> Result<(), GpuError> {
        let caps = self.surface.get_capabilities(&gpu.adapter);
        let format = choose_surface_format(&caps.formats)
            .ok_or_else(|| GpuError::NoSurfaceFormat(gpu.adapter.get_info().name))?;
        if !format.is_srgb() {
            tracing::warn!(
                ?format,
                "no sRGB surface format; colours will look too dark"
            );
        }
        self.config.format = format;
        self.config.present_mode = choose_present_mode(self.vsync, &caps.present_modes);
        self.present_modes = caps.present_modes;
        tracing::info!(
            ?format,
            present_mode = ?self.config.present_mode,
            vsync = self.vsync,
            "window surface"
        );
        Ok(())
    }

    /// Replaces a lost surface with a new one for the same window. The new surface may
    /// support other formats and present modes, so they are picked again.
    fn recreate(&mut self, gpu: &Gpu) -> Result<(), GpuError> {
        self.surface = gpu.instance.create_surface(Arc::clone(&self.window))?;
        self.apply_capabilities(gpu)?;
        self.configure(gpu);
        Ok(())
    }

    fn configure(&mut self, gpu: &Gpu) {
        self.reconfigure = false;
        // wgpu panics on a 0×0 configuration; `acquire` skips frames until it isn't.
        if !self.is_zero_sized() {
            self.surface.configure(&gpu.device, &self.config);
        }
    }
}

#[cfg(test)]
mod tests {
    use wgpu::PresentMode::{AutoNoVsync, AutoVsync, Fifo, FifoRelaxed, Immediate, Mailbox};
    use wgpu::TextureFormat::{
        Bgra8Unorm, Bgra8UnormSrgb, Rgba8Unorm, Rgba8UnormSrgb, Rgba16Float,
    };

    use super::{Acquired, after_failure, choose_present_mode, choose_surface_format, clamp_size};

    #[test]
    fn vsync_on_is_always_fifo() {
        assert_eq!(choose_present_mode(true, &[Mailbox, Immediate, Fifo]), Fifo);
        assert_eq!(choose_present_mode(true, &[Fifo]), Fifo);
        // Fifo is guaranteed, so it's chosen even if a buggy list leaves it out.
        assert_eq!(choose_present_mode(true, &[]), Fifo);
    }

    #[test]
    fn vsync_off_prefers_mailbox_then_immediate_then_fifo() {
        assert_eq!(
            choose_present_mode(false, &[Fifo, Immediate, Mailbox]),
            Mailbox
        );
        assert_eq!(choose_present_mode(false, &[Fifo, Immediate]), Immediate);
        assert_eq!(choose_present_mode(false, &[Fifo, FifoRelaxed]), Fifo);
        // The `Auto*` modes are never reported as supported; they are not picked.
        assert_eq!(
            choose_present_mode(false, &[AutoNoVsync, AutoVsync, Fifo]),
            Fifo
        );
    }

    #[test]
    fn surface_format_prefers_srgb() {
        assert_eq!(
            choose_surface_format(&[Bgra8Unorm, Rgba16Float, Bgra8UnormSrgb, Rgba8UnormSrgb]),
            Some(Bgra8UnormSrgb)
        );
        assert_eq!(
            choose_surface_format(&[Rgba8Unorm, Bgra8Unorm]),
            Some(Rgba8Unorm)
        );
        assert_eq!(choose_surface_format(&[]), None);
    }

    #[test]
    fn surface_size_is_clamped_to_the_largest_texture() {
        assert_eq!(clamp_size((1280, 720), 8192), (1280, 720));
        assert_eq!(clamp_size((8192, 8192), 8192), (8192, 8192));
        assert_eq!(clamp_size((10_240, 1440), 8192), (8192, 1440));
        assert_eq!(clamp_size((0, 0), 8192), (0, 0));
    }

    #[test]
    fn a_failed_acquire_retries_once_then_waits() {
        assert!(matches!(after_failure(1), Acquired::Retry));
        assert!(matches!(after_failure(2), Acquired::Wait));
        assert!(matches!(after_failure(u32::MAX), Acquired::Wait));
    }
}
