use crate::particle::data::{Access, ParticleView};
use crate::particle::modules::{UpdateModule};

use super::{InitModule, ParticleMask};

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct InitPos {
    pub min: f32,
    pub max: f32,
}

impl InitModule for InitPos {
    fn write_mask(&self) -> u32 {
        ParticleMask::POSITION
    }

    fn init(&self, view: ParticleView<'_>) {
        let Some(Access::Write(pos)) = view.pos else {
            unreachable!("Le moteur doit fournir le tableau position en écriture");
        };

        for ps in pos {
            let x = super::sample_uniform(self.min, self.max);
            let y = super::sample_uniform(self.min, self.max);

            ps[0] = x;
            ps[1] = y;
        }
    }
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct UpdatePos;

impl UpdateModule for UpdatePos {
    fn read_mask(&self) -> u32 {
        ParticleMask::VELOCITY | ParticleMask::ACCEL
    }

    fn write_mask(&self) -> u32 {
        ParticleMask::POSITION
    }

    fn update(&self, view: ParticleView<'_>, dt: f32) {
        let Some(Access::Write(pos)) = view.pos else {
            unreachable!("Le moteur doit fournir le tableau position en écriture");
        };
        let (Some(Access::Read(velo)), Some(Access::Read(accel))) = (view.velocity, view.accel)
        else {
            unreachable!("Le moteur doit fournir les tableaux velocity et accel en écriture");
        };

        for (i, pos) in pos.iter_mut().enumerate() {
            pos[0] += velo[i][0] * accel[i] * dt;
            pos[1] += velo[i][1] * accel[i] * dt;
        }
    }
}
