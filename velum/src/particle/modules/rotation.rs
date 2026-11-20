use super::{InitModule, ParticleMask, UpdateModule, sample_uniform};
use super::{RenderModule, RenderModuleOutput};
use crate::particle::data::{Access, ParticleView};

/// Adds particle rotation to an existing renderer producing one instance per particle.
/// Use WithRotation(RenderSprite { .. }) or WithRotation(RenderShape { .. }).
/// The wrapped modules remain unchanged. Do not wrap RenderTrail (multiple segments).
pub struct WithRotation<R>(pub R);

impl<R: RenderModule> RenderModule for WithRotation<R> {
    fn read_mask(&self) -> u32 {
        self.0.read_mask() | ParticleMask::ROTATION
    }

    fn prepare(&self, view: ParticleView<'_>, output: &mut RenderModuleOutput) {
        let Some(Access::Read(rotations)) = &view.rotation else {
            unreachable!("rotation must be readable");
        };
        let rotations = *rotations;
        let sprite_start = output.sprite.len();
        let shape_start = output.shape.len();
        self.0.prepare(view, output);
        for (instance, angle) in output.sprite[sprite_start..].iter_mut().zip(rotations) {
            instance.render.rotation = *angle;
        }
        for (instance, angle) in output.shape[shape_start..].iter_mut().zip(rotations) {
            instance.render.rotation = *angle;
        }
    }
}

/// Initial orientation in radians. Equal bounds give a fixed orientation.
pub struct InitRotation {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitRotation {
    fn write_mask(&self) -> u32 {
        ParticleMask::ROTATION
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(rotations)) = view.rotation else {
            unreachable!("rotation must be writable");
        };
        for rotation in rotations {
            *rotation = sample_uniform(self.min, self.max);
        }
    }
}

/// Angular velocity in radians per second.
pub struct InitAngularVelocity {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitAngularVelocity {
    fn write_mask(&self) -> u32 {
        ParticleMask::ANGULAR_VELOCITY
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(velocities)) = view.angular_velocity else {
            unreachable!("angular_velocity must be writable");
        };
        for velocity in velocities {
            *velocity = sample_uniform(self.min, self.max);
        }
    }
}

pub struct IntegrateRotation;

impl UpdateModule for IntegrateRotation {
    fn read_mask(&self) -> u32 {
        ParticleMask::ANGULAR_VELOCITY
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::ROTATION
    }

    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let (Some(Access::Write(rotations)), Some(Access::Read(velocities))) =
            (view.rotation, view.angular_velocity)
        else {
            unreachable!("rotation and angular_velocity are required");
        };
        for (rotation, velocity) in rotations.iter_mut().zip(velocities) {
            *rotation += velocity * dt;
        }
    }
}
