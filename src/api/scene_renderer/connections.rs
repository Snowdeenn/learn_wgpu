use super::EffectId;
use crate::backend::target::RenderTargetId;
#[derive(Clone, Copy)]
pub enum EffectSource {
    Target(RenderTargetId),
    Output(EffectId),
}

use super::{SceneRenderer, ScheduleError};
use crate::backend::target;
impl SceneRenderer {
    pub fn connect(
        &mut self,
        id: EffectId,
        name: &str,
        source: EffectSource,
    ) -> Result<(), ScheduleError> {
        if !self
            .entry(id)?
            .effect
            .inputs()
            .iter()
            .any(|i| i.name == name)
        {
            return Err(ScheduleError(format!("Unknown effect input: {name}")));
        }
        match source {
            EffectSource::Output(producer) => {
                self.entry(producer)?;
            }
            EffectSource::Target(t) => {
                if target::get(t).is_none() {
                    return Err(ScheduleError("Missing input target".into()));
                }
            }
        }
        let old = self
            .effects
            .get_mut(id)
            .unwrap()
            .inputs
            .insert(name.into(), source);
        if let Err(e) = self
            .visit_dependencies(id, &mut Vec::new())
            .and_then(|_| self.validate_dependencies())
        {
            let inputs = &mut self.effects.get_mut(id).unwrap().inputs;
            if let Some(old) = old {
                inputs.insert(name.into(), old);
            } else {
                inputs.remove(name);
            }
            return Err(e);
        }
        Ok(())
    }
    pub fn resolve_source(&self, source: EffectSource) -> Result<RenderTargetId, ScheduleError> {
        self.resolve(source, &mut Vec::new())
    }
    fn resolve(
        &self,
        source: EffectSource,
        path: &mut Vec<EffectId>,
    ) -> Result<RenderTargetId, ScheduleError> {
        match source {
            EffectSource::Target(id) => {
                target::get(id).ok_or_else(|| ScheduleError("Missing input target".into()))?;
                Ok(id)
            }
            EffectSource::Output(id) => {
                if path.contains(&id) {
                    return Err(ScheduleError("Cyclic effect connections".into()));
                }
                path.push(id);
                let entry = self.entry(id)?;
                let result = if entry.active {
                    self.resolve(EffectSource::Target(entry.effect.output()), path)
                } else {
                    let name = entry
                        .effect
                        .bypass_input()
                        .ok_or_else(|| ScheduleError("Disabled effect has no bypass".into()))?;
                    let input = *entry
                        .inputs
                        .get(name)
                        .ok_or_else(|| ScheduleError("Missing bypass input".into()))?;
                    self.resolve(input, path)
                };
                path.pop();
                result
            }
        }
    }
}
