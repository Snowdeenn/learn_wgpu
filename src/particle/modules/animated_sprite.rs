use new_collection::Id;
use std::ops::Range;

use super::over_lifetime::normalized_age;
use super::{ParticleMask, ParticleSpriteInstance, RenderModule, RenderModuleOutput};
use crate::backend::{RenderInstance, Sprite, texture::TextureResource};
use crate::particle::data::{Access, ParticleView};

pub struct RenderAnimatedSprite {
    pub texture: Id<TextureResource>,
    pub indices: Range<u32>,
    pub layers: Range<u32>,
}

impl RenderModule for RenderAnimatedSprite {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION
            | ParticleMask::SCALE
            | ParticleMask::COLOR
            | ParticleMask::CURR_LIFETIME
            | ParticleMask::LIFETIME
            | ParticleMask::ROTATION
    }

    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput) {
        let Some(frame_count) = self
            .layers
            .end
            .checked_sub(self.layers.start)
            .filter(|&count| count > 0)
        else {
            return;
        };
        let (
            Some(Access::Read(positions)),
            Some(Access::Read(scales)),
            Some(Access::Read(colors)),
            Some(Access::Read(ages)),
            Some(Access::Read(lifetimes)),
            Some(Access::Read(rotations)),
        ) = (
            view.pos,
            view.scale,
            view.color,
            view.curr_lifetime,
            view.lifetime,
            view.rotation,
        )
        else {
            unreachable!("animated sprite attributes are required");
        };
        output.sprite.reserve(positions.len());
        for i in 0..positions.len() {
            let progress = normalized_age(ages[i], lifetimes[i]);
            let frame = ((progress * frame_count as f32) as u32).min(frame_count - 1);
            let layer = self.layers.start + frame;
            output.sprite.push(ParticleSpriteInstance {
                render: RenderInstance {
                    color: colors[i],
                    offset: positions[i],
                    scale: scales[i],
                    layer,
                    rotation: rotations[i],
                    _padding: [0; 2],
                    uv_min: [0.0; 2],
                    uv_max: [1.0; 2],
                },
                sprite: Sprite::Array {
                    texture: self.texture,
                    layer,
                    indices: self.indices.clone(),
                },
            });
        }
    }
}
