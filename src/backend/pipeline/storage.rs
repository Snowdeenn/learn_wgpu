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

impl<T> Clone for PipelineId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

fn storage() -> &'static RwLock<Arena<StoredContext>> {
    STORAGE.get_or_init(|| RwLock::new(Arena::new()))
}

pub fn register<T: PipelineContext + Send + Sync + 'static>(context: T) -> PipelineId<T> {
    insert(context)
}

pub(super) fn insert<T: Send + Sync + 'static>(context: T) -> PipelineId<T> {
    PipelineId {
        raw: storage().write().unwrap().insert(Arc::new(context)),
        _marker: PhantomData,
    }
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
