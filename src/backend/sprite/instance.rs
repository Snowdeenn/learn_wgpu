use crate::{
    backend::texture::{AtlasRegion, TextureResource},
    scene::resources::InstanceData,
};
use new_collection::Id;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub layer: u32,
    pub _padding: [u32; 3],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
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
    Animated {
        texture: Id<TextureResource>,
        layers: std::ops::Range<u32>,
        current_layer: u32,
        elapsed: f32,
        indices: std::ops::Range<u32>,
    },
}

pub trait SpriteInstance {
    fn render_instance(&self) -> RenderInstance;
    fn sprite(&self) -> &Sprite;
}

pub fn run_animated(instances: &mut InstanceData, dt: f32) {
    for inst in &mut **instances {
        if let Sprite::Animated {
            ref layers,
            ref mut current_layer,
            ref mut elapsed,
            ..
        } = inst.sprite
        {
            let frame_count = layers.end.saturating_sub(layers.start);
            if frame_count > 0 {
                let animation_fps = frame_count as f32;
                let duration = frame_count as f32 / animation_fps;

                *elapsed = (*elapsed + dt) % duration;
                let frame = (*elapsed * animation_fps) as u32;
                *current_layer = layers.start + frame.min(frame_count - 1);
            }
        }
    }
}
