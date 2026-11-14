use new_collection::Id;

use super::{Batch, Sprite, SpriteInstance};
use crate::backend::{self, context, texture::TextureResource};
use crate::backend::instances::cache;

fn get_bind_group<T: SpriteInstance>(
    texture: Id<TextureResource>,
    inst: &T,
) -> Option<wgpu::BindGroup> {
    let storage = context::texture_storage().read().unwrap();
    let Some(resource) = storage.get(texture) else {
        tracing::error!("Unknown texture {:?}", texture);
        return None;
    };
    let resource = match (inst.sprite(), resource) {
        (Sprite::Separate { .. }, TextureResource::Separate(resource))
        | (Sprite::Array { .. } | Sprite::Animated { .. }, TextureResource::Array(resource))
        | (Sprite::Atlas { .. }, TextureResource::Atlas { resource, .. }) => resource,
        _ => {
            tracing::error!("Texture resource does not match sprite {:?}", inst.sprite());
            return None;
        }
    };
    Some(resource.bind_group.clone())
}

pub fn draw<T: SpriteInstance>(
    instances: &mut [T],
    batch: &Batch,
    pass: &mut wgpu::RenderPass<'_>,
) {
    if batch.range.is_empty() {
        return;
    }
    assert!(
        batch.range.end as usize <= instances.len(),
        "Instance range out of bounds"
    );
    let inst = &instances[batch.range.start as usize];
    let Some(texture_bind_group) = get_bind_group(batch.resource, inst) else {
        return;
    };
    pass.set_bind_group(2, &texture_bind_group, &[]);
    
    crate::backend::metrics::batch();
    for range in cache::chunk_ranges(batch.range.clone()) {
        let instance_count = range.end - range.start;
        let instance_buffer_group = cache::prepare(instances, range);
        pass.set_bind_group(0, &instance_buffer_group, &[]);
        crate::backend::metrics::draw();
        pass.draw_indexed(batch.indices.clone(), 0, 0..instance_count);
    }
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub fn draw_all<T: SpriteInstance>(
    instances: &mut [T],
    pass: &mut wgpu::RenderPass<'_>,
) {
    let end = instances.len() as u32;
    let batches = backend::batch(instances, 0..end);

    for batch in batches {
        draw(instances, &batch, pass);
    }
}
