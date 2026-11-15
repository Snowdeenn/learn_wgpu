use new_collection::Arena;

use super::SceneError;
use super::scene::{Scene, SceneId};
use crate::backend::renderer::Frame;

#[derive(Default)]
pub struct Velum {
    scenes: Arena<Scene>,
    active: Option<SceneId>,
}

impl Velum {
    pub fn new() -> Self {
        Velum {
            scenes: Arena::new(),
            active: None,
        }
    }

    #[inline]
    pub fn add_scene(&mut self, scene: Scene) -> SceneId {
        self.scenes.insert(scene)
    }

    #[inline]
    pub fn activate(&mut self, scene: SceneId) -> Result<(), SceneError> {
        self.scenes
            .get(scene)
            .ok_or(SceneError::MissingScene(scene))?;
        self.active = Some(scene);
        Ok(())
    }

    pub fn active_scene(&self) -> Option<SceneId> {
        self.active
    }

    pub fn deactivate(&mut self) {
        self.active = None;
    }

    #[inline]
    pub fn scene_mut(&mut self, scene: SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(scene)
    }

    #[inline]
    pub fn scene(&self, scene: SceneId) -> Option<&Scene> {
        self.scenes.get(scene)
    }

    #[inline]
    pub fn remove_scene(&mut self, scene: SceneId) -> Option<Scene> {
        self.scenes.get(scene)?;
        if let Some(active) = self.active {
            if active == scene {
                self.active = None;
            }
        }
        self.scenes.remove(scene)
    }

    pub fn render(&mut self, frame: &mut Frame) -> Result<(), SceneError> {
        let Some(scene) = self.active else {
            return Err(SceneError::NoActiveScene);
        };

        let Scene { renderer, data } = self
            .scene_mut(scene)
            .ok_or(SceneError::MissingScene(scene))?;

        if let Err(e) = renderer.execute(frame, data) {
            return Err(SceneError::Pass(e));
        };

        Ok(())
    }
}
