use super::{Access, InitModule, ParticleMask, ParticleView, UpdateModule, sample_uniform};

pub struct RandomLifetime {
    pub min: f32,
    pub max: f32,
}

impl InitModule for RandomLifetime {
    fn write_mask(&self) -> u32 {
        ParticleMask::LIFETIME
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(lifetimes)) = view.lifetime else {
            unreachable!("Le moteur doit fournir le tableau lifetime en écriture");
        };

        for lifetime in lifetimes {
            *lifetime = sample_uniform(self.min, self.max);
        }
    }
}

pub struct InitAge;

impl InitModule for InitAge {
    fn write_mask(&self) -> u32 {
        ParticleMask::CURR_LIFETIME
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(ages)) = view.curr_lifetime else {
            unreachable!("Le moteur doit fournir le tableau curr_lifetime en écriture");
        };

        ages.fill(0.0);
    }
}

pub struct UpdateAge;

impl UpdateModule for UpdateAge {
    fn read_mask(&self) -> u32 {
        0
    }

    fn write_mask(&self) -> u32 {
        ParticleMask::CURR_LIFETIME
    }

    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let Some(Access::Write(ages)) = view.curr_lifetime else {
            unreachable!("Le moteur doit fournir le tableau curr_lifetime en écriture");
        };

        for age in ages {
            *age += dt;
        }
    }
}
