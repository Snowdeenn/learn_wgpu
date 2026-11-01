#![allow(unused)]
use super::bind_group::Bindable;
use super::context;

use wgpu::util::DeviceExt;

pub struct BufferBuilder<'a> {
    data: Option<&'a [u8]>,
    desc: wgpu::BufferDescriptor<'a>,
}

impl<'a> BufferBuilder<'a> {
    pub const DEFAULT_SIZE: u64 = 0;
    pub const DEFAULT_USAGE: wgpu::BufferUsages =
        wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_DST);
    pub const DEFAULT_MAPPED_AT_CREATION: bool = false;
    pub const DEFAULT_BUFFER: wgpu::BufferDescriptor<'a> = wgpu::BufferDescriptor {
        label: None,
        size: Self::DEFAULT_SIZE,
        usage: Self::DEFAULT_USAGE,
        mapped_at_creation: Self::DEFAULT_MAPPED_AT_CREATION,
    };

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.desc.label = Some(label);
        self
    }

    pub fn with_data(mut self, data: &'a [u8]) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_usages(mut self, usages: wgpu::BufferUsages) -> Self {
        self.desc.usage = usages;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.desc.size = size;
        self
    }

    pub fn build(self) -> wgpu::Buffer {
        let device = context::device();

        match self.data {
            Some(data) => device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.desc.label,
                contents: data,
                usage: self.desc.usage,
            }),
            None => device.create_buffer(&self.desc),
        }
    }
}

impl<'a> Default for BufferBuilder<'a> {
    fn default() -> Self {
        Self {
            data: None,
            desc: Self::DEFAULT_BUFFER,
        }
    }
}

pub struct Buffer<T, K> {
    raw: wgpu::Buffer,
    _phantom: std::marker::PhantomData<(T, K)>,
}

crate::create_marker!(StorageRead, StorageReadWrite, Uniform, Index, Vertex);

pub trait BufferKind {
    const USAGE: wgpu::BufferUsages;
}

pub trait BindableBufferKind: BufferKind {
    const BINDING_TYPE: wgpu::BufferBindingType;
}

impl BindableBufferKind for StorageRead {
    const BINDING_TYPE: wgpu::BufferBindingType =
        wgpu::BufferBindingType::Storage { read_only: true };
}

impl BindableBufferKind for StorageReadWrite {
    const BINDING_TYPE: wgpu::BufferBindingType =
        wgpu::BufferBindingType::Storage { read_only: false };
}

impl BindableBufferKind for Uniform {
    const BINDING_TYPE: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
}

impl BufferKind for StorageReadWrite {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE;
}

impl BufferKind for StorageRead {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE;
}

impl BufferKind for Uniform {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM;
}

impl BufferKind for Index {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::INDEX;
}

impl BufferKind for Vertex {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX;
}

impl<T, K: BufferKind> Buffer<T, K> {
    fn new(label: &str, capacity: usize) -> Self {
        let raw = BufferBuilder::new()
            .with_label(label)
            .with_usages(K::USAGE | wgpu::BufferUsages::COPY_DST)
            .with_size((std::mem::size_of::<T>() * capacity) as u64)
            .build();

        Buffer {
            raw,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn as_raw(&self) -> &wgpu::Buffer {
        &self.raw
    }
}

impl<T, K: BindableBufferKind> Bindable for Buffer<T, K> {
    fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: K::BINDING_TYPE,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    fn binding_resource(&self) -> wgpu::BindingResource<'_> {
        self.raw.as_entire_binding()
    }
}

pub fn storage<T>(label: &str, capacity: usize) -> Buffer<T, StorageRead> {
    Buffer::new(label, capacity)
}

/// A storage buffer that can also be bound as a vertex buffer.
pub fn storage_vertex<T>(label: &str, capacity: usize) -> Buffer<T, StorageRead> {
    Buffer {
        raw: BufferBuilder::new()
            .with_label(label)
            .with_usages(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
            .with_size((std::mem::size_of::<T>() * capacity) as u64)
            .build(),
        _phantom: std::marker::PhantomData,
    }
}

pub fn storage_rw<T>(label: &str, capacity: usize) -> Buffer<T, StorageReadWrite> {
    Buffer::new(label, capacity)
}

pub fn uniform<T>(label: &str, capacity: usize) -> Buffer<T, Uniform> {
    Buffer::new(label, capacity)
}

pub fn vertex<T>(label: &str, capacity: usize) -> Buffer<T, Vertex> {
    Buffer::new(label, capacity)
}

pub fn index<T>(label: &str, capacity: usize) -> Buffer<T, Index> {
    Buffer::new(label, capacity)
}
