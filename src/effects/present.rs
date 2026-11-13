use crate::backend::{
    self,
    binding_set::{self, TextureBinding, TextureBindingSetId},
    pass::PassError,
    pipeline::{self, PipelineId},
    target::{self, RenderTargetId},
};

crate::define_pipeline_context!(Present);
pub struct PresentEffect {
    pub pipeline: PipelineId<PresentPipelineContext>,
    pub source: TextureBindingSetId,
}
impl PresentEffect {
    pub fn new(source: RenderTargetId, format: wgpu::TextureFormat) -> Self {
        let sampler = backend::sampler::filtering("Scene sampler");
        let scene_target = target::get(source).expect("Cible de scène introuvable");

        let present_layout = crate::bind_group_layout! {
            wgpu::ShaderStages::FRAGMENT => scene_target.texture(),
            wgpu::ShaderStages::FRAGMENT => sampler,
        };

        let scene_bindings = binding_set::register(
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
        .expect("Impossible de créer les bindings de la scène");

        let present_shader =
            backend::load_shader(Some("Present"), "assets/shaders/present.wgsl").unwrap();

        let present_context = crate::pipeline_context! {
            Present {
                shader: &present_shader,
                format: format,
                fragment: "fs_main",
                blend: None,
                bind_group: [(present_layout,)],
                index_buffer: None,
            }
        };
        let present_pipeline = backend::pipeline::register(present_context);

        Self {
            pipeline: present_pipeline,
            source: scene_bindings,
        }
    }
}
crate::define_render_operation! {
    Present {
        pipeline: PipelineId<PresentPipelineContext>,
        source: TextureBindingSetId,
    }

    execute(this, pass, _frame) {
        let context = pipeline::get(this.pipeline)
            .ok_or(PassError::MissingPipeline)?;

        let group = binding_set::group(this.source)?;

        backend::bind(&*context, pass);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);

        Ok(())
    }
}
