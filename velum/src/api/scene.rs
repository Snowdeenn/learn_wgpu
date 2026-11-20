use std::any::Any;

use new_collection::{Arena, Id};

use super::scene_renderer::SceneRenderer;
pub struct RenderDataId<T> {
    raw: Id<Box<dyn Any>>,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Debug)]
pub struct RenderDataError {
    pub handle: String,
    pub expected_type: &'static str,
}

impl std::fmt::Display for RenderDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Missing or invalid render data {} (expected {})",
            self.handle, self.expected_type
        )
    }
}

impl std::error::Error for RenderDataError {}

impl<T> Copy for RenderDataId<T> {}

impl<T> Clone for RenderDataId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Default)]
pub struct RenderData {
    data: Arena<Box<dyn Any>>,
}

impl RenderData {
    pub fn new() -> Self {
        RenderData { data: Arena::new() }
    }

    pub fn insert<T: 'static>(&mut self, data: T) -> RenderDataId<T> {
        let raw = self.data.insert(Box::new(data));
        RenderDataId {
            raw,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get<T: 'static>(&self, id: RenderDataId<T>) -> Option<&T> {
        self.data.get(id.raw)?.downcast_ref()
    }

    pub fn get_mut<T: 'static>(&mut self, id: RenderDataId<T>) -> Option<&mut T> {
        self.data.get_mut(id.raw)?.downcast_mut()
    }

    pub fn remove<T: 'static>(&mut self, id: RenderDataId<T>) -> Option<T> {
        self.get(id)?;
        self.data.remove(id.raw)?.downcast().ok().map(|b| *b)
    }

    pub fn require<T: 'static>(&self, id: RenderDataId<T>) -> Result<&T, RenderDataError> {
        self.get(id).ok_or_else(|| RenderDataError {
            handle: id.raw.to_string(),
            expected_type: std::any::type_name::<T>(),
        })
    }

    pub fn require_mut<T: 'static>(
        &mut self,
        id: RenderDataId<T>,
    ) -> Result<&mut T, RenderDataError> {
        self.get_mut(id).ok_or_else(|| RenderDataError {
            handle: id.raw.to_string(),
            expected_type: std::any::type_name::<T>(),
        })
    }
}

pub struct Scene {
    pub renderer: SceneRenderer,
    pub data: RenderData,
}

pub type SceneId = Id<Scene>;
