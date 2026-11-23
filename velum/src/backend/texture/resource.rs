use super::Texture;
use crate::backend::{context, storage};
use crate::{OwnedResource, ResourceHandle};
use std::sync::Arc;

/// Identifies a texture and the GPU context it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureId {
    context: Option<context::ContextId>,
    raw: storage::ResourceId<TextureResource>,
}

impl TextureId {
    pub fn context_id(self) -> Option<context::ContextId> {
        self.context
    }

    #[cfg(test)]
    pub(crate) fn for_test(raw: new_collection::Id<Arc<TextureResource>>) -> Self {
        Self {
            context: None,
            raw: storage::ResourceId::from_raw(raw),
        }
    }
}

impl ResourceHandle for TextureId {
    fn release(self) {
        remove(self);
    }
}

pub(super) fn insert(
    context: Option<context::ContextId>,
    resource: TextureResource,
) -> OwnedResource<TextureId> {
    OwnedResource::new(TextureId {
        context,
        raw: storage::insert_id(resource),
    })
}

/// Resolves a registered texture while its GPU context is available.
pub fn get(id: TextureId) -> Option<Arc<TextureResource>> {
    context::try_context(id.context)?;
    storage::get(id.raw)
}

/// Removes a texture and its atlas regions, if any.
pub fn remove(id: TextureId) -> bool {
    storage::remove(id.raw)
}

#[derive(Debug)]
pub enum TextureResource {
    Separate(Resource),
    Array(Resource),
    Atlas {
        resource: Resource,
        regions: new_collection::Arena<AtlasRegion>,
    },
}
storage::stored!(TextureResource);

#[derive(Debug)]
pub struct Resource {
    pub texture: Texture,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct AtlasRegion {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}
