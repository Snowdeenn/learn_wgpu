use crate::particle::data::{Access, ParticleView};
use super::{InitModule, ParticleMask, sample_uniform};

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct InitColor {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitColor {
    fn write_mask(&self) -> u32 {
        ParticleMask::COLOR
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(values)) = view.color else {
            unreachable!("Le moteur doit fournir le tableau color en écriture");
        };

        // A voir si on choisi la couleur arbitrairement
        for value in values {
            for component in value {
                *component = sample_uniform(self.min, self.max);
            }
        }
    }
}
