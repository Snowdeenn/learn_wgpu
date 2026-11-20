//! Ready-made Velum shape/sprite format, using the same objects as custom materials.
use super::{
    BindingResource, BindingSchema, BindingSet, BindingSetId, GeometryId, InstanceBinding,
    InstanceStream, Material, MaterialId, MaterialTemplate, MaterialTemplateId, RenderError,
    RenderObject, RenderResult, bindings, object::RenderPhase,
};
use crate::backend::{
    context::ContextId,
    shape::ShapeInstance,
    sprite::{RenderInstance, Sprite, SpriteInstance},
    texture::TextureResource,
};
use crate::{OwnedResource, SharedResource};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct Slots {
    pub instances: (u32, u32),
    pub camera: (u32, u32),
    pub texture: (u32, u32),
    pub sampler: (u32, u32),
    pub vertices: u32,
}
impl Default for Slots {
    fn default() -> Self {
        Self {
            instances: (0, 0),
            camera: (1, 0),
            texture: (2, 0),
            sampler: (2, 1),
            vertices: 0,
        }
    }
}
impl Slots {
    pub fn instance_binding(self) -> InstanceBinding {
        InstanceBinding::Storage {
            group: self.instances.0,
            binding: self.instances.1,
        }
    }
}
pub fn try_template(
    context: Option<ContextId>,
    format: wgpu::TextureFormat,
    slots: Slots,
    sprite_array: Option<bool>,
    blend: Option<wgpu::BlendState>,
    depth: Option<wgpu::DepthStencilState>,
) -> RenderResult<OwnedResource<MaterialTemplateId>> {
    let source = match sprite_array {
        None => include_str!("../../assets/shaders/mesh.wgsl"),
        Some(false) => include_str!("../../assets/shaders/texture.wgsl"),
        Some(true) => include_str!("../../assets/shaders/texture_array.wgsl"),
    };
    let source = source
        .replace("@group(0) @binding(0)", "__INSTANCES__")
        .replace("@group(1) @binding(0)", "__CAMERA__")
        .replace("@group(2) @binding(0)", "__TEXTURE__")
        .replace("@group(2) @binding(1)", "__SAMPLER__");
    let declaration = |(group, binding)| format!("@group({group}) @binding({binding})");
    let source = source
        .replace("__INSTANCES__", &declaration(slots.instances))
        .replace("__CAMERA__", &declaration(slots.camera))
        .replace("__TEXTURE__", &declaration(slots.texture))
        .replace("__SAMPLER__", &declaration(slots.sampler));
    let shader = super::try_shader(context, "Velum standard material", &source)?;
    let mut groups: BTreeMap<u32, Vec<super::BindingEntry>> = BTreeMap::new();
    groups
        .entry(slots.instances.0)
        .or_default()
        .push(bindings::buffer_entry(
            slots.instances.1,
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Storage { read_only: true },
            std::mem::size_of::<RenderInstance>() as u64,
            false,
        ));
    groups
        .entry(slots.camera.0)
        .or_default()
        .push(bindings::buffer_entry(
            slots.camera.1,
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
            64,
            false,
        ));
    if let Some(array) = sprite_array {
        groups
            .entry(slots.texture.0)
            .or_default()
            .push(super::BindingEntry {
                binding: slots.texture.1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if array {
                        wgpu::TextureViewDimension::D2Array
                    } else {
                        wgpu::TextureViewDimension::D2
                    },
                    multisampled: false,
                },
                count: None,
            });
        groups
            .entry(slots.sampler.0)
            .or_default()
            .push(super::BindingEntry {
                binding: slots.sampler.1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
    }
    let mut template =
        MaterialTemplate::fullscreen(context, "Velum standard material", shader, format);
    template.schema = BindingSchema {
        groups: groups
            .into_iter()
            .map(|(index, entries)| super::GroupSchema { index, entries })
            .collect(),
    };
    let max_buffers = super::gpu(context)?.device.limits().max_vertex_buffers;
    if slots.vertices >= max_buffers {
        return Err(RenderError::new(
            "standard material",
            "vertex slot exceeds device limits",
        ));
    }
    template
        .vertex_layouts
        .resize(slots.vertices as usize + 1, None);
    template.vertex_layouts[slots.vertices as usize] =
        Some(crate::geometry::Vertex::layout().into());
    template.targets[0].as_mut().unwrap().blend = blend;
    template.depth_stencil = depth;
    super::material::try_template(template)
}
pub fn try_sprite_material(
    template: SharedResource<MaterialTemplateId>,
    texture: SharedResource<new_collection::Id<TextureResource>>,
    slots: Slots,
) -> RenderResult<OwnedResource<MaterialId>> {
    let mut groups: BTreeMap<u32, Vec<(u32, BindingResource)>> = BTreeMap::new();
    groups.entry(slots.texture.0).or_default().push((
        slots.texture.1,
        BindingResource::LegacyTexture(texture.clone()),
    ));
    groups
        .entry(slots.sampler.0)
        .or_default()
        .push((slots.sampler.1, BindingResource::LegacySampler(texture)));
    let sets = groups
        .into_iter()
        .map(|(group, entries)| {
            bindings::try_register(BindingSet {
                context: None,
                group,
                entries,
            })
            .map(OwnedResource::share)
        })
        .collect::<RenderResult<Vec<_>>>()?;
    super::material::try_register(Material {
        template,
        bindings: sets,
    })
}

/// Prepare once; the resulting phase can mix shapes, sprites and custom objects.
pub fn submit_shapes<T: ShapeInstance>(
    instances: &[T],
    material: MaterialId,
    binding: InstanceBinding,
    phase: &mut RenderPhase,
) -> RenderResult<()> {
    let context = super::material::get(material)
        .and_then(|m| super::material::get(m.template.id()))
        .ok_or_else(|| RenderError::new("shapes", "missing material/template"))?
        .description
        .context;
    let mut pending = vec![];
    for chunk in instances.chunks(max_instances()) {
        let data: Vec<_> = chunk.iter().map(ShapeInstance::render_instance).collect();
        let stream = InstanceStream::upload_as(context, &data, binding.clone())?;
        for (i, instance) in chunk.iter().enumerate() {
            let geometry = crate::backend::shape::geometry(instance.mesh())
                .ok_or_else(|| RenderError::new("shapes", "missing mesh"))?;
            let mut object = RenderObject::new(geometry.id(), material)
                .with_instances(stream.clone(), i as u32..i as u32 + 1);
            object.phase = phase.id;
            object.sort_key = instance.sort_key();
            pending.push(object);
        }
    }
    for object in pending {
        phase.push(object);
    }
    Ok(())
}
pub fn submit_sprites<T: SpriteInstance>(
    instances: &[T],
    geometry: GeometryId,
    binding: InstanceBinding,
    phase: &mut RenderPhase,
    mut material: impl FnMut(&Sprite) -> RenderResult<MaterialId>,
) -> RenderResult<()> {
    let context = super::geometry::get(geometry)
        .ok_or_else(|| RenderError::new("sprites", "missing geometry"))?
        .context;
    let mut pending = vec![];
    for chunk in instances.chunks(max_instances()) {
        let data = chunk
            .iter()
            .map(resolve_sprite)
            .collect::<RenderResult<Vec<_>>>()?;
        let stream = InstanceStream::upload_as(context, &data, binding.clone())?;
        for (i, instance) in chunk.iter().enumerate() {
            let mut object = RenderObject::new(geometry, material(instance.sprite())?)
                .with_instances(stream.clone(), i as u32..i as u32 + 1);
            object.phase = phase.id;
            object.sort_key = instance.sort_key();
            pending.push(object);
        }
    }
    for object in pending {
        phase.push(object);
    }
    Ok(())
}
fn max_instances() -> usize {
    crate::backend::buffer::BufferArena::PAGE_SIZE as usize / std::mem::size_of::<RenderInstance>()
}
fn resolve_sprite<T: SpriteInstance>(instance: &T) -> RenderResult<RenderInstance> {
    let mut render = instance.render_instance();
    let (texture, indices) = match instance.sprite() {
        Sprite::Separate { texture, indices }
        | Sprite::Array {
            texture, indices, ..
        }
        | Sprite::Atlas {
            texture, indices, ..
        }
        | Sprite::Animated {
            texture, indices, ..
        } => (*texture, indices),
    };
    if *indices != (0..6) {
        return Err(RenderError::new(
            "sprites",
            "standard quad requires indices 0..6; use a custom render object for other geometry",
        ));
    }
    let gpu = super::gpu(None)?;
    let textures = gpu.texture_storage.read().unwrap();
    let texture = textures
        .get(texture)
        .ok_or_else(|| RenderError::new("sprites", "missing texture"))?;
    match (instance.sprite(), texture) {
        (Sprite::Separate { .. }, TextureResource::Separate(_)) => render.layer = 0,
        (Sprite::Array { layer, .. }, TextureResource::Array(resource))
        | (
            Sprite::Animated {
                current_layer: layer,
                ..
            },
            TextureResource::Array(resource),
        ) => {
            if *layer >= resource.texture.as_raw().depth_or_array_layers() {
                return Err(RenderError::new("sprites", "texture layer out of bounds"));
            }
            render.layer = *layer;
        }
        (Sprite::Atlas { region, .. }, TextureResource::Atlas { regions, .. }) => {
            let region = regions
                .get(*region)
                .ok_or_else(|| RenderError::new("sprites", "missing atlas region"))?;
            render.uv_min = region.uv_min;
            render.uv_max = region.uv_max;
            render.layer = 0;
        }
        _ => return Err(RenderError::new("sprites", "texture kind mismatch")),
    }
    Ok(render)
}

/// Two camera bindings at application-selected locations. Screen coordinates are logical
/// pixels, top-left origin, matching `TextSpace::Screen`; world coordinates are unchanged.
pub struct Cameras {
    world:
        SharedResource<crate::backend::buffer::Buffer<math::Mat4, crate::backend::buffer::Uniform>>,
    screen:
        SharedResource<crate::backend::buffer::Buffer<math::Mat4, crate::backend::buffer::Uniform>>,
    pub world_bindings: SharedResource<BindingSetId>,
    pub screen_bindings: SharedResource<BindingSetId>,
}
impl Cameras {
    pub fn new(context: Option<ContextId>, location: (u32, u32)) -> RenderResult<Self> {
        use crate::backend::buffer;
        let world = buffer::allocate_in::<math::Mat4, buffer::Uniform>(context, 1)
            .map_err(|e| RenderError::new("world camera", e.to_string()))?
            .share();
        let screen = buffer::allocate_in::<math::Mat4, buffer::Uniform>(context, 1)
            .map_err(|e| RenderError::new("screen camera", e.to_string()))?
            .share();
        let bindings = |buffer| {
            bindings::try_register(BindingSet {
                context,
                group: location.0,
                entries: vec![(
                    location.1,
                    BindingResource::buffer(super::BufferRegion::from_shared(buffer)?),
                )],
            })
            .map(OwnedResource::share)
        };
        let world_bindings = bindings(world.clone())?;
        let screen_bindings = bindings(screen.clone())?;
        Ok(Self {
            world,
            screen,
            world_bindings,
            screen_bindings,
        })
    }
    pub fn update(
        &self,
        camera: &crate::camera::Camera2D,
        physical: [u32; 2],
        dpi: f32,
    ) -> RenderResult<()> {
        if physical.contains(&0)
            || !dpi.is_finite()
            || dpi <= 0.0
            || !camera.pos().x.is_finite()
            || !camera.pos().y.is_finite()
        {
            return Err(RenderError::new("cameras", "invalid resolution or DPI"));
        }
        let world = camera.get_view_proj(physical[0] as f32, physical[1] as f32);
        let screen = math::Mat4::orthographic_wgpu(
            0.0,
            physical[0] as f32 / dpi,
            physical[1] as f32 / dpi,
            0.0,
            -1.0,
            1.0,
        );
        crate::backend::buffer::write_buffer(&self.world.id(), 0, &[world]);
        crate::backend::buffer::write_buffer(&self.screen.id(), 0, &[screen]);
        Ok(())
    }
}
