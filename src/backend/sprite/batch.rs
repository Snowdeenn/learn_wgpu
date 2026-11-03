use super::SpriteInstance;
use crate::backend::texture::TextureResource;
use new_collection::Id;

pub struct Batch {
    resource: Id<TextureResource>,
    range: std::ops::Range<u32>,
}

pub fn batch<T: SpriteInstance>(
    instances: &mut [T],
    instance_range: std::ops::Range<u32>,
) -> Vec<Batch> {
    todo!()
}
