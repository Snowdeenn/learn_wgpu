use super::{Batch, Sprite, SpriteInstance};
use crate::backend::{
    self, context,
    instances::{InstanceError, cache},
    texture::TextureResource,
};
use new_collection::Id;

fn get_bind_group<T: SpriteInstance>(
    texture: Id<TextureResource>,
    inst: &T,
) -> Result<wgpu::BindGroup, InstanceError> {
    let owner = context::texture_storage(None);
    let storage = owner.read().unwrap();
    let resource = storage.get(texture).ok_or(InstanceError::MissingTexture)?;
    let resource = match (inst.sprite(), resource) {
        (Sprite::Separate { .. }, TextureResource::Separate(resource)) => resource,
        (Sprite::Array { layer, .. }, TextureResource::Array(resource))
        | (
            Sprite::Animated {
                current_layer: layer,
                ..
            },
            TextureResource::Array(resource),
        ) => {
            if *layer >= resource.texture.as_raw().depth_or_array_layers() {
                return Err(InstanceError::InvalidTextureLayer);
            }
            resource
        }
        (Sprite::Atlas { region, .. }, TextureResource::Atlas { resource, regions }) => {
            regions
                .get(*region)
                .ok_or(InstanceError::MissingAtlasRegion)?;
            resource
        }
        _ => return Err(InstanceError::TextureKindMismatch),
    };
    Ok(resource.bind_group.clone())
}

pub fn draw<T: SpriteInstance>(
    instances: &mut [T],
    batch: &Batch,
    pass: &mut wgpu::RenderPass<'_>,
) -> Result<(), InstanceError> {
    let slice = instances
        .get(batch.range.start as usize..batch.range.end as usize)
        .ok_or(InstanceError::InvalidRange)?;
    let Some(inst) = slice.first() else {
        return Ok(());
    };
    let texture_bind_group = get_bind_group(batch.resource, inst)?;
    pass.set_bind_group(2, &texture_bind_group, &[]);
    crate::backend::metrics::batch();
    for range in cache::chunk_ranges(batch.range.clone()) {
        let count = range.end - range.start;
        let group = cache::prepare(instances, range)?;
        pass.set_bind_group(0, &group, &[]);
        crate::backend::metrics::draw();
        pass.draw_indexed(batch.indices.clone(), 0, 0..count);
    }
    Ok(())
}

#[allow(dead_code, reason = "Public drawing API")]
pub fn draw_all<T: SpriteInstance>(
    instances: &mut [T],
    pass: &mut wgpu::RenderPass<'_>,
) -> Result<(), InstanceError> {
    let end = u32::try_from(instances.len()).map_err(|_| InstanceError::BatchTooLarge)?;
    for batch in backend::batch(instances, 0..end) {
        draw(instances, &batch, pass)?;
    }
    Ok(())
}
