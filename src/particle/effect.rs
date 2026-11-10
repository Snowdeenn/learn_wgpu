use std::sync::Arc;

use super::data::{ParticleData, ParticleView};
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
    render_instances: Vec<ParticleSpriteInstance>,
}

impl EffectInstance {
    pub fn new(description: Arc<EffectDescription>) -> Self {
        let count = description.emitter.count as usize;
        let emitter = EmitterInstance::new(&description.emitter);
        let mut particles = ParticleData::new(count);

        for module in &description.init_modules {
            let view = crate::make_particle_view!(&mut particles, 0, module.write_mask());
            module.init(view);
        }

        EffectInstance {
            description,
            emitter,
            particles,
            render_instances: vec![],
        }
    }

    pub fn update(&mut self, dt: f32) {
        for module in &self.description.update_modules {
            let view = crate::make_particle_view!(
                &mut self.particles,
                module.read_mask(),
                module.write_mask()
            );
            module.update(view, dt);
        }
    }

    pub fn prepare(&mut self) -> &[ParticleSpriteInstance] {
        self.render_instances.clear();
        
        for module in &self.description.render_modules {
            let view = crate::make_particle_view!(&mut self.particles, module.read_mask(), 0);
            module.prepare(view, &mut self.render_instances);
        }
        &self.render_instances
    }
}
