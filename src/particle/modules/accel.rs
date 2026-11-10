use crate::particle::data::{Access, ParticleView};
use super::{InitModule, ParticleMask, sample_uniform};

pub struct InitAccel {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitAccel {
    fn write_mask(&self) -> u32 {
        ParticleMask::ACCEL
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(values)) = view.accel else {
            unreachable!("Le moteur doit fournir le tableau accel en écriture");
        };

        for value in values {
            *value = sample_uniform(self.min, self.max);
        }
    }
}
