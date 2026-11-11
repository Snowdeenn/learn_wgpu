use std::f32::consts::{PI, TAU};

use super::{InitModule, ParticleMask, sample_uniform};
use crate::particle::data::{Access, ParticleView};

/// In 2D, Volume samples the interior and Surface samples the boundary.
#[derive(Clone, Copy)]
pub enum SpawnRegion {
    Volume,
    Surface,
}

#[derive(Clone, Copy)]
pub enum PositionShape {
    Point([f32; 2]),
    Circle {
        center: [f32; 2],
        radius: f32,
    },
    Rectangle {
        center: [f32; 2],
        half_extents: [f32; 2],
    },
}

pub struct InitPosition {
    pub shape: PositionShape,
    pub region: SpawnRegion,
}

impl InitModule for InitPosition {
    fn write_mask(&self) -> u32 {
        ParticleMask::POSITION
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(positions)) = view.pos else {
            unreachable!("position must be writable");
        };
        for position in positions {
            *position = match self.shape {
                PositionShape::Point(point) => point,
                PositionShape::Circle { center, radius } => {
                    assert!(radius.is_finite() && radius >= 0.0);
                    let angle = sample_uniform(0.0, TAU);
                    let radius = radius
                        * match self.region {
                            SpawnRegion::Volume => sample_uniform(0.0, 1.0).sqrt(),
                            SpawnRegion::Surface => 1.0,
                        };
                    let (sin, cos) = angle.sin_cos();
                    [center[0] + radius * cos, center[1] + radius * sin]
                }
                PositionShape::Rectangle {
                    center,
                    half_extents: [x, y],
                } => {
                    assert!(x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0);
                    let offset = match self.region {
                        SpawnRegion::Volume => [sample_uniform(-x, x), sample_uniform(-y, y)],
                        SpawnRegion::Surface => {
                            // Select an edge in proportion to its length.
                            let distance = sample_uniform(0.0, 4.0 * (x + y));
                            if distance < 2.0 * x {
                                [-x + distance, -y]
                            } else if distance < 2.0 * x + 2.0 * y {
                                [x, -y + distance - 2.0 * x]
                            } else if distance < 4.0 * x + 2.0 * y {
                                [x - (distance - 2.0 * x - 2.0 * y), y]
                            } else {
                                [-x, y - (distance - 4.0 * x - 2.0 * y)]
                            }
                        }
                    };
                    [center[0] + offset[0], center[1] + offset[1]]
                }
            };
        }
    }
}

/// A 2D cone (angular sector). Angles are radians; spread is the half-angle.
pub struct ConeVelocity {
    pub direction: f32,
    pub spread: f32,
    pub min_speed: f32,
    pub max_speed: f32,
}

impl InitModule for ConeVelocity {
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }

    fn init(&self, view: ParticleView<'_>) {
        assert!(self.spread.is_finite() && (0.0..=PI).contains(&self.spread));
        assert!(self.min_speed >= 0.0 && self.max_speed >= self.min_speed);
        let Some(Access::Write(velocities)) = view.velocity else {
            unreachable!("velocity must be writable");
        };
        for velocity in velocities {
            let angle = self.direction + sample_uniform(-self.spread, self.spread);
            let speed = sample_uniform(self.min_speed, self.max_speed);
            let (sin, cos) = angle.sin_cos();
            *velocity = [cos * speed, sin * speed];
        }
    }
}

/// Run after position initialization. The center is local to the emitter.
pub struct RadialVelocity {
    pub center: [f32; 2],
    pub min_speed: f32,
    pub max_speed: f32,
}

impl InitModule for RadialVelocity {
    fn read_mask(&self) -> u32 {
        ParticleMask::POSITION
    }
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }

    fn init(&self, view: ParticleView<'_>) {
        assert!(self.min_speed >= 0.0 && self.max_speed >= self.min_speed);
        let (Some(Access::Read(positions)), Some(Access::Write(velocities))) =
            (view.pos, view.velocity)
        else {
            unreachable!("position and velocity are required");
        };
        for (position, velocity) in positions.iter().zip(velocities) {
            let delta = [position[0] - self.center[0], position[1] - self.center[1]];
            let length = delta[0].hypot(delta[1]);
            *velocity = if length > 0.0 {
                let speed = sample_uniform(self.min_speed, self.max_speed);
                [delta[0] / length * speed, delta[1] / length * speed]
            } else {
                [0.0; 2]
            };
        }
    }
}
