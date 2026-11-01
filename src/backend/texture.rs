#![allow(unused)]
use crate::backend::{self, bind_group::Bindable, context};

pub struct TextureBuilder<'a> {
    pub desc: wgpu::TextureDescriptor<'a>,
}

impl<'a> TextureBuilder<'a> {
    pub const DEFAULT_MIP_LEVEL_COUNT: u32 = 1;
    pub const DEFAULT_SAMPLE_COUNT: u32 = 1;
    pub const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    pub const DEFAULT_USAGE: wgpu::TextureUsages =
        wgpu::TextureUsages::TEXTURE_BINDING.union(wgpu::TextureUsages::COPY_DST);
    pub const DEFAULT_VIEW_FORMATS: &'a [wgpu::TextureFormat] = &[wgpu::TextureFormat::Rgba8Unorm];
    pub const DEFAULT_SIZE: wgpu::Extent3d = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    pub const DEFAULT_DIMENSION: wgpu::TextureDimension = wgpu::TextureDimension::D2;
    pub const DEFAULT_TEXTURE: wgpu::TextureDescriptor<'a> = wgpu::TextureDescriptor {
        label: None,
        mip_level_count: Self::DEFAULT_MIP_LEVEL_COUNT,
        sample_count: Self::DEFAULT_SAMPLE_COUNT,
        format: Self::DEFAULT_TEXTURE_FORMAT,
        usage: Self::DEFAULT_USAGE,
        view_formats: Self::DEFAULT_VIEW_FORMATS,
        size: Self::DEFAULT_SIZE,
        dimension: Self::DEFAULT_DIMENSION,
    };

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_mip_level_count(mut self, count: u32) -> Self {
        self.desc.mip_level_count = count;
        self
    }

    pub fn with_sample_count(mut self, count: u32) -> Self {
        self.desc.sample_count = count;
        self
    }

    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.desc.format = format;
        self
    }

    pub fn with_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.desc.usage = usage;
        self
    }

    pub fn with_view_formats(mut self, formats: &'a [wgpu::TextureFormat]) -> Self {
        self.desc.view_formats = formats;
        self
    }

    pub fn with_size(mut self, size: wgpu::Extent3d) -> Self {
        self.desc.size = size;
        self
    }

    pub fn with_dimension(mut self, dimension: wgpu::TextureDimension) -> Self {
        self.desc.dimension = dimension;
        self
    }

    pub fn build(self) -> wgpu::Texture {
        context::device().create_texture(&self.desc)
    }
}

impl<'a> Default for TextureBuilder<'a> {
    fn default() -> Self {
        TextureBuilder {
            desc: Self::DEFAULT_TEXTURE,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    raw: wgpu::Texture,
    view: wgpu::TextureView,
    view_dimension: wgpu::TextureViewDimension,
    sample_type: wgpu::TextureSampleType,
}

impl Texture {
    pub fn from_file(label: &str, path: &str) -> Result<Self, image::ImageError> {
        Ok(Self::from_raw(backend::load_texture(label, path)?))
    }

    pub fn from_raw(raw: wgpu::Texture) -> Self {
        Self::with_view(raw, &Default::default())
    }

    pub fn with_view(raw: wgpu::Texture, desc: &wgpu::TextureViewDescriptor<'_>) -> Self {
        let view_dimension = desc.dimension.unwrap_or_else(|| match raw.dimension() {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D2 => {
                if raw.depth_or_array_layers() == 1 {
                    wgpu::TextureViewDimension::D2
                } else {
                    wgpu::TextureViewDimension::D2Array
                }
            }
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        });

        let sample_type = desc
            .format
            .unwrap_or(raw.format())
            .sample_type(Some(desc.aspect), None)
            .expect("Texture view format cannot be sampled");

        let view = raw.create_view(desc);

        Self {
            raw,
            view,
            view_dimension,
            sample_type,
        }
    }

    pub fn as_raw(&self) -> &wgpu::Texture {
        &self.raw
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

impl Bindable for Texture {
    fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: self.sample_type,
            view_dimension: self.view_dimension,
            multisampled: self.raw.sample_count() > 1,
        }
    }

    fn binding_resource(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::TextureView(&self.view)
    }
}

#[derive(Debug)]
pub enum TextureResource {
    Separate(Resource),
    Array(Resource),
    Atlas {
        resource: Resource,
        regions: new_collection::Arena<AtlasRegion>,
    },
}

#[derive(Debug)]
pub struct Resource {
    pub texture: Texture,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct AtlasRegion {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}
