pub(crate) mod cache;

pub use cache::memory_stats;
pub(crate) use cache::{reset_frame, submitted};

#[derive(Debug)]
pub enum InstanceError {
    Allocation(crate::backend::buffer::AllocError),
    Binding(crate::backend::buffer::BindingError),
    InvalidRange,
    BatchTooLarge,
    InstanceCountMismatch,
    MissingTexture,
    TextureKindMismatch,
    MissingAtlasRegion,
    InvalidTextureLayer,
    MissingMesh,
}
impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allocation(e) => write!(f, "Instance allocation failed: {e}"),
            Self::Binding(e) => write!(f, "Instance binding failed: {e}"),
            Self::InvalidRange => f.write_str("Instance range out of bounds"),
            Self::BatchTooLarge => f.write_str("Instance batch exceeds slot capacity"),
            Self::InstanceCountMismatch => {
                f.write_str("Instance iterator length does not match the range")
            }
            Self::MissingTexture => f.write_str("Unknown texture"),
            Self::TextureKindMismatch => f.write_str("Texture does not match sprite kind"),
            Self::MissingAtlasRegion => f.write_str("Unknown atlas region"),
            Self::InvalidTextureLayer => f.write_str("Invalid texture array layer"),
            Self::MissingMesh => f.write_str("Unknown mesh"),
        }
    }
}
impl std::error::Error for InstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Allocation(e) => Some(e),
            Self::Binding(e) => Some(e),
            _ => None,
        }
    }
}
impl From<crate::backend::buffer::AllocError> for InstanceError {
    fn from(e: crate::backend::buffer::AllocError) -> Self {
        Self::Allocation(e)
    }
}
impl From<crate::backend::buffer::BindingError> for InstanceError {
    fn from(e: crate::backend::buffer::BindingError) -> Self {
        Self::Binding(e)
    }
}

use new_collection::{Arena, Id};
use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub trait Instance: 'static {}

pub struct InstanceData<T: Instance> {
    pub data: Vec<T>,
}

impl<T: Instance> Default for InstanceData<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

impl<T: Instance> From<Vec<T>> for InstanceData<T> {
    fn from(data: Vec<T>) -> Self {
        Self { data }
    }
}

impl<T: Instance> Deref for InstanceData<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: Instance> DerefMut for InstanceData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
pub struct InstanceId<T: Instance> {
    raw: Id<Box<dyn Any>>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Instance> Copy for InstanceId<T> {}

impl<T: Instance> Clone for InstanceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Default)]
pub struct InstanceStorage {
    data: Arena<Box<dyn Any>>,
}

impl InstanceStorage {
    #[allow(
        dead_code,
        reason = "API du moteur conservee pour les autres scenes et modules."
    )]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Instance>(&mut self, data: InstanceData<T>) -> InstanceId<T> {
        InstanceId {
            raw: self.data.insert(Box::new(data)),
            _marker: PhantomData,
        }
    }

    /// IDs belong to the storage that created them.
    #[allow(
        dead_code,
        reason = "API du moteur conservee pour les autres scenes et modules."
    )]
    pub fn get<T: Instance>(&self, id: &InstanceId<T>) -> Option<&InstanceData<T>> {
        self.data.get(id.raw)?.downcast_ref()
    }

    pub fn get_mut<T: Instance>(&mut self, id: &InstanceId<T>) -> Option<&mut InstanceData<T>> {
        self.data.get_mut(id.raw)?.downcast_mut()
    }

    #[allow(
        dead_code,
        reason = "API du moteur conservee pour les autres scenes et modules."
    )]
    pub fn remove<T: Instance>(&mut self, id: &InstanceId<T>) -> Option<InstanceData<T>> {
        // Check before removing: Arena::remove must not be called twice on an empty slot.
        self.get(id)?;
        self.data.remove(id.raw)?.downcast().ok().map(|data| *data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position(u32);
    impl Instance for Position {}
    struct Label(String);
    impl Instance for Label {}

    #[test]
    fn multiple_sets_keep_their_concrete_type_and_independent_contents() {
        let mut storage = InstanceStorage::new();
        let first = storage.insert(vec![Position(1)].into());
        let second = storage.insert(vec![Position(2)].into());
        let text = storage.insert(vec![Label("label".into())].into());
        let copied = first;
        storage.get_mut(&copied).unwrap().data[0].0 = 9;
        assert_eq!(storage.get(&first).unwrap()[0].0, 9);
        assert_eq!(storage.get(&second).unwrap()[0].0, 2);
        assert_eq!(storage.get(&text).unwrap()[0].0, "label");
    }

    #[test]
    fn removed_ids_stay_invalid_when_slots_are_reused() {
        let mut storage = InstanceStorage::new();
        let old = storage.insert(vec![Position(1)].into());
        assert_eq!(storage.remove(&old).unwrap()[0].0, 1);
        assert!(storage.remove(&old).is_none());
        let new = storage.insert(vec![Position(2)].into());
        let other = storage.insert(vec![Position(3)].into());
        assert!(storage.get(&old).is_none());
        assert!(storage.get_mut(&old).is_none());
        assert!(storage.remove(&old).is_none());
        assert_eq!(storage.get(&new).unwrap()[0].0, 2);
        assert_eq!(storage.get(&other).unwrap()[0].0, 3);
    }
}
