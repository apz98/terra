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
}

impl WindowSurface {
    pub(crate) fn new(
        gpu: &Gpu,
        window: Arc<dyn wgpu::DisplayAndWindowHandle>,
        surface: wgpu::Surface<'static>,
        (width, height): (u32, u32),
        vsync: bool,
    ) -> Result<Self, GpuError> {
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = choose_surface_format(&caps.formats)
            .ok_or_else(|| GpuError::NoSurfaceFormat(gpu.adapter.get_info().name))?;
        if !format.is_srgb() {
            tracing::warn!(
                ?format,
                "no sRGB surface format; colours will look too dark"
            );
        }
        let present_mode = choose_present_mode(vsync, &caps.present_modes);
        tracing::info!(?format, ?present_mode, vsync, "window surface");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        let mut surface = Self {
            window,
            surface,
            config,
            present_modes: caps.present_modes,
            vsync,
            reconfigure: false,
        };
        surface.configure(gpu);
        Ok(surface)
    }

    /// The surface's texture format.
    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    /// The surface size in physical pixels (0×0 while the window is minimized).
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
    pub fn resize(&mut self, gpu: &Gpu, (width, height): (u32, u32)) {
        if (width, height) == self.size() {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.configure(gpu);
    }

    /// Turns vsync on or off, picking the present mode with [`choose_present_mode`].
    pub fn set_vsync(&mut self, gpu: &Gpu, vsync: bool) {
        self.vsync = vsync;
        self.config.present_mode = choose_present_mode(vsync, &self.present_modes);
        tracing::info!(vsync, present_mode = ?self.config.present_mode, "vsync changed");
        self.configure(gpu);
    }

    /// Acquires the texture to draw the next frame into, or `None` to skip this frame:
    /// the window is minimized, acquiring timed out, or the surface was outdated or lost
    /// (it is reconfigured or recreated here, ready for the next frame).
    ///
    /// Draw into the texture, submit, then present it with `gpu.queue.present`.
    pub fn acquire(&mut self, gpu: &Gpu) -> Option<wgpu::SurfaceTexture> {
        if self.is_zero_sized() {
            return None;
        }
        if self.reconfigure {
            self.configure(gpu);
        }
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => Some(frame),
            CurrentSurfaceTexture::Suboptimal(frame) => {
                // Configuring while a frame is alive panics, so do it before the next one.
                self.reconfigure = true;
                Some(frame)
            }
            CurrentSurfaceTexture::Timeout => {
                tracing::debug!("surface timed out; skipping a frame");
                None
            }
            CurrentSurfaceTexture::Occluded => None,
            CurrentSurfaceTexture::Outdated => {
                tracing::debug!("surface outdated; reconfiguring");
                self.configure(gpu);
                None
            }
            CurrentSurfaceTexture::Lost => {
                tracing::warn!("surface lost; recreating it");
                match gpu.instance.create_surface(Arc::clone(&self.window)) {
                    Ok(surface) => self.surface = surface,
                    Err(error) => tracing::error!(%error, "can't recreate the surface"),
                }
                self.configure(gpu);
                None
            }
            CurrentSurfaceTexture::Validation => {
                tracing::error!("validation error while acquiring a frame; skipping it");
                None
            }
        }
    }

    fn is_zero_sized(&self) -> bool {
        self.config.width == 0 || self.config.height == 0
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

    use super::{choose_present_mode, choose_surface_format};

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
}
