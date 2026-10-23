//! The GPU: instance, adapter, device and queue.

use std::env;
use std::sync::{Arc, OnceLock};

use wgpu::wgt::WgpuHasDisplayHandle;

use crate::surface::WindowSurface;

/// Environment variable that picks the adapter by name: the first adapter whose name
/// contains the value, ignoring case (e.g. `WGPU_ADAPTER_NAME=intel`). The same name
/// wgpu's own examples use. `WGPU_BACKEND` (e.g. `vulkan`, `gl`) is read by wgpu itself.
pub const ADAPTER_NAME_VAR: &str = "WGPU_ADAPTER_NAME";

/// Why the GPU couldn't be set up.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// wgpu couldn't create a surface for the window.
    #[error("can't create a GPU surface for the window: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    /// No adapter (GPU or software renderer) is usable.
    #[error("no usable GPU adapter (is a Vulkan driver installed? see README): {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    /// `WGPU_ADAPTER_NAME` is set, but no adapter's name contains it.
    #[error("{ADAPTER_NAME_VAR}={wanted:?} matches no adapter; available: {available}")]
    NoAdapterNamed {
        /// The value of `WGPU_ADAPTER_NAME`.
        wanted: String,
        /// The adapters that were found (name and backend), comma-separated.
        available: String,
    },
    /// The adapter refused to create a device.
    #[error("can't create a GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    /// The surface reports no formats for this adapter.
    #[error("the window surface supports no texture format on adapter {0:?}")]
    NoSurfaceFormat(String),
    /// The GPU device was lost (driver reset, GPU hang, unplugged GPU). Everything on the
    /// GPU is gone; Terra can't recreate it yet, so it stops.
    #[error("the GPU device was lost: {0}")]
    DeviceLost(String),
}

/// The GPU Terra renders with.
///
/// Uncaptured wgpu errors (validation errors) keep wgpu's default handler, which
/// panics: a validation error is a bug, and the crash report shows where it happened.
/// Errors caused by a lost device are the exception: wgpu drops them and reports the
/// loss once, which [`Gpu::lost`] records.
#[derive(Debug)]
pub struct Gpu {
    /// The wgpu instance. Kept to recreate a lost window surface.
    pub instance: wgpu::Instance,
    /// The chosen adapter.
    pub adapter: wgpu::Adapter,
    /// The logical device.
    pub device: wgpu::Device,
    /// The device's queue.
    pub queue: wgpu::Queue,
    /// Set by the device-lost callback: why the device was lost.
    lost: Arc<OnceLock<String>>,
}

impl Gpu {
    /// Sets up a GPU that presents to `window` and creates the window's surface.
    ///
    /// `display` is the windowing system's display handle (winit's
    /// `OwnedDisplayHandle`); wgpu's OpenGL backend needs it on Wayland. `size` is the
    /// window's inner size in physical pixels.
    pub async fn for_window(
        display: Box<dyn WgpuHasDisplayHandle>,
        window: Arc<dyn wgpu::DisplayAndWindowHandle>,
        size: (u32, u32),
        vsync: bool,
    ) -> Result<(Self, WindowSurface), GpuError> {
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle_from_env(display),
        );
        let surface = instance.create_surface(Arc::clone(&window))?;
        let gpu = Self::new(instance, Some(&surface)).await?;
        let surface = WindowSurface::new(&gpu, window, surface, size, vsync)?;
        Ok((gpu, surface))
    }

    /// Sets up a GPU without a window, for rendering offscreen (GPU tests).
    pub async fn headless() -> Result<Self, GpuError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        Self::new(instance, None).await
    }

    async fn new(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, GpuError> {
        let adapter = select_adapter(&instance, surface).await?;
        let info = adapter.get_info();
        tracing::info!(
            adapter = %info.name,
            backend = %info.backend,
            device_type = ?info.device_type,
            driver = %info.driver,
            driver_info = %info.driver_info,
            "GPU adapter selected"
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("terra device"),
                ..Default::default()
            })
            .await?;
        let lost = Arc::new(OnceLock::new());
        let lost_in_callback = Arc::clone(&lost);
        device.set_device_lost_callback(move |reason, message| {
            tracing::error!(?reason, %message, "GPU device lost");
            // wgpu calls this at most once per device, so the cell is always empty here.
            let _ = lost_in_callback.set(lost_description(reason, &message));
        });

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            lost,
        })
    }

    /// Why the device was lost, or `None` while it works. Once lost, it stays lost:
    /// nothing drawn after that reaches the screen.
    pub fn lost(&self) -> Option<&str> {
        self.lost.get().map(String::as_str)
    }

    /// [`GpuError::DeviceLost`] with the reason wgpu gave, if it has given one yet.
    pub(crate) fn device_lost_error(&self) -> GpuError {
        GpuError::DeviceLost(self.lost().unwrap_or("no reason given").to_owned())
    }
}

/// How a device loss is described in [`GpuError::DeviceLost`].
fn lost_description(reason: wgpu::DeviceLostReason, message: &str) -> String {
    let reason = match reason {
        wgpu::DeviceLostReason::Unknown => "driver error or reset",
        wgpu::DeviceLostReason::Destroyed => "destroyed",
    };
    if message.is_empty() {
        reason.to_owned()
    } else {
        format!("{reason} ({message})")
    }
}

/// Picks the adapter: by name if [`ADAPTER_NAME_VAR`] is set, otherwise the
/// highest-performance one wgpu offers (a discrete GPU before an integrated one, a
/// software renderer last).
async fn select_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, GpuError> {
    let wanted = env::var(ADAPTER_NAME_VAR).unwrap_or_default();
    if wanted.is_empty() {
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            ..Default::default()
        };
        return Ok(instance.request_adapter(&options).await?);
    }

    // `Backends::all()` lists only the backends the instance enabled (`WGPU_BACKEND`).
    let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
    let mut names = Vec::with_capacity(adapters.len());
    for adapter in adapters {
        let info = adapter.get_info();
        let usable = surface.is_none_or(|surface| adapter.is_surface_supported(surface));
        if usable && adapter_name_matches(&info.name, &wanted) {
            return Ok(adapter);
        }
        names.push(if usable {
            format!("{} ({})", info.name, info.backend)
        } else {
            format!(
                "{} ({}, can't present to this window)",
                info.name, info.backend
            )
        });
    }
    Err(GpuError::NoAdapterNamed {
        wanted,
        available: if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(", ")
        },
    })
}

/// Whether an adapter called `name` matches a [`ADAPTER_NAME_VAR`] value: `wanted` is
/// a substring of `name`, ignoring case.
pub fn adapter_name_matches(name: &str, wanted: &str) -> bool {
    name.to_lowercase().contains(&wanted.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{adapter_name_matches, lost_description};

    #[test]
    fn adapter_name_matches_substrings_ignoring_case() {
        let name = "Intel(R) Arc(tm) Graphics (MTL)";
        assert!(adapter_name_matches(name, "intel"));
        assert!(adapter_name_matches(name, "ARC(TM)"));
        assert!(!adapter_name_matches(name, "nvidia"));
        assert!(adapter_name_matches(
            "llvmpipe (LLVM 20.1.2, 256 bits)",
            "llvmpipe"
        ));
    }

    #[test]
    fn device_loss_is_described_with_the_message_if_any() {
        use wgpu::DeviceLostReason::{Destroyed, Unknown};

        assert_eq!(lost_description(Destroyed, ""), "destroyed");
        assert_eq!(
            lost_description(Unknown, "VK_ERROR_DEVICE_LOST"),
            "driver error or reset (VK_ERROR_DEVICE_LOST)"
        );
    }
}
