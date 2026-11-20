use super::{
    BufferRegion, RenderError, RenderResult,
    storage::{self, ResourceId},
};
use crate::backend::{
    context::ContextId,
    target::{self, RenderTargetId},
};
use crate::{OwnedResource, SharedResource};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

pub type BindingEntry = wgpu::BindGroupLayoutEntry;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingSchema {
    pub groups: Vec<GroupSchema>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupSchema {
    pub index: u32,
    pub entries: Vec<BindingEntry>,
}

impl BindingSchema {
    pub fn normalized(&self) -> RenderResult<Self> {
        let mut result = self.clone();
        result.groups.sort_by_key(|g| g.index);
        let mut groups = BTreeSet::new();
        for group in &mut result.groups {
            if !groups.insert(group.index) {
                return Err(RenderError::new("binding schema", "duplicate group"));
            }
            group.entries.sort_by_key(|e| e.binding);
            let mut bindings = BTreeSet::new();
            for entry in &group.entries {
                if !bindings.insert(entry.binding) || entry.visibility.is_empty() {
                    return Err(RenderError::new(
                        format!("group {} binding {}", group.index, entry.binding),
                        "duplicate binding or empty visibility",
                    ));
                }
            }
        }
        Ok(result)
    }
    pub fn try_layouts(&self, context: Option<ContextId>) -> RenderResult<ResolvedLayouts> {
        let gpu = super::gpu(context)?;
        let schema = self.normalized()?;
        let limits = gpu.device.limits();
        let mut groups = vec![];
        for group in &schema.groups {
            if group.index >= limits.max_bind_groups
                || group.entries.len() > limits.max_bindings_per_bind_group as usize
            {
                return Err(RenderError::new(
                    "binding schema",
                    "device group limit exceeded",
                ));
            }
            for entry in &group.entries {
                if entry.binding >= limits.max_bindings_per_bind_group {
                    return Err(RenderError::new(
                        "binding schema",
                        "device binding limit exceeded",
                    ));
                }
            }
            groups.resize_with(group.index as usize + 1, || None);
            groups[group.index as usize] =
                Some(super::checked(&gpu.device, "binding layout", || {
                    gpu.device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("Velum explicit group"),
                            entries: &group.entries,
                        })
                })?);
        }
        let refs: Vec<_> = groups.iter().map(Option::as_ref).collect();
        let pipeline = super::checked(&gpu.device, "pipeline layout", || {
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Velum explicit layout"),
                    bind_group_layouts: &refs,
                    immediate_size: 0,
                })
        })?;
        Ok(ResolvedLayouts {
            schema,
            groups,
            pipeline,
        })
    }
    pub(crate) fn entry(&self, group: u32, binding: u32) -> RenderResult<&BindingEntry> {
        self.groups
            .iter()
            .find(|g| g.index == group)
            .and_then(|g| g.entries.iter().find(|e| e.binding == binding))
            .ok_or_else(|| {
                RenderError::new(
                    format!("group {group} binding {binding}"),
                    "not declared in schema",
                )
            })
    }
}
pub struct ResolvedLayouts {
    pub schema: BindingSchema,
    pub groups: Vec<Option<wgpu::BindGroupLayout>>,
    pub pipeline: wgpu::PipelineLayout,
}

#[derive(Clone)]
pub enum BindingResource {
    /// `size` is the binding window. Dynamic offsets must stay within `region`.
    Buffer {
        region: BufferRegion,
        size: u64,
    },
    Texture(wgpu::TextureView),
    Sampler(wgpu::Sampler),
    /// Resolved every preparation; target revision/resize never leaves a stale view.
    Target(SharedResource<RenderTargetId>),
    LegacyTexture(SharedResource<new_collection::Id<crate::backend::texture::TextureResource>>),
    LegacySampler(SharedResource<new_collection::Id<crate::backend::texture::TextureResource>>),
    TextureArray(Vec<wgpu::TextureView>),
    SamplerArray(Vec<wgpu::Sampler>),
    BufferArray(Vec<BufferRegion>),
}
impl BindingResource {
    pub fn buffer(region: BufferRegion) -> Self {
        let size = region.len();
        Self::Buffer { region, size }
    }
    pub fn uniform<T: bytemuck::Pod + Send + Sync + 'static>(
        context: Option<ContextId>,
        value: &T,
    ) -> RenderResult<Self> {
        let buffer =
            crate::backend::buffer::allocate_in::<T, crate::backend::buffer::Uniform>(context, 1)
                .map_err(|e| RenderError::new("material parameter", e.to_string()))?;
        crate::backend::buffer::try_write_buffer(&buffer, 0, std::slice::from_ref(value))
            .map_err(|e| RenderError::new("material parameter", e.to_string()))?;
        Ok(Self::buffer(BufferRegion::from_shared(buffer.share())?))
    }
}
#[derive(Clone)]
pub struct BindingSet {
    pub context: Option<ContextId>,
    pub group: u32,
    pub entries: Vec<(u32, BindingResource)>,
}
pub type BindingSetId = ResourceId<BindingSet>;
storage::stored!(BindingSet);
pub use storage::{get, remove};

#[derive(Clone)]
pub struct BindingUse {
    pub set: BindingSetId,
    /// Dynamic offsets in increasing binding order within this set.
    pub dynamic_offsets: Vec<u32>,
    /// Explicit object substitutions. Ordinary contributions reject collisions.
    pub replace: bool,
}
impl From<BindingSetId> for BindingUse {
    fn from(set: BindingSetId) -> Self {
        Self {
            set,
            dynamic_offsets: vec![],
            replace: false,
        }
    }
}
impl BindingUse {
    pub fn overriding(set: BindingSetId) -> Self {
        Self {
            replace: true,
            ..set.into()
        }
    }
    pub fn with_offsets(mut self, offsets: Vec<u32>) -> Self {
        self.dynamic_offsets = offsets;
        self
    }
}

fn validate_set(set: &BindingSet) -> RenderResult<()> {
    let gpu = super::gpu(set.context)?;
    if set.group >= gpu.device.limits().max_bind_groups {
        return Err(RenderError::new(
            "binding set",
            "group exceeds device limits",
        ));
    }
    let mut seen = BTreeSet::new();
    for (binding, resource) in &set.entries {
        if !seen.insert(*binding) || *binding >= gpu.device.limits().max_bindings_per_bind_group {
            return Err(RenderError::new(
                "binding set",
                "duplicate binding or device limit exceeded",
            ));
        }
        match resource {
            BindingResource::Buffer { region, size } => {
                region.validate(set.context, wgpu::BufferUsages::empty())?;
                if *size == 0 || *size > region.len() {
                    return Err(RenderError::new(
                        "binding set",
                        "buffer window outside allocation",
                    ));
                }
            }
            BindingResource::Target(owner) => {
                target::get(owner.id())
                    .ok_or_else(|| RenderError::new("binding set", "missing target"))?;
            }
            BindingResource::LegacyTexture(owner) | BindingResource::LegacySampler(owner) => {
                legacy_texture(owner.id())?;
            }
            BindingResource::BufferArray(regions) => {
                if regions.is_empty() {
                    return Err(RenderError::new("binding set", "empty buffer array"));
                }
                for region in regions {
                    region.validate(set.context, wgpu::BufferUsages::empty())?;
                }
            }
            BindingResource::TextureArray(values) if values.is_empty() => {
                return Err(RenderError::new("binding set", "empty texture array"));
            }
            BindingResource::SamplerArray(values) if values.is_empty() => {
                return Err(RenderError::new("binding set", "empty sampler array"));
            }
            _ => {}
        }
    }
    Ok(())
}
pub fn try_register(set: BindingSet) -> RenderResult<OwnedResource<BindingSetId>> {
    validate_set(&set)?;
    Ok(storage::insert(set))
}
pub fn try_update(id: BindingSetId, set: BindingSet) -> RenderResult<()> {
    validate_set(&set)?;
    if !storage::replace(id, set) {
        return Err(RenderError::new("binding set", "missing resource"));
    }
    Ok(())
}

pub struct ResolvedGroup {
    pub index: u32,
    pub group: wgpu::BindGroup,
    pub offsets: Vec<u32>,
    // Retain allocations and target owners through encoding, even if their sets are removed.
    pub(crate) _resources: Vec<BindingResource>,
}
pub(crate) fn resolve(
    context: Option<ContextId>,
    layouts: &ResolvedLayouts,
    uses: &[BindingUse],
) -> RenderResult<Vec<ResolvedGroup>> {
    let gpu = super::gpu(context)?;
    let mut merged: BTreeMap<(u32, u32), (BindingResource, u32)> = BTreeMap::new();
    for usage in uses {
        let set =
            get(usage.set).ok_or_else(|| RenderError::new("binding set", "missing resource"))?;
        if set.context != context {
            return Err(RenderError::new("binding set", "GPU context mismatch"));
        }
        let mut entries: Vec<_> = set.entries.iter().collect();
        entries.sort_by_key(|e| e.0);
        let mut offsets = usage.dynamic_offsets.iter();
        for (binding, resource) in entries {
            let entry = layouts.schema.entry(set.group, *binding)?;
            let dynamic = matches!(
                entry.ty,
                wgpu::BindingType::Buffer {
                    has_dynamic_offset: true,
                    ..
                }
            );
            let offset = if dynamic {
                *offsets
                    .next()
                    .ok_or_else(|| RenderError::new("binding set", "missing dynamic offset"))?
            } else {
                0
            };
            if merged.contains_key(&(set.group, *binding)) && !usage.replace {
                return Err(RenderError::new(
                    format!("group {} binding {}", set.group, binding),
                    "conflicting binding contributions",
                ));
            }
            validate_resource(context, &gpu.device, entry, resource, offset)?;
            merged.insert((set.group, *binding), (resource.clone(), offset));
        }
        if offsets.next().is_some() {
            return Err(RenderError::new("binding set", "extra dynamic offsets"));
        }
    }
    let mut result = vec![];
    for schema in &layouts.schema.groups {
        let mut resources = vec![];
        let mut retained = vec![];
        let mut offsets = vec![];
        for entry in &schema.entries {
            let (resource, offset) =
                merged
                    .remove(&(schema.index, entry.binding))
                    .ok_or_else(|| {
                        RenderError::new(
                            format!("group {} binding {}", schema.index, entry.binding),
                            "missing binding",
                        )
                    })?;
            if matches!(
                entry.ty,
                wgpu::BindingType::Buffer {
                    has_dynamic_offset: true,
                    ..
                }
            ) {
                offsets.push(offset);
            }
            retained.push(resource.clone());
            resources.push(match resource {
                BindingResource::Target(ref owner) => BindingResource::Texture(
                    target::get(owner.id())
                        .ok_or_else(|| RenderError::new("binding target", "missing resource"))?
                        .texture()
                        .view()
                        .clone(),
                ),
                BindingResource::LegacyTexture(ref owner) => {
                    BindingResource::Texture(legacy_texture(owner.id())?.0)
                }
                BindingResource::LegacySampler(ref owner) => {
                    BindingResource::Sampler(legacy_texture(owner.id())?.1)
                }
                _ => resource.clone(),
            });
        }
        let texture_arrays: Vec<Vec<&wgpu::TextureView>> = resources
            .iter()
            .map(|r| match r {
                BindingResource::TextureArray(a) => a.iter().collect(),
                _ => vec![],
            })
            .collect();
        let sampler_arrays: Vec<Vec<&wgpu::Sampler>> = resources
            .iter()
            .map(|r| match r {
                BindingResource::SamplerArray(a) => a.iter().collect(),
                _ => vec![],
            })
            .collect();
        let buffer_arrays: Vec<Vec<wgpu::BufferBinding<'_>>> = resources
            .iter()
            .map(|r| match r {
                BindingResource::BufferArray(a) => a
                    .iter()
                    .map(|r| wgpu::BufferBinding {
                        buffer: &r.buffer,
                        offset: r.bytes.start,
                        size: wgpu::BufferSize::new(r.len()),
                    })
                    .collect(),
                _ => vec![],
            })
            .collect();
        let entries: Vec<_> = schema
            .entries
            .iter()
            .zip(&resources)
            .enumerate()
            .map(|(i, (entry, resource))| wgpu::BindGroupEntry {
                binding: entry.binding,
                resource: match resource {
                    BindingResource::Buffer { region, size } => {
                        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &region.buffer,
                            offset: region.bytes.start,
                            size: wgpu::BufferSize::new(*size),
                        })
                    }
                    BindingResource::Texture(view) => wgpu::BindingResource::TextureView(view),
                    BindingResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
                    BindingResource::TextureArray(_) => {
                        wgpu::BindingResource::TextureViewArray(&texture_arrays[i])
                    }
                    BindingResource::SamplerArray(_) => {
                        wgpu::BindingResource::SamplerArray(&sampler_arrays[i])
                    }
                    BindingResource::BufferArray(_) => {
                        wgpu::BindingResource::BufferArray(&buffer_arrays[i])
                    }
                    BindingResource::Target(_)
                    | BindingResource::LegacyTexture(_)
                    | BindingResource::LegacySampler(_) => unreachable!(),
                },
            })
            .collect();
        let group = super::checked(&gpu.device, "binding group", || {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Velum resolved resources"),
                layout: layouts.groups[schema.index as usize].as_ref().unwrap(),
                entries: &entries,
            })
        })?;
        // Use the original resources so target owners are retained as well.
        result.push(ResolvedGroup {
            index: schema.index,
            group,
            offsets,
            _resources: retained,
        });
    }
    Ok(result)
}

pub(crate) fn validate_resource(
    context: Option<ContextId>,
    device: &wgpu::Device,
    entry: &BindingEntry,
    resource: &BindingResource,
    offset: u32,
) -> RenderResult<()> {
    let fail = |message| RenderError::new(format!("binding {}", entry.binding), message);
    if let Some(count) = entry.count {
        let len = match resource {
            BindingResource::TextureArray(v) => v.len(),
            BindingResource::SamplerArray(v) => v.len(),
            BindingResource::BufferArray(v) => v.len(),
            _ => return Err(fail("expected binding array")),
        };
        if len != count.get() as usize {
            return Err(fail("array count mismatch"));
        }
    } else if matches!(
        resource,
        BindingResource::TextureArray(_)
            | BindingResource::SamplerArray(_)
            | BindingResource::BufferArray(_)
    ) {
        return Err(fail("unexpected binding array"));
    }
    match (entry.ty, resource) {
        (
            wgpu::BindingType::Buffer {
                ty,
                min_binding_size,
                ..
            },
            BindingResource::Buffer { region, size },
        ) => {
            let limits = device.limits();
            let (usage, alignment, max_size) = match ty {
                wgpu::BufferBindingType::Uniform => (
                    wgpu::BufferUsages::UNIFORM,
                    limits.min_uniform_buffer_offset_alignment,
                    limits.max_uniform_buffer_binding_size,
                ),
                wgpu::BufferBindingType::Storage { .. } => (
                    wgpu::BufferUsages::STORAGE,
                    limits.min_storage_buffer_offset_alignment,
                    limits.max_storage_buffer_binding_size,
                ),
            };
            region.validate(context, usage)?;
            if *size == 0
                || *size > max_size
                || *size < min_binding_size.map_or(0, |s| s.get())
                || size
                    .checked_add(offset as u64)
                    .is_none_or(|end| end > region.len())
                || region.bytes.start % alignment as u64 != 0
                || offset as u64 % alignment as u64 != 0
            {
                return Err(fail(
                    "buffer size, alignment or dynamic range violates allocation",
                ));
            }
        }
        (
            wgpu::BindingType::Buffer {
                ty,
                min_binding_size,
                ..
            },
            BindingResource::BufferArray(regions),
        ) => {
            for region in regions {
                let one = BindingEntry {
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty,
                        min_binding_size,
                        has_dynamic_offset: false,
                    },
                    ..*entry
                };
                validate_resource(
                    context,
                    device,
                    &one,
                    &BindingResource::buffer(region.clone()),
                    0,
                )?;
            }
        }
        (
            wgpu::BindingType::Texture { .. } | wgpu::BindingType::StorageTexture { .. },
            BindingResource::Texture(_) | BindingResource::TextureArray(_),
        ) => {}
        (
            wgpu::BindingType::Texture { .. } | wgpu::BindingType::StorageTexture { .. },
            BindingResource::Target(owner),
        ) => {
            if context.is_some() {
                return Err(fail("historical targets belong to the default context"));
            }
            target::get(owner.id()).ok_or_else(|| fail("missing target"))?;
        }
        (wgpu::BindingType::Texture { .. }, BindingResource::LegacyTexture(owner))
        | (wgpu::BindingType::Sampler(_), BindingResource::LegacySampler(owner)) => {
            if context.is_some() {
                return Err(fail("historical textures belong to the default context"));
            }
            legacy_texture(owner.id())?;
        }
        (
            wgpu::BindingType::Sampler(_),
            BindingResource::Sampler(_) | BindingResource::SamplerArray(_),
        ) => {}
        _ => return Err(fail("resource kind differs from schema")),
    }
    Ok(())
}

fn legacy_texture(
    id: new_collection::Id<crate::backend::texture::TextureResource>,
) -> RenderResult<(wgpu::TextureView, wgpu::Sampler)> {
    use crate::backend::texture::TextureResource;
    let ctx = super::gpu(None)?;
    let textures = ctx.texture_storage.read().unwrap();
    let resource = match textures
        .get(id)
        .ok_or_else(|| RenderError::new("texture", "missing resource"))?
    {
        TextureResource::Separate(resource)
        | TextureResource::Array(resource)
        | TextureResource::Atlas { resource, .. } => resource,
    };
    Ok((resource.texture.view().clone(), resource.sampler.clone()))
}

/// Helpers keep locations explicit without repeating the full wgpu entry structure.
pub fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
    min_size: u64,
    dynamic: bool,
) -> BindingEntry {
    BindingEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: dynamic,
            min_binding_size: wgpu::BufferSize::new(min_size),
        },
        count: None,
    }
}
pub fn array_entry(mut entry: BindingEntry, count: NonZeroU32) -> BindingEntry {
    entry.count = Some(count);
    entry
}
