use super::resources::*;
use crate::{backend::{self, Sprite}, core::Resources};

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
            backend::bind(&*ctx, pass);

            let mut instances = resources.write_resource::<InstanceData>();
            let count = 0..instances.len() as u32;
            for i in count.clone() {
                if matches!(instances[i as usize].sprite, Sprite::Separate { .. }) {
                    backend::draw_all(&mut *instances, i..i + 1, pass);
                }
                if matches!(instances[i as usize].sprite, Sprite::Atlas { .. }) {
                    backend::draw_all(&mut instances, i..i + 1, pass);
                }
            }

            let array_ctx = resources.read_resource::<TextureArrayPipelineContext>();
            backend::bind(&*array_ctx, pass);
            for i in count {
                if matches!(instances[i as usize].sprite, Sprite::Array { .. }) {
                    backend::draw_all(&mut *instances, i..i + 1, pass);
                }
                if matches!(instances[i as usize].sprite, Sprite::Animated { .. }) {
                    backend::draw_all(&mut instances, i..i + 1, pass);
                }
            }
        }
    }
}
