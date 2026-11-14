use crate::backend::{
    self,
    binding_set::{self, TextureBinding, TextureBindingSetId},
    pass::PassError,
    pipeline::{self, PipelineId},
    target::{self, RenderTargetId},
};

crate::define_pipeline_context!(Composition);
pub struct CompositeEffect {
    pub pipeline: PipelineId<CompositionPipelineContext>,
    pub source: TextureBindingSetId,
}
impl CompositeEffect {
    pub fn new(source: RenderTargetId, format: wgpu::TextureFormat) -> Self {
        let sampler = backend::sampler::filtering("Composition sampler");
        let source_target = target::get(source).unwrap();
        let present_layout = crate::bind_group_layout! {
            wgpu::ShaderStages::FRAGMENT => source_target.texture(),
            wgpu::ShaderStages::FRAGMENT => sampler,
        };
        let composition_shader = backend::load_shader(
            Some("Composition shader"),
            "assets/shaders/composition.wgsl",
        )
        .unwrap();

        let composition_ctx = crate::pipeline_context! {
            Composition {
                shader: &composition_shader,
                format: format,
                fragment: "fs_main",
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), //jsp
                bind_group: [
                    (present_layout,)
                ],
                index_buffer: None,
            }
        };
        let composition = backend::pipeline::register(composition_ctx);

        let composition_binding = binding_set::register(
            &present_layout,
            vec![
                TextureBinding::Texture {
                    binding: 0,
                    target: source,
                },
                TextureBinding::Sampler {
                    binding: 1,
                    sampler: sampler.as_raw().clone(),
                },
            ],
        )
        .unwrap();

        Self {
            pipeline: composition,
            source: composition_binding,
        }
    }
}
crate::define_render_operation! {
    Composition {
        pipeline: PipelineId<CompositionPipelineContext>,
        source: TextureBindingSetId
    }

    execute(this, pass, _frame) {
        let composition = pipeline::get(this.pipeline)
            .ok_or(PassError::MissingPipeline)?;

        let group = binding_set::group(this.source)?;

        backend::bind(&*composition, pass);
        pass.set_bind_group(0, &group, &[]);
        // Binder ici les paramètres de direction verticale.
        crate::backend::metrics::draw();
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
