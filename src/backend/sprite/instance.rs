use crate::backend::texture::{AtlasRegion, TextureResource};
use new_collection::Id;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
}

#[derive(Debug)]
pub enum Sprite {
    Separate {
        texture: Id<TextureResource>,
        indices: std::ops::Range<u32>,
    },
    Array {
        texture: Id<TextureResource>,
        layer: u32,
        indices: std::ops::Range<u32>,
    },
    Atlas {
        texture: Id<TextureResource>,
        region: Id<AtlasRegion>,
        indices: std::ops::Range<u32>,
    },
}

pub trait SpriteInstance {
    fn render_instance(&self) -> RenderInstance;
    fn sprite(&self) -> &Sprite;
}
