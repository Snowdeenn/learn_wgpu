use super::{EffectError, EffectInputDesc, EffectInputKind, RenderEffect};
use crate::backend::{
    self,
    binding_set::{self, TextureBinding, TextureBindingSetId},
    pass::{self, PassError, PassList},
    pipeline::{self, PipelineId},
    target::{self, RenderTargetId},
};
use crate::backend::{
    buffer::{self, Buffer, Uniform},
    target::{RenderTargetDescriptor, TargetSize},
};
use crate::core::OwnedResource;
crate::define_pipeline_context!(Blur);
pub const MAX_BLUR_RADIUS: usize = 16;

#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
pub(crate) use gpu_tests::gpu_regression;

#[derive(Clone, Copy, PartialEq)]
pub struct BlurSettings {
    pub spacing: f32,
    pub radius: u32,
    pub sigma: f32,
    pub manual: bool,
    pub normalize: bool,
    pub weights: [f32; MAX_BLUR_RADIUS + 1],
}

impl Default for BlurSettings {
    fn default() -> Self {
        let mut settings = Self {
            spacing: 1.0,
            radius: 2,
            sigma: 1.0,
            manual: false,
            normalize: true,
            weights: [0.0; MAX_BLUR_RADIUS + 1],
        };
        settings.generate_gaussian();
        settings
    }
}

impl BlurSettings {
    pub fn generate_gaussian(&mut self) {
        let sigma = self.sigma.max(0.05);
        for (distance, weight) in self.weights.iter_mut().enumerate() {
            *weight = (-(distance as f32).powi(2) / (2.0 * sigma * sigma)).exp();
        }
    }

    pub fn params(&self, direction: [f32; 2]) -> BlurParams {
        let radius = (self.radius as usize).min(MAX_BLUR_RADIUS);
        let mut weights = [[0.0; 4]; MAX_BLUR_RADIUS + 1];
        for (distance, weight) in weights.iter_mut().enumerate().take(radius + 1) {
            weight[0] = if self.manual {
                self.weights[distance].max(0.0)
            } else {
                (-(distance as f32).powi(2) / (2.0 * self.sigma.max(0.05).powi(2))).exp()
            };
        }
        let sum = weights[0][0] + 2.0 * weights[1..=radius].iter().map(|w| w[0]).sum::<f32>();
        if sum <= f32::EPSILON {
            weights[0][0] = 1.0;
        } else if self.normalize {
            for weight in &mut weights {
                weight[0] /= sum;
            }
        }
        BlurParams {
            direction,
            spacing: self.spacing,
            radius: radius as u32,
            weights,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurParams {
    pub direction: [f32; 2],
    pub spacing: f32,
    pub radius: u32,
    // vec4 stride keeps the uniform array compatible with WGSL alignment.
    pub weights: [[f32; 4]; MAX_BLUR_RADIUS + 1],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_kernel_preserves_constant_colors_for_every_radius() {
        for radius in 0..=MAX_BLUR_RADIUS as u32 {
            let settings = BlurSettings {
                radius,
                ..Default::default()
            };
            let params = settings.params([1.0, 0.0]);
            let sum = params.weights[0][0]
                + 2.0
                    * params.weights[1..=radius as usize]
                        .iter()
                        .map(|w| w[0])
                        .sum::<f32>();
            assert!((sum - 1.0).abs() < 0.00001);
        }
    }

    #[test]
    fn zero_manual_weights_fall_back_to_identity() {
        let settings = BlurSettings {
            manual: true,
            weights: [0.0; MAX_BLUR_RADIUS + 1],
            ..Default::default()
        };
        let params = settings.params([0.0, 1.0]);
        assert_eq!(params.weights[0][0], 1.0);
        assert!(params.weights[1..].iter().all(|w| w[0] == 0.0));
    }
}

pub struct BlurEffect {
    pub target_a: OwnedResource<RenderTargetId>,
    pub target_b: OwnedResource<RenderTargetId>,
    pub pipeline: OwnedResource<PipelineId<BlurPipelineContext>>,
    pub source_a: OwnedResource<TextureBindingSetId>,
    pub source_b: OwnedResource<TextureBindingSetId>,
    pub vertical_group: wgpu::BindGroup,
    pub horizontal_group: wgpu::BindGroup,
    pub settings: BlurSettings,
    prev_settings: BlurSettings,
    vertical_params: OwnedResource<Buffer<BlurParams, Uniform>>,
    horizontal_params: OwnedResource<Buffer<BlurParams, Uniform>>,
    passes: PassList,
    input: Option<RenderTargetId>,
    input_dirty: bool,
    sampler: wgpu::Sampler,
}
impl BlurEffect {
    pub const SOURCE: &'static str = "source";
    const INPUTS: [EffectInputDesc; 1] = [EffectInputDesc {
        name: Self::SOURCE,
        required: true,
        kind: EffectInputKind::Color,
    }];
    pub fn new(format: wgpu::TextureFormat, resolution: [u32; 2]) -> Self {
        let blur_target_a = target::register(None, RenderTargetDescriptor {
            label: "Blur A",
            size: TargetSize::Window {
                width: resolution[0],
                height: resolution[1],
            },
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            sample_count: 1, // jsp combien et ce que ça modifie
        });

        let blur_target_b = target::register(None, RenderTargetDescriptor {
            label: "Blur B",
            size: TargetSize::Window {
                width: resolution[0],
                height: resolution[1],
            },
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            sample_count: 1, // jsp combien et ce que ça modifie
        });

        let blur_sampler = backend::sampler::filtering(None, "Blur sampler");

        let blur_a = target::get(blur_target_a.id()).unwrap();

        let blur_layout = crate::bind_group_layout! {
            wgpu::ShaderStages::FRAGMENT => blur_a.texture(),
            wgpu::ShaderStages::FRAGMENT => blur_sampler,
        };

        let vertical_unif = buffer::uniform::<BlurParams>(None, 1).unwrap();
        let horizontal_unif = buffer::uniform::<BlurParams>(None, 1).unwrap();

        let blur_settings = BlurSettings::default();
        backend::write_buffer(&horizontal_unif, 0, &[blur_settings.params([1.0, 0.0])]);
        backend::write_buffer(&vertical_unif, 0, &[blur_settings.params([0.0, 1.0])]);

        let (vertical_layout, vertical_group) = crate::bind_group! {
            wgpu::ShaderStages::VERTEX_FRAGMENT => vertical_unif
        };

        let (_, horizontal_group) = crate::bind_group! {
            wgpu::ShaderStages::VERTEX_FRAGMENT => horizontal_unif
        };

        let blur_shader = backend::try_shader(
            None,
            "Blur shader",
            include_str!("../../assets/shaders/blur.wgsl"),
        )
        .unwrap();

        let blur_pipeline_ctx = crate::pipeline_context! {
            Blur {
                shader: &blur_shader,
                format: format,
                fragment: "fs_main",
                blend: None,
                bind_group: [
                    (blur_layout,),
                    (vertical_layout,),
                ],
                index_buffer: None,
            }
        };
        let blur_pipeline = backend::pipeline::register(blur_pipeline_ctx);

        let blur_binding_a = binding_set::register(
            None,
            &blur_layout,
            vec![
                TextureBinding::Texture {
                    binding: 0,
                    target: blur_target_a.id(),
                },
                TextureBinding::Sampler {
                    binding: 1,
                    sampler: blur_sampler.as_raw().clone(),
                },
            ],
        )
        .unwrap();

        let blur_binding_b = binding_set::register(
            None,
            &blur_layout,
            vec![
                TextureBinding::Texture {
                    binding: 0,
                    target: blur_target_b.id(),
                },
                TextureBinding::Sampler {
                    binding: 1,
                    sampler: blur_sampler.as_raw().clone(),
                },
            ],
        )
        .unwrap();

        let mut passes = PassList::new();
        passes.insert(pass::render(
            blur_target_b.id(),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            BlurHorizontalOperation {
                pipeline: blur_pipeline.id(),
                source: blur_binding_a.id(),
                direction: horizontal_group.clone(),
            },
            true,
        ));
        passes.insert(pass::render(
            blur_target_a.id(),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            BlurVerticalOperation {
                pipeline: blur_pipeline.id(),
                source: blur_binding_b.id(),
                direction: vertical_group.clone(),
            },
            true,
        ));

        Self {
            target_a: blur_target_a,
            target_b: blur_target_b,
            pipeline: blur_pipeline,
            source_a: blur_binding_a,
            source_b: blur_binding_b,
            vertical_group,
            horizontal_group,
            settings: blur_settings,
            prev_settings: blur_settings,
            vertical_params: vertical_unif,
            horizontal_params: horizontal_unif,
            passes,
            input: None,
            input_dirty: false,
            sampler: blur_sampler.as_raw().clone(),
        }
    }
    fn prepare_settings(&mut self) {
        if self.settings == self.prev_settings {
            return;
        }

        let vertical = self.settings.params([0.0, 1.0]);
        let horizontal = BlurParams {
            direction: [1.0, 0.0],
            ..vertical
        };

        self.prev_settings = self.settings;

        backend::write_buffer(&self.vertical_params, 0, &[vertical]);
        backend::write_buffer(&self.horizontal_params, 0, &[horizontal]);
    }
}

impl RenderEffect for BlurEffect {
    fn inputs(&self) -> &[EffectInputDesc] {
        &Self::INPUTS
    }

    fn set_input(&mut self, name: &str, target: RenderTargetId) -> Result<(), EffectError> {
        if name != Self::SOURCE {
            return Err(EffectError::UnknownInput(name.to_owned()));
        }
        self.validate_input(target)?;
        if self.input != Some(target) {
            self.input = Some(target);
            self.input_dirty = true;
        }
        Ok(())
    }

    fn prepare(&mut self) -> Result<(), EffectError> {
        let input = self
            .input
            .ok_or_else(|| EffectError::MissingInput(Self::SOURCE.to_owned()))?;
        self.validate_input(input)?;
        if self.input_dirty {
            binding_set::update(
                self.source_a.id(),
                vec![
                    TextureBinding::Texture {
                        binding: 0,
                        target: input,
                    },
                    TextureBinding::Sampler {
                        binding: 1,
                        sampler: self.sampler.clone(),
                    },
                ],
            )
            .map_err(|error| EffectError::IncompatibleInput {
                name: Self::SOURCE.to_owned(),
                reason: error.to_string(),
            })?;
            self.input_dirty = false;
        }
        self.prepare_settings();
        Ok(())
    }

    fn output(&self) -> RenderTargetId {
        self.target_a.id()
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

impl BlurEffect {
    fn validate_input(&self, input: RenderTargetId) -> Result<(), EffectError> {
        let incompatible = |reason: &str| EffectError::IncompatibleInput {
            name: Self::SOURCE.to_owned(),
            reason: reason.to_owned(),
        };
        if input == self.target_a.id() || input == self.target_b.id() {
            return Err(incompatible(
                "Source must be distinct from both private blur targets",
            ));
        }
        let source = target::get(input).ok_or(EffectError::MissingTarget(input))?;
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
                "Source must be a filterable, single-sample 2D color texture with TEXTURE_BINDING usage",
            ));
        }
        for id in [self.target_a.id(), self.target_b.id()] {
            let output = target::get(id).ok_or(EffectError::MissingTarget(id))?;
            if source.size() != output.size() {
                return Err(incompatible(
                    "Source and blur targets must have matching dimensions",
                ));
            }
        }
        Ok(())
    }
}
crate::define_render_operation! {
    BlurVertical {
        pipeline: PipelineId<BlurPipelineContext>,
        source: TextureBindingSetId,
        direction: wgpu::BindGroup,
    }

    execute(this, pass, _frame) {
        let blur = pipeline::get(this.pipeline)
            .ok_or(PassError::MissingPipeline)?;

        let group = binding_set::group(this.source)?;

        backend::bind(&*blur, pass);
        pass.set_bind_group(0, &group, &[]);
        pass.set_bind_group(1, &this.direction, &[]);
        crate::backend::metrics::draw();
        crate::backend::report::record_instances(1);
        pass.draw(0..3, 0..1);

        Ok(())
    }
}

crate::define_render_operation! {
    BlurHorizontal {
        pipeline: PipelineId<BlurPipelineContext>,
        source: TextureBindingSetId,
        direction: wgpu::BindGroup,
    }

    execute(this, pass, _frame) {
        let blur = pipeline::get(this.pipeline)
            .ok_or(PassError::MissingPipeline)?;

        let group = binding_set::group(this.source)?;

        backend::bind(&*blur, pass);
        pass.set_bind_group(0, &group, &[]);
        pass.set_bind_group(1, &this.direction, &[]);
        crate::backend::metrics::draw();
        crate::backend::report::record_instances(1);
        pass.draw(0..3, 0..1);

        Ok(())
    }
}
