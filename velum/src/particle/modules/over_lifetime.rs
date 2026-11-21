use super::{ParticleMask, UpdateModule};
use crate::particle::data::{Access, ParticleView};

pub(super) fn normalized_age(age: f32, lifetime: f32) -> f32 {
    if lifetime > 0.0 {
        (age / lifetime).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

pub struct ColorOverLifetime {
    pub start: [f32; 4],
    pub end: [f32; 4],
}

impl UpdateModule for ColorOverLifetime {
    fn read_mask(&self) -> u32 {
        ParticleMask::CURR_LIFETIME | ParticleMask::LIFETIME
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::COLOR
    }
    fn update(&self, view: ParticleView<'_>, _dt: f32) {
        let (Some(Access::Read(ages)), Some(Access::Read(lifetimes)), Some(Access::Write(colors))) =
            (view.curr_lifetime, view.lifetime, view.color)
        else {
            unreachable!("age, lifetime and color are required");
        };
        for ((age, lifetime), color) in ages.iter().zip(lifetimes).zip(colors) {
            let t = normalized_age(*age, *lifetime);
            *color = std::array::from_fn(|i| self.start[i] + (self.end[i] - self.start[i]) * t);
        }
    }
}

pub struct ScaleOverLifetime {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

impl UpdateModule for ScaleOverLifetime {
    fn read_mask(&self) -> u32 {
        ParticleMask::CURR_LIFETIME | ParticleMask::LIFETIME
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::SCALE
    }
    fn update(&self, view: ParticleView<'_>, _dt: f32) {
        let (Some(Access::Read(ages)), Some(Access::Read(lifetimes)), Some(Access::Write(scales))) =
            (view.curr_lifetime, view.lifetime, view.scale)
        else {
            unreachable!("age, lifetime and scale are required");
        };
        for ((age, lifetime), scale) in ages.iter().zip(lifetimes).zip(scales) {
            let t = normalized_age(*age, *lifetime);
            *scale = std::array::from_fn(|i| self.start[i] + (self.end[i] - self.start[i]) * t);
        }
    }
}
