use super::InstanceError;
use crate::core::OwnedResource;
use std::ops::Range;
use std::sync::{Mutex, OnceLock};

use crate::backend::buffer::{self, Buffer, StorageRead};
use crate::backend::{RenderInstance, Sprite, SpriteInstance};

// Slots grow to a power of two, so their rounded capacity must still fit a page.
const MAX_INSTANCES_PER_SLOT: usize =
    1 << (buffer::BufferArena::PAGE_SIZE as usize / size_of::<RenderInstance>()).ilog2();

/// Split a CPU range without reordering it or allocating a temporary list.
pub(in crate::backend) fn chunk_ranges(mut range: Range<u32>) -> impl Iterator<Item = Range<u32>> {
    std::iter::from_fn(move || {
        if range.is_empty() {
            return None;
        }
        let start = range.start;
        let count = (range.end - start).min(MAX_INSTANCES_PER_SLOT as u32);
        range.start += count;
        Some(start..range.start)
    })
}

struct InstanceSlot {
    buffer: OwnedResource<Buffer<RenderInstance, StorageRead>>,
    group: wgpu::BindGroup,
    capacity: usize,
}

impl InstanceSlot {
    fn new(required: usize) -> Result<Self, InstanceError> {
        if required == 0 {
            return Err(InstanceError::InvalidRange);
        }
        if required > MAX_INSTANCES_PER_SLOT {
            return Err(InstanceError::BatchTooLarge);
        }
        let capacity = required.next_power_of_two();
        let buffer = buffer::storage::<RenderInstance>(None, capacity as u64)?;
        crate::backend::metrics::allocation();
        let (_, group) = crate::try_bind_group! {
            wgpu::ShaderStages::VERTEX => buffer,
        }?;
        Ok(Self {
            buffer,
            group,
            capacity,
        })
    }
}

#[derive(Default)]
struct InstanceCache {
    slots: Vec<InstanceSlot>,
    next: usize,
    staging: Vec<RenderInstance>,
}

static CACHE: OnceLock<Mutex<InstanceCache>> = OnceLock::new();

pub(in crate::backend) fn prepare<T: SpriteInstance>(
    instances: &[T],
    range: std::ops::Range<u32>,
) -> Result<wgpu::BindGroup, InstanceError> {
    let slice = instances
        .get(range.start as usize..range.end as usize)
        .ok_or(InstanceError::InvalidRange)?;
    let render_instances = slice.iter().map(|instance| {
        let mut render = instance.render_instance();
        if let Sprite::Atlas {
            texture, region, ..
        } = instance.sprite()
        {
            let resource = crate::backend::texture::get(*texture)
                .ok_or(InstanceError::MissingTexture)?;
            let crate::backend::texture::TextureResource::Atlas { regions, .. } = resource.as_ref() else {
                return Err(InstanceError::TextureKindMismatch);
            };
            let region = regions
                .get(*region)
                .ok_or(InstanceError::MissingAtlasRegion)?;
            render.uv_min = region.uv_min;
            render.uv_max = region.uv_max;
        }
        Ok(render)
    });
    try_prepare_render_instances(render_instances, range)
}

pub(in crate::backend) fn prepare_render_instances(
    instances: impl Iterator<Item = RenderInstance>,
    range: std::ops::Range<u32>,
) -> Result<wgpu::BindGroup, InstanceError> {
    try_prepare_render_instances(instances.map(Ok), range)
}

fn try_prepare_render_instances(
    instances: impl Iterator<Item = Result<RenderInstance, InstanceError>>,
    range: Range<u32>,
) -> Result<wgpu::BindGroup, InstanceError> {
    let required = range
        .end
        .checked_sub(range.start)
        .ok_or(InstanceError::InvalidRange)? as usize;
    if required > MAX_INSTANCES_PER_SLOT {
        return Err(InstanceError::BatchTooLarge);
    }
    if required == 0 {
        return Err(InstanceError::InvalidRange);
    }
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let index = cache.next;

    cache.staging.clear();
    for instance in instances.take(required + 1) {
        cache.staging.push(instance?);
    }
    if cache.staging.len() != required {
        return Err(InstanceError::InstanceCountMismatch);
    }

    if index == cache.slots.len() {
        cache.slots.push(InstanceSlot::new(required)?);
    } else if cache.slots[index].capacity < required {
        let old = std::mem::replace(&mut cache.slots[index], InstanceSlot::new(required)?);
        drop(old);
    }

    crate::backend::metrics::upload((cache.staging.len() * size_of::<RenderInstance>()) as u64);
    let slot = &cache.slots[index];
    buffer::write_buffer(&slot.buffer, 0, &cache.staging);
    let group = slot.group.clone();
    // Each call before submission needs its own slot: queue writes precede all draws.
    cache.next += 1;
    Ok(group)
}

pub(crate) fn submitted() {
    reset_frame();
}

/// Call only after submitting or discarding the frame's commands.
pub(crate) fn reset_frame() {
    if let Some(cache) = CACHE.get() {
        // No unsubmitted frame commands may still reference these slots.
        cache.lock().unwrap().next = 0;
    }
}

/// Logical GPU allocations retained by the instance cache, excluding arena page slack.
pub fn memory_stats() -> (usize, u64) {
    CACHE.get().map_or((0, 0), |cache| {
        let cache = cache.lock().unwrap();
        (
            cache.slots.len(),
            cache
                .slots
                .iter()
                .map(|slot| (slot.capacity * size_of::<RenderInstance>()) as u64)
                .sum(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_batches_return_errors_without_initializing_the_gpu() {
        assert!(matches!(
            prepare_render_instances(std::iter::empty(), 4..3),
            Err(InstanceError::InvalidRange)
        ));
        assert!(matches!(
            prepare_render_instances(std::iter::empty(), 0..0),
            Err(InstanceError::InvalidRange)
        ));
        assert!(matches!(
            prepare_render_instances(std::iter::empty(), 0..MAX_INSTANCES_PER_SLOT as u32 + 1),
            Err(InstanceError::BatchTooLarge)
        ));
        assert!(matches!(
            InstanceSlot::new(usize::MAX),
            Err(InstanceError::BatchTooLarge)
        ));
    }

    #[test]
    fn large_batch_keeps_every_instance_in_order_and_fits_pages() {
        let source = 9000..109000;
        let mut next = source.start;
        let mut count = 0;
        for chunk in chunk_ranges(source.clone()) {
            assert_eq!(chunk.start, next);
            let len = (chunk.end - chunk.start) as usize;
            assert!(len > 0);
            let allocation_bytes = len.next_power_of_two() * size_of::<RenderInstance>();
            assert!(allocation_bytes as u64 <= buffer::BufferArena::PAGE_SIZE);
            next = chunk.end;
            count += len;
        }
        assert_eq!(next, source.end);
        assert_eq!(count, 100_000);
    }

    #[test]
    fn chunk_boundaries_handle_empty_full_and_overflowing_batches() {
        let limit = MAX_INSTANCES_PER_SLOT as u32;
        assert_eq!(chunk_ranges(7..7).count(), 0);
        assert_eq!(chunk_ranges(0..limit).collect::<Vec<_>>(), vec![0..limit]);
        assert_eq!(
            chunk_ranges(0..limit + 1).collect::<Vec<_>>(),
            vec![0..limit, limit..limit + 1],
        );
        assert_eq!(
            chunk_ranges(u32::MAX - limit - 1..u32::MAX).collect::<Vec<_>>(),
            vec![u32::MAX - limit - 1..u32::MAX - 1, u32::MAX - 1..u32::MAX],
        );
    }
}
