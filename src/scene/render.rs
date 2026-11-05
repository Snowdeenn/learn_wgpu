use super::resources::*;
use crate::{
    backend::{self, Sprite},
    core::Resources,
};

pub fn render(resources: &Resources, pass: &mut wgpu::RenderPass<'_>) {
    let index_count = resources.read_resource::<IndexCount>();
    let scene = *resources.read_resource::<ScenePipeline>();

    match scene {
        ScenePipeline::Mesh => {
            let ctx = resources.read_resource::<MeshPipelineContext>();
            let instances = resources.read_resource::<InstanceData>();
            let count = 0..instances.len() as u32;
            backend::bind(&*ctx, pass);
            pass.draw_indexed(0..**index_count, 0, count);
        }
        ScenePipeline::Texture => {
            let ctx = resources.read_resource::<TexturePipelineContext>();
            let array_ctx = resources.read_resource::<TextureArrayPipelineContext>();
            let mut instances = resources.write_resource::<InstanceData>();
            let end = instances.len() as u32;
            let batches = backend::batch(&mut instances, 0..end);

            for batch in batches {
                match &instances[batch.range.start as usize].sprite {
                    Sprite::Separate { .. } | Sprite::Atlas { .. } => {
                        backend::bind(&*ctx, pass);
                    }
                    Sprite::Array { .. } | Sprite::Animated { .. } => {
                        backend::bind(&*array_ctx, pass);
                    }
                }
                backend::draw(&mut instances, &batch, pass);
            }
        }
    }
}
