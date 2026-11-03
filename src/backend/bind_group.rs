#![allow(unused)]
use super::context;

#[derive(Debug, Default)]
pub struct BindGroupLayoutBuilder<'a> {
    label: Option<&'a str>,
    bindings: Vec<(wgpu::ShaderStages, wgpu::BindingType)>,
}

/// Simplified creation of a bind group.
#[derive(Debug, Default)]
pub struct BindGroupBuilder<'a> {
    label: Option<&'a str>,
    resources: Vec<wgpu::BindingResource<'a>>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    /// Begin building the bind group layout.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Specify a new binding.
    ///
    /// The `with_binding` position of each binding will be inferred as the index within the order that
    /// they are added to this builder type. If you require manually specifying the binding
    /// location, you may be better off not using the `BindGroupLayoutBuilder` and instead
    /// constructing the `BindGroupLayout` and `BindGroup` manually.
    pub fn with_binding(mut self, visibility: wgpu::ShaderStages, ty: wgpu::BindingType) -> Self {
        self.bindings.push((visibility, ty));
        self
    }

    /// Add a uniform buffer binding to the layout.
    pub fn with_uniform_buffer(
        self,
        visibility: wgpu::ShaderStages,
        has_dynamic_offset: bool,
    ) -> Self {
        let ty = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset,
            // wgpu 0.5-0.6 TODO: potential perf hit, investigate this field
            min_binding_size: None,
        };
        self.with_binding(visibility, ty)
    }

    /// Add a storage buffer binding to the layout.
    pub fn with_storage_buffer(
        self,
        visibility: wgpu::ShaderStages,
        has_dynamic_offset: bool,
        read_only: bool,
    ) -> Self {
        let ty = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset,
            // wgpu 0.5-0.6 TODO: potential perf hit, investigate this field
            min_binding_size: None,
        };
        self.with_binding(visibility, ty)
    }

    /// Add a sampler binding to the layout.
    pub fn with_sampler(self, visibility: wgpu::ShaderStages, filtering: bool) -> Self {
        let ty = wgpu::BindingType::Sampler(if filtering {
            wgpu::SamplerBindingType::Filtering
        } else {
            wgpu::SamplerBindingType::NonFiltering
        });
        self.with_binding(visibility, ty)
    }

    /// Add a sampler binding to the layout.
    pub fn with_comparison_sampler(self, visibility: wgpu::ShaderStages) -> Self {
        let ty = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison);
        self.with_binding(visibility, ty)
    }

    /// Add a texture binding to the layout.
    pub fn with_texture(
        self,
        visibility: wgpu::ShaderStages,
        multisampled: bool,
        view_dimension: wgpu::TextureViewDimension,
        sample_type: wgpu::TextureSampleType,
    ) -> Self {
        // fix sample type in certain scenarios (constraint given by wgpu)
        let sample_type = if multisampled
            && matches!(
                sample_type,
                wgpu::TextureSampleType::Float { filterable: true }
            ) {
            wgpu::TextureSampleType::Float { filterable: false }
        } else {
            sample_type
        };
        let ty = wgpu::BindingType::Texture {
            multisampled,
            view_dimension,
            sample_type,
        };
        self.with_binding(visibility, ty)
    }

    /// Short-hand for adding a texture binding for a full view of the given texture to the layout.
    ///
    /// The `multisampled` and `dimension` parameters are retrieved from the `Texture` itself.
    ///
    /// Note that if you wish to take a `Cube` or `CubeArray` view of the given texture, you will
    /// need to manually specify the `TextureViewDimension` via the `with_texture` method
    /// instead.
    pub fn with_texture_from(
        self,
        visibility: wgpu::ShaderStages,
        texture: &wgpu::Texture,
    ) -> Self {
        let view_dimension = match texture.dimension() {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D2 => match texture.depth_or_array_layers() {
                1 => wgpu::TextureViewDimension::D2,
                _ => wgpu::TextureViewDimension::D2Array,
            },
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        };
        let sample_type = texture
            .format()
            .sample_type(None, None)
            .expect("Texture format should have a Sample Type");

        self.with_texture(
            visibility,
            texture.sample_count() > 1,
            view_dimension,
            sample_type,
        )
    }

    /// Add a storage texture binding to the layout.
    pub fn with_storage_texture(
        self,
        visibility: wgpu::ShaderStages,
        format: wgpu::TextureFormat,
        view_dimension: wgpu::TextureViewDimension,
        access: wgpu::StorageTextureAccess,
    ) -> Self {
        let ty = wgpu::BindingType::StorageTexture {
            view_dimension,
            format,
            access,
        };
        self.with_binding(visibility, ty)
    }

    /// Short-hand for adding a storage texture binding for a full view of the given texture to the
    /// layout.
    ///
    /// The `format`, `dimension` and `sample_type` are inferred from the given `with_texture`.
    pub fn with_storage_texture_from(
        self,
        visibility: wgpu::ShaderStages,
        texture: &wgpu::Texture,
        access: wgpu::StorageTextureAccess,
    ) -> Self {
        let view_dimension = match texture.dimension() {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D2 => match texture.depth_or_array_layers() {
                1 => wgpu::TextureViewDimension::D2,
                _ => wgpu::TextureViewDimension::D2Array,
            },
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        };
        self.with_storage_texture(visibility, texture.format(), view_dimension, access)
    }

    /// Build the bind group layout from the specified parameters.
    pub fn build(self) -> wgpu::BindGroupLayout {
        let mut entries = Vec::with_capacity(self.bindings.len());
        for (i, (visibility, ty)) in self.bindings.into_iter().enumerate() {
            let layout_binding = wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility,
                ty,
                count: None,
            };
            entries.push(layout_binding);
        }
        let descriptor = wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &entries,
        };
        context::device().create_bind_group_layout(&descriptor)
    }
}

impl<'a> BindGroupBuilder<'a> {
    /// Begin building the bind group.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Specify a new binding.
    ///
    /// The `with_binding` position of each binding will be inferred as the index within the order that
    /// they are added to this builder type. If you require manually specifying the binding
    /// location, you may be better off not using the `BindGroupBuilder` and instead constructing
    /// the `BindGroupLayout` and `BindGroup` manually.
    pub fn with_binding(mut self, resource: wgpu::BindingResource<'a>) -> Self {
        self.resources.push(resource);
        self
    }

    /// Specify a slice of a buffer to be bound.
    ///
    /// The given `range` represents the start and end point of the buffer to be bound in bytes.
    pub fn with_buffer_bytes(
        self,
        buffer: &'a wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) -> Self {
        let resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset,
            size,
        });
        self.with_binding(resource)
    }

    /// Specify a slice of a buffer of elements of type `T` to be bound.
    ///
    /// This method is similar to `with_buffer_bytes`, but expects a range of **elements** rather than a
    /// range of **bytes**.
    ///
    /// Type `T` *must* be either `#[repr(C)]` or `#[repr(transparent)]`.
    // NOTE: We might want to change this to match the wgpu API by using a NonZeroU64 for size.
    pub fn with_buffer<T>(self, buffer: &'a wgpu::Buffer, range: std::ops::Range<usize>) -> Self
    where
        T: Copy,
    {
        let size_bytes = std::mem::size_of::<T>() as wgpu::BufferAddress;
        let start = range.start as wgpu::BufferAddress * size_bytes;
        let end = range.end as wgpu::BufferAddress * size_bytes;
        let size = std::num::NonZeroU64::new(end - start).expect("buffer slice must not be empty");
        self.with_buffer_bytes(buffer, start, Some(size))
    }

    /// Specify a sampler to be bound.
    pub fn with_sampler(self, sampler: &'a wgpu::Sampler) -> Self {
        let resource = wgpu::BindingResource::Sampler(sampler);
        self.with_binding(resource)
    }

    /// Specify a texture view to be bound.
    pub fn with_texture_view(self, view: &'a wgpu::TextureView) -> Self {
        let resource = wgpu::BindingResource::TextureView(view);
        self.with_binding(resource)
    }

    /// Build the bind group with the specified resources.
    pub fn build(self, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        let device = context::device();
        let mut entries = Vec::with_capacity(self.resources.len());
        for (i, resource) in self.resources.into_iter().enumerate() {
            let binding = wgpu::BindGroupEntry {
                binding: i as u32,
                resource,
            };
            entries.push(binding);
        }
        let descriptor = wgpu::BindGroupDescriptor {
            label: self.label,
            layout,
            entries: &entries,
        };
        device.create_bind_group(&descriptor)
    }
}

pub trait Bindable {
    fn binding_type(&self) -> wgpu::BindingType;

    fn binding_resource(&self) -> wgpu::BindingResource<'_>;
}

#[macro_export]
macro_rules! bind_group {
    ($($visibility:expr => $resource:expr),+ $(,)?) => {{
        use $crate::backend::bind_group::Bindable;
        use $crate::backend::context;

        let mut layout_entries = Vec::new();
        let mut entries = Vec::new();

        $(
            let binding = layout_entries.len() as u32;
            let resource = &$resource;
            let resolved = resource.resolve_binding();

            layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: $visibility,
                ty: resolved.binding_type(),
                count: None,
            });

            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: resolved.binding_resource(),
            });
        )*

        let layout = context::device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            });

        let group = context::device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &entries,
            });

        (layout, group)
    }};
}
