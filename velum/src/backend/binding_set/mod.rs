mod storage;

#[allow(
    unused_imports,
    reason = "Reexport de l'API du moteur, non utilise par la scene actuelle."
)]
pub use storage::{TextureBindingSetId, group, register, remove, update};

use std::sync::Arc;

use super::{
    context,
    target::{self, RenderTarget, RenderTargetId},
};

/// Binding numbers are explicit and must match the supplied layout.
/// Texture entries also support storage textures when the layout and usage allow it.
pub enum TextureBinding {
    Texture {
        binding: u32,
        target: RenderTargetId,
    },
    Sampler {
        binding: u32,
        sampler: wgpu::Sampler,
    },
}

pub struct TextureBindingSet {
    context_id: Option<context::ContextId>,
    layout: wgpu::BindGroupLayout,
    entries: Vec<TextureBinding>,
    source_revisions: Vec<u64>,
    group: wgpu::BindGroup,
}

#[derive(Debug)]
pub enum BindingSetError {
    MissingSet(TextureBindingSetId),
    MissingTarget(RenderTargetId),
    ContextMismatch(RenderTargetId),
    MissingContext,
}

impl std::fmt::Display for BindingSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContext => f.write_str("GPU context is missing"),
            Self::ContextMismatch(id) => write!(f, "Render target belongs to a different context: {id}"),
            Self::MissingSet(id) => write!(f, "Unknown texture binding set: {id}"),
            Self::MissingTarget(id) => write!(f, "Unknown source render target: {id}"),
        }
    }
}

impl std::error::Error for BindingSetError {}

impl TextureBindingSet {
    fn new(
        context_id: Option<context::ContextId>,
        layout: wgpu::BindGroupLayout,
        entries: Vec<TextureBinding>,
    ) -> Result<Self, BindingSetError> {
        let sources = resolve_sources(context_id, &entries)?;
        let group = build_group(context_id, &layout, &entries, &sources)?;
        let source_revisions = sources.iter().map(|target| target.revision()).collect();
        Ok(Self {
            context_id,
            layout,
            entries,
            source_revisions,
            group,
        })
    }

    fn refresh(&mut self) -> Result<(), BindingSetError> {
        let sources = resolve_sources(self.context_id, &self.entries)?;
        let revisions: Vec<_> = sources.iter().map(|target| target.revision()).collect();
        if revisions != self.source_revisions {
            self.group = build_group(self.context_id, &self.layout, &self.entries, &sources)?;
            self.source_revisions = revisions;
        }
        Ok(())
    }
}

fn resolve_sources(context_id: Option<context::ContextId>, entries: &[TextureBinding]) -> Result<Vec<Arc<RenderTarget>>, BindingSetError> {
    let mut sources = Vec::new();
    for entry in entries {
        if let TextureBinding::Texture { target, .. } = entry {
            let source = target::get(*target).ok_or(BindingSetError::MissingTarget(*target))?;
            if source.context_id() != context_id {
                return Err(BindingSetError::ContextMismatch(*target));
            }
            sources.push(source);
        }
    }
    Ok(sources)
}

fn build_group(
    context_id: Option<context::ContextId>,
    layout: &wgpu::BindGroupLayout,
    entries: &[TextureBinding],
    sources: &[Arc<RenderTarget>],
) -> Result<wgpu::BindGroup, BindingSetError> {
    let mut sources = sources.iter();
    let entries: Vec<_> = entries
        .iter()
        .map(|entry| match entry {
            TextureBinding::Texture { binding, .. } => wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::TextureView(
                    sources
                        .next()
                        .expect("Resolved texture source")
                        .texture()
                        .view(),
                ),
            },
            TextureBinding::Sampler { binding, sampler } => wgpu::BindGroupEntry {
                binding: *binding,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        })
        .collect();
    let context = context::try_context(context_id).ok_or(BindingSetError::MissingContext)?;
    Ok(context.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Render target bindings"),
        layout,
        entries: &entries,
    }))
}
