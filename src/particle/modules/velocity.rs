use crate::particle::data::{Access, ParticleView};
use super::{InitModule, ParticleMask, sample_uniform};

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct InitVelocity {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitVelocity {
    fn write_mask(&self) -> u32 {
        ParticleMask::VELOCITY
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(values)) = view.velocity else {
            unreachable!("Le moteur doit fournir le tableau velocity en écriture");
        };

        for value in values {
            for channel in value {
                *channel = sample_uniform(self.min, self.max);
            }
        }
    }
}
