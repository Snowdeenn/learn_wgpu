//! GPU regression suite. Run with `cargo test --test material_gpu -- --ignored`.
use velum::{
    OwnedResource, RenderData, Scene, Velum,
    backend::{buffer, context, object::RenderPhase, pass, renderer::OffscreenFrame, report, *},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Tint([f32; 4]);
velum::define_compute_operation! {
    TextureWrite {
        context: velum::backend::pipeline::PipelineId<pipeline::ComputeContext>,
        bindings: BindingSetId,
    }
    execute(this, pass, _data) {
        let context=velum::backend::pipeline::compute::get(this.context).ok_or(pass::PassError::MissingPipeline)?;
        pipeline::dispatch(&context,&[this.bindings.into()],[4,4,1],pass)?;
        Ok(())
    }
}
impl velum::backend::instances::Instance for Tint {}
impl GpuInstance for Tint {
    fn binding() -> InstanceBinding {
        InstanceBinding::Storage {
            group: 3,
            binding: 7,
        }
    }
}
const SHADER: &str = r#"
@group(3) @binding(7) var<storage, read> colors: array<vec4f>;
@group(0) @binding(5) var<uniform> multiplier: vec4f;
struct Out { @builtin(position) position: vec4f, @location(0) color: vec4f }
@vertex fn vs_main(@builtin(vertex_index) v: u32, @builtin(instance_index) i: u32) -> Out {
    var positions = array<vec2f, 3>(vec2f(-1., -1.), vec2f(3., -1.), vec2f(-1., 3.));
    return Out(vec4f(positions[v], 0., 1.), colors[i] * multiplier);
}
@fragment fn fs_main(i: Out) -> @location(0) vec4f { return i.color; }
"#;
fn template() -> OwnedResource<MaterialTemplateId> {
    let shader = velum::backend::try_shader(None, "custom GPU data", SHADER).unwrap();
    let mut template = MaterialTemplate::fullscreen(
        None,
        "custom GPU data",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    template.schema = velum::binding_schema! {
        3 => [bindings::buffer_entry(7, wgpu::ShaderStages::VERTEX, wgpu::BufferBindingType::Storage { read_only: true }, 16, false)],
        0 => [bindings::buffer_entry(5, wgpu::ShaderStages::VERTEX, wgpu::BufferBindingType::Uniform, 16, false)],
    };
    velum::try_material_template!(template).unwrap()
}
fn output(width: u32, height: u32) -> wgpu::Texture {
    context::device(None).create_texture(&wgpu::TextureDescriptor {
        label: Some("Material pixels"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
fn pixel(texture: &wgpu::Texture) -> [u8; 4] {
    let device = context::device(None);
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * texture.height() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    context::queue(None).submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let result = read.get_mapped_range(..).unwrap()[0..4].try_into().unwrap();
    read.unmap();
    result
}
fn render(texture: &wgpu::Texture, phase: RenderPhase) -> report::FrameReport {
    let mut data = RenderData::new();
    let phase = data.insert(phase);
    let mut renderer = velum::scene_renderer::renderer();
    renderer.add_pass(pass::render(
        pass::PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        object::PhaseOperation { phase },
        true,
    ));
    let mut frame = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    frame.execute(&mut renderer.build(), &mut data).unwrap();
    frame.finish()
}

#[test]
#[ignore = "Requires a GPU adapter"]
fn described_rendering_resources_ranges_phases_resize_compute_and_reporting() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
    context::init(&adapter).unwrap();
    custom_instances_and_ownership();
    dynamic_bindings_and_transactional_updates();
    graphics_dynamic_offsets();
    geometry_regions_and_vertex_instances();
    standard_materials_particles_and_cameras();
    sprite_kinds_and_stable_blending();
    frame_leases_and_contexts();
    target_revisions();
    compute_and_abandon();
    compute_storage_texture_and_text_cleanup();
}
fn custom_instances_and_ownership() {
    let template = template().share();
    let multiplier = velum::try_binding_set! { context: None, group: 0, entries: [5 => BindingResource::uniform(None, &[1.0_f32; 4]).unwrap()] }.unwrap().share();
    let material =
        velum::try_material! { template: template.clone(), bindings: [multiplier.clone()] }
            .unwrap()
            .share();
    let geometry = geometry::try_register(Geometry::generated(None, 0..3))
        .unwrap()
        .share();
    let stream =
        InstanceStream::upload(None, &[Tint([1., 0., 0., 1.]), Tint([0., 1., 0., 1.])]).unwrap();
    let object = RenderObject::new(geometry.id(), material.id()).with_instances(stream, 1..2);
    let mut invalid = object.clone();
    invalid.instances = 0..3;
    assert!(object::prepare(&invalid, &[]).is_err());
    let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::DrawOrder);
    phase.push(object.clone());
    let texture = output(4, 4);
    let report = render(&texture, phase);
    assert_eq!(pixel(&texture), [0, 255, 0, 255]);
    assert_eq!(
        (
            report.render_passes,
            report.draw_calls,
            report.instances,
            report.batches
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(report.outcome, report::FrameOutcome::Submitted);
    assert!(report.diagnostics.is_empty());
    let common = [BindingUse::from(multiplier.id())];
    assert!(
        object::prepare(&object, &common)
            .unwrap_err_string()
            .contains("conflicting")
    );
    assert!(object::prepare(&object, &[BindingUse::overriding(multiplier.id())]).is_err()); // common precedes material; substitution is per object
    let mut overridden = object.clone();
    overridden
        .bindings
        .push(BindingUse::overriding(multiplier.id()));
    assert!(object::prepare(&overridden, &[]).is_ok());
    // Scene owners, without any external owner, survive removal in either order.
    for reverse in [false, true] {
        let owned_material = material::try_register(Material {
            template: template.clone(),
            bindings: vec![multiplier.clone()],
        })
        .unwrap()
        .share();
        let owned_geometry = geometry::try_register(Geometry::generated(None, 0..3))
            .unwrap()
            .share();
        let mid = owned_material.id();
        let gid = owned_geometry.id();
        let mut velum = Velum::new();
        let scene = || {
            let mut data = RenderData::new();
            data.insert((owned_material.clone(), owned_geometry.clone()));
            Scene {
                renderer: velum::scene_renderer::renderer().build(),
                data,
            }
        };
        let a = velum.add_scene(scene());
        let b = velum.add_scene(scene());
        drop(owned_material);
        drop(owned_geometry);
        let (first, last) = if reverse { (b, a) } else { (a, b) };
        velum.remove_scene(first).unwrap();
        assert!(material::get(mid).is_some());
        assert!(geometry::get(gid).is_some());
        velum.remove_scene(last).unwrap();
        assert!(material::get(mid).is_none());
        assert!(geometry::get(gid).is_none());
    }
    let stale = material.id();
    drop(material);
    assert!(material::get(stale).is_none());
    assert!(object::prepare(&object, &[]).is_err());
    drop(template);
    drop(multiplier);
    drop(geometry);
}

fn sprite_kinds_and_stable_blending() {
    use velum::backend::{RenderInstance, Sprite, texture};
    use velum::particle::modules::ParticleSpriteInstance;
    let slots = standard::Slots {
        instances: (3, 6),
        camera: (0, 5),
        texture: (1, 9),
        sampler: (2, 4),
        vertices: 0,
    };
    let cameras = standard::Cameras::new(None, slots.camera).unwrap();
    cameras
        .update(&velum::camera::Camera2D::new(), [4, 4], 1.)
        .unwrap();
    let geometry =
        geometry::try_mesh(None, &velum::geometry::create_rect_vertices().0).unwrap();
    let output = output(4, 4);
    for kind in 0..3 {
        // Atlas case puts every category in one group, with distinct bindings.
        let slots = if kind == 2 {
            standard::Slots {
                instances: (0, 6),
                texture: (0, 9),
                sampler: (0, 4),
                ..slots
            }
        } else {
            slots
        };
        let raw = context::device(None).create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 2,
                height: 1,
                depth_or_array_layers: if kind == 1 { 2 } else { 1 },
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes = if kind == 1 {
            vec![
                255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
            ]
        } else {
            vec![255, 0, 0, 255, 0, 0, 255, 255]
        };
        context::queue(None).write_texture(
            raw.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(1),
            },
            raw.size(),
        );
        let (owner, sprite) = match kind {
            0 => {
                let (owner, _) = texture::register_texture(None, raw);
                let sprite = Sprite::Separate {
                    texture: owner.id(),
                    indices: 0..6,
                };
                (owner, sprite)
            }
            1 => {
                let (owner, _) = texture::register_texture_array(None, texture::Texture::with_view(
                    raw,
                    &wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2Array),
                        ..Default::default()
                    },
                ))
                .unwrap();
                let sprite = Sprite::Animated {
                    texture: owner.id(),
                    layers: 0..2,
                    current_layer: 1,
                    elapsed: 0.,
                    indices: 0..6,
                };
                (owner, sprite)
            }
            _ => {
                let (owner, regions) = texture::register_atlas(
                    None,
                    raw,
                    vec![texture::AtlasRegion {
                        uv_min: [0.5, 0.],
                        uv_max: [1., 1.],
                    }],
                );
                let sprite = Sprite::Atlas {
                    texture: owner.id(),
                    region: regions[0],
                    indices: 0..6,
                };
                (owner, sprite)
            }
        };
        let template = standard::try_template(
            None,
            wgpu::TextureFormat::Rgba8Unorm,
            slots,
            Some(kind == 1),
            None,
            None,
        )
        .unwrap()
        .share();
        let material = standard::try_sprite_material(template, owner.share(), slots).unwrap();
        let instances = [ParticleSpriteInstance {
            sprite,
            render: RenderInstance {
                color: [1.; 4],
                scale: [100.; 2],
                uv_max: [1.; 2],
                ..bytemuck::Zeroable::zeroed()
            },
        }];
        let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
        phase.common = vec![cameras.world_bindings.id().into()];
        standard::submit_sprites(
            &instances,
            geometry.id(),
            slots.instance_binding(),
            &mut phase,
            |_| Ok(material.id()),
        )
        .unwrap();
        render(&output, phase);
        let color = pixel(&output);
        if kind == 1 {
            assert_eq!(color, [0, 255, 0, 255]);
        } else if kind == 2 {
            assert_eq!(color, [0, 0, 255, 255]);
        } else {
            assert_eq!(color[3], 255);
        }
    }
    // Stable ties and adjacent batching must preserve the alpha painter order.
    let mut desc = material::get(template().id()).unwrap().description.clone();
    desc.targets[0].as_mut().unwrap().blend = Some(wgpu::BlendState::ALPHA_BLENDING);
    let template = material::try_template(desc).unwrap().share();
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 0,
        entries: vec![(5, BindingResource::uniform(None, &[1.0_f32; 4]).unwrap())],
    })
    .unwrap()
    .share();
    let material = material::try_register(Material {
        template,
        bindings: vec![set],
    })
    .unwrap();
    let geometry = geometry::try_register(Geometry::generated(None, 0..3)).unwrap();
    let stream = InstanceStream::upload(
        None,
        &[
            Tint([1., 0., 0., 0.5]),
            Tint([0., 1., 0., 0.5]),
            Tint([0., 0., 1., 0.5]),
        ],
    )
    .unwrap();
    for (policy, expected, draws) in [
        (SortPolicy::PreserveOrder, [32, 64, 128, 255], 1),
        (SortPolicy::DrawOrder, [64, 32, 128, 255], 3),
        (SortPolicy::BackToFront, [32, 128, 64, 255], 3),
        (SortPolicy::FrontToBack, [64, 32, 128, 255], 3),
    ] {
        let mut phase = RenderPhase::new(PhaseId::OPAQUE, policy);
        for i in 0..3 {
            let mut object = RenderObject::new(geometry.id(), material.id())
                .with_instances(stream.clone(), i..i + 1);
            object.sort_key = DrawSortKey {
                draw_order: if i == 1 { 1 } else { 2 },
                depth: if i == 1 { 0.1 } else { 0.9 },
            };
            phase.push(object);
        }
        let report = render(&output, phase);
        let actual = pixel(&output);
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(&a, b)| (a as i32 - b as i32).abs() <= 1),
            "{policy:?}: {actual:?}"
        );
        assert_eq!(report.draw_calls, draws);
    }
}

fn frame_leases_and_contexts() {
    let mut desc = material::get(template().id()).unwrap().description.clone();
    desc.targets[0].as_mut().unwrap().blend = Some(wgpu::BlendState::ALPHA_BLENDING);
    let template = material::try_template(desc).unwrap().share();
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 0,
        entries: vec![(5, BindingResource::uniform(None, &[1.0_f32; 4]).unwrap())],
    })
    .unwrap()
    .share();
    let material = material::try_register(Material {
        template,
        bindings: vec![set],
    })
    .unwrap();
    let geometry = geometry::try_register(Geometry::generated(None, 0..3)).unwrap();
    let output = output(4, 4);
    let mut frame = OffscreenFrame::new(None, output.create_view(&Default::default())).unwrap();
    let mut first = None;
    for color in [[1., 0., 0., 0.5], [0., 1., 0., 0.5]] {
        let stream = InstanceStream::upload(None, &[Tint(color)]).unwrap();
        let region = stream.region().raw_region().1;
        if let Some(ref previous) = first {
            assert_ne!(
                previous, &region,
                "previous pass must retain its allocation"
            );
        } else {
            first = Some(region);
        }
        let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
        phase.push(RenderObject::new(geometry.id(), material.id()).with_instances(stream, 0..1));
        let (encoder, view) = frame.render_parts();
        let mut pass = pass::RenderPassBuilder::new()
            .with_color_attachment(view, |a| {
                a.with_load_op(if color[0] == 1. {
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                } else {
                    wgpu::LoadOp::Load
                })
            })
            .begin(encoder);
        phase.execute(&mut pass).unwrap();
        assert!(phase.objects().is_empty());
    }
    let report = frame.finish();
    assert_eq!(
        (report.draw_calls, report.instances, report.uploaded_bytes),
        (2, 2, 32)
    );
    let actual = pixel(&output);
    assert!(actual[0].abs_diff(64) <= 1 && actual[1].abs_diff(128) <= 1);
    let ctx = context::register(context::device(None), context::queue(None));
    let shader=velum::backend::try_shader(Some(ctx.id()),"explicit context",r#"
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {var p=array<vec2f,3>(vec2f(-1.,-1.),vec2f(3.,-1.),vec2f(-1.,3.));return vec4f(p[i],0.,1.);}
@fragment fn fs_main()->@location(0) vec4f {return vec4f(0.,0.,1.,1.);}
"#).unwrap();
    let template = material::try_template(MaterialTemplate::fullscreen(
        Some(ctx.id()),
        "explicit context",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    ))
    .unwrap()
    .share();
    let material = material::try_register(Material {
        template,
        bindings: vec![],
    })
    .unwrap();
    let geometry = geometry::try_register(Geometry::generated(Some(ctx.id()), 0..3)).unwrap();
    let mut frame =
        OffscreenFrame::new(Some(ctx.id()), output.create_view(&Default::default())).unwrap();
    {
        let (encoder, view) = frame.render_parts();
        let mut pass = pass::RenderPassBuilder::new()
            .with_color_attachment(view, |a| {
                a.with_load_op(wgpu::LoadOp::Clear(wgpu::Color::BLACK))
            })
            .begin(encoder);
        object::draw(
            &RenderObject::new(geometry.id(), material.id()),
            &[],
            &mut pass,
        )
        .unwrap();
    }
    frame.finish();
    assert_eq!(pixel(&output), [0, 0, 255, 255]);
    let context_id = Some(ctx.id());
    let buffer = buffer::storage::<u32>(context_id, 4).unwrap();
    assert_eq!(buffer.context_id(), context_id);
    let descriptor = target::RenderTargetDescriptor {
        label: "context-scoped target",
        size: target::TargetSize::Window { width: 4, height: 4 },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        sample_count: 1,
    };
    let default_target = target::register(None, descriptor);
    let explicit_target = target::register(context_id, descriptor);
    target::resize_window_targets(context_id, [8, 8]);
    assert_eq!(target::get(default_target.id()).unwrap().size(), [4, 4]);
    let resized = target::get(explicit_target.id()).unwrap();
    assert_eq!(resized.size(), [8, 8]);
    assert_eq!(resized.context_id(), context_id);
    let raw = texture::TextureBuilder::new().build_in(context_id);
    let (image, _) = texture::register_texture(context_id, raw);
    let image_id = image.id();
    assert_eq!(image_id.context_id(), context_id);
    assert!(texture::get(image_id).is_some());
    let image = image.share();
    assert!(velum::try_binding_set! {
        context: None,
        group: 0,
        entries: [0 => BindingResource::LegacyTexture(image.clone())]
    }.is_err());
    assert!(velum::try_binding_set! {
        context: context_id,
        group: 0,
        entries: [0 => BindingResource::LegacyTexture(image.clone())]
    }.is_ok());
    assert!(texture::remove(image_id));
    assert!(texture::get(image_id).is_none());
    buffer::try_write_buffer(&buffer, 0, &[1, 2, 3, 4]).unwrap();
    assert!(buffer::try_write_buffer(&buffer, 4, &[1, 2, 3, 4]).is_err());
    let region = BufferRegion::from_shared(buffer.share()).unwrap();
    assert!(
        geometry::try_register(Geometry {
            context: None,
            vertices: vec![VertexBuffer { slot: 0, region }],
            indices: None,
            elements: 0..1,
            base_vertex: 0
        })
        .is_err()
    );
    let stale = ctx.id();
    drop(ctx);
    assert!(context::remove(stale).is_none());
    let replacement = context::register(context::device(None), context::queue(None));
    assert!(context::try_context(Some(replacement.id())).is_some());
}
trait ErrorText {
    fn unwrap_err_string(self) -> String;
}
impl<T> ErrorText for RenderResult<T> {
    fn unwrap_err_string(self) -> String {
        self.err().expect("expected error").to_string()
    }
}

fn dynamic_bindings_and_transactional_updates() {
    let buffer = buffer::uniform::<[f32; 4]>(None, 64).unwrap().share();
    let region = BufferRegion::from_shared(buffer).unwrap();
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 0,
        entries: vec![(
            4,
            BindingResource::Buffer {
                region: region.clone(),
                size: 16,
            },
        )],
    })
    .unwrap();
    let schema = velum::binding_schema! { 0 => [bindings::buffer_entry(4, wgpu::ShaderStages::COMPUTE, wgpu::BufferBindingType::Uniform, 16, true)] };
    let shader = velum::backend::try_shader(None, "dynamic", "@group(0) @binding(4) var<uniform> x:vec4f; @compute @workgroup_size(1) fn cs_main() { let y=x; }").unwrap();
    let ctx = velum::try_compute_context! { context: None, shader: ShaderEntry::new(shader, "cs_main"), schema: schema }.unwrap();
    let mut encoder = context::device(None).create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        assert!(
            pipeline::dispatch(
                &ctx,
                &[BindingUse::from(set.id()).with_offsets(vec![1])],
                [1; 3],
                &mut pass
            )
            .is_err()
        );
        assert!(
            pipeline::dispatch(
                &ctx,
                &[BindingUse::from(set.id()).with_offsets(vec![1024])],
                [1; 3],
                &mut pass
            )
            .is_err()
        );
        pipeline::dispatch(
            &ctx,
            &[BindingUse::from(set.id()).with_offsets(vec![256])],
            [1; 3],
            &mut pass,
        )
        .unwrap();
    }
    context::queue(None).submit([encoder.finish()]);
    velum::backend::submitted();
    let result = bindings::try_update(
        set.id(),
        BindingSet {
            context: None,
            group: 0,
            entries: vec![
                (4, BindingResource::buffer(region.clone())),
                (4, BindingResource::buffer(region)),
            ],
        },
    );
    assert!(result.is_err());
    assert_eq!(bindings::get(set.id()).unwrap().entries.len(), 1);
}
fn graphics_dynamic_offsets() {
    let alignment = context::device(None)
        .limits()
        .min_uniform_buffer_offset_alignment as u64;
    let _prefix = buffer::uniform::<[f32; 4]>(None, 1).unwrap();
    let buffer = buffer::uniform::<[f32; 4]>(None, (alignment + 16).div_ceil(16)).unwrap();
    buffer::write_buffer(&buffer, 0, &[[1., 0., 0., 1.]]);
    buffer::write_buffer(&buffer, alignment, &[[0., 1., 0., 1.]]);
    let region = BufferRegion::from_shared(buffer.share()).unwrap();
    assert!(region.raw_region().1.start > 0);
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 2,
        entries: vec![(7, BindingResource::Buffer { region, size: 16 })],
    })
    .unwrap();
    let shader = velum::backend::try_shader(
        None,
        "graphics dynamic offsets",
        r#"
@group(2) @binding(7) var<uniform> color: vec4f;
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    var p=array<vec2f,3>(vec2f(-1.,-1.),vec2f(3.,-1.),vec2f(-1.,3.));
    return vec4f(p[i],0.,1.);
}
@fragment fn fs_main()->@location(0) vec4f {return color;}
"#,
    )
    .unwrap();
    let mut description = MaterialTemplate::fullscreen(
        None,
        "graphics dynamic offsets",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    description.schema = velum::binding_schema! { 2 => [bindings::buffer_entry(7, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Uniform, 16, true)] };
    let template = material::try_template(description).unwrap().share();
    let material = material::try_register(Material {
        template,
        bindings: vec![],
    })
    .unwrap();
    let geometry = geometry::try_register(Geometry::generated(None, 0..3)).unwrap();
    let mut object = RenderObject::new(geometry.id(), material.id());
    object
        .bindings
        .push(BindingUse::from(set.id()).with_offsets(vec![alignment as u32]));
    let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
    phase.push(object);
    let texture = output(4, 4);
    assert_eq!(render(&texture, phase).draw_calls, 1);
    assert_eq!(pixel(&texture), [0, 255, 0, 255]);
}

fn geometry_regions_and_vertex_instances() {
    let poison = buffer::vertex::<[f32; 2]>(None, 3).unwrap();
    buffer::write_buffer(&poison, 0, &[[100.; 2]; 3]);
    let vertices = buffer::vertex::<[f32; 2]>(None, 3).unwrap();
    buffer::write_buffer(&vertices, 0, &[[-1., -1.], [3., -1.], [-1., 3.]]);
    let indices = buffer::index::<u16>(None, 6).unwrap();
    buffer::write_buffer(&indices, 0, &[0, 1, 2, 0, 1, 2]);
    let region = BufferRegion::from_shared(vertices.share()).unwrap();
    assert!(region.raw_region().1.start > 0);
    assert!(region.subregion(0..region.len() + 1).is_err());
    let geometry = geometry::try_register(Geometry {
        context: None,
        vertices: vec![VertexBuffer { slot: 2, region }],
        indices: Some(IndexBuffer {
            region: BufferRegion::from_shared(indices.share()).unwrap(),
            format: wgpu::IndexFormat::Uint16,
        }),
        elements: 3..6,
        base_vertex: 0,
    })
    .unwrap();
    let shader = velum::backend::try_shader(None, "vertex instances", r#"
struct Out { @builtin(position) p:vec4f, @location(0) color:vec4f }
@vertex fn vs_main(@location(4) p:vec2f, @location(8) color:vec4f)->Out {return Out(vec4f(p,0.,1.),color);}
@fragment fn fs_main(o:Out)->@location(0) vec4f {return o.color;}
"#).unwrap();
    let layout = VertexLayout {
        stride: 16,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: wgpu::vertex_attr_array![8 => Float32x4].to_vec(),
    };
    let mut desc = MaterialTemplate::fullscreen(
        None,
        "vertex instances",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    desc.vertex_layouts = vec![
        None,
        None,
        Some(VertexLayout {
            stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![4 => Float32x2].to_vec(),
        }),
        Some(layout.clone()),
    ];
    let template = material::try_template(desc).unwrap().share();
    let material = material::try_register(Material {
        template,
        bindings: vec![],
    })
    .unwrap();
    let stream = InstanceStream::upload_as(
        None,
        &[[0.0_f32, 0., 1., 1.]],
        InstanceBinding::Vertex { slot: 3, layout },
    )
    .unwrap();
    let object = RenderObject::new(geometry.id(), material.id()).with_instances(stream, 0..1);
    let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
    phase.push(object);
    let texture = output(4, 4);
    render(&texture, phase);
    assert_eq!(pixel(&texture), [0, 0, 255, 255]);
    let mut invalid = geometry::get(geometry.id()).unwrap().as_ref().clone();
    invalid.elements = 0..7;
    assert!(geometry::try_update(geometry.id(), invalid).is_err());
    assert_eq!(geometry::get(geometry.id()).unwrap().elements, 3..6);
}

fn standard_materials_particles_and_cameras() {
    let slots = standard::Slots {
        instances: (3, 6),
        camera: (0, 5),
        texture: (1, 9),
        sampler: (2, 4),
        vertices: 0,
    };
    let shape = standard::try_template(
        None,
        wgpu::TextureFormat::Rgba8Unorm,
        slots,
        None,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        None,
    )
    .unwrap()
    .share();
    let _sprite = standard::try_template(
        None,
        wgpu::TextureFormat::Rgba8Unorm,
        slots,
        Some(false),
        Some(wgpu::BlendState::ALPHA_BLENDING),
        None,
    )
    .unwrap();
    let _array = standard::try_template(
        None,
        wgpu::TextureFormat::Rgba8Unorm,
        slots,
        Some(true),
        Some(wgpu::BlendState::ALPHA_BLENDING),
        None,
    )
    .unwrap();
    let material = material::try_register(Material {
        template: shape,
        bindings: vec![],
    })
    .unwrap();
    let cameras = standard::Cameras::new(None, slots.camera).unwrap();
    cameras
        .update(&velum::camera::Camera2D::new(), [4, 4], 2.0)
        .unwrap();
    let mesh =
        velum::backend::shape::register(None, velum::geometry::create_rect_vertices().0)
            .share();
    let geometry = velum::backend::shape::geometry(mesh.id()).unwrap();
    let particles = [velum::particle::modules::ParticleShapeInstance {
        mesh: mesh.id(),
        render: velum::backend::RenderInstance {
            color: [1., 0., 0., 1.],
            offset: [0.; 2],
            scale: [100.; 2],
            ..bytemuck::Zeroable::zeroed()
        },
    }];
    let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::BackToFront);
    phase.common = vec![cameras.world_bindings.id().into()];
    standard::submit_shapes(
        &particles,
        material.id(),
        slots.instance_binding(),
        &mut phase,
    )
    .unwrap();
    drop(mesh); // The explicitly shared geometry retains its allocations.
    let texture = output(4, 4);
    render(&texture, phase);
    assert_eq!(pixel(&texture), [255, 0, 0, 255]);
    drop(geometry);
}
fn target_revisions() {
    use velum::backend::target;
    let target = target::register(None, target::RenderTargetDescriptor {
        label: "resize",
        size: target::TargetSize::Window {
            width: 2,
            height: 2,
        },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        sample_count: 1,
    })
    .share();
    let shader = velum::backend::try_shader(None, "resize sampling", r#"
@group(3) @binding(11) var t:texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {var p=array<vec2f,3>(vec2f(-1.,-1.),vec2f(3.,-1.),vec2f(-1.,3.));return vec4f(p[i],0.,1.);}
@fragment fn fs_main()->@location(0) vec4f {return textureLoad(t,vec2i(0),0);}
"#).unwrap();
    let mut desc = MaterialTemplate::fullscreen(
        None,
        "resize sampling",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    desc.schema = velum::binding_schema! {3 => [BindingEntry {binding:11, visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture {sample_type:wgpu::TextureSampleType::Float {filterable:true}, view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None}]};
    let template = material::try_template(desc).unwrap().share();
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 3,
        entries: vec![(11, BindingResource::Target(target.clone()))],
    })
    .unwrap()
    .share();
    let material = material::try_register(Material {
        template,
        bindings: vec![set],
    })
    .unwrap();
    let geometry = geometry::try_register(Geometry::generated(None, 0..3)).unwrap();
    let texture = output(4, 4);
    for (size, color) in [(2, [255, 0, 0, 255]), (4, [0, 255, 0, 255])] {
        target::resize_window_targets(None, [size, size]);
        let raw = target::get(target.id()).unwrap();
        context::queue(None).write_texture(
            raw.texture().as_raw().as_image_copy(),
            &color.repeat((size * size) as usize),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: None,
            },
            raw.texture().as_raw().size(),
        );
        let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
        phase.push(RenderObject::new(geometry.id(), material.id()));
        render(&texture, phase);
        assert_eq!(pixel(&texture), color);
    }
    assert_eq!(target::get(target.id()).unwrap().revision(), 1);
}
fn compute_and_abandon() {
    let shader = velum::backend::try_shader(None, "compute", "@group(2) @binding(9) var<storage,read_write> x:array<u32>; @compute @workgroup_size(1) fn cs_main(){x[0]=42u;}").unwrap();
    let schema = velum::binding_schema! {2 => [bindings::buffer_entry(9,wgpu::ShaderStages::COMPUTE,wgpu::BufferBindingType::Storage {read_only:false},4,false)]};
    let ctx = velum::compute_context! {context:None,shader:ShaderEntry::new(shader,"cs_main"),schema:schema};
    let buffer = buffer::storage_rw::<u32>(None, 1).unwrap().share();
    let set = bindings::try_register(BindingSet {
        context: None,
        group: 2,
        entries: vec![(
            9,
            BindingResource::buffer(BufferRegion::from_shared(buffer).unwrap()),
        )],
    })
    .unwrap();
    let texture = output(4, 4);
    let mut frame = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    {
        let (encoder, _) = frame.render_parts();
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pipeline::dispatch(&ctx, &[set.id().into()], [1; 3], &mut pass).unwrap();
    }
    drop(frame);
    assert_eq!(report::snapshot().outcome, report::FrameOutcome::Abandoned);
    assert_eq!(report::snapshot().dispatches, 1);
    let frame = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    assert_eq!(report::snapshot().dispatches, 0);
    drop(frame);
}

fn compute_storage_texture_and_text_cleanup() {
    let texture = context::device(None).create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let shader=velum::backend::try_shader(None,"storage texture",r#"
@group(2) @binding(9) var image:texture_storage_2d<rgba8unorm,write>;
@compute @workgroup_size(1) fn cs_main(@builtin(global_invocation_id) id:vec3u) {textureStore(image,vec2i(id.xy),vec4f(1.,0.,1.,1.));}
"#).unwrap();
    let schema = velum::binding_schema! {2 => [BindingEntry {binding:9,visibility:wgpu::ShaderStages::COMPUTE,ty:wgpu::BindingType::StorageTexture {access:wgpu::StorageTextureAccess::WriteOnly,format:wgpu::TextureFormat::Rgba8Unorm,view_dimension:wgpu::TextureViewDimension::D2},count:None}]};
    let context=velum::try_compute_context! {context:None,shader:ShaderEntry::new(shader,"cs_main"),schema:schema}.unwrap();
    let context = velum::backend::pipeline::compute::register(context);
    let bindings=velum::try_binding_set! {context:None,group:2,entries:[9 => BindingResource::Texture(texture.create_view(&Default::default()))]}.unwrap();
    let mut renderer = velum::SceneRenderer::new();
    renderer.add_pass(pass::compute(
        TextureWriteOperation {
            context: context.id(),
            bindings: bindings.id(),
        },
        true,
    ));
    let mut data = RenderData::new();
    let mut frame = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    frame.execute(&mut renderer, &mut data).unwrap();
    let report = frame.finish();
    assert_eq!(
        (
            report.compute_passes,
            report.dispatches,
            report.render_passes
        ),
        (1, 1, 0)
    );
    assert_eq!(pixel(&texture), [255, 0, 255, 255]);

    use velum::backend::text;
    text::init(wgpu::TextureFormat::Rgba8Unorm).unwrap();
    let label = text::register(text::TextDescription {
        content: "Velum",
        font_size: 12.,
        line_height: 16.,
        ..Default::default()
    })
    .unwrap();
    let instance = text::Text::new(label.id(), text::TextSpace::Screen);
    let camera = velum::camera::Camera2D::new();
    let abandoned = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    text::prepare(
        &[instance],
        &text::TextView {
            resolution: [4, 4],
            scale_factor: 2.,
            camera: &camera,
        },
    )
    .unwrap();
    drop(abandoned);
    let mut next = OffscreenFrame::new(None, texture.create_view(&Default::default())).unwrap();
    text::prepare(
        &[instance],
        &text::TextView {
            resolution: [4, 4],
            scale_factor: 2.,
            camera: &camera,
        },
    )
    .unwrap();
    {
        let (encoder, view) = next.render_parts();
        let mut pass = pass::RenderPassBuilder::new()
            .with_color_attachment(view, |a| a.with_load_op(wgpu::LoadOp::Load))
            .begin(encoder);
        text::draw_all(&mut pass).unwrap();
    }
    assert_eq!(next.finish().text_draws, 1);
}

velum::define_pipeline_context!(CustomGpu);

#[test]
#[ignore = "Requires a GPU adapter"]
fn construction_macros_use_custom_context() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
    let custom = context::create(&adapter).unwrap();
    let context_id = Some(custom.id());
    let uniform = buffer::uniform::<[f32; 4]>(context_id, 1).unwrap();
    let layout = velum::bind_group_layout! {
        context_id: context_id,
        wgpu::ShaderStages::VERTEX => uniform,
    };
    let (_, group) = velum::bind_group! {
        context_id: context_id,
        wgpu::ShaderStages::VERTEX => uniform,
    };

    let source = r#"
        @vertex fn vs_main() -> @builtin(position) vec4f { return vec4f(0.0); }
        @fragment fn fs_main() -> @location(0) vec4f { return vec4f(1.0); }
        @compute @workgroup_size(1) fn cs_main() {}
    "#;
    let shader = try_shader(context_id, "construction macros", source).unwrap();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let raw = velum::pipeline_context! {
        CustomGpu {
            context_id: context_id,
            shader: &shader,
            format: format,
            fragment: "fs_main",
            bind_group: [(layout, group)],
            index_buffer: None,
        }
    };
    assert!(raw.bind_groups[0].is_some());

    let _bindings = velum::binding_set! {
        context_id: context_id,
        group: 0,
        entries: [],
    };
    let empty_schema = BindingSchema::default();
    let _compute = velum::compute_context! {
        context_id: context_id,
        shader: ShaderEntry::new(shader.clone(), "cs_main"),
        schema: empty_schema,
    };
    let template = velum::material_template! {
        label: String::from("construction macros"),
        context_id: context_id,
        vertex: ShaderEntry::new(shader.clone(), "vs_main"),
        fragment: Some(ShaderEntry::new(shader, "fs_main")),
        schema: BindingSchema::default(),
        vertex_layouts: vec![],
        primitive: Default::default(),
        depth_stencil: None,
        targets: vec![Some(format.into())],
        multisample: Default::default(),
    }.share();
    let _material = velum::material! {
        context_id: context_id,
        template: template.clone(),
        bindings: [],
    };
    let _pipeline = velum::pipeline_context! {
        context_id: context_id,
        template: template.id(),
    };
    assert!(velum::try_material! {
        context_id: None,
        template: template.clone(),
        bindings: [],
    }.is_err());
    assert!(velum::try_pipeline_context! {
        context_id: None,
        template: template.id(),
    }.is_err());
}
