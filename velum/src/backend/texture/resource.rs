use super::Texture;
use crate::backend::context;
use crate::core::ResourceHandle;
use new_collection::Id;

/// Identifies a texture in the context that owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureId {
    pub(crate) context: Option<context::ContextId>,
    pub(crate) raw: Id<TextureResource>,
}

impl TextureId {
    pub fn context_id(self) -> Option<context::ContextId> {
        self.context
    }
}

/// Removes a texture and its atlas regions, if any.
pub fn remove(id: TextureId) -> bool {
    let Some(ctx) = context::try_context(id.context) else {
        return false;
    };
    let storage_owner = &ctx.texture_storage;
    let mut textures = storage_owner.write().unwrap();
    if textures.get(id.raw).is_none() {
        return false;
    }
    textures.remove(id.raw).is_some()
}

impl ResourceHandle for TextureId {
    fn release(self) {
        remove(self);
    }
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
