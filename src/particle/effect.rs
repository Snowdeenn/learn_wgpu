use std::sync::Arc;

use super::data::ParticleData;
use super::emitter::{EmitterInstance, EmitterSettings};
use super::modules::{InitModule, ParticleSpriteInstance, RenderModule, UpdateModule};

pub struct EffectDescription {
    pub emitter: EmitterSettings,
    pub init_modules: Vec<Box<dyn InitModule>>,
    pub update_modules: Vec<Box<dyn UpdateModule>>,
    pub render_modules: Vec<Box<dyn RenderModule>>,
}

pub struct EffectInstance {
    description: Arc<EffectDescription>,
    emitter: EmitterInstance,
    particles: ParticleData,
    alive_count: usize,
    render_instances: Vec<ParticleSpriteInstance>,
}

impl EffectInstance {
    pub fn new(description: Arc<EffectDescription>) -> Self {
        let count = description.emitter.count as usize;
        let emitter = EmitterInstance::new(&description.emitter);
        let particles = ParticleData::new(count);

        EffectInstance {
            description,
            emitter,
            particles,
            alive_count: 0,
            render_instances: vec![],
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

            for module in &self.description.init_modules {
                let view =
                    crate::make_particle_view!(particles, start..end, 0, module.write_mask());
                module.init(view);
            }
            self.alive_count = end;
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
                self.alive_count = last;
            } else {
                index += 1;
            }
        }
    }

    pub fn prepare(&mut self) -> &mut [ParticleSpriteInstance] {
        self.render_instances.clear();

        for module in &self.description.render_modules {
            let view = crate::make_particle_view!(
                &mut self.particles,
                0..self.alive_count,
                module.read_mask(),
                0,
            );
            module.prepare(view, &mut self.render_instances);
        }
        &mut self.render_instances
    }
}
