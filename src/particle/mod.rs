mod emitter;
mod data;
mod effect;
mod storage;
pub(crate) mod modules;

pub use effect::{EffectDescription, EffectInstance};
pub use emitter::EmitterSettings;
pub(crate) use storage::for_each_prepared_effect;
#[allow(unused_imports, reason = "Reexport de l'API du moteur, non utilise par la scene actuelle.")]
pub use storage::{remove_effect, spawn_effect, update_effects};
