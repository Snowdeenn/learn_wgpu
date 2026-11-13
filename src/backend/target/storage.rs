use std::sync::{Arc, OnceLock, RwLock};

use new_collection::{Arena, Id};

use super::{RenderTarget, RenderTargetDescriptor, TargetSize, create_target};

type RenderTargetStorage = Arena<Arc<RenderTarget>, RenderTarget>;
pub type RenderTargetId = Id<RenderTarget>;

static STORAGE: OnceLock<RwLock<RenderTargetStorage>> = OnceLock::new();

fn storage() -> &'static RwLock<RenderTargetStorage> {
    STORAGE.get_or_init(|| RwLock::new(RenderTargetStorage::new()))
}

pub fn register(descriptor: RenderTargetDescriptor) -> RenderTargetId {
    let target = Arc::new(create_target(&descriptor));
    storage().write().unwrap().insert(target)
}

/// Snapshot of the current resource; resolve again after a resize.
/// The storage lock is released before the caller uses any GPU resources.
pub fn get(id: RenderTargetId) -> Option<Arc<RenderTarget>> {
    storage().read().unwrap().get(id).cloned()
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub fn remove(id: RenderTargetId) -> bool {
    let mut targets = storage().write().unwrap();
    if targets.get(id).is_none() {
        return false;
    }
    targets.remove(id).is_some()
}

/// Fixed targets and zero-sized windows are left unchanged.
pub fn resize_window_targets([width, height]: [u32; 2]) {
    if width == 0 || height == 0 {
        return;
    }
    let mut targets = storage().write().unwrap();
    for target in targets.iter_mut() {
        if matches!(target.size_policy(), TargetSize::Window { .. })
            && target.size() != [width, height]
        {
            *target = Arc::new(target.resized(width, height));
        }
    }
}
