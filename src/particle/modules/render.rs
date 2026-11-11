use std::ops::Range;

use new_collection::Id;

use crate::backend::shape::MeshResource;
use crate::backend::texture::TextureResource;
use crate::backend::{RenderInstance, Sprite};
use crate::particle::data::{Access, ParticleView};
use crate::particle::modules::RenderModuleOutput;

use super::{ParticleMask, ParticleSpriteInstance, ParticleShapeInstance, RenderModule};

pub struct RenderSprite {
    pub texture: Id<TextureResource>,
    pub indices: Range<u32>,
}

impl RenderModule for RenderSprite {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION | ParticleMask::SCALE | ParticleMask::COLOR
    }

    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput) {
        let (Some(Access::Read(positions)), Some(Access::Read(scales)), Some(Access::Read(colors))) =
            (view.pos, view.scale, view.color)
        else {
            unreachable!("Le moteur doit fournir position, scale et color en lecture");
        };

        output.sprite.reserve(positions.len());

        for ((&position, &scale), &color) in positions.iter().zip(scales).zip(colors) {
            output.sprite.push(ParticleSpriteInstance {
                render: RenderInstance {
                    color,
                    offset: position,
                    scale,
                    layer: 0,
                    rotation: 0.0,
                    _padding: [0; 2],
                    uv_min: [0.0; 2],
                    uv_max: [1.0; 2],
                },
                sprite: Sprite::Separate {
                    texture: self.texture.clone(),
                    indices: self.indices.clone(),
                },
            });
        }
    }
}

pub struct RenderShape {
    pub mesh: Id<MeshResource>,
    pub indices: Range<u32>,
}

impl RenderModule for RenderShape {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION | ParticleMask::SCALE | ParticleMask::COLOR
    }

    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput) {
        let (Some(Access::Read(positions)), Some(Access::Read(scales)), Some(Access::Read(colors))) =
            (view.pos, view.scale, view.color)
        else {
            unreachable!("Le moteur doit fournir position, scale et color en lecture");
        };

        output.shape.reserve(positions.len());

        for ((&position, &scale), &color) in positions.iter().zip(scales).zip(colors) {
            output.shape.push(ParticleShapeInstance {
                render: RenderInstance {
                    color,
                    offset: position,
                    scale,
                    layer: 0,
                    rotation: 0.0,
                    _padding: [0; 2],
                    uv_min: [0.0; 2],
                    uv_max: [1.0; 2],
                },
                mesh: self.mesh,
            });
        }
    }
}
