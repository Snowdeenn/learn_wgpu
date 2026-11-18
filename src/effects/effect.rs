use crate::backend::{pass::PassList, target::RenderTargetId};

use super::{EffectError, EffectInputDesc};

/// Effect-owned passes and resources, connected and scheduled by SceneRenderer.
pub trait RenderEffect: EffectAny {
    fn inputs(&self) -> &[EffectInputDesc];

    fn set_input(&mut self, name: &str, target: RenderTargetId) -> Result<(), EffectError>;

    /// Validates inputs and updates bindings/parameters before pass encoding.
    /// Does not execute passes or submit GPU commands.
    fn prepare(&mut self) -> Result<(), EffectError>;

    fn output(&self) -> RenderTargetId;
    /// Input used as the effective output when this effect is disabled.
    fn bypass_input(&self) -> Option<&str>;

    fn passes(&self) -> &PassList;
    fn passes_mut(&mut self) -> &mut PassList;
}

pub trait EffectAny {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
impl<T: 'static> EffectAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
