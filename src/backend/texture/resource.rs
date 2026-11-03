use super::Texture;

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
