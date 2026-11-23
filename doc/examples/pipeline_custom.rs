//! Fullscreen color operation with an inline shader and a mutable uniform.
use velum::backend::{
    buffer::{self, Buffer, Uniform},
    context,
    pass::{self, PassError, PassTarget, RenderOperation},
    pipeline::{self, PipelineId},
};
use velum::{OwnedResource, RenderData, Scene, SceneRenderer};

velum::define_pipeline_context!(Solid);

pub struct SolidResources {
    pub pipeline: OwnedResource<PipelineId<SolidPipelineContext>>,
    pub color: OwnedResource<Buffer<[f32; 4], Uniform>>,
}

pub fn create_resources(
    format: wgpu::TextureFormat,
) -> Result<SolidResources, Box<dyn std::error::Error>> {
    let color = buffer::uniform::<[f32; 4]>(None, 1)?;
    buffer::write_buffer(&color, 0, &[[0.1, 0.4, 0.8, 1.0]]);
    let (layout, group) = velum::try_bind_group! {
        wgpu::ShaderStages::FRAGMENT => color
    }?;
    let shader = context::device(None).create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Documentation inline color"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
            @group(0) @binding(0) var<uniform> color: vec4f;
            @vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
                let positions = array<vec2f, 3>(vec2f(-1.,-1.), vec2f(3.,-1.), vec2f(-1.,3.));
                return vec4f(positions[i], 0., 1.);
            }
            @fragment fn fs_main() -> @location(0) vec4f { return color; }
        "#
            .into(),
        ),
    });
    let pipeline = pipeline::register(velum::pipeline_context! {
        Solid {
            shader: &shader,
            format: format,
            fragment: "fs_main",
            blend: None,
            bind_group: [(layout, group)],
            index_buffer: None,
        }
    });
    Ok(SolidResources { pipeline, color })
}

pub struct SolidOperation {
    pub pipeline: PipelineId<SolidPipelineContext>,
}
impl RenderOperation for SolidOperation {
    fn execute(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        _: &mut RenderData,
    ) -> Result<(), PassError> {
        let pipeline = pipeline::get(self.pipeline).ok_or(PassError::MissingPipeline)?;
        pipeline::bind(&*pipeline, pass);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

pub fn build_scene(format: wgpu::TextureFormat) -> Result<Scene, Box<dyn std::error::Error>> {
    let resources = create_resources(format)?;
    let mut renderer = SceneRenderer::new();
    renderer.add_pass(pass::render(
        PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        SolidOperation {
            pipeline: resources.pipeline.id(),
        },
        true,
    ));
    let mut data = RenderData::new();
    data.insert(resources);
    Ok(Scene { renderer, data })
}
