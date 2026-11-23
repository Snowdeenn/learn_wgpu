use super::*;
use crate::backend::{PipelineContext, pipeline};
use new_collection::Arena;

struct TestPipeline;
impl PipelineContext for TestPipeline {
    fn pipeline(&self) -> &wgpu::RenderPipeline {
        unreachable!()
    }
    fn bind_groups(&self) -> &[Option<wgpu::BindGroup>] {
        &[]
    }
    fn index_buffer(&self) -> Option<&(wgpu::Buffer, std::ops::Range<u64>)> {
        None
    }
    fn vertex_buffer(&self) -> &[Option<(wgpu::Buffer, std::ops::Range<u64>)>] {
        &[]
    }
}

fn instance(marker: f32) -> RenderInstance {
    RenderInstance {
        offset: [marker, 0.0],
        ..bytemuck::Zeroable::zeroed()
    }
}

#[test]
fn policies_are_stable_and_do_not_modify_sources() {
    let pipeline = pipeline::register(TestPipeline);
    let mut ids = Arena::<(), MeshResource>::new();
    let mesh = ids.insert(());
    let input = [(0.2, 2), (0.8, 1), (0.2, 1), (0.5, 0)];
    for (policy, expected) in [
        (SortPolicy::PreserveOrder, vec![0., 1., 2., 3.]),
        (SortPolicy::DrawOrder, vec![3., 1., 2., 0.]),
        (SortPolicy::FrontToBack, vec![0., 2., 3., 1.]),
        (SortPolicy::BackToFront, vec![1., 3., 0., 2.]),
    ] {
        let mut phase = RenderPhase::new(policy);
        for (index, &(depth, draw_order)) in input.iter().enumerate() {
            phase.push(DrawCommand::shape(
                pipeline.id(),
                mesh,
                instance(index as f32),
                DrawSortKey { depth, draw_order },
            ));
        }
        phase.prepare().unwrap();
        assert_eq!(
            phase
                .commands()
                .iter()
                .map(|d| d.instance.offset[0])
                .collect::<Vec<_>>(),
            expected
        );
    }
    assert_eq!(input, [(0.2, 2), (0.8, 1), (0.2, 1), (0.5, 0)]);
    let mut phase = RenderPhase::new(SortPolicy::FrontToBack);
    phase.push(DrawCommand::shape(
        pipeline.id(),
        mesh,
        instance(0.),
        DrawSortKey {
            depth: f32::NAN,
            draw_order: 0,
        },
    ));
    assert!(phase.prepare().is_err());
}

#[test]
fn existing_batchers_only_merge_consecutive_resources() {
    let mut meshes = Arena::<(), MeshResource>::new();
    let a = meshes.insert(());
    let b = meshes.insert(());
    let mut shapes = vec![
        PreparedShape {
            mesh: a,
            instance: instance(0.),
        },
        PreparedShape {
            mesh: a,
            instance: instance(1.),
        },
        PreparedShape {
            mesh: b,
            instance: instance(2.),
        },
        PreparedShape {
            mesh: a,
            instance: instance(3.),
        },
    ];
    let batches = shape::batch(&mut shapes, 0..4);
    assert_eq!(
        batches.iter().map(|b| b.range.clone()).collect::<Vec<_>>(),
        vec![0..2, 2..3, 3..4]
    );
    assert_eq!(
        shapes
            .iter()
            .map(|s| s.instance.offset[0])
            .collect::<Vec<_>>(),
        vec![0., 1., 2., 3.]
    );

    let mut textures = Arena::<(), TextureResource>::new();
    let a = textures.insert(());
    let b = textures.insert(());
    let mut sprites: Vec<_> = [a, a, b, a]
        .into_iter()
        .map(|texture| PreparedSprite {
            sprite: Sprite::Separate {
                texture: crate::backend::texture::TextureId { context: None, raw: texture },
                indices: 0..6,
            },
            instance: instance(0.),
        })
        .collect();
    let batches = sprite::batch(&mut sprites, 0..4);
    assert_eq!(
        batches.iter().map(|b| b.range.clone()).collect::<Vec<_>>(),
        vec![0..2, 2..3, 3..4]
    );
}

#[test]
fn pipeline_changes_split_runs_even_with_the_same_geometry() {
    let a = pipeline::register(TestPipeline);
    let b = pipeline::register(TestPipeline);
    let mut meshes = Arena::<(), MeshResource>::new();
    let mesh = meshes.insert(());
    let first = DrawCommand::shape(a.id(), mesh, instance(0.), DrawSortKey::default());
    let same = DrawCommand::shape(a.id(), mesh, instance(1.), DrawSortKey::default());
    let other = DrawCommand::shape(b.id(), mesh, instance(2.), DrawSortKey::default());
    assert!(first.same_state(&same));
    assert!(!first.same_state(&other));
}

/// Invoked from the single GPU regression entry point after context initialization.
pub(crate) fn gpu_regression() {
    use crate::backend::{buffer, metrics, pipeline, target, texture};
    use crate::geometry::{Mesh, Vertex};
    crate::define_pipeline_context!(PhaseGpu);
    let device = context::device(None);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Phase regression"),
        source: wgpu::ShaderSource::Wgsl(r#"
            struct Instance {
                color: vec4f, offset: vec2f, scale: vec2f,
                layer: u32, rotation: f32, depth: f32, padding: u32,
                uv_min: vec2f, uv_max: vec2f,
            }
            struct Out { @builtin(position) pos: vec4f, @location(0) color: vec4f }
            @group(0) @binding(0) var<storage, read> instances: array<Instance>;
            @group(1) @binding(0) var<uniform> tint: vec4f;
            @group(2) @binding(0) var image: texture_2d<f32>;
            @group(2) @binding(1) var image_sampler: sampler;
            @vertex fn vs_main(@builtin(vertex_index) v: u32, @builtin(instance_index) i: u32) -> Out {
                let positions = array<vec2f, 3>(vec2f(-1., -1.), vec2f(3., -1.), vec2f(-1., 3.));
                var o: Out;
                o.pos = vec4f(positions[v], instances[i].depth, 1.);
                o.color = instances[i].color * tint;
                return o;
            }
            @fragment fn fs_flat(o: Out) -> @location(0) vec4f { return o.color; }
            @fragment fn fs_texture(o: Out) -> @location(0) vec4f {
                return textureSample(image, image_sampler, vec2f(0.5)) * o.color;
            }
        "#.into()),
    });
    let instances = buffer::storage::<RenderInstance>(None, 1).unwrap();
    let (instance_layout, _) = crate::bind_group! { wgpu::ShaderStages::VERTEX => instances };
    let tint = buffer::uniform::<[f32; 4]>(None, 1).unwrap();
    buffer::write_buffer(&tint, 0, &[[1.; 4]]);
    let (tint_layout, tint_group) = crate::bind_group! { wgpu::ShaderStages::VERTEX => tint };
    let dark = buffer::uniform::<[f32; 4]>(None, 1).unwrap();
    buffer::write_buffer(&dark, 0, &[[0., 1., 0., 1.]]);
    let (_, dark_group) = crate::bind_group! { wgpu::ShaderStages::VERTEX => dark };
    let make_texture = |color: [u8; 4]| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        context::queue(None).write_texture(
            texture.as_image_copy(),
            &color,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            texture.size(),
        );
        texture::register_texture(None, texture)
    };
    let (white, texture_layout) = make_texture([255; 4]);
    let (blue, _) = make_texture([0, 0, 255, 255]);
    let indices = buffer::index::<u32>(None, 3).unwrap();
    buffer::write_buffer(&indices, 0, &[0, 1, 2]);
    let flat = pipeline::register(crate::pipeline_context! {
        PhaseGpu {
            shader: &shader, format: wgpu::TextureFormat::Rgba8Unorm, fragment: "fs_flat",
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            bind_group: [(instance_layout,), (tint_layout, tint_group.clone())], index_buffer: None,
        }
    });
    let textured = pipeline::register(crate::pipeline_context! {
        PhaseGpu {
            shader: &shader, format: wgpu::TextureFormat::Rgba8Unorm, fragment: "fs_texture",
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            bind_group: [(instance_layout,), (tint_layout, tint_group.clone()), (texture_layout,)],
            index_buffer: Some(&indices),
        }
    });
    let mut mesh = Mesh::new();
    mesh.extend_vertices_from_slice(&[Vertex::default(); 3]);
    mesh.extend_indices_from_slice(&[0, 1, 2]);
    let mesh = shape::register(None, mesh);
    let target = target::register(None, target::RenderTargetDescriptor {
        label: "Phase pixels",
        size: target::TargetSize::Fixed {
            width: 4,
            height: 4,
        },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        sample_count: 1,
    });
    let output = target::get(target.id()).unwrap();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4 * 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let render = |phase: &mut RenderPhase| {
        metrics::reset();
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.texture().view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            phase.execute(&mut pass).unwrap();
        }
        assert!(phase.commands().is_empty());
        encoder.copy_texture_to_buffer(
            output.texture().as_raw().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        context::queue(None).submit([encoder.finish()]);
        crate::backend::instances::submitted();
        let (tx, rx) = std::sync::mpsc::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let pixel = staging.get_mapped_range(..).unwrap()[0..4].to_vec();
        staging.unmap();
        (pixel, metrics::snapshot().draw_calls)
    };
    let mut phase = RenderPhase::new(SortPolicy::BackToFront);
    for (color, depth) in [([1., 0., 0., 0.5], 0.2), ([0., 0., 1., 0.5], 0.8)] {
        phase.push(DrawCommand::shape(
            flat.id(),
            mesh.id(),
            RenderInstance {
                color,
                depth,
                ..bytemuck::Zeroable::zeroed()
            },
            DrawSortKey {
                depth,
                draw_order: 0,
            },
        ));
    }
    let (pixel, draws) = render(&mut phase);
    assert!(
        (pixel[0] as i32 - 128).abs() <= 1 && (pixel[2] as i32 - 64).abs() <= 1,
        "{pixel:?}"
    );
    assert_eq!(draws, 1);
    // Consumed commands must not replay on the next image.
    assert_eq!(render(&mut phase).0, vec![0, 0, 0, 255]);
    for (texture, color) in [
        (white.id(), [1., 0., 0., 1.]),
        (blue.id(), [1.; 4]),
        (white.id(), [0., 1., 0., 1.]),
    ] {
        phase.push(DrawCommand::sprite(
            textured.id(),
            Sprite::Separate {
                texture,
                indices: 0..3,
            },
            RenderInstance {
                color,
                depth: 0.5,
                ..bytemuck::Zeroable::zeroed()
            },
            DrawSortKey::default(),
        ));
    }
    let (pixel, draws) = render(&mut phase);
    assert_eq!(pixel, vec![0, 255, 0, 255]);
    assert_eq!(draws, 3); // A, B, A cannot be reordered into A, A, B.
    for group in [tint_group, dark_group] {
        phase.push(
            DrawCommand::shape(
                flat.id(),
                mesh.id(),
                RenderInstance {
                    color: [1.; 4],
                    depth: 0.5,
                    ..bytemuck::Zeroable::zeroed()
                },
                DrawSortKey::default(),
            )
            .with_bind_group(1, group),
        );
    }
    assert_eq!(render(&mut phase), (vec![0, 255, 0, 255], 2));
    let missing = mesh.id();
    drop(mesh);
    phase.push(DrawCommand::shape(
        flat.id(),
        missing,
        RenderInstance {
            color: [1.; 4],
            ..bytemuck::Zeroable::zeroed()
        },
        DrawSortKey::default(),
    ));
    metrics::reset();
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output.texture().view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        assert_eq!(phase.execute(&mut pass).unwrap_err().reason, "Missing mesh");
    }
    assert!(phase.commands().is_empty());
    assert_eq!(metrics::snapshot().draw_calls, 0);
}
