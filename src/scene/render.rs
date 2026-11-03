use super::resources::*;
use crate::{backend, core::Resources};

pub fn render(resources: &Resources, pass: &mut wgpu::RenderPass<'_>) {
    let index_count = resources.read_resource::<IndexCount>();
    let scene = *resources.read_resource::<ScenePipeline>();

    match scene {
        ScenePipeline::Mesh => {
            let ctx = resources.read_resource::<MeshPipelineContext>();
            backend::bind(&*ctx, pass);
            pass.draw_indexed(0..**index_count, 0, 0..1);
        }
        ScenePipeline::Texture => {
            let ctx = resources.read_resource::<TexturePipelineContext>();
            backend::bind(&*ctx, pass);

            let mut instances = resources.write_resource::<InstanceData>();
            let count = 0..instances.len() as u32;
            backend::draw_all(&mut *instances, count, pass);
        }
    }
}
