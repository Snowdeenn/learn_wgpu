use new_collection::Id;

use crate::{
    app::InstanceData,
    backend::{
        self, buffer, context,
        texture::{AtlasRegion, TextureResource},
    },
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderInstance {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
}

#[derive(Debug)]
pub enum Sprite {
    Separate {
        texture: Id<TextureResource>,
        indices: std::ops::Range<u32>,
    },
    Array {
        texture: Id<TextureResource>,
        layer: u32,
        indices: std::ops::Range<u32>,
    },
    Atlas {
        texture: Id<TextureResource>,
        region: Id<AtlasRegion>,
        indices: std::ops::Range<u32>,
    },
}

pub struct Batch {
    resource: Id<TextureResource>,
    range: std::range::Range<u32>,
}

pub fn batch(
    instances: &mut Vec<InstanceData>,
    instance_range: std::range::Range<u32>,
) -> Vec<Batch> {
    todo!()
}

pub fn draw(batches: &Batch, pass: &mut wgpu::RenderPass<'_>) {}

pub struct Ranges {
    pub indices: std::ops::Range<u32>,
}

fn prepare_instance(instances: &mut InstanceData, range: std::ops::Range<u32>) -> wgpu::BindGroup {
    let render_instances_buffer =
        buffer::storage::<RenderInstance>("Render Instance buffer", instances.len());

    for inst_idx in range {
        let inst = &instances[inst_idx as usize];
        let render_inst = RenderInstance {
            color: inst.color,
            offset: inst.offset,
            scale: inst.scale,
        };

        backend::write_buffer(
            &render_instances_buffer.as_raw(),
            inst_idx as u64 * size_of::<RenderInstance>() as u64,
            bytemuck::bytes_of(&render_inst),
        );
    }
    let (_, group) = crate::bind_group! {
        wgpu::ShaderStages::VERTEX => render_instances_buffer,
    };
    group
}

pub fn draw_all(
    instances: &mut InstanceData,
    range: std::ops::Range<u32>,
    pass: &mut wgpu::RenderPass<'_>,
) {
    let group = prepare_instance(instances, range.clone());

    for inst_idx in range {
        let inst = &instances[inst_idx as usize];

        match &inst.sprite {
            Sprite::Separate { texture, indices } => {
                let bind_group = {
                    let storage = context::texture_storage().read().unwrap();
                    let resource = storage.get(*texture).unwrap();
                    let TextureResource::Separate(r) = resource else {
                        tracing::error!("Unknow Sprite format {:?}", &inst.sprite);
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
