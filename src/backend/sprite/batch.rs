use super::SpriteInstance;
use crate::backend::{Sprite, texture::TextureResource};
use new_collection::Id;

pub struct Batch {
    pub resource: Id<TextureResource>,
    pub indices: std::ops::Range<u32>,
    pub range: std::ops::Range<u32>,
}

pub fn batch<T: SpriteInstance>(
    instances: &mut [T],
    instance_range: std::ops::Range<u32>,
) -> Vec<Batch> {
    let mut batches = Vec::new();
    batch_into(instances, instance_range, &mut batches);
    batches
}

/// Groups only consecutive compatible sprites; never sorts the source list.
pub fn batch_into<T: SpriteInstance>(instances: &[T], instance_range: std::ops::Range<u32>, batches: &mut Vec<Batch>) {
    let start = instance_range.start;
    let slice = &instances[start as usize..instance_range.end as usize];
    batches.clear();

    for (offset, instance) in slice.iter().enumerate() {
        let (resource, first_index, end_index) = sort_key(instance);
        let indices = first_index..end_index;
        let index = start + offset as u32;

        if let Some(last) = batches.last_mut()
            && last.resource == resource
            && last.indices == indices
        {
            last.range.end = index + 1;
            continue;
        }

        batches.push(Batch {
            resource,
            indices,
            range: index..index + 1,
        });
    }

}

fn sort_key<T: SpriteInstance>(inst: &T) -> (Id<TextureResource>, u32, u32) {
    match inst.sprite() {
        Sprite::Separate { texture, indices }
        | Sprite::Animated {
            texture, indices, ..
        }
        | Sprite::Array {
            texture, indices, ..
        }
        | Sprite::Atlas {
            texture, indices, ..
        } => (*texture, indices.start, indices.end),
    }
}
