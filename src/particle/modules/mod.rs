use crate::particle::data::ParticleView;
use new_collection::Id;
use rand::distr::Distribution;

use crate::backend::shape::{MeshResource, ShapeInstance};
use crate::backend::{RenderInstance, Sprite, SpriteInstance};

mod accel;
mod animated_sprite;
mod color;
mod forces;
mod lifetime;
mod over_lifetime;
mod position;
mod render;
mod rotation;
mod scale;
mod spawn;
#[cfg(test)]
mod tests;
mod trail;
mod velocity;

pub use animated_sprite::*;
pub use forces::*;
pub use over_lifetime::*;
pub use rotation::*;
pub use spawn::*;
pub use trail::*;

pub use accel::InitAccel;
pub use color::InitColor;
pub use lifetime::*;
pub use position::*;
pub use render::*;
pub use scale::*;
pub use velocity::InitVelocity;

pub trait InitModule: Send + Sync {
    fn read_mask(&self) -> u32 {
        0
    }
    fn write_mask(&self) -> u32;
    fn init(&self, view: ParticleView<'_>);
}

pub trait UpdateModule: Send + Sync {
    fn read_mask(&self) -> u32;
    fn write_mask(&self) -> u32;
    fn update(&self, view: ParticleView<'_>, dt: f32);
}

pub trait RenderModule: Send + Sync {
    /// Number of positions retained per particle by the effect, including its head.
    fn trail_capacity(&self) -> usize {
        0
    }
    fn read_mask(&self) -> u32;
    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput);
}

pub struct ParticleSpriteInstance {
    pub render: RenderInstance,
    pub sprite: Sprite,
}

impl SpriteInstance for ParticleSpriteInstance {
    fn render_instance(&self) -> RenderInstance {
        self.render
    }

    fn sprite(&self) -> &Sprite {
        &self.sprite
    }
}

pub struct ParticleShapeInstance {
    pub mesh: Id<MeshResource>,
    pub render: RenderInstance,
}

impl ShapeInstance for ParticleShapeInstance {
    fn mesh(&self) -> Id<MeshResource> {
        self.mesh
    }

    fn render_instance(&self) -> RenderInstance {
        self.render
    }
}

#[derive(Default)]
pub struct RenderModuleOutput {
    sprite: Vec<ParticleSpriteInstance>,
    shape: Vec<ParticleShapeInstance>,
}

impl RenderModuleOutput {
    pub fn clear(&mut self) {
        self.shape.clear();
        self.sprite.clear();
    }

    pub fn sprite(&mut self) -> &mut [ParticleSpriteInstance] {
        &mut self.sprite
    }

    pub fn shape(&mut self) -> &mut [ParticleShapeInstance] {
        &mut self.shape
    }
}

pub struct ParticleMask;

impl ParticleMask {
    pub const POSITION: u32 = 1 << 0;
    pub const SCALE: u32 = 1 << 1;
    pub const VELOCITY: u32 = 1 << 2;
    pub const ACCEL: u32 = 1 << 3;
    pub const LIFETIME: u32 = 1 << 4;
    pub const CURR_LIFETIME: u32 = 1 << 5;
    pub const COLOR: u32 = 1 << 6;
    pub const ROTATION: u32 = 1 << 7;
    pub const ANGULAR_VELOCITY: u32 = 1 << 8;
    pub const TRAIL: u32 = 1 << 9;
}

pub(super) fn sample_uniform(min: f32, max: f32) -> f32 {
    let mut rng = rand::rng();
    rand::distr::Uniform::new_inclusive(min, max)
        .unwrap()
        .sample(&mut rng)
}
