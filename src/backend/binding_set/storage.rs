use std::sync::{Mutex, OnceLock};

use new_collection::{Arena, Id};

use super::{BindingSetError, TextureBinding, TextureBindingSet};

pub type TextureBindingSetId = Id<TextureBindingSet>;
static STORAGE: OnceLock<Mutex<Arena<TextureBindingSet>>> = OnceLock::new();

fn storage() -> &'static Mutex<Arena<TextureBindingSet>> {
    STORAGE.get_or_init(|| Mutex::new(Arena::new()))
}

pub fn register(
    layout: &wgpu::BindGroupLayout,
    entries: Vec<TextureBinding>,
) -> Result<TextureBindingSetId, BindingSetError> {
    let set = TextureBindingSet::new(layout.clone(), entries)?;
    Ok(storage().lock().unwrap().insert(set))
}

/// Resolve immediately before binding. A resize refreshes only the sets being used.
/// Samplers are owned immutable handles, so they need no revision tracking.
pub fn group(id: TextureBindingSetId) -> Result<wgpu::BindGroup, BindingSetError> {
    let mut sets = storage().lock().unwrap();
    let set = sets.get_mut(id).ok_or(BindingSetError::MissingSet(id))?;
    set.refresh()?;
    Ok(set.group.clone())
}

pub fn remove(id: TextureBindingSetId) -> bool {
    let mut sets = storage().lock().unwrap();
    if sets.get(id).is_none() {
        return false;
    }
    sets.remove(id).is_some()
}
