mod data;
mod effect;
mod emitter;
pub mod modules;
mod storage;

pub use effect::{EffectDescription, EffectInstance};
pub use emitter::EmitterSettings;
pub use storage::for_each_prepared_effect;
#[allow(
    unused_imports,
    reason = "Reexport de l'API du moteur, non utilise par la scene actuelle."
)]
pub use storage::{prepare_registered_effects, remove_effect, spawn_effect, update_effects};
