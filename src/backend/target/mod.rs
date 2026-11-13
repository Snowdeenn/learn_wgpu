mod storage;

pub use storage::{RenderTargetId, get, register, remove, resize_window_targets};

use crate::backend::texture::TextureBuilder;

use super::texture::Texture;

pub struct RenderTarget {
    texture: Texture,
    revision: u64,
    descriptor: RenderTargetDescriptor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetSize {
    Fixed { width: u32, height: u32 },
    Window { width: u32, height: u32 },
}

#[derive(Clone, Copy, Debug)]
pub enum PassTarget {
    Offscreen(RenderTargetId),
    Surface,
}

impl From<RenderTargetId> for PassTarget {
    fn from(id: RenderTargetId) -> Self {
        Self::Offscreen(id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderTargetDescriptor {
    pub label: &'static str,
    pub size: TargetSize,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub sample_count: u32,
}

impl RenderTarget {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn size_policy(&self) -> TargetSize {
        self.descriptor.size
    }

    pub fn size(&self) -> [u32; 2] {
        let size = self.texture.as_raw().size();
        [size.width, size.height]
    }

    fn resized(&self, width: u32, height: u32) -> Self {
        let mut descriptor = self.descriptor;
        descriptor.size = TargetSize::Window { width, height };
        let mut target = create_target(&descriptor);
        target.revision = self
            .revision
            .checked_add(1)
            .expect("Target revision overflow");
        target
    }
}

pub fn create_target(desc: &RenderTargetDescriptor) -> RenderTarget {
    let size = match desc.size {
        TargetSize::Fixed { width, height } => wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TargetSize::Window { width, height } => wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    };

    assert!(
        size.width > 0 && size.height > 0,
        "Render target size must be nonzero"
    );

    let raw = TextureBuilder::new()
        .with_label(desc.label)
        .with_format(desc.format)
        .with_size(size)
        .with_usage(desc.usage)
        .with_sample_count(desc.sample_count)
        .with_view_formats(&[])
        .build();

    let texture = Texture::from_raw(raw);

    RenderTarget {
        texture,
        revision: 0,
        descriptor: *desc,
    }
}
