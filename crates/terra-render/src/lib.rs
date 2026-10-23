//! Terra's renderer, built on [`wgpu`].
//!
//! All GPU code lives in this crate (`docs/adr/0003-crate-layout.md`). So far it has:
//!
//! - [`Gpu`]: the instance, adapter, device and queue, for a window
//!   ([`Gpu::for_window`]) or headless ([`Gpu::headless`], used by GPU tests);
//! - [`WindowSurface`]: the window's swapchain, with resize, vsync and recovery from
//!   lost or outdated surfaces;
//! - [`clear`]: clearing a texture to a colour (the sky, until there is a world).
//!
//! `wgpu` is re-exported so dependents use exactly the version this crate does.

pub mod clear;
mod gpu;
mod surface;

pub use gpu::{ADAPTER_NAME_VAR, Gpu, GpuError, adapter_name_matches};
pub use surface::{WindowSurface, choose_present_mode, choose_surface_format};
pub use wgpu;
