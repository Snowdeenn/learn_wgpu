use std::any::Any;

use new_collection::{Arena, Id};

use crate::backend::{
    self,
    pass::{PassContext, PassError, PassList},
    renderer::Frame,
};

pub struct SceneRenderer {
    passes: PassList,
}

impl SceneRenderer {
    pub fn new() -> Self {
        SceneRenderer {
            passes: PassList::new(),
        }
    }

    pub fn add_pass(mut self, pass: PassContext) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn execute(&mut self, frame: &mut Frame, data: &mut RenderData) -> Result<(), PassError> {
        backend::pass::execute(&mut self.passes, &mut frame.encoder, &frame.view, data)
    }
}

pub struct RenderDataId<T> {
    raw: Id<Box<dyn Any>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Copy for RenderDataId<T> {}

impl<T> Clone for RenderDataId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

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
        self.data.remove(id.raw)?.downcast().ok().map(|b| *b)
    }
}
