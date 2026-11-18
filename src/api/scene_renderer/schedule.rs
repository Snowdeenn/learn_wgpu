use super::EffectId;
use crate::backend::pass::PassContext;
use new_collection::Id;
use std::fmt;

pub type PassId = Id<PassContext>;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScheduledPass {
    Local(PassId),
    Effect { effect: EffectId, pass: PassId },
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScheduleAnchor {
    Local(PassId),
    Effect(EffectId),
}
#[derive(Debug)]
pub struct ScheduleError(pub String);
impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ScheduleError {}

use super::SceneRenderer;
impl SceneRenderer {
    pub fn append_effect(&mut self, id: EffectId) -> Result<ScheduleAnchor, ScheduleError> {
        let passes = self.entry(id)?.effect.passes().ids().to_vec();
        if self
            .schedule
            .iter()
            .any(|p| matches!(p, ScheduledPass::Effect {effect,..} if *effect == id))
        {
            return Err(ScheduleError("Effect already scheduled".into()));
        }
        if passes.is_empty() {
            return Err(ScheduleError("Effect has no passes".into()));
        }
        let start = self.schedule.len();
        self.schedule.extend(
            passes
                .into_iter()
                .map(|pass| ScheduledPass::Effect { effect: id, pass }),
        );
        if let Err(error) = self.validate() {
            self.schedule.truncate(start);
            return Err(error);
        }
        Ok(ScheduleAnchor::Effect(id))
    }
    fn anchor_matches(pass: &ScheduledPass, anchor: ScheduleAnchor) -> bool {
        match (pass, anchor) {
            (ScheduledPass::Local(a), ScheduleAnchor::Local(b)) => *a == b,
            (ScheduledPass::Effect { effect, .. }, ScheduleAnchor::Effect(b)) => *effect == b,
            _ => false,
        }
    }
    /// Moves a complete existing block; effects cannot be split by insertion.
    pub fn insert_before(
        &mut self,
        anchor: ScheduleAnchor,
        item: ScheduleAnchor,
    ) -> Result<(), ScheduleError> {
        self.move_block(anchor, item, false)
    }
    pub fn insert_after(
        &mut self,
        anchor: ScheduleAnchor,
        item: ScheduleAnchor,
    ) -> Result<(), ScheduleError> {
        self.move_block(anchor, item, true)
    }
    fn move_block(
        &mut self,
        anchor: ScheduleAnchor,
        item: ScheduleAnchor,
        after: bool,
    ) -> Result<(), ScheduleError> {
        if anchor == item {
            return Err(ScheduleError(
                "Cannot move a block relative to itself".into(),
            ));
        }
        let old = self.schedule.clone();
        if !old.iter().any(|p| Self::anchor_matches(p, anchor))
            || !old.iter().any(|p| Self::anchor_matches(p, item))
        {
            return Err(ScheduleError("Missing schedule anchor".into()));
        }
        let moving: Vec<_> = old
            .iter()
            .copied()
            .filter(|p| Self::anchor_matches(p, item))
            .collect();
        self.schedule.retain(|p| !Self::anchor_matches(p, item));
        let index = if after {
            self.schedule
                .iter()
                .rposition(|p| Self::anchor_matches(p, anchor))
                .unwrap()
                + 1
        } else {
            self.schedule
                .iter()
                .position(|p| Self::anchor_matches(p, anchor))
                .unwrap()
        };
        self.schedule.splice(index..index, moving);
        if let Err(e) = self.validate() {
            self.schedule = old;
            return Err(e);
        }
        Ok(())
    }
}
