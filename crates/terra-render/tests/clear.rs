//! GPU test: clear an offscreen texture to the sky colour and read it back.
//!
//! Needs a GPU adapter; see [`common::gpu_or_skip`].

mod common;

use common::gpu_or_skip;
use terra_render::clear::{SKY_BLUE, encode_clear};
use terra_render::wgpu;

/// The test texture's width and height.
const SIZE: u32 = 64;
/// 64 RGBA8 texels are 256 bytes, so each row meets `COPY_BYTES_PER_ROW_ALIGNMENT`.
const ROW_BYTES: u32 = SIZE * 4;

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
