use new_collection::Id;

use super::{Batch, Sprite, SpriteInstance};
use crate::backend::{context, texture::TextureResource};

pub fn draw(batches: &Batch, pass: &mut wgpu::RenderPass<'_>) {}

fn prepare_instance<T: SpriteInstance>(
    instances: &mut [T],
    range: std::ops::Range<u32>,
) -> wgpu::BindGroup {
    super::cache::prepare(instances, range)
}

pub fn draw_all<T: SpriteInstance>(
    instances: &mut [T],
    range: std::ops::Range<u32>,
    pass: &mut wgpu::RenderPass<'_>,
) {
    if range.is_empty() {
        return;
    }
    assert!(
        range.end as usize <= instances.len(),
        "Instance range out of bounds"
    );
    let group = prepare_instance(instances, range.clone());

    let get_bind_group = |texture: Id<TextureResource>, inst: &T| -> Option<wgpu::BindGroup> {
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
    };

    for inst_idx in range {
        let inst = &instances[inst_idx as usize];
        let (texture, indices) = match inst.sprite() {
            Sprite::Separate { texture, indices }
            | Sprite::Array { texture, indices, .. }
            | Sprite::Atlas { texture, indices, .. }
            | Sprite::Animated { texture, indices, .. } => (*texture, indices),
        };
        let Some(bind_group) = get_bind_group(texture, inst) else {
            continue;
        };

        pass.set_bind_group(0, &group, &[]);
        pass.set_bind_group(2, &bind_group, &[]);
        pass.draw_indexed(indices.clone(), 0, inst_idx..inst_idx + 1);
    }
}