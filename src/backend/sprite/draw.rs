use super::{Batch, RenderInstance, Sprite, SpriteInstance};
use crate::backend::{self, buffer, context, texture::TextureResource};

pub fn draw(batches: &Batch, pass: &mut wgpu::RenderPass<'_>) {}

pub struct Ranges {
    pub indices: std::ops::Range<u32>,
}

fn prepare_instance<T: SpriteInstance>(
    instances: &mut [T],
    range: std::ops::Range<u32>,
) -> wgpu::BindGroup {
    let render_instances_buffer = buffer::storage::<RenderInstance>(instances.len() as u64)
        .expect("Impossible d'allouer le buffer d'instances de rendu");

    for inst_idx in range {
        let inst = &instances[inst_idx as usize];
        let render_inst = inst.render_instance();

        backend::write_buffer(
            &render_instances_buffer,
            inst_idx as u64 * size_of::<RenderInstance>() as u64,
            &[render_inst],
        );
    }
    let (_, group) = crate::bind_group! {
        wgpu::ShaderStages::VERTEX => render_instances_buffer,
    };
    group
}

pub fn draw_all<T: SpriteInstance>(
    instances: &mut [T],
    range: std::ops::Range<u32>,
    pass: &mut wgpu::RenderPass<'_>,
) {
    let group = prepare_instance(instances, range.clone());

    for inst_idx in range {
        let inst = &instances[inst_idx as usize];

        match inst.sprite() {
            Sprite::Separate { texture, indices } => {
                let bind_group = {
                    let storage = context::texture_storage().read().unwrap();
                    let resource = storage.get(*texture).unwrap();
                    let TextureResource::Separate(r) = resource else {
                        tracing::error!("Unknow Sprite format {:?}", inst.sprite());
                        continue;
                    };
                    r.bind_group.clone()
                };

                pass.set_bind_group(0, &group, &[]);
                pass.set_bind_group(2, &bind_group, &[]);
                pass.draw_indexed(indices.clone(), 0, inst_idx..inst_idx + 1);
            }
            Sprite::Array {
                texture,
                layer,
                indices,
            } => {}
            Sprite::Atlas { .. } => {}
        }
    }
}
