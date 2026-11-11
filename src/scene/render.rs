use super::resources::*;
use crate::{
    backend::{self, Sprite, SpriteInstance, shape}, core::Resources, particle,
};

pub fn render(resources: &Resources, pass: &mut wgpu::RenderPass<'_>) {
    let scene = *resources.read_resource::<ScenePipeline>();

    match scene {
        ScenePipeline::Mesh => {
            let ctx = resources.read_resource::<MeshPipelineContext>();
            let mut instances = resources.write_resource::<InstanceData>();
            backend::bind(&*ctx, pass);
            backend::shape::draw_all(&mut instances, pass);
        }
        ScenePipeline::Texture => {
            let mut instances = resources.write_resource::<InstanceData>();
            draw_sprites(resources, pass, &mut instances);
        }
    }

    particle::for_each_prepared_effect(|instances| shape::draw_all(instances.shape(), pass));
    particle::for_each_prepared_effect(|instances| draw_sprites(resources, pass, instances.sprite()));
}

fn draw_sprites<T: SpriteInstance>(
    resources: &Resources,
    pass: &mut wgpu::RenderPass<'_>,
    instances: &mut [T],
) {
    let ctx = resources.read_resource::<TexturePipelineContext>();
    let array_ctx = resources.read_resource::<TextureArrayPipelineContext>();
    let end = instances.len() as u32;

    for batch in backend::batch(instances, 0..end) {
        match instances[batch.range.start as usize].sprite() {
            Sprite::Separate { .. } | Sprite::Atlas { .. } => backend::bind(&*ctx, pass),
            Sprite::Array { .. } | Sprite::Animated { .. } => backend::bind(&*array_ctx, pass),
        }
        backend::draw(instances, &batch, pass);
    }
}
