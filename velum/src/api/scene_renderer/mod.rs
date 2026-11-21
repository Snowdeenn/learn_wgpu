mod connections;
mod schedule;
#[cfg(test)]
mod tests;
mod validation;

use super::scene::RenderData;
use crate::backend::{
    pass::{self, PassContext, PassError, PassList},
    renderer::Frame,
};
use crate::effects::RenderEffect;
pub use connections::EffectSource;
use new_collection::{Arena, Id};
pub use schedule::{PassId, ScheduleAnchor, ScheduleError, ScheduledPass};
use std::collections::HashMap;

pub struct EffectEntry {
    effect: Box<dyn RenderEffect>,
    active: bool,
    inputs: HashMap<String, EffectSource>,
}
pub type EffectId = Id<EffectEntry>;

pub struct SceneRenderer {
    passes: PassList,
    effects: Arena<EffectEntry>,
    schedule: Vec<ScheduledPass>,
}
impl Default for SceneRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl SceneRenderer {
    pub fn new() -> Self {
        Self {
            passes: PassList::new(),
            effects: Arena::new(),
            schedule: Vec::new(),
        }
    }
    pub fn schedule(&self) -> &[ScheduledPass] {
        &self.schedule
    }
    pub fn add_pass(&mut self, pass: PassContext) -> PassId {
        let id = self.passes.insert(pass);
        self.schedule.push(ScheduledPass::Local(id));
        id
    }
    pub fn add_effect<E: RenderEffect + 'static>(&mut self, effect: E) -> EffectId {
        self.effects.insert(EffectEntry {
            effect: Box::new(effect),
            active: true,
            inputs: HashMap::new(),
        })
    }
    pub fn effect<E: RenderEffect + 'static>(&self, id: EffectId) -> Result<&E, ScheduleError> {
        self.entry(id)?
            .effect
            .as_ref()
            .as_any()
            .downcast_ref()
            .ok_or_else(|| {
                ScheduleError(format!(
                    "Wrong effect type: expected {}",
                    std::any::type_name::<E>()
                ))
            })
    }
    pub fn effect_mut<E: RenderEffect + 'static>(
        &mut self,
        id: EffectId,
    ) -> Result<&mut E, ScheduleError> {
        self.effects
            .get_mut(id)
            .ok_or_else(|| ScheduleError("Missing effect".into()))?
            .effect
            .as_mut()
            .as_any_mut()
            .downcast_mut()
            .ok_or_else(|| {
                ScheduleError(format!(
                    "Wrong effect type: expected {}",
                    std::any::type_name::<E>()
                ))
            })
    }
    fn entry(&self, id: EffectId) -> Result<&EffectEntry, ScheduleError> {
        self.effects
            .get(id)
            .ok_or_else(|| ScheduleError("Missing effect".into()))
    }
    pub fn set_effect_active(&mut self, id: EffectId, active: bool) -> Result<(), ScheduleError> {
        let old = self.entry(id)?.active;
        self.effects.get_mut(id).unwrap().active = active;
        let check = self.validate_dependencies().and_then(|_| {
            for effect in self.effects.iter() {
                for source in effect.inputs.values() {
                    self.resolve_source(*source)?;
                }
            }
            Ok(())
        });
        if let Err(e) = check {
            self.effects.get_mut(id).unwrap().active = old;
            return Err(e);
        }
        Ok(())
    }
    pub fn remove_effect(&mut self, id: EffectId) -> Result<Box<dyn RenderEffect>, ScheduleError> {
        self.entry(id)?;
        self.schedule
            .retain(|p| !matches!(p,ScheduledPass::Effect{effect,..} if *effect==id));
        Ok(self.effects.remove(id).unwrap().effect)
    }
    pub fn execute(&mut self, frame: &mut Frame, data: &mut RenderData) -> Result<(), PassError> {
        self.execute_into(&mut frame.encoder, &frame.view, data)
    }

    pub fn execute_into(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data: &mut RenderData,
    ) -> Result<(), PassError> {
        self.prepare_effects().map_err(|error| {
            crate::backend::report::diagnostic("effect preparation", error.to_string());
            PassError::Schedule(error)
        })?;
        for scheduled in &self.schedule {
            let pass = match *scheduled {
                ScheduledPass::Local(id) => self.passes.get_mut(id).unwrap(),
                ScheduledPass::Effect { effect, pass } => {
                    let entry = self.effects.get_mut(effect).unwrap();
                    if !entry.active {
                        continue;
                    }
                    entry.effect.passes_mut().get_mut(pass).unwrap()
                }
            };
            pass::execute_one(pass, encoder, view, data)?;
        }
        Ok(())
    }
    pub fn activate_pass(&mut self, id: PassId) {
        self.set_pass_active(id, true);
    }
    pub fn deactivate_pass(&mut self, id: PassId) {
        self.set_pass_active(id, false);
    }
    fn set_pass_active(&mut self, id: PassId, value: bool) {
        if let Some(pass) = self.passes.get_mut(id) {
            match pass {
                PassContext::Render { active, .. } | PassContext::Compute { active, .. } => {
                    *active = value
                }
            }
        }
    }
}
pub fn renderer() -> SceneRendererBuilder {
    SceneRendererBuilder::new()
}
#[must_use]
pub struct SceneRendererBuilder {
    renderer: SceneRenderer,
}
impl Default for SceneRendererBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl SceneRendererBuilder {
    pub fn new() -> Self {
        Self {
            renderer: SceneRenderer::new(),
        }
    }
    pub fn add_pass(&mut self, pass: PassContext) -> PassId {
        self.renderer.add_pass(pass)
    }
    pub fn build(self) -> SceneRenderer {
        self.renderer
    }
}
