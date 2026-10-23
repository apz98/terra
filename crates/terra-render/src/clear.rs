//! Clearing a texture to a solid colour: the sky, until there is a world to draw.

use crate::Gpu;

/// The sky colour, in sRGB (what a colour picker shows): "sky blue", `#87CEEB`.
pub const SKY_BLUE: [u8; 3] = [135, 206, 235];

/// Converts one sRGB-encoded channel (0–255) to linear light (0.0–1.0), with the
/// exact sRGB transfer function.
pub fn srgb_to_linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// An opaque sRGB colour as a wgpu clear colour. Clear colours are linear; rendering
/// into an sRGB texture encodes them back, so the pixels end up as `srgb`.
pub fn clear_color([r, g, b]: [u8; 3]) -> wgpu::Color {
    wgpu::Color {
        r: srgb_to_linear(r),
        g: srgb_to_linear(g),
        b: srgb_to_linear(b),
        a: 1.0,
    }
}

/// Records a render pass that clears `view` to the sRGB colour `srgb`.
pub fn encode_clear(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, srgb: [u8; 3]) {
    // The pass does nothing but clear; dropping it ends it.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color(srgb)),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}

/// Clears `view` to the sRGB colour `srgb` and submits the work.
pub fn clear(gpu: &Gpu, view: &wgpu::TextureView, srgb: [u8; 3]) {
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear"),
        });
    encode_clear(&mut encoder, view, srgb);
    gpu.queue.submit([encoder.finish()]);
}

#[cfg(test)]
mod tests {
    use super::srgb_to_linear;

    #[test]
    fn srgb_to_linear_matches_the_transfer_function() {
        assert!(srgb_to_linear(0).abs() < f64::EPSILON);
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-12);
        // Mid-grey: sRGB 128 is about 21.6 % linear light, not 50 %.
        assert!((srgb_to_linear(128) - 0.215_861).abs() < 1e-6);
        // The linear segment near black.
        assert!((srgb_to_linear(10) - 10.0 / 255.0 / 12.92).abs() < 1e-12);
    }
}
