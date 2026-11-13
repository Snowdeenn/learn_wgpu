use std::collections::VecDeque;
use std::ops::Range;

use super::modules::ParticleMask;

pub struct ParticleData {
    pub(super) pos: Vec<[f32; 2]>,
    pub(super) scale: Vec<[f32; 2]>,
    pub(super) velocity: Vec<[f32; 2]>,
    pub(super) accel: Vec<f32>,
    pub(super) lifetime: Vec<f32>,
    pub(super) curr_lifetime: Vec<f32>,
    pub(super) color: Vec<[f32; 4]>,
    pub(super) rotation: Vec<f32>,
    pub(super) angular_velocity: Vec<f32>,
    pub(super) trail: Vec<VecDeque<[f32; 2]>>,
}

impl ParticleData {
    pub fn new(count: usize) -> Self {
        Self {
            pos: vec![[0.0; 2]; count],
            scale: vec![[1.0; 2]; count],
            velocity: vec![[0.0; 2]; count],
            accel: vec![0.0; count],
            lifetime: vec![0.0; count],
            curr_lifetime: vec![0.0; count],
            color: vec![[1.0; 4]; count],
            rotation: vec![0.0; count],
            angular_velocity: vec![0.0; count],
            trail: vec![VecDeque::new(); count],
        }
    }

    pub fn len(&self) -> usize {
        self.pos.len()
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty()
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn resize(&mut self, count: usize) {
        self.pos.resize(count, [0.0; 2]);
        self.scale.resize(count, [1.0; 2]);
        self.velocity.resize(count, [0.0; 2]);
        self.accel.resize(count, 0.0);
        self.lifetime.resize(count, 0.0);
        self.curr_lifetime.resize(count, 0.0);
        self.color.resize(count, [1.0; 4]);
        self.rotation.resize(count, 0.0);
        self.angular_velocity.resize(count, 0.0);
        self.trail.resize_with(count, VecDeque::new);
    }
}

pub enum Access<'a, T> {
    Read(&'a [T]),
    Write(&'a mut [T]),
}

macro_rules! particle_view {
    ($($field:ident: $ty:ty => $bit:path),+ $(,)?) => {
        #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
        pub struct ParticleView<'a> {
            $(pub $field: Option<Access<'a, $ty>>),+
        }

        impl<'a> ParticleView<'a> {
            pub fn from_masks(
                data: &'a mut ParticleData,
                range: Range<usize>,
                read: u32,
                write: u32,
            ) -> Self {
                Self {
                    $(
                        $field: if write & $bit != 0 {
                            Some(Access::Write(&mut data.$field[range.clone()]))
                        } else if read & $bit != 0 {
                            Some(Access::Read(&data.$field[range.clone()]))
                        } else {
                            None
                        },
                    )+
                }
            }
        }
    };
}

particle_view! {
    pos: [f32; 2] => ParticleMask::POSITION,
    scale: [f32; 2] => ParticleMask::SCALE,
    velocity: [f32; 2] => ParticleMask::VELOCITY,
    accel: f32 => ParticleMask::ACCEL,
    lifetime: f32 => ParticleMask::LIFETIME,
    curr_lifetime: f32 => ParticleMask::CURR_LIFETIME,
    color: [f32; 4] => ParticleMask::COLOR,
    rotation: f32 => ParticleMask::ROTATION,
    angular_velocity: f32 => ParticleMask::ANGULAR_VELOCITY,
    trail: VecDeque<[f32; 2]> => ParticleMask::TRAIL,
}

#[macro_export]
macro_rules! make_particle_view {
    ($data:expr, $range:expr, $read:expr, $write:expr $(,)?) => {
        $crate::particle::data::ParticleView::from_masks($data, $range, $read, $write)
    };
}
