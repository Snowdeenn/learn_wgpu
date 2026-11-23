use crate::{OwnedResource, ResourceHandle};
use new_collection::{Arena, Id};
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

pub struct Registry<T> {
    entries: RwLock<Arena<Arc<T>>>,
}
impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            entries: RwLock::new(Arena::new()),
        }
    }
}
pub trait Stored: Sized + Send + Sync + 'static {
    fn registry() -> &'static Registry<Self>;
}

/// Generational access identifier; copying an ID does not retain its resource.
pub struct ResourceId<T: Stored> {
    raw: Id<Arc<T>>,
    marker: PhantomData<fn() -> T>,
}
impl<T: Stored> Copy for ResourceId<T> {}
impl<T: Stored> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Stored> PartialEq for ResourceId<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.raw == rhs.raw
    }
}
impl<T: Stored> Eq for ResourceId<T> {}
impl<T: Stored> PartialOrd for ResourceId<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}
impl<T: Stored> Ord for ResourceId<T> {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&rhs.raw)
    }
}
#[cfg(test)]
impl<T: Stored> ResourceId<T> {
    pub(crate) fn from_raw(raw: Id<Arc<T>>) -> Self {
        Self {
            raw,
            marker: PhantomData,
        }
    }
}
impl<T: Stored> std::fmt::Debug for ResourceId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", std::any::type_name::<T>(), self.raw)
    }
}
impl<T: Stored> ResourceHandle for ResourceId<T> {
    fn release(self) {
        remove(self);
    }
}
pub fn insert<T: Stored>(value: T) -> OwnedResource<ResourceId<T>> {
    OwnedResource::new(insert_id(value))
}

pub(super) fn insert_id<T: Stored>(value: T) -> ResourceId<T> {
    ResourceId {
        raw: T::registry()
            .entries
            .write()
            .unwrap()
            .insert(Arc::new(value)),
        marker: PhantomData,
    }
}
pub fn get<T: Stored>(id: ResourceId<T>) -> Option<Arc<T>> {
    T::registry().entries.read().unwrap().get(id.raw).cloned()
}
pub fn replace<T: Stored>(id: ResourceId<T>, value: T) -> bool {
    let old = {
        let mut entries = T::registry().entries.write().unwrap();
        let Some(entry) = entries.get_mut(id.raw) else {
            return false;
        };
        std::mem::replace(entry, Arc::new(value))
    };
    // Dependencies may live in this same registry: never drop under its lock.
    drop(old);
    true
}
pub fn remove<T: Stored>(id: ResourceId<T>) -> bool {
    let old = {
        let mut entries = T::registry().entries.write().unwrap();
        if entries.get(id.raw).is_none() {
            return false;
        }
        entries.remove(id.raw)
    };
    old.is_some()
}

macro_rules! stored {
    ($ty:ty) => {
        impl $crate::backend::storage::Stored for $ty {
            fn registry() -> &'static $crate::backend::storage::Registry<Self> {
                static STORE: std::sync::OnceLock<$crate::backend::storage::Registry<$ty>> =
                    std::sync::OnceLock::new();
                STORE.get_or_init(Default::default)
            }
        }
    };
}
pub(crate) use stored;
