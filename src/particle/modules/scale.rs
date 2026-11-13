use crate::particle::data::{Access, ParticleView};
use super::{InitModule, ParticleMask, UpdateModule, sample_uniform};

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct InitScale {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitScale {
    fn write_mask(&self) -> u32 {
        ParticleMask::SCALE
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(values)) = view.scale else {
            unreachable!("Le moteur doit fournir le tableau scale en écriture");
        };

        for value in values {
            for component in value {
                *component = sample_uniform(self.min, self.max);
            }
        }
    }
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct UpdateScale {
    pub rate: [f32; 2],
}

impl UpdateModule for UpdateScale {
    fn read_mask(&self) -> u32 {
        0
    }

    fn write_mask(&self) -> u32 {
        ParticleMask::SCALE
    }

    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let Some(Access::Write(scales)) = view.scale else {
            unreachable!("Le moteur doit fournir le tableau scale en écriture");
        };

        for [x, y] in scales {
            *x += self.rate[0] * dt;
            *y += self.rate[1] * dt;
        }
    }
}
