use super::{
    BindingSchema, BindingSetId, RenderError, RenderResult,
    bindings::ResolvedLayouts,
    pipeline::MaterialPipelineContext,
    storage::{self, ResourceId},
};
use crate::backend::{context::ContextId, pipeline::PipelineId};
use crate::{OwnedResource, SharedResource};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexLayout {
    pub stride: u64,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}
impl From<wgpu::VertexBufferLayout<'_>> for VertexLayout {
    fn from(layout: wgpu::VertexBufferLayout<'_>) -> Self {
        Self {
            stride: layout.array_stride,
            step_mode: layout.step_mode,
            attributes: layout.attributes.to_vec(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct ShaderEntry {
    pub shader: wgpu::ShaderModule,
    pub entry_point: String,
    pub constants: Vec<(String, f64)>,
}
impl ShaderEntry {
    pub fn new(shader: wgpu::ShaderModule, entry: impl Into<String>) -> Self {
        Self {
            shader,
            entry_point: entry.into(),
            constants: vec![],
        }
    }
}
#[derive(Clone, Debug)]
pub struct MaterialTemplate {
    pub context: Option<ContextId>,
    pub label: String,
    pub vertex: ShaderEntry,
    pub fragment: Option<ShaderEntry>,
    pub schema: BindingSchema,
    /// Index is the explicit vertex buffer slot. Holes remain holes.
    pub vertex_layouts: Vec<Option<VertexLayout>>,
    pub primitive: wgpu::PrimitiveState,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub targets: Vec<Option<wgpu::ColorTargetState>>,
    pub multisample: wgpu::MultisampleState,
}
impl MaterialTemplate {
    pub fn fullscreen(
        context: Option<ContextId>,
        label: impl Into<String>,
        shader: wgpu::ShaderModule,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            context,
            label: label.into(),
            vertex: ShaderEntry::new(shader.clone(), "vs_main"),
            fragment: Some(ShaderEntry::new(shader, "fs_main")),
            schema: BindingSchema::default(),
            vertex_layouts: vec![],
            primitive: Default::default(),
            depth_stencil: None,
            targets: vec![Some(format.into())],
            multisample: Default::default(),
        }
    }
}
/// Immutable validated description and its eagerly constructed context. No cache.
#[derive(Debug)]
pub struct TemplateResource {
    pub description: MaterialTemplate,
    pub layouts: ResolvedLayouts,
    pub pipeline: SharedResource<PipelineId<MaterialPipelineContext>>,
}
pub type MaterialTemplateId = ResourceId<TemplateResource>;
storage::stored!(TemplateResource);

#[derive(Clone)]
pub struct Material {
    pub template: SharedResource<MaterialTemplateId>,
    /// Parameters are persistent uniform bindings, created with `BindingResource::uniform`.
    pub bindings: Vec<SharedResource<BindingSetId>>,
}
pub type MaterialId = ResourceId<Material>;
storage::stored!(Material);
pub use storage::{get, remove};

pub fn try_template(
    description: MaterialTemplate,
) -> RenderResult<OwnedResource<MaterialTemplateId>> {
    let layouts = description.schema.try_layouts(description.context)?;
    let context = super::pipeline::try_build(&description, &layouts)?;
    let pipeline = crate::backend::pipeline::register(context).share();
    Ok(storage::insert(TemplateResource {
        description,
        layouts,
        pipeline,
    }))
}
pub fn try_register(material: Material) -> RenderResult<OwnedResource<MaterialId>> {
    validate(&material)?;
    Ok(storage::insert(material))
}
/// Registers a material after checking the context of its template.
pub fn try_register_in(
    material: Material,
    context_id: Option<ContextId>,
) -> RenderResult<OwnedResource<MaterialId>> {
    validate_in(&material, Some(context_id))?;
    Ok(storage::insert(material))
}
pub fn try_update(id: MaterialId, material: Material) -> RenderResult<()> {
    validate(&material)?;
    if !storage::replace(id, material) {
        return Err(RenderError::new("material", "missing resource"));
    }
    Ok(())
}
fn validate(material: &Material) -> RenderResult<()> {
    validate_in(material, None)
}
fn validate_in(material: &Material, context_id: Option<Option<ContextId>>) -> RenderResult<()> {
    let template = get(material.template.id())
        .ok_or_else(|| RenderError::new("material", "missing template"))?;
    if context_id.is_some_and(|id| id != template.description.context) {
        return Err(RenderError::new("material", "GPU context mismatch"));
    }
    let gpu = super::gpu(template.description.context)?;
    let mut seen = std::collections::BTreeSet::new();
    for owner in &material.bindings {
        let set = super::bindings::get(owner.id())
            .ok_or_else(|| RenderError::new("material", "missing bindings"))?;
        if set.context != template.description.context {
            return Err(RenderError::new("material", "GPU context mismatch"));
        }
        for (binding, resource) in &set.entries {
            let entry = template.layouts.schema.entry(set.group, *binding)?;
            super::bindings::validate_resource(set.context, &gpu.device, entry, resource, 0)?;
            if !seen.insert((set.group, *binding)) {
                return Err(RenderError::new("material", "conflicting bindings"));
            }
        }
    }
    Ok(())
}
