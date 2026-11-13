mod storage;

#[allow(unused_imports, reason = "Reexport de l'API du moteur, non utilise par la scene actuelle.")]
pub use storage::{TextureBindingSetId, group, register, remove};

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
    layout: wgpu::BindGroupLayout,
    entries: Vec<TextureBinding>,
    source_revisions: Vec<u64>,
    group: wgpu::BindGroup,
}

#[derive(Debug)]
pub enum BindingSetError {
    MissingSet(TextureBindingSetId),
    MissingTarget(RenderTargetId),
}

impl std::fmt::Display for BindingSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSet(id) => write!(f, "Unknown texture binding set: {id}"),
            Self::MissingTarget(id) => write!(f, "Unknown source render target: {id}"),
        }
    }
}

impl std::error::Error for BindingSetError {}

impl TextureBindingSet {
    fn new(
        layout: wgpu::BindGroupLayout,
        entries: Vec<TextureBinding>,
    ) -> Result<Self, BindingSetError> {
        let sources = resolve_sources(&entries)?;
        let group = build_group(&layout, &entries, &sources);
        let source_revisions = sources.iter().map(|target| target.revision()).collect();
        Ok(Self {
            layout,
            entries,
            source_revisions,
            group,
        })
    }

    fn refresh(&mut self) -> Result<(), BindingSetError> {
        let sources = resolve_sources(&self.entries)?;
        let revisions: Vec<_> = sources.iter().map(|target| target.revision()).collect();
        if revisions != self.source_revisions {
            self.group = build_group(&self.layout, &self.entries, &sources);
            self.source_revisions = revisions;
        }
        Ok(())
    }
}

fn resolve_sources(entries: &[TextureBinding]) -> Result<Vec<Arc<RenderTarget>>, BindingSetError> {
    let mut sources = Vec::new();
    for entry in entries {
        if let TextureBinding::Texture { target, .. } = entry {
            sources.push(target::get(*target).ok_or(BindingSetError::MissingTarget(*target))?);
        }
    }
    Ok(sources)
}

fn build_group(
    layout: &wgpu::BindGroupLayout,
    entries: &[TextureBinding],
    sources: &[Arc<RenderTarget>],
) -> wgpu::BindGroup {
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
    context::device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Render target bindings"),
        layout,
        entries: &entries,
    })
}
