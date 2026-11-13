use super::*;
use crate::backend::{
    self,
    binding_set::{self, TextureBinding},
    context,
    frame_data::FrameData,
    instances::{Instance, InstanceId},
    pipeline::{self, compute},
    target::{self, TargetSize},
};

pub struct Step(u32);
impl Instance for Step {}

crate::define_pipeline_context!(Readback);
crate::define_compute_pipeline_context!(CopyTarget);

crate::define_render_operation! {
    Record { trace: InstanceId<Step>, value: u32 }
    execute(this, _pass, frame) {
        frame.instances.get_mut(&this.trace).ok_or(PassError::MissingInstances)?.push(Step(this.value));
        Ok(())
    }
}

crate::define_compute_operation! {
    CopyTarget {
        pipeline: compute::ComputePipelineId<CopyTargetComputePipelineContext>,
        bindings: binding_set::TextureBindingSetId,
        trace: InstanceId<Step>,
    }
    execute(this, pass, frame) {
        let context = compute::get(this.pipeline).ok_or(PassError::MissingPipeline)?;
        compute::bind(&*context, pass);
        let group = binding_set::group(this.bindings)?;
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(8, 8, 1);
        frame.instances.get_mut(&this.trace).ok_or(PassError::MissingInstances)?.push(Step(3));
        Ok(())
    }
}

crate::define_render_operation! {
    Readback {
        pipeline: pipeline::PipelineId<ReadbackPipelineContext>,
        bindings: binding_set::TextureBindingSetId,
        trace: InstanceId<Step>,
    }
    execute(this, pass, frame) {
        let context = pipeline::get(this.pipeline).ok_or(PassError::MissingPipeline)?;
        backend::bind(&*context, pass);
        let group = binding_set::group(this.bindings)?;
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
        frame.instances.get_mut(&this.trace).ok_or(PassError::MissingInstances)?.push(Step(4));
        Ok(())
    }
}

fn descriptor(size: TargetSize) -> target::RenderTargetDescriptor {
    target::RenderTargetDescriptor {
        label: "Framebuffer test",
        size,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        sample_count: 1,
    }
}

fn read_pixels(buffer: &wgpu::Buffer) -> Vec<u8> {
    let (send, receive) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        send.send(result).unwrap()
    });
    context::device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    let bytes = buffer.get_mapped_range(..).unwrap().to_vec();
    buffer.unmap();
    bytes
}

#[test]
#[ignore = "Requires a GPU adapter; run explicitly with --ignored"]
fn gpu_framebuffer_passes_resize_and_resource_lifecycle() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter required for this test");
    context::init(&adapter);
    let device = context::device();

    let source = target::register(descriptor(TargetSize::Window {
        width: 4,
        height: 4,
    }));
    let output = target::register(descriptor(TargetSize::Window {
        width: 4,
        height: 4,
    }));
    let fixed = target::register(descriptor(TargetSize::Fixed {
        width: 4,
        height: 4,
    }));
    let surface = target::create_target(&descriptor(TargetSize::Fixed {
        width: 8,
        height: 8,
    }));
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    let copy_layout = backend::bind_group::BindGroupLayoutBuilder::new()
        .with_texture(
            wgpu::ShaderStages::COMPUTE,
            false,
            wgpu::TextureViewDimension::D2,
            wgpu::TextureSampleType::Float { filterable: true },
        )
        .with_sampler(wgpu::ShaderStages::COMPUTE, true)
        .with_storage_texture(
            wgpu::ShaderStages::COMPUTE,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureViewDimension::D2,
            wgpu::StorageTextureAccess::WriteOnly,
        )
        .build();
    let copy_bindings = binding_set::register(
        &copy_layout,
        vec![
            TextureBinding::Texture {
                binding: 0,
                target: source,
            },
            TextureBinding::Sampler {
                binding: 1,
                sampler: sampler.clone(),
            },
            TextureBinding::Texture {
                binding: 2,
                target: output,
            },
        ],
    )
    .unwrap();
    let copy_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            r#"
            @group(0) @binding(0) var input: texture_2d<f32>;
            @group(0) @binding(1) var input_sampler: sampler;
            @group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;
            @compute @workgroup_size(1)
            fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
                let size = textureDimensions(output);
                if any(id.xy >= size) { return; }
                let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
                textureStore(output, id.xy, textureSampleLevel(input, input_sampler, uv, 0.0));
            }
        "#
            .into(),
        ),
    });
    let copy_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&copy_layout)],
        immediate_size: 0,
    });
    let copy_pipeline = compute::register(CopyTargetComputePipelineContext {
        pipeline: compute::ComputePipelineBuilder::from_layout(&copy_pipeline_layout, &copy_shader)
            .build(),
        bind_groups: vec![None],
    });

    let read_sampler = backend::sampler::filtering("Readback sampler");
    let read_layout = {
        let output_target = target::get(output).unwrap();
        crate::bind_group_layout! {
            wgpu::ShaderStages::FRAGMENT => output_target.texture(),
            wgpu::ShaderStages::FRAGMENT => read_sampler,
        }
    };
    let read_bindings = binding_set::register(
        &read_layout,
        vec![
            TextureBinding::Texture {
                binding: 0,
                target: output,
            },
            TextureBinding::Sampler {
                binding: 1,
                sampler: read_sampler.as_raw().clone(),
            },
        ],
    )
    .unwrap();
    let read_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            r#"
            @group(0) @binding(0) var input: texture_2d<f32>;
            @group(0) @binding(1) var input_sampler: sampler;
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let xy = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                return vec4(xy[id], 0.0, 1.0);
            }
            @fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
                return textureSampleLevel(input, input_sampler, position.xy / 8.0, 0.0);
            }
        "#
            .into(),
        ),
    });
    let read_context = crate::pipeline_context!(Readback {
        shader: &read_shader,
        format: wgpu::TextureFormat::Rgba8Unorm,
        fragment: "fs_main",
        blend: None,
        bind_group: [(read_layout,)],
        index_buffer: None,
    });
    let read_pipeline = pipeline::register(read_context);
    let mut frame = FrameData::new();
    let trace = frame.instances.insert(Vec::<Step>::new().into());
    let mut passes = vec![
        render(
            source,
            wgpu::LoadOp::Clear(wgpu::Color::RED),
            RenderSequence::new(vec![
                Box::new(RecordOperation { trace, value: 1 }),
                Box::new(RecordOperation { trace, value: 2 }),
            ]),
        ),
        compute(CopyTargetOperation {
            pipeline: copy_pipeline,
            bindings: copy_bindings,
            trace,
        }),
        render(
            PassTarget::Surface,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            ReadbackOperation {
                pipeline: read_pipeline,
                bindings: read_bindings,
                trace,
            },
        ),
        render(
            PassTarget::Surface,
            wgpu::LoadOp::Load,
            RecordOperation { trace, value: 5 },
        ),
    ];
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 8 * 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    for expected in [[255, 0, 0, 255], [0, 255, 0, 255]] {
        frame.instances.get_mut(&trace).unwrap().clear();
        let mut encoder = device.create_command_encoder(&Default::default());
        execute(
            &mut passes,
            &mut encoder,
            surface.texture().view(),
            &mut frame,
        )
        .unwrap();
        encoder.copy_texture_to_buffer(
            surface.texture().as_raw().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
        );
        context::queue().submit([encoder.finish()]);
        let pixels = read_pixels(&staging);
        for row in 0..8 {
            for column in 0..8 {
                assert_eq!(
                    &pixels[row * 256 + column * 4..row * 256 + column * 4 + 4],
                    &expected
                );
            }
        }
        assert_eq!(
            frame
                .instances
                .get(&trace)
                .unwrap()
                .iter()
                .map(|step| step.0)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );

        target::resize_window_targets([8, 8]);
        target::resize_window_targets([8, 8]);
        target::resize_window_targets([0, 0]);
        assert_eq!(target::get(source).unwrap().revision(), 1);
        assert_eq!(target::get(output).unwrap().revision(), 1);
        assert_eq!(target::get(source).unwrap().size(), [8, 8]);
        assert_eq!(target::get(fixed).unwrap().revision(), 0);
        assert_eq!(target::get(fixed).unwrap().size(), [4, 4]);
        if let PassContext::Render { load, .. } = &mut passes[0] {
            *load = wgpu::LoadOp::Clear(wgpu::Color::GREEN);
        }
    }

    // A failed operation stops the sequence and the following passes.
    let missing = frame.instances.insert(Vec::<Step>::new().into());
    frame.instances.remove(&missing).unwrap();
    frame.instances.get_mut(&trace).unwrap().clear();
    let mut failing = vec![
        render(
            PassTarget::Surface,
            wgpu::LoadOp::Load,
            RenderSequence::new(vec![
                Box::new(RecordOperation { trace, value: 1 }),
                Box::new(RecordOperation {
                    trace: missing,
                    value: 2,
                }),
                Box::new(RecordOperation { trace, value: 3 }),
            ]),
        ),
        render(
            PassTarget::Surface,
            wgpu::LoadOp::Load,
            RecordOperation { trace, value: 4 },
        ),
    ];
    let mut encoder = device.create_command_encoder(&Default::default());
    assert!(matches!(
        execute(
            &mut failing,
            &mut encoder,
            surface.texture().view(),
            &mut frame
        ),
        Err(PassError::MissingInstances)
    ));
    assert_eq!(
        frame
            .instances
            .get(&trace)
            .unwrap()
            .iter()
            .map(|step| step.0)
            .collect::<Vec<_>>(),
        [1]
    );
    drop(encoder);

    let mut encoder = device.create_command_encoder(&Default::default());
    execute(&mut [], &mut encoder, surface.texture().view(), &mut frame).unwrap();
    context::queue().submit([encoder.finish()]);

    assert!(target::remove(source));
    assert!(!target::remove(source));
    let replacement = target::register(descriptor(TargetSize::Fixed {
        width: 8,
        height: 8,
    }));
    assert_ne!(source, replacement);
    assert!(matches!(
        binding_set::group(copy_bindings),
        Err(binding_set::BindingSetError::MissingTarget(_))
    ));
    let mut encoder = device.create_command_encoder(&Default::default());
    assert!(matches!(
        execute(
            &mut passes,
            &mut encoder,
            surface.texture().view(),
            &mut frame
        ),
        Err(PassError::MissingTarget(_))
    ));
    drop(encoder);

    assert!(binding_set::remove(copy_bindings));
    assert!(!binding_set::remove(copy_bindings));
    assert!(matches!(
        binding_set::group(copy_bindings),
        Err(binding_set::BindingSetError::MissingSet(_))
    ));
    assert!(binding_set::remove(read_bindings));
    assert!(pipeline::remove(read_pipeline));
    assert!(!pipeline::remove(read_pipeline));
    assert!(pipeline::get(read_pipeline).is_none());
    assert!(compute::remove(copy_pipeline));
    assert!(target::remove(output));
    assert!(target::remove(fixed));
    assert!(target::remove(replacement));
}
