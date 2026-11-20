use std::sync::Arc;

use crate::particle::modules::RenderModuleOutput;

use super::data::ParticleData;
use super::emitter::{EmitterInstance, EmitterSettings};
use super::modules::{InitModule, RenderModule, UpdateModule};

pub struct EffectDescription {
    pub emitter: EmitterSettings,
    pub init_modules: Vec<Box<dyn InitModule>>,
    pub update_modules: Vec<Box<dyn UpdateModule>>,
    pub render_modules: Vec<Box<dyn RenderModule>>,
}

pub struct EffectInstance {
    description: Arc<EffectDescription>,
    position: [f32; 2],
    emitter: EmitterInstance,
    particles: ParticleData,
    alive_count: usize,
    render_output: RenderModuleOutput,
    trail_capacity: usize,
}

impl EffectInstance {
    pub fn new(description: Arc<EffectDescription>, position: [f32; 2]) -> Self {
        let count = description.emitter.count as usize;
        let emitter = EmitterInstance::new(&description.emitter);
        let particles = ParticleData::new(count);
        let trail_capacity = description
            .render_modules
            .iter()
            .map(|module| module.trail_capacity())
            .max()
            .unwrap_or(0);

        EffectInstance {
            description,
            position,
            emitter,
            particles,
            alive_count: 0,
            render_output: RenderModuleOutput::default(),
            trail_capacity,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }

        let requested = self.emitter.tick(dt) as usize;
        self.spawn_particles(requested);

        for module in &self.description.update_modules {
            let view = crate::make_particle_view!(
                &mut self.particles,
                0..self.alive_count,
                module.read_mask(),
                module.write_mask(),
            );
            module.update(view, dt);
        }

        self.kill_expired_particles();
        if dt > 0.0 {
            self.record_trails();
        }
    }

    fn record_trails(&mut self) {
        if self.trail_capacity == 0 {
            return;
        }
        for (history, &position) in self.particles.trail[..self.alive_count]
            .iter_mut()
            .zip(&self.particles.pos[..self.alive_count])
        {
            if history.back() == Some(&position) {
                continue;
            }
            while history.len() >= self.trail_capacity {
                history.pop_front();
            }
            history.push_back(position);
        }
    }

    fn spawn_particles(&mut self, requested: usize) {
        let start = self.alive_count;
        let end = start + requested.min(self.particles.len() - start);

        if start < end {
            let particles = &mut self.particles;
            particles.pos[start..end].fill([0.0; 2]);
            particles.scale[start..end].fill([1.0; 2]);
            particles.velocity[start..end].fill([0.0; 2]);
            particles.accel[start..end].fill(0.0);
            particles.lifetime[start..end].fill(0.0);
            particles.curr_lifetime[start..end].fill(0.0);
            particles.color[start..end].fill([1.0; 4]);
            particles.rotation[start..end].fill(0.0);
            particles.angular_velocity[start..end].fill(0.0);
            for history in &mut particles.trail[start..end] {
                history.clear();
            }

            for module in &self.description.init_modules {
                let view = crate::make_particle_view!(
                    particles,
                    start..end,
                    module.read_mask(),
                    module.write_mask()
                );
                module.init(view);
            }

            for position in &mut particles.pos[start..end] {
                position[0] += self.position[0];
                position[1] += self.position[1];
            }
            self.alive_count = end;
            if self.trail_capacity > 0 {
                for index in start..end {
                    particles.trail[index].push_back(particles.pos[index]);
                }
            }
        }
    }

    fn kill_expired_particles(&mut self) {
        let particles = &mut self.particles;
        let mut index = 0;
        while index < self.alive_count {
            if particles.curr_lifetime[index] >= particles.lifetime[index] {
                let last = self.alive_count - 1;
                particles.pos.swap(index, last);
                particles.scale.swap(index, last);
                particles.velocity.swap(index, last);
                particles.accel.swap(index, last);
                particles.lifetime.swap(index, last);
                particles.curr_lifetime.swap(index, last);
                particles.color.swap(index, last);
                particles.rotation.swap(index, last);
                particles.angular_velocity.swap(index, last);
                particles.trail.swap(index, last);
                self.alive_count = last;
            } else {
                index += 1;
            }
        }
    }

    pub fn prepare(&mut self) -> &mut RenderModuleOutput {
        self.render_output.clear();

        for module in &self.description.render_modules {
            let view = crate::make_particle_view!(
                &mut self.particles,
                0..self.alive_count,
                module.read_mask(),
                0,
            );
            module.prepare(view, &mut self.render_output);
        }
        &mut self.render_output
    }

    pub(crate) fn render_output_mut(&mut self) -> &mut RenderModuleOutput {
        &mut self.render_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle::modules::{InitPos, RandomLifetime};

    #[test]
    fn radial_initialization_reads_local_positions_before_world_offset() {
        use crate::particle::modules::*;
        let description = Arc::new(EffectDescription {
            emitter: EmitterSettings::once(1),
            init_modules: vec![
                Box::new(InitPosition {
                    shape: PositionShape::Point([3.0, 4.0]),
                    region: SpawnRegion::Volume,
                }),
                Box::new(RadialVelocity {
                    center: [0.0; 2],
                    min_speed: 5.0,
                    max_speed: 5.0,
                }),
                Box::new(RandomLifetime {
                    min: 10.0,
                    max: 10.0,
                }),
            ],
            update_modules: vec![Box::new(IntegratePosition)],
            render_modules: vec![],
        });
        let mut effect = EffectInstance::new(description, [100.0, 200.0]);
        effect.update(1.0);
        assert_eq!(effect.particles.velocity[0], [3.0, 4.0]);
        assert_eq!(effect.particles.pos[0], [106.0, 208.0]);
    }

    #[test]
    fn histories_and_rotation_follow_compaction_and_reset_on_respawn() {
        use crate::particle::modules::*;
        let description = Arc::new(EffectDescription {
            emitter: EmitterSettings::once(2),
            init_modules: vec![Box::new(RandomLifetime {
                min: 10.0,
                max: 10.0,
            })],
            update_modules: vec![Box::new(UpdateAge), Box::new(IntegratePosition)],
            render_modules: vec![Box::new(RenderTrail {
                texture: new_collection::Id::zero(),
                indices: 0..6,
                max_points: 3,
                width: 1.0,
            })],
        });
        let mut first = EffectInstance::new(description.clone(), [0.0; 2]);
        let mut second = EffectInstance::new(description, [100.0; 2]);
        first.update(0.0);
        second.update(0.0);
        first.particles.velocity[1] = [2.0, 0.0];
        first.particles.rotation[1] = 1.5;
        first.particles.angular_velocity[1] = 3.0;
        first.particles.lifetime[0] = 0.5;
        first.update(1.0);
        assert_eq!(first.alive_count, 1);
        assert_eq!(first.particles.rotation[0], 1.5);
        assert_eq!(first.particles.angular_velocity[0], 3.0);
        assert_eq!(
            first.particles.trail[0].iter().copied().collect::<Vec<_>>(),
            [[0.0, 0.0], [2.0, 0.0]]
        );
        let history = first.particles.trail[0].clone();
        first.prepare();
        first.prepare();
        assert_eq!(first.particles.trail[0], history);
        first.update(1.0);
        first.update(1.0);
        assert_eq!(
            first.particles.trail[0].iter().copied().collect::<Vec<_>>(),
            [[2.0, 0.0], [4.0, 0.0], [6.0, 0.0]]
        );
        assert_eq!(
            second.particles.trail[0]
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [[100.0, 100.0]]
        );
        first.particles.curr_lifetime[0] = 10.0;
        first.kill_expired_particles();
        first.spawn_particles(2);
        assert_eq!(first.particles.rotation, [0.0; 2]);
        assert_eq!(first.particles.angular_velocity, [0.0; 2]);
        for trail in &first.particles.trail {
            assert_eq!(trail.iter().copied().collect::<Vec<_>>(), [[0.0, 0.0]]);
        }
    }

    #[test]
    fn spawn_position_offsets_local_initial_positions_once() {
        let description = Arc::new(EffectDescription {
            emitter: EmitterSettings::once(1),
            init_modules: vec![
                Box::new(InitPos { min: 2.0, max: 2.0 }),
                Box::new(RandomLifetime { min: 1.0, max: 1.0 }),
            ],
            update_modules: vec![],
            render_modules: vec![],
        });
        let mut first = EffectInstance::new(description.clone(), [100.0, -50.0]);
        let mut second = EffectInstance::new(description, [-10.0, 20.0]);
        first.update(0.0);
        second.update(0.0);
        assert_eq!(first.particles.pos[0], [102.0, -48.0]);
        assert_eq!(second.particles.pos[0], [-8.0, 22.0]);

        first.update(0.1);
        assert_eq!(first.particles.pos[0], [102.0, -48.0]);
    }
}
