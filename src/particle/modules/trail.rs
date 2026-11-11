use new_collection::Id;
use std::ops::Range;

use super::{ParticleMask, ParticleSpriteInstance, RenderModule, RenderModuleOutput};
use crate::backend::{RenderInstance, Sprite, texture::TextureResource};
use crate::particle::data::{Access, ParticleView};

pub struct RenderTrail {
    pub texture: Id<TextureResource>,
    pub indices: Range<u32>,
    pub max_points: usize,
    pub width: f32,
}

impl RenderModule for RenderTrail {
    fn trail_capacity(&self) -> usize {
        self.max_points
    }

    fn read_mask(&self) -> u32 {
        ParticleMask::TRAIL | ParticleMask::COLOR
    }

    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput) {
        assert!(self.width.is_finite() && self.width >= 0.0);
        if self.max_points < 2 || self.width == 0.0 {
            return;
        }
        let (Some(Access::Read(histories)), Some(Access::Read(colors))) = (view.trail, view.color)
        else {
            unreachable!("trail and color are required");
        };
        for (history, &color) in histories.iter().zip(colors) {
            let start = history.len().saturating_sub(self.max_points);
            for (a, b) in history
                .iter()
                .skip(start)
                .zip(history.iter().skip(start + 1))
            {
                let delta = [b[0] - a[0], b[1] - a[1]];
                let length = delta[0].hypot(delta[1]);
                if length == 0.0 {
                    continue;
                }
                output.sprite.push(ParticleSpriteInstance {
                    render: RenderInstance {
                        color,
                        offset: [a[0] + delta[0] * 0.5, a[1] + delta[1] * 0.5],
                        scale: [length, self.width],
                        layer: 0,
                        rotation: delta[1].atan2(delta[0]),
                        _padding: [0; 2],
                        uv_min: [0.0; 2],
                        uv_max: [1.0; 2],
                    },
                    sprite: Sprite::Separate {
                        texture: self.texture,
                        indices: self.indices.clone(),
                    },
                });
            }
        }
    }
}
