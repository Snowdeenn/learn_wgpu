//! Example effect: composite a premultiplied color image over transparent black.
use velum::OwnedResource;
use velum::backend::{
    pass::{self, PassList},
    target::{self, RenderTargetDescriptor, RenderTargetId, TargetSize},
};
use velum::effects::{
    EffectError, EffectInputDesc, EffectInputKind, RenderEffect,
    composite::{CompositeEffect, CompositionOperation},
};

pub struct OverlayEffect {
    output: OwnedResource<RenderTargetId>,
    composition: CompositeEffect,
    passes: PassList,
    input: Option<RenderTargetId>,
}

impl OverlayEffect {
    pub const SOURCE: &'static str = "source";
    const INPUTS: [EffectInputDesc; 1] = [EffectInputDesc {
        name: Self::SOURCE,
        required: true,
        kind: EffectInputKind::Color,
    }];

    /// Call only after initializing Velum's default GPU context.
    pub fn new(format: wgpu::TextureFormat, resolution: [u32; 2]) -> Self {
        let output = target::register(None, RenderTargetDescriptor {
            label: "Custom overlay output",
            size: TargetSize::Window {
                width: resolution[0],
                height: resolution[1],
            },
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            sample_count: 1,
        });
        // Temporary binding. prepare() must reconnect it before this pass executes.
        let composition = CompositeEffect::new(output.id(), format);
        let mut passes = PassList::new();
        passes.insert(pass::render(
            output.id(),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            CompositionOperation {
                pipeline: composition.pipeline.id(),
                source: composition.source.id(),
            },
            true,
        ));
        Self {
            output,
            composition,
            passes,
            input: None,
        }
    }

    fn validate_source(&self, id: RenderTargetId) -> Result<(), EffectError> {
        let incompatible = |reason: &str| EffectError::IncompatibleInput {
            name: Self::SOURCE.into(),
            reason: reason.into(),
        };
        if id == self.output.id() {
            return Err(incompatible("Cannot read and write the same target"));
        }
        let source = target::get(id).ok_or(EffectError::MissingTarget(id))?;
        let texture = source.texture().as_raw();
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.depth_or_array_layers() != 1
            || texture.sample_count() != 1
            || !texture
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            || texture.format().sample_type(None, None)
                != Some(wgpu::TextureSampleType::Float { filterable: true })
        {
            return Err(incompatible(
                "Expected a filterable single-sample 2D color texture",
            ));
        }
        Ok(())
    }
}

impl RenderEffect for OverlayEffect {
    fn inputs(&self) -> &[EffectInputDesc] {
        &Self::INPUTS
    }
    fn set_input(&mut self, name: &str, target: RenderTargetId) -> Result<(), EffectError> {
        if name != Self::SOURCE {
            return Err(EffectError::UnknownInput(name.into()));
        }
        self.validate_source(target)?;
        self.input = Some(target);
        Ok(())
    }
    fn prepare(&mut self) -> Result<(), EffectError> {
        let source = self
            .input
            .ok_or_else(|| EffectError::MissingInput(Self::SOURCE.into()))?;
        self.validate_source(source)?;
        self.composition
            .set_source(source)
            .map_err(|error| EffectError::IncompatibleInput {
                name: Self::SOURCE.into(),
                reason: error.to_string(),
            })
    }
    fn output(&self) -> RenderTargetId {
        self.output.id()
    }
    fn bypass_input(&self) -> Option<&str> {
        Some(Self::SOURCE)
    }
    fn passes(&self) -> &PassList {
        &self.passes
    }
    fn passes_mut(&mut self) -> &mut PassList {
        &mut self.passes
    }
}
