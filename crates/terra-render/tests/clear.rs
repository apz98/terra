//! GPU test: clear an offscreen texture to the sky colour and read it back.
//!
//! Needs a GPU adapter (a software one such as lavapipe is fine). Without one it skips
//! with a message, unless `TERRA_REQUIRE_GPU=1` (set in CI), where it fails.

use terra_render::clear::{SKY_BLUE, encode_clear};
use terra_render::{Gpu, wgpu};

/// Set to `1` to make a missing GPU adapter a test failure instead of a skip.
const REQUIRE_GPU_VAR: &str = "TERRA_REQUIRE_GPU";

/// The test texture's width and height.
const SIZE: u32 = 64;
/// 64 RGBA8 texels are 256 bytes, so each row meets `COPY_BYTES_PER_ROW_ALIGNMENT`.
const ROW_BYTES: u32 = SIZE * 4;

/// The headless GPU, or `None` (after saying why) if there is none and it isn't required.
fn gpu_or_skip() -> Option<Gpu> {
    match pollster::block_on(Gpu::headless()) {
        Ok(gpu) => Some(gpu),
        Err(error) if std::env::var(REQUIRE_GPU_VAR).is_ok_and(|v| v == "1") => {
            panic!("{REQUIRE_GPU_VAR}=1 but there is no GPU: {error}")
        }
        Err(error) => {
            eprintln!("skipping GPU test, no GPU ({error}); set {REQUIRE_GPU_VAR}=1 to fail");
            None
        }
    }
}

#[test]
fn clears_an_offscreen_texture_to_sky_blue() {
    let Some(gpu) = gpu_or_skip() else {
        return;
    };

    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("clear target"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("clear readback"),
        size: u64::from(ROW_BYTES * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encode_clear(
        &mut encoder,
        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        SKY_BLUE,
    );
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: None,
            },
        },
        extent,
    );
    gpu.queue.submit([encoder.finish()]);

    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map readback"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("wait for the GPU");
    let pixels = readback
        .get_mapped_range(..)
        .expect("mapped range")
        .to_vec();

    let [r, g, b] = SKY_BLUE;
    let (texels, rest) = pixels.as_chunks::<4>();
    assert_eq!((texels.len(), rest.len()), ((SIZE * SIZE) as usize, 0));
    for (i, pixel) in texels.iter().enumerate() {
        // sRGB → linear → sRGB may round by one step.
        let close = |got: u8, want: u8| got.abs_diff(want) <= 1;
        let [pr, pg, pb, pa] = *pixel;
        assert!(
            close(pr, r) && close(pg, g) && close(pb, b) && pa == 255,
            "pixel {i} is {pixel:?}, want {SKY_BLUE:?} with alpha 255"
        );
    }
}
