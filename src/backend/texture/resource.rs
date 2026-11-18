use super::Texture;
use crate::backend::context;
use crate::core::ResourceHandle;
use new_collection::Id;

/// Removes a texture and its atlas regions, if any.
pub fn remove(id: Id<TextureResource>) -> bool {
    let storage_owner = context::texture_storage(None);
    let mut textures = storage_owner.write().unwrap();
    if textures.get(id).is_none() {
        return false;
    }
    textures.remove(id).is_some()
}

impl ResourceHandle for Id<TextureResource> {
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
