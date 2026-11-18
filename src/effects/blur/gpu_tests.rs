use super::*;
use crate::{RenderData, backend::context};

// Called by the single GPU test entry point after initializing the global device.
pub(crate) fn gpu_regression() {
    let device = context::device(None);
    let source = || {
        target::register(RenderTargetDescriptor {
            label: "Blur source regression",
            size: TargetSize::Window {
                width: 5,
                height: 5,
            },
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            sample_count: 1,
        })
    };
    let red = source();
    let blue = source();
    let mut blur = BlurEffect::new(wgpu::TextureFormat::Rgba8Unorm, [5, 5]);
    assert!(matches!(blur.prepare(), Err(EffectError::MissingInput(_))));
    assert!(matches!(
        blur.set_input("unknown", red.id()),
        Err(EffectError::UnknownInput(_))
    ));
    assert!(matches!(
        blur.set_input(BlurEffect::SOURCE, blur.output()),
        Err(EffectError::IncompatibleInput { .. })
    ));
    assert_eq!(blur.bypass_input(), Some(BlurEffect::SOURCE));
    let upload = |id, color: [u8; 4]| {
        let target = target::get(id).unwrap();
        let [w, h] = target.size();
        let bytes: Vec<u8> = (0..w * h).flat_map(|_| color).collect();
        context::queue(None).write_texture(
            target.texture().as_raw().as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            target.texture().as_raw().size(),
        );
    };
    let render = |blur: &mut BlurEffect| {
        blur.prepare().unwrap();
        let output = target::get(blur.output()).unwrap();
        let [w, h] = output.size();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: h as u64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        pass::execute(
            blur.passes_mut(),
            &mut encoder,
            output.texture().view(),
            &mut RenderData::new(),
        )
        .unwrap();
        encoder.copy_texture_to_buffer(
            output.texture().as_raw().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(h),
                },
            },
            output.texture().as_raw().size(),
        );
        context::queue(None).submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = staging.get_mapped_range(..).unwrap();
        let mut pixels = Vec::new();
        for row in 0..h as usize {
            pixels.extend_from_slice(&mapped[row * 256..row * 256 + w as usize * 4]);
        }
        drop(mapped);
        staging.unmap();
        pixels
    };
    upload(red.id(), [255, 0, 0, 255]);
    upload(blue.id(), [0, 0, 255, 255]);
    blur.set_input(BlurEffect::SOURCE, red.id()).unwrap();
    blur.settings.radius = 0;
    assert_eq!(&render(&mut blur)[..4], &[255, 0, 0, 255]);
    let stable_binding = blur.source_a.id();
    // Both sources have the same revision: changing the ID must still reconnect.
    blur.set_input(BlurEffect::SOURCE, blue.id()).unwrap();
    assert_eq!(&render(&mut blur)[..4], &[0, 0, 255, 255]);
    assert_eq!(stable_binding, blur.source_a.id());

    // A single red pixel must spread in both axes through the owned H/V passes.
    let mut impulse = vec![0_u8; 5 * 5 * 4];
    impulse[(2 * 5 + 2) * 4] = 255;
    let texture = target::get(blue.id()).unwrap();
    context::queue(None).write_texture(
        texture.texture().as_raw().as_image_copy(),
        &impulse,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(20),
            rows_per_image: Some(5),
        },
        texture.texture().as_raw().size(),
    );
    blur.settings.radius = 1;
    blur.settings.manual = true;
    blur.settings.weights.fill(1.0);
    let pixels = render(&mut blur);
    for (x, y) in [(2, 2), (1, 2), (2, 1), (1, 1)] {
        let value = pixels[(y * 5 + x) * 4];
        assert!((value as i32 - 28).abs() <= 2, "({x},{y}): {value}");
    }
    // The input must not be overwritten by the effect (radius-zero readback).
    blur.settings.radius = 0;
    assert_eq!(render(&mut blur), impulse);

    target::resize_window_targets([8, 8]);
    upload(blue.id(), [0, 255, 0, 255]);
    assert_eq!(&render(&mut blur)[..4], &[0, 255, 0, 255]);
    assert_eq!(target::get(blur.output()).unwrap().size(), [8, 8]);

    // The same two effects are now owned and encoded through one renderer.
    use crate::api::scene_renderer::{EffectSource, SceneRenderer};
    let mut renderer = SceneRenderer::new();
    let mut first = BlurEffect::new(wgpu::TextureFormat::Rgba8Unorm, [8, 8]);
    first.settings.radius = 0;
    let first = renderer.add_effect(first);
    let mut second = BlurEffect::new(wgpu::TextureFormat::Rgba8Unorm, [8, 8]);
    second.settings.radius = 0;
    let second = renderer.add_effect(second);
    renderer
        .connect(first, BlurEffect::SOURCE, EffectSource::Target(blue.id()))
        .unwrap();
    renderer
        .connect(second, BlurEffect::SOURCE, EffectSource::Output(first))
        .unwrap();
    renderer.append_effect(first).unwrap();
    renderer.append_effect(second).unwrap();
    let render_chain = |renderer: &mut SceneRenderer, expected_draws| {
        let output_id = renderer
            .resolve_source(EffectSource::Output(second))
            .unwrap();
        let output = target::get(output_id).unwrap();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 8 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        crate::backend::metrics::reset();
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer
            .execute_into(
                &mut encoder,
                output.texture().view(),
                &mut RenderData::new(),
            )
            .unwrap();
        encoder.copy_texture_to_buffer(
            output.texture().as_raw().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(8),
                },
            },
            output.texture().as_raw().size(),
        );
        context::queue(None).submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        assert_eq!(
            &staging.get_mapped_range(..).unwrap()[..4],
            &[0, 255, 0, 255]
        );
        staging.unmap();
        assert_eq!(
            crate::backend::metrics::snapshot().draw_calls,
            expected_draws
        );
    };
    render_chain(&mut renderer, 4);
    renderer.set_effect_active(first, false).unwrap();
    render_chain(&mut renderer, 2);
    renderer.set_effect_active(second, false).unwrap();
    assert_eq!(
        renderer
            .resolve_source(EffectSource::Output(second))
            .unwrap(),
        blue.id()
    );
    // No effect work remains when both blocks are disabled.
    crate::backend::metrics::reset();
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer
        .execute_into(
            &mut encoder,
            target::get(blur.output()).unwrap().texture().view(),
            &mut RenderData::new(),
        )
        .unwrap();
    assert_eq!(crate::backend::metrics::snapshot().draw_calls, 0);
    drop(encoder);
    let id = blue.id();
    drop(blue);
    assert!(matches!(blur.prepare(), Err(EffectError::MissingTarget(missing)) if missing == id));
}
