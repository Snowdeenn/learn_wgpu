use super::{EffectId, EffectSource, SceneRenderer, ScheduleError, ScheduledPass};
use crate::backend::{pass::PassContext, target};
use crate::effects::EffectInputKind;

impl SceneRenderer {
    pub fn validate(&self) -> Result<(), ScheduleError> {
        let mut visited = Vec::new();
        let mut index = 0;
        while index < self.schedule.len() {
            match self.schedule[index] {
                ScheduledPass::Local(id) => {
                    if self.passes.get(id).is_none() {
                        return Err(ScheduleError("Missing local pass".into()));
                    }
                    if self.schedule[..index].contains(&ScheduledPass::Local(id)) {
                        return Err(ScheduleError("Duplicate local pass".into()));
                    }
                    index += 1;
                }
                ScheduledPass::Effect { effect, .. } => {
                    if visited.contains(&effect) {
                        return Err(ScheduleError("Effect block is duplicated or split".into()));
                    }
                    let ids = self.entry(effect)?.effect.passes().ids();
                    if ids.is_empty() {
                        return Err(ScheduleError("Effect has no passes".into()));
                    }
                    for (offset, pass) in ids.iter().enumerate() {
                        if self.schedule.get(index + offset)
                            != Some(&ScheduledPass::Effect {
                                effect,
                                pass: *pass,
                            })
                        {
                            return Err(ScheduleError(
                                "Effect pass order changed after registration".into(),
                            ));
                        }
                        if self.entry(effect)?.effect.passes().get(*pass).is_none() {
                            return Err(ScheduleError("Missing effect pass".into()));
                        }
                    }
                    visited.push(effect);
                    index += ids.len();
                }
            }
        }
        self.validate_dependencies()
    }

    pub(super) fn validate_dependencies(&self) -> Result<(), ScheduleError> {
        for id in self.scheduled_effects() {
            self.visit_dependencies(id, &mut Vec::new())?;
        }
        Ok(())
    }

    pub(super) fn visit_dependencies(
        &self,
        id: EffectId,
        path: &mut Vec<EffectId>,
    ) -> Result<(), ScheduleError> {
        if path.contains(&id) {
            return Err(ScheduleError("Cyclic effect connections".into()));
        }
        path.push(id);
        let entry = self.entry(id)?;
        for source in entry.inputs.values() {
            if let EffectSource::Output(producer) = source {
                self.visit_dependencies(*producer, path)?;
                let producer_start = self
                    .schedule
                    .iter()
                    .position(|p| matches!(p,ScheduledPass::Effect{effect,..} if effect==producer));
                let consumer_start = self
                    .schedule
                    .iter()
                    .position(|p| matches!(p,ScheduledPass::Effect{effect,..} if *effect==id));
                if consumer_start.is_some()
                    && self.entry(*producer)?.active
                    && (producer_start.is_none() || producer_start >= consumer_start)
                {
                    return Err(ScheduleError(
                        "Producer must be scheduled before its consumer".into(),
                    ));
                }
                // Target existence and bypass resolution are checked before encoding.
            }
        }
        path.pop();
        Ok(())
    }

    pub(super) fn scheduled_effects(&self) -> Vec<EffectId> {
        let mut ids = Vec::new();
        for item in &self.schedule {
            if let ScheduledPass::Effect { effect, .. } = item {
                if !ids.contains(effect) {
                    ids.push(*effect);
                }
            }
        }
        ids
    }

    pub fn prepare_effects(&mut self) -> Result<(), ScheduleError> {
        self.validate()?;
        for id in self.scheduled_effects() {
            if !self.entry(id)?.active {
                continue;
            }
            let mut resolved = Vec::new();
            for desc in self.entry(id)?.effect.inputs() {
                let source = self.entry(id)?.inputs.get(desc.name).copied();
                let Some(source) = source else {
                    if desc.required {
                        return Err(ScheduleError(format!(
                            "Missing required input: {}",
                            desc.name
                        )));
                    }
                    continue;
                };
                let target_id = self.resolve_source(source)?;
                let target = target::get(target_id)
                    .ok_or_else(|| ScheduleError("Missing input target".into()))?;
                let texture = target.texture().as_raw();
                let is_depth = texture.format().is_depth_stencil_format();
                if !texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
                    || is_depth != (desc.kind == EffectInputKind::Depth)
                {
                    return Err(ScheduleError(format!("Incompatible input: {}", desc.name)));
                }
                resolved.push((desc.name.to_owned(), target_id));
            }
            let entry = self.effects.get_mut(id).unwrap();
            for (name, target) in &resolved {
                entry
                    .effect
                    .set_input(name, *target)
                    .map_err(|e| ScheduleError(e.to_string()))?;
            }
            entry
                .effect
                .prepare()
                .map_err(|e| ScheduleError(e.to_string()))?;
            for pass_id in entry.effect.passes().ids() {
                if let Some(PassContext::Render {
                    target: crate::backend::target::PassTarget::Offscreen(output),
                    ..
                }) = entry.effect.passes().get(*pass_id)
                {
                    if resolved.iter().any(|(_, input)| input == output) {
                        return Err(ScheduleError(
                            "Effect writes to one of its input targets".into(),
                        ));
                    }
                }
            }
            let output = target::get(entry.effect.output())
                .ok_or_else(|| ScheduleError("Missing effect output".into()))?;
            if !output
                .texture()
                .as_raw()
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(ScheduleError(
                    "Effect output is not a render attachment".into(),
                ));
            }
        }
        // An implementation must not change the registered pass structure in prepare.
        self.validate()
    }
}
