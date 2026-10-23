//! GPU test: a lost device is recorded in `Gpu`, so the client can stop with an error
//! instead of drawing into a dead device.
//!
//! Needs a GPU adapter; see [`common::gpu_or_skip`].

mod common;

use common::gpu_or_skip;
use terra_render::wgpu;

#[test]
fn a_destroyed_device_is_reported_lost() {
    let Some(gpu) = gpu_or_skip() else {
        return;
    };
    assert_eq!(gpu.lost(), None);

    // `destroy` is the one way to lose a device on purpose. wgpu reports it from `poll`
    // once the queue is idle, which a destroyed device may report as an error.
    gpu.device.destroy();
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());

    assert_eq!(gpu.lost(), Some("destroyed"));
}
