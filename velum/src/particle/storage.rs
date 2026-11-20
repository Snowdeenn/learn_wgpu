use crate::core::OwnedResource;
use crate::core::ResourceHandle;
use std::sync::{Arc, OnceLock, RwLock};

use new_collection::{Arena, Id};

use crate::particle::modules::RenderModuleOutput;

use super::{EffectDescription, EffectInstance};

type EffectStorage = Arena<EffectInstance>;

static EFFECTS: OnceLock<RwLock<EffectStorage>> = OnceLock::new();

fn effect_storage() -> &'static RwLock<EffectStorage> {
    EFFECTS.get_or_init(|| RwLock::new(EffectStorage::new()))
}

pub fn spawn_effect(
    description: Arc<EffectDescription>,
    position: [f32; 2],
) -> OwnedResource<Id<EffectInstance>> {
    let instance = EffectInstance::new(description, position);
    OwnedResource::new(effect_storage().write().unwrap().insert(instance))
}

pub fn update_effects(dt: f32) {
    let mut effects = effect_storage().write().unwrap();
    for effect in effects.iter_mut() {
        effect.update(dt);
    }
}

pub fn prepare_registered_effects() {
    let mut effects = effect_storage().write().unwrap();
    for effect in effects.iter_mut() {
        effect.prepare();
    }
}

pub fn for_each_prepared_effect(mut visit: impl FnMut(&mut RenderModuleOutput)) {
    let mut effects = effect_storage().write().unwrap();
    for effect in effects.iter_mut() {
        visit(effect.render_output_mut());
    }
}
#[allow(
    dead_code,
    reason = "API du moteur conservee pour les autres scenes et modules."
)]
pub fn remove_effect(id: Id<EffectInstance>) -> bool {
    let removed = {
        let mut effects = effect_storage().write().unwrap();
        if effects.get(id).is_none() {
            return false;
        }
        effects.remove(id)
    };
    removed.is_some()
}

impl ResourceHandle for Id<EffectInstance> {
    fn release(self) {
        remove_effect(self);
    }
}
