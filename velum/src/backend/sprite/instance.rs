use crate::backend::DrawSortKey;
use crate::backend::{
    instances::{Instance, InstanceData},
    texture::{AtlasRegion, TextureId},
};
use new_collection::Id;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub layer: u32,
    /// Rotation in radians.
    pub rotation: f32,
    pub depth: f32,
    pub _padding: u32,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

#[derive(Debug, Clone)]
pub enum Sprite {
    Separate {
        texture: TextureId,
        indices: std::ops::Range<u32>,
    },
    #[allow(
        dead_code,
        reason = "API du moteur conservee pour les autres scenes et modules."
    )]
    Array {
        texture: TextureId,
        layer: u32,
        indices: std::ops::Range<u32>,
    },
    Atlas {
        texture: TextureId,
        region: Id<AtlasRegion>,
        indices: std::ops::Range<u32>,
    },
    Animated {
        texture: TextureId,
        layers: std::ops::Range<u32>,
        current_layer: u32,
        elapsed: f32,
        indices: std::ops::Range<u32>,
    },
}

pub trait SpriteInstance: Instance {
    fn render_instance(&self) -> RenderInstance;
    /// Override to supply an explicit order or avoid constructing a render instance.
    fn sort_key(&self) -> DrawSortKey {
        DrawSortKey {
            draw_order: 0,
            depth: self.render_instance().depth,
        }
    }
    fn sprite(&self) -> &Sprite;
    fn sprite_mut(&mut self) -> &mut Sprite;
}

pub fn run_animated<T: SpriteInstance>(instances: &mut InstanceData<T>, dt: f32) {
    for inst in &mut **instances {
        if let Sprite::Animated {
            layers,
            current_layer,
            elapsed,
            ..
        } = inst.sprite_mut()
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
