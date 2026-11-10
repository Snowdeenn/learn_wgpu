use super::{Access, InitModule, ParticleMask, ParticleView, sample_uniform};

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
            for component in value {
                *component = sample_uniform(self.min, self.max);
            }
        }
    }
}
