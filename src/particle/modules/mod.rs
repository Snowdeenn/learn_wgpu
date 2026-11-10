use rand::distr::Distribution;

use crate::backend::{RenderInstance, Sprite, SpriteInstance};

pub mod accel;
pub mod color;
pub mod lifetime;
pub mod position;
pub mod scale;
pub mod velocity;

pub trait InitModule {
    fn write_mask(&self) -> u32;
    fn init(&self, view: ParticleView<'_>);
}

pub trait UpdateModule {
    fn read_mask(&self) -> u32;
    fn write_mask(&self) -> u32;
    fn update(&self, view: ParticleView<'_>, dt: f32);
}

pub trait RenderModule {
    fn read_mask(&self) -> u32;
    fn prepare(&self, view: ParticleView<'_>, output: &mut Vec<ParticleSpriteInstance>);
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

pub enum Access<'a, T> {
    Read(&'a [T]),
    Write(&'a mut [T]),
}

// TODO: Définir les données des particules pour pouvoir créer la view
pub struct ParticleView<'a> {
    pub pos: Option<Access<'a, [f32; 2]>>,
    pub scale: Option<Access<'a, [f32; 2]>>,
    pub velocity: Option<Access<'a, [f32; 2]>>,
    pub accel: Option<Access<'a, f32>>,
    pub lifetime: Option<Access<'a, f32>>,
    pub curr_lifetime: Option<Access<'a, f32>>,
    pub color: Option<Access<'a, [f32; 4]>>,
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
}

pub(super) fn sample_uniform(min: f32, max: f32) -> f32 {
    let mut rng = rand::rng();
    rand::distr::Uniform::new_inclusive(min, max)
        .unwrap()
        .sample(&mut rng)
}
