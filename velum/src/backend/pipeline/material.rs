use crate::backend::{MaterialTemplate, RenderResult, bindings::ResolvedLayouts};

#[derive(Clone, Debug)]
pub struct AttachmentSignature {
    pub colors: Vec<Option<wgpu::TextureFormat>>,
    pub depth: Option<wgpu::TextureFormat>,
    pub samples: u32,
}
thread_local! { static ATTACHMENTS: std::cell::RefCell<Option<AttachmentSignature>> = const { std::cell::RefCell::new(None) }; }
/// External render passes can declare their attachments with this guard as well.
pub struct AttachmentScope {
    previous: Option<AttachmentSignature>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl AttachmentScope {
    pub fn new(signature: AttachmentSignature) -> Self {
        Self {
            previous: ATTACHMENTS.replace(Some(signature)),
            _thread: std::marker::PhantomData,
        }
    }
}
impl Drop for AttachmentScope {
    fn drop(&mut self) {
        ATTACHMENTS.replace(self.previous.take());
    }
}
pub(crate) fn validate_current_attachments(template: &MaterialTemplate) -> RenderResult<()> {
    ATTACHMENTS.with_borrow(|attachments| {
        let Some(signature) = attachments else {
            return Ok(());
        };
        let formats: Vec<_> = template
            .targets
            .iter()
            .map(|t| t.as_ref().map(|t| t.format))
            .collect();
        if formats != signature.colors
            || template.depth_stencil.as_ref().map(|d| d.format) != signature.depth
            || template.multisample.count != signature.samples
        {
            return Err(crate::backend::RenderError::new(
                "material attachments",
                "color/depth formats or sample count differ from the pass",
            ));
        }
        Ok(())
    })
}

/// Also works with Velum's typed storage and heterogeneous `DrawPipeline` handles.
#[derive(Debug)]
pub struct MaterialPipelineContext {
    pub pipeline: wgpu::RenderPipeline,
}
pub fn try_context(
    template: crate::backend::MaterialTemplateId,
) -> RenderResult<MaterialPipelineContext> {
    let template = crate::backend::material::get(template)
        .ok_or_else(|| crate::backend::RenderError::new("pipeline context", "missing template"))?;
    let pipeline = crate::backend::pipeline::get(template.pipeline.id())
        .ok_or_else(|| crate::backend::RenderError::new("pipeline context", "missing pipeline"))?;
    Ok(MaterialPipelineContext {
        pipeline: pipeline.pipeline.clone(),
    })
}
impl crate::backend::PipelineContext for MaterialPipelineContext {
    fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
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
pub fn try_build(
    template: &MaterialTemplate,
    layouts: &ResolvedLayouts,
) -> RenderResult<MaterialPipelineContext> {
    let gpu = crate::backend::gpu(template.context)?;
    let buffers: Vec<_> = template
        .vertex_layouts
        .iter()
        .map(|layout| {
            layout.as_ref().map(|layout| wgpu::VertexBufferLayout {
                array_stride: layout.stride,
                step_mode: layout.step_mode,
                attributes: &layout.attributes,
            })
        })
        .collect();
    let vertex_constants: Vec<_> = template
        .vertex
        .constants
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect();
    let fragment_constants: Vec<_> = template
        .fragment
        .as_ref()
        .map(|f| {
            f.constants
                .iter()
                .map(|(name, value)| (name.as_str(), *value))
                .collect()
        })
        .unwrap_or_default();
    let pipeline = crate::backend::checked(&gpu.device, &template.label, || {
        gpu.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&template.label),
                layout: Some(&layouts.pipeline),
                vertex: wgpu::VertexState {
                    module: &template.vertex.shader,
                    entry_point: Some(&template.vertex.entry_point),
                    buffers: &buffers,
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &vertex_constants,
                        ..Default::default()
                    },
                },
                fragment: template
                    .fragment
                    .as_ref()
                    .map(|fragment| wgpu::FragmentState {
                        module: &fragment.shader,
                        entry_point: Some(&fragment.entry_point),
                        targets: &template.targets,
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants: &fragment_constants,
                            ..Default::default()
                        },
                    }),
                primitive: template.primitive,
                depth_stencil: template.depth_stencil.clone(),
                multisample: template.multisample,
                multiview_mask: None,
                cache: None,
            })
    })?;
    Ok(MaterialPipelineContext { pipeline })
}

/// Compute uses the same explicit schemas and resource resolution as graphics.
pub struct ComputeContext {
    pub context: Option<crate::backend::context::ContextId>,
    pub pipeline: wgpu::ComputePipeline,
    pub layouts: ResolvedLayouts,
}
impl crate::backend::pipeline::compute::ComputePipelineContext for ComputeContext {
    fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }
    fn bind_groups(&self) -> &[Option<wgpu::BindGroup>] {
        &[]
    }
}
pub fn try_compute(
    context: Option<crate::backend::context::ContextId>,
    shader: &crate::backend::ShaderEntry,
    schema: &crate::backend::BindingSchema,
) -> RenderResult<ComputeContext> {
    let gpu = crate::backend::gpu(context)?;
    let layouts = schema.try_layouts(context)?;
    let constants: Vec<_> = shader
        .constants
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect();
    let pipeline = crate::backend::checked(&gpu.device, "compute pipeline", || {
        gpu.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Velum compute"),
                layout: Some(&layouts.pipeline),
                module: &shader.shader,
                entry_point: Some(&shader.entry_point),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                cache: None,
            })
    })?;
    Ok(ComputeContext {
        context,
        pipeline,
        layouts,
    })
}
pub fn dispatch(
    context: &ComputeContext,
    bindings: &[crate::backend::BindingUse],
    workgroups: [u32; 3],
    pass: &mut wgpu::ComputePass<'_>,
) -> RenderResult<()> {
    let limits = crate::backend::gpu(context.context)?.device.limits();
    if workgroups
        .iter()
        .any(|&v| v > limits.max_compute_workgroups_per_dimension)
    {
        return Err(crate::backend::RenderError::new(
            "compute dispatch",
            "workgroup count exceeds device limit",
        ));
    }
    let groups = crate::backend::bindings::resolve(context.context, &context.layouts, bindings)?;
    crate::backend::retain(
        groups
            .iter()
            .flat_map(|g| g._resources.iter().cloned())
            .collect::<Vec<_>>(),
    );
    pass.set_pipeline(&context.pipeline);
    for group in &groups {
        pass.set_bind_group(group.index, &group.group, &group.offsets);
    }
    pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
    crate::backend::report::record_dispatch();
    Ok(())
}
