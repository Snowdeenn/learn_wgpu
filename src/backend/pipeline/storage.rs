use crate::core::OwnedResource;
use crate::core::ResourceHandle;
use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, OnceLock, RwLock},
};

use new_collection::{Arena, Id};

use super::PipelineContext;

type StoredContext = Arc<dyn Any + Send + Sync>;
static STORAGE: OnceLock<RwLock<Arena<StoredContext>>> = OnceLock::new();

pub struct PipelineId<T> {
    raw: Id<StoredContext>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Copy for PipelineId<T> {}

impl<T: Send + Sync + 'static> ResourceHandle for PipelineId<T> {
    fn release(self) { remove(self); }
}

impl<T> Clone for PipelineId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

fn storage() -> &'static RwLock<Arena<StoredContext>> {
    STORAGE.get_or_init(|| RwLock::new(Arena::new()))
}

pub fn register<T: PipelineContext + Send + Sync + 'static>(context: T) -> OwnedResource<PipelineId<T>> {
    insert(context)
}

pub(super) fn insert<T: Send + Sync + 'static>(context: T) -> OwnedResource<PipelineId<T>> {
    OwnedResource::new(PipelineId {
        raw: storage().write().unwrap().insert(Arc::new(context)),
        _marker: PhantomData,
    })
}

/// No storage lock is held during bindings or drawing.
pub fn get<T: Send + Sync + 'static>(id: PipelineId<T>) -> Option<Arc<T>> {
    let context = storage().read().unwrap().get(id.raw)?.clone();
    context.downcast().ok()
}

pub fn remove<T: Send + Sync + 'static>(id: PipelineId<T>) -> bool {
    let mut contexts = storage().write().unwrap();
    if contexts.get(id.raw).is_none() {
        return false;
    }
    contexts.remove(id.raw).is_some()
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    use crate::core::ResourceHandle;

    #[test]
    fn shared_owner_releases_storage_entry_only_after_last_drop() {
        let owner = insert(42_u32);
        let id = owner.id();
        let shared = owner.share();
        let other = shared.clone();
        assert_eq!(*get(id).unwrap(), 42);
        drop(shared);
        assert!(get(id).is_some());
        drop(other);
        assert!(get(id).is_none());
        id.release(); // A second release must not alter the arena free list.
        let replacement = insert(17_u32);
        let independent = insert(19_u32);
        assert_eq!(*get(replacement.id()).unwrap(), 17);
        assert_eq!(*get(independent.id()).unwrap(), 19);
    }
}
