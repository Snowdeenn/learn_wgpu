//! cargo run --example offscreen -- /tmp/velum-offscreen.png
//! Runs three images: blur enabled, bypassed, then resized and enabled again.
use std::sync::Arc;
use velum::{
    EffectSource, RenderData, SceneRenderer,
    backend::{
        PhaseId, SortPolicy, context,
        object::{self, RenderPhase},
        pass::{self, RenderSequence},
        renderer::OffscreenFrame,
        standard,
        target::{self, RenderTargetDescriptor, TargetSize},
        text,
    },
    effects::{
        RenderEffect,
        blur::BlurEffect,
        composite::{CompositeEffect, CompositionOperation},
    },
    particle::{EffectDescription, EffectInstance, EmitterSettings, modules::*},
};

velum::define_render_operation! {
    Labels {} execute(_this, pass, _data) { text::draw_all(pass)?; Ok(()) }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    context::init(&adapter)?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    text::init(format)?;
    let slots = standard::Slots {
        instances: (3, 7),
        camera: (0, 5),
        texture: (1, 9),
        sampler: (1, 10),
        vertices: 0,
    };
    let template = standard::try_template(
        None,
        format,
        slots,
        None,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        None,
    )?
    .share();
    let material = velum::try_material! { template: template, bindings: [] }?;
    let cameras = standard::Cameras::new(None, slots.camera)?;
    let mesh =
        velum::backend::shape::register(velum::geometry::primitives::create_rect_vertices().0)
            .share();
    let mut particles = EffectInstance::new(
        Arc::new(EffectDescription {
            emitter: EmitterSettings::once(16),
            init_modules: vec![
                Box::new(RandomLifetime { min: 10., max: 10. }),
                Box::new(InitPos {
                    min: -55.,
                    max: 55.,
                }),
                Box::new(InitScale { min: 8., max: 20. }),
            ],
            update_modules: vec![],
            render_modules: vec![Box::new(
                RenderShape {
                    mesh: mesh.id(),
                    indices: 0..6,
                }
                .with_resource(mesh.clone()),
            )],
        }),
        [0.; 2],
    );
    particles.update(0.016);
    let source = target::register(RenderTargetDescriptor {
        label: "World",
        size: TargetSize::Window {
            width: 320,
            height: 180,
        },
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        sample_count: 1,
    });
    let mut renderer = SceneRenderer::new();
    let mut data = RenderData::new();
    let world = data.insert(RenderPhase::new(PhaseId::OPAQUE, SortPolicy::BackToFront));
    let hud = data.insert(RenderPhase::new(PhaseId::OVERLAY, SortPolicy::DrawOrder));
    renderer.add_pass(pass::render(
        source.id(),
        wgpu::LoadOp::Clear(wgpu::Color {
            r: 0.015,
            g: 0.025,
            b: 0.06,
            a: 1.,
        }),
        object::PhaseOperation { phase: world },
        true,
    ));
    let blur = BlurEffect::new(format, [320, 180]);
    let presenter = CompositeEffect::new(blur.output(), format);
    let blur = renderer.add_effect(blur);
    renderer.connect(blur, BlurEffect::SOURCE, EffectSource::Target(source.id()))?;
    renderer.append_effect(blur)?;
    renderer.add_pass(pass::render(
        pass::PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        RenderSequence::new(vec![
            Box::new(CompositionOperation {
                pipeline: presenter.pipeline.id(),
                source: presenter.source.id(),
            }),
            Box::new(object::PhaseOperation { phase: hud }),
            Box::new(LabelsOperation {}),
        ]),
        true,
    ));
    let mut presenter = presenter;
    let label = text::register(text::TextDescription {
        content: "Velum + Prism",
        font_size: 18.,
        line_height: 24.,
        ..Default::default()
    })?;
    let mut title = text::Text::new(label.id(), text::TextSpace::Screen);
    title.position = [12., 8.];
    let camera = velum::camera::Camera2D::new();
    let mut final_texture = None;
    for (index, size) in [[320, 180], [320, 180], [480, 270]].into_iter().enumerate() {
        let output = context::device(None).create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen output"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut frame = OffscreenFrame::new(None, output.create_view(&Default::default()))?;
        target::resize_window_targets(size);
        renderer.set_effect_active(blur, index != 1)?;
        presenter.set_source(renderer.resolve_source(EffectSource::Output(blur))?)?;
        let dpi = if index == 2 { 1.5 } else { 1. };
        cameras.update(&camera, size, dpi)?;
        let phase = data.require_mut(world)?;
        phase.common = vec![cameras.world_bindings.id().into()];
        let prepared = particles.prepare();
        standard::submit_shapes(
            prepared.shapes(),
            material.id(),
            slots.instance_binding(),
            phase,
        )?;
        // Visiting the sprite category does not prepare or simulate the effect again.
        assert!(prepared.sprites().is_empty());
        let phase = data.require_mut(hud)?;
        phase.common = vec![cameras.screen_bindings.id().into()];
        let panel = [ParticleShapeInstance {
            mesh: mesh.id(),
            render: velum::backend::RenderInstance {
                color: [0.15, 0.65, 1., 0.85],
                offset: [160., 88.],
                scale: [100., 5.],
                ..bytemuck::Zeroable::zeroed()
            },
        }];
        standard::submit_shapes(&panel, material.id(), slots.instance_binding(), phase)?;
        text::prepare(
            &[title],
            &text::TextView {
                resolution: size,
                scale_factor: dpi,
                camera: &camera,
            },
        )?;
        frame.execute(&mut renderer, &mut data)?;
        let report = frame.finish();
        println!(
            "image {index}: {} passes, {} dessins, {} instances, {} octets transférés, {} appel texte",
            report.render_passes,
            report.draw_calls,
            report.instances,
            report.uploaded_bytes,
            report.text_draws
        );
        assert!(report.diagnostics.is_empty());
        final_texture = Some(output);
    }
    let output = final_texture.unwrap();
    let row = (output.width() * 4).div_ceil(256) * 256;
    let device = context::device(None);
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: row as u64 * output.height() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        output.size(),
    );
    context::queue(None).submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    rx.recv()??;
    let mapped = read.get_mapped_range(..)?;
    let pixels: Vec<u8> = mapped
        .chunks(row as usize)
        .flat_map(|row| row[..output.width() as usize * 4].iter().copied())
        .collect();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/velum-offscreen.png".into());
    image::save_buffer(
        &path,
        &pixels,
        output.width(),
        output.height(),
        image::ColorType::Rgba8,
    )?;
    println!("{path}");
    Ok(())
}
