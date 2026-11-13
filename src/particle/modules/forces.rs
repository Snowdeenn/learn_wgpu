use super::{ParticleMask, UpdateModule};
use crate::particle::data::{Access, ParticleView};

pub struct Gravity {
    pub acceleration: [f32; 2],
}

impl UpdateModule for Gravity {
    fn read_mask(&self) -> u32 {
        0
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }
    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let Some(Access::Write(velocities)) = view.velocity else {
            unreachable!("velocity must be writable");
        };
        for velocity in velocities {
            velocity[0] += self.acceleration[0] * dt;
            velocity[1] += self.acceleration[1] * dt;
        }
    }
}

pub struct Drag {
    pub coefficient: f32,
}

impl UpdateModule for Drag {
    fn read_mask(&self) -> u32 {
        0
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }
    fn update(&self, view: ParticleView<'_>, dt: f32) {
        assert!(self.coefficient.is_finite() && self.coefficient >= 0.0);
        let Some(Access::Write(velocities)) = view.velocity else {
            unreachable!("velocity must be writable");
        };
        let damping = (-self.coefficient * dt).exp();
        for velocity in velocities {
            velocity[0] *= damping;
            velocity[1] *= damping;
        }
    }
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct Attractor {
    pub point: [f32; 2],
    pub strength: f32,
}

impl UpdateModule for Attractor {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }
    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let (Some(Access::Read(positions)), Some(Access::Write(velocities))) =
            (view.pos, view.velocity)
        else {
            unreachable!("position and velocity are required");
        };
        for (position, velocity) in positions.iter().zip(velocities) {
            let delta = [self.point[0] - position[0], self.point[1] - position[1]];
            let distance = delta[0].hypot(delta[1]);
            if distance > 0.0 {
                velocity[0] += delta[0] / distance * self.strength * dt;
                velocity[1] += delta[1] / distance * self.strength * dt;
            }
        }
    }
}

pub struct NoiseForce {
    pub strength: f32,
    pub frequency: f32,
    pub seed: u32,
}

impl UpdateModule for NoiseForce {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }
    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let (Some(Access::Read(positions)), Some(Access::Write(velocities))) =
            (view.pos, view.velocity)
        else {
            unreachable!("position and velocity are required");
        };
        for (position, velocity) in positions.iter().zip(velocities) {
            let x = position[0] * self.frequency;
            let y = position[1] * self.frequency;
            velocity[0] += noise(x, y, self.seed) * self.strength * dt;
            velocity[1] += noise(x, y, self.seed.wrapping_add(1)) * self.strength * dt;
        }
    }
}

fn noise(x: f32, y: f32, seed: u32) -> f32 {
    fn hash(x: i32, y: i32, seed: u32) -> f32 {
        let mut value =
            (x as u32).wrapping_mul(0x9e3779b9) ^ (y as u32).wrapping_mul(0x85ebca6b) ^ seed;
        value ^= value >> 16;
        value = value.wrapping_mul(0x7feb352d);
        value ^= value >> 15;
        value = value.wrapping_mul(0x846ca68b);
        value ^= value >> 16;
        (value as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32
    }
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let smooth = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let tx = smooth(x - x.floor());
    let ty = smooth(y - y.floor());
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(
        lerp(hash(ix, iy, seed), hash(ix.wrapping_add(1), iy, seed), tx),
        lerp(
            hash(ix, iy.wrapping_add(1), seed),
            hash(ix.wrapping_add(1), iy.wrapping_add(1), seed),
            tx,
        ),
        ty,
    )
}

pub struct IntegratePosition;

impl UpdateModule for IntegratePosition {
    fn read_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::POSITION
    }
    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let (Some(Access::Write(positions)), Some(Access::Read(velocities))) =
            (view.pos, view.velocity)
        else {
            unreachable!("position and velocity are required");
        };
        for (position, velocity) in positions.iter_mut().zip(velocities) {
            position[0] += velocity[0] * dt;
            position[1] += velocity[1] * dt;
        }
    }
}

/// In 2D the plane is a line through `point`; `normal` points to the allowed side.
/// Run after IntegratePosition. Particles are treated as points.
#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct PlaneCollision {
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub restitution: f32,
    /// Fraction of tangential velocity lost on impact, from 0 to 1.
    pub friction: f32,
}

impl UpdateModule for PlaneCollision {
    fn read_mask(&self) -> u32 {
        0
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::POSITION | ParticleMask::VELOCITY
    }
    fn update(&self, view: ParticleView<'_>, _dt: f32) {
        let length = self.normal[0].hypot(self.normal[1]);
        assert!(length.is_finite() && length > 0.0);
        assert!((0.0..=1.0).contains(&self.restitution) && (0.0..=1.0).contains(&self.friction));
        let normal = [self.normal[0] / length, self.normal[1] / length];
        let (Some(Access::Write(positions)), Some(Access::Write(velocities))) =
            (view.pos, view.velocity)
        else {
            unreachable!("position and velocity must be writable");
        };
        for (position, velocity) in positions.iter_mut().zip(velocities) {
            let distance = (position[0] - self.point[0]) * normal[0]
                + (position[1] - self.point[1]) * normal[1];
            if distance <= 0.0 {
                position[0] -= distance * normal[0];
                position[1] -= distance * normal[1];
                let incoming = velocity[0] * normal[0] + velocity[1] * normal[1];
                if incoming < 0.0 {
                    for axis in 0..2 {
                        let tangent = velocity[axis] - incoming * normal[axis];
                        velocity[axis] = tangent * (1.0 - self.friction)
                            - incoming * self.restitution * normal[axis];
                    }
                }
            }
        }
    }
}
