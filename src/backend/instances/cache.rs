use std::sync::{Mutex, OnceLock};

use crate::backend::{RenderInstance, Sprite, SpriteInstance};
use crate::backend::buffer::{self, Buffer, StorageRead};

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
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let index = cache.next;
    let required = (range.end - range.start) as usize;

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
