use super::*;
use crate::backend::{pass::ComputeOperation, target::RenderTarget};
use crate::effects::{EffectError, EffectInputDesc, EffectInputKind};

struct Noop;
impl ComputeOperation for Noop {
    fn execute(
        &mut self,
        _: &mut wgpu::ComputePass<'_>,
        _: &mut RenderData,
    ) -> Result<(), PassError> {
        Ok(())
    }
}
struct TestEffect {
    passes: PassList,
    output: Id<RenderTarget>,
}
impl TestEffect {
    fn new() -> Self {
        let mut passes = PassList::new();
        passes.insert(pass::compute(Noop, true));
        passes.insert(pass::compute(Noop, true));
        let mut ids = Arena::<(), RenderTarget>::new();
        Self {
            passes,
            output: ids.insert(()),
        }
    }
}
impl RenderEffect for TestEffect {
    fn inputs(&self) -> &[EffectInputDesc] {
        &[EffectInputDesc {
            name: "source",
            required: true,
            kind: EffectInputKind::Color,
        }]
    }
    fn set_input(&mut self, _: &str, _: Id<RenderTarget>) -> Result<(), EffectError> {
        Ok(())
    }
    fn prepare(&mut self) -> Result<(), EffectError> {
        Ok(())
    }
    fn output(&self) -> Id<RenderTarget> {
        self.output
    }
    fn bypass_input(&self) -> Option<&str> {
        Some("source")
    }
    fn passes(&self) -> &PassList {
        &self.passes
    }
    fn passes_mut(&mut self) -> &mut PassList {
        &mut self.passes
    }
}
#[test]
fn blocks_move_as_a_unit_and_cannot_be_duplicated() {
    let mut renderer = SceneRenderer::new();
    let local = renderer.add_pass(pass::compute(Noop, true));
    let effect = renderer.add_effect(TestEffect::new());
    let anchor = renderer.append_effect(effect).unwrap();
    assert!(renderer.append_effect(effect).is_err());
    renderer
        .insert_before(ScheduleAnchor::Local(local), anchor)
        .unwrap();
    assert!(matches!(
        renderer.schedule()[0],
        ScheduledPass::Effect { .. }
    ));
    assert!(matches!(
        renderer.schedule()[1],
        ScheduledPass::Effect { .. }
    ));
    assert!(matches!(renderer.schedule()[2], ScheduledPass::Local(_)));
    renderer.validate().unwrap();
    renderer.remove_effect(effect).unwrap();
    assert_eq!(renderer.schedule().len(), 1);
    assert!(renderer.effect::<TestEffect>(effect).is_err());
}
#[test]
fn graph_rejects_cycles_and_reversed_producers_transactionally() {
    let mut renderer = SceneRenderer::new();
    let a = renderer.add_effect(TestEffect::new());
    let b = renderer.add_effect(TestEffect::new());
    renderer
        .connect(b, "source", EffectSource::Output(a))
        .unwrap();
    assert!(
        renderer
            .connect(a, "source", EffectSource::Output(b))
            .is_err()
    );
    renderer.append_effect(a).unwrap();
    renderer.append_effect(b).unwrap();
    assert!(
        renderer
            .insert_before(ScheduleAnchor::Effect(a), ScheduleAnchor::Effect(b))
            .is_err()
    );
    renderer.validate().unwrap();
    assert!(
        renderer
            .connect(a, "typo", EffectSource::Output(b))
            .is_err()
    );
    renderer.remove_effect(a).unwrap();
    assert!(renderer.validate().is_err());
}
#[test]
fn changing_internal_passes_is_detected() {
    let mut renderer = SceneRenderer::new();
    let id = renderer.add_effect(TestEffect::new());
    renderer.append_effect(id).unwrap();
    renderer
        .effect_mut::<TestEffect>(id)
        .unwrap()
        .passes
        .insert(pass::compute(Noop, true));
    assert!(renderer.validate().is_err());
}
