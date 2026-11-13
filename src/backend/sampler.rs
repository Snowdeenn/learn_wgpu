#![allow(unused)]

use crate::backend::bind_group::Bindable;

use super::context;

/// Simplifies the construction of a `Sampler` with a set of reasonable defaults.
#[derive(Debug)]
pub struct SamplerBuilder<'a> {
    pub descriptor: wgpu::SamplerDescriptor<'a>,
}

impl<'a> SamplerBuilder<'a> {
    pub const DEFAULT_ADDRESS_MODE_U: wgpu::AddressMode = wgpu::AddressMode::ClampToEdge;
    pub const DEFAULT_ADDRESS_MODE_V: wgpu::AddressMode = wgpu::AddressMode::ClampToEdge;
    pub const DEFAULT_ADDRESS_MODE_W: wgpu::AddressMode = wgpu::AddressMode::ClampToEdge;
    pub const DEFAULT_MAG_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
    pub const DEFAULT_MIN_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
    pub const DEFAULT_MIPMAP_FILTER: wgpu::MipmapFilterMode = wgpu::MipmapFilterMode::Nearest;
    pub const DEFAULT_LOD_MIN_CLAMP: f32 = 0.0;
    pub const DEFAULT_LOD_MAX_CLAMP: f32 = 100.0;
    pub const DEFAULT_COMPARE: Option<wgpu::CompareFunction> = None;
    pub const DEFAULT_ANISOTROPY_CLAMP: u16 = 1;
    pub const DEFAULT_LABEL: &'static str = "nannou-sampler";
    pub const DEFAULT_BORDER_COLOR: Option<wgpu::SamplerBorderColor> = None;
    pub const DEFAULT_DESCRIPTOR: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        address_mode_u: Self::DEFAULT_ADDRESS_MODE_U,
        address_mode_v: Self::DEFAULT_ADDRESS_MODE_V,
        address_mode_w: Self::DEFAULT_ADDRESS_MODE_W,
        mag_filter: Self::DEFAULT_MAG_FILTER,
        min_filter: Self::DEFAULT_MIN_FILTER,
        mipmap_filter: Self::DEFAULT_MIPMAP_FILTER,
        lod_min_clamp: Self::DEFAULT_LOD_MIN_CLAMP,
        lod_max_clamp: Self::DEFAULT_LOD_MAX_CLAMP,
        compare: Self::DEFAULT_COMPARE,
        anisotropy_clamp: Self::DEFAULT_ANISOTROPY_CLAMP,
        label: Some(Self::DEFAULT_LABEL),
        border_color: Self::DEFAULT_BORDER_COLOR,
    };

    /// Begin building a `Sampler`, starting with the `Default` parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// How the implementation should behave when sampling outside of the texture coordinates range
    /// [0.0, 1.0].
    pub fn address_mode_u(mut self, mode: wgpu::AddressMode) -> Self {
        self.descriptor.address_mode_u = mode;
        self
    }

    /// How the implementation should behave when sampling outside of the texture coordinates range
    /// [0.0, 1.0].
    pub fn address_mode_v(mut self, mode: wgpu::AddressMode) -> Self {
        self.descriptor.address_mode_v = mode;
        self
    }

    /// How the implementation should behave when sampling outside of the texture coordinates range
    /// [0.0, 1.0].
    pub fn address_mode_w(mut self, mode: wgpu::AddressMode) -> Self {
        self.descriptor.address_mode_w = mode;
        self
    }

    /// How the implementation should behave when sampling outside of the texture coordinates range
    /// [0.0, 1.0].
    ///
    /// Applies the same address mode to all axes.
    pub fn address_mode(self, mode: wgpu::AddressMode) -> Self {
        self.address_mode_u(mode)
            .address_mode_v(mode)
            .address_mode_w(mode)
    }

    /// How the implementation should sample from the image when it is respectively larger than the
    /// original.
    pub fn mag_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.descriptor.mag_filter = filter;
        self
    }

    /// How the implementation should sample from the image when it is respectively smaller than
    /// the original.
    pub fn min_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.descriptor.min_filter = filter;
        self
    }

    /// How the implementation should choose which mipmap to use.
    pub fn mipmap_filter(mut self, filter: wgpu::MipmapFilterMode) -> Self {
        self.descriptor.mipmap_filter = filter;
        self
    }

    /// The minimum mipmap level to use.
    pub fn lod_min_clamp(mut self, min: f32) -> Self {
        self.descriptor.lod_min_clamp = min;
        self
    }

    /// The maximum mipmap level to use.
    pub fn lod_max_clamp(mut self, max: f32) -> Self {
        self.descriptor.lod_max_clamp = max;
        self
    }

    /// The comparison function to use, if any.
    pub fn compare(mut self, f: Option<wgpu::CompareFunction>) -> Self {
        self.descriptor.compare = f;
        self
    }

    /// The anisotropy level to clamp to, if any.
    pub fn anisotropy_clamp(mut self, clamp: u16) -> Self {
        self.descriptor.anisotropy_clamp = clamp;
        self
    }

    /// The label to use, if any.
    pub fn label(mut self, label: Option<&'static str>) -> Self {
        self.descriptor.label = label;
        self
    }

    /// Calls `device.create_sampler(&self.descriptor)` internally and consume the Builder.
    pub fn build(self) -> wgpu::Sampler {
        context::device().create_sampler(&self.descriptor)
    }
}

impl<'a> Default for SamplerBuilder<'a> {
    fn default() -> Self {
        SamplerBuilder {
            descriptor: Self::DEFAULT_DESCRIPTOR,
        }
    }
}

impl From<SamplerBuilder<'static>> for wgpu::SamplerDescriptor<'static> {
    fn from(builder: SamplerBuilder<'static>) -> Self {
        builder.descriptor
    }
}

impl<'a> From<wgpu::SamplerDescriptor<'static>> for SamplerBuilder<'a> {
    fn from(descriptor: wgpu::SamplerDescriptor<'static>) -> Self {
        SamplerBuilder { descriptor }
    }
}

pub struct Sampler<K> {
    raw: wgpu::Sampler,
    _phantom: std::marker::PhantomData<K>,
}

impl<K> Sampler<K> {
    pub(crate) fn resolve_binding(&self) -> &Self {
        self
    }

    pub fn as_raw(&self) -> &wgpu::Sampler {
        &self.raw
    }
}

impl<K: SamplingKind> Bindable for Sampler<K> {
    fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(K::BINDING_TYPE)
    }

    fn binding_resource(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Sampler(&self.raw)
    }
}

crate::create_marker!(Filtering, NonFiltering, Comparison);

pub trait SamplingKind {
    const BINDING_TYPE: wgpu::SamplerBindingType;
}

impl SamplingKind for Filtering {
    const BINDING_TYPE: wgpu::SamplerBindingType = wgpu::SamplerBindingType::Filtering;
}

impl SamplingKind for NonFiltering {
    const BINDING_TYPE: wgpu::SamplerBindingType = wgpu::SamplerBindingType::NonFiltering;
}

impl SamplingKind for Comparison {
    const BINDING_TYPE: wgpu::SamplerBindingType = wgpu::SamplerBindingType::Comparison;
}

fn create<K: SamplingKind>(desc: wgpu::SamplerDescriptor<'_>) -> Sampler<K> {
    Sampler {
        raw: context::device().create_sampler(&desc),
        _phantom: std::marker::PhantomData,
    }
}

pub fn filtering(label: &str) -> Sampler<Filtering> {
    create(wgpu::SamplerDescriptor {
        label: Some(label),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..SamplerBuilder::DEFAULT_DESCRIPTOR
    })
}

pub fn non_filtering(label: &str) -> Sampler<NonFiltering> {
    create(wgpu::SamplerDescriptor {
        label: Some(label),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

pub fn comparison(label: &str, compare: wgpu::CompareFunction) -> Sampler<Comparison> {
    create(wgpu::SamplerDescriptor {
        label: Some(label),
        compare: Some(compare),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

pub fn pixel_art(label: &str) -> Sampler<Filtering> {
    create(wgpu::SamplerDescriptor {
        label: Some(label),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..SamplerBuilder::DEFAULT_DESCRIPTOR
    })
}
