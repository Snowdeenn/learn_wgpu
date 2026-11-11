use std::sync::{Arc, OnceLock, RwLock};

use new_collection::{Arena, Id};

use crate::particle::modules::RenderModuleOutput;

use super::{EffectDescription, EffectInstance};

type EffectStorage = Arena<EffectInstance>;

static EFFECTS: OnceLock<RwLock<EffectStorage>> = OnceLock::new();

fn effect_storage() -> &'static RwLock<EffectStorage> {
    EFFECTS.get_or_init(|| RwLock::new(EffectStorage::new()))
}

pub fn spawn_effect(description: Arc<EffectDescription>, position: [f32; 2]) -> Id<EffectInstance> {
    let instance = EffectInstance::new(description, position);
    effect_storage().write().unwrap().insert(instance)
}

pub fn update_effects(dt: f32) {
    let mut effects = effect_storage().write().unwrap();
    for effect in effects.iter_mut() {
        effect.update(dt);
    }
}

pub(crate) fn for_each_prepared_effect(mut visit: impl FnMut(&mut RenderModuleOutput)) {
    let mut effects = effect_storage().write().unwrap();
    for effect in effects.iter_mut() {
        visit(effect.prepare());
    }
}

pub fn remove_effect(id: Id<EffectInstance>) -> bool {
    let removed = effect_storage().write().unwrap().remove(id);
    removed.is_some()
}
