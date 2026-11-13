use std::ops::Range;
use std::sync::{Mutex, OnceLock};

use crate::backend::{RenderInstance, Sprite, SpriteInstance};
use crate::backend::buffer::{self, Buffer, StorageRead};

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
    buffer: Buffer<RenderInstance, StorageRead>,
    group: wgpu::BindGroup,
    capacity: usize,
}

impl InstanceSlot {
    fn new(required: usize) -> Self {
        let capacity = required.next_power_of_two();
        let buffer = buffer::storage::<RenderInstance>(capacity as u64)
            .expect("Impossible d'allouer le buffer d'instances de rendu");
        let (_, group) = crate::bind_group! {
            wgpu::ShaderStages::VERTEX => buffer,
        };
        Self {
            buffer,
            group,
            capacity,
        }
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
) -> wgpu::BindGroup {
    let render_instances = instances[range.start as usize..range.end as usize]
        .iter()
        .map(|instance| {
            let mut render = instance.render_instance();
            if let Sprite::Atlas {
                texture, region, ..
            } = instance.sprite()
            {
                let storage = crate::backend::context::texture_storage().read().unwrap();
                let resource = storage.get(*texture).expect("Unknown texture");
                let crate::backend::texture::TextureResource::Atlas { regions, .. } = resource
                else {
                    panic!("Expected an atlas texture");
                };
                let region = regions.get(*region).expect("Unknown atlas region");
                render.uv_min = region.uv_min;
                render.uv_max = region.uv_max;
            }
            render
        });
    prepare_render_instances(render_instances, range)
}

pub(in crate::backend) fn prepare_render_instances(
    instances: impl Iterator<Item = RenderInstance>,
    range: std::ops::Range<u32>,
) -> wgpu::BindGroup {
    let required = (range.end - range.start) as usize;
    assert!(required <= MAX_INSTANCES_PER_SLOT, "Split instance batches before preparing them");
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let index = cache.next;

    if index == cache.slots.len() {
        cache.slots.push(InstanceSlot::new(required));
    } else if cache.slots[index].capacity < required {
        let old = std::mem::replace(&mut cache.slots[index], InstanceSlot::new(required));
        buffer::delete_buffer(old.buffer);
    }

    cache.staging.clear();
    cache.staging.extend(instances);
    let slot = &cache.slots[index];
    buffer::write_buffer(
        &slot.buffer,
        0,
        &cache.staging,
    );
    let group = slot.group.clone();
    // Each call before submission needs its own slot: queue writes precede all draws.
    cache.next += 1;
    group
}

pub(crate) fn submitted() {
    if let Some(cache) = CACHE.get() {
        // Called after queue.submit, so subsequent writes follow the previous draws.
        cache.lock().unwrap().next = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
