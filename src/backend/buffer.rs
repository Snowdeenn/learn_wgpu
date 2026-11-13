#![allow(unused)]
use super::bind_group::Bindable;
use super::context;

use new_collection::{Arena, Id};
use wgpu::util::DeviceExt;

pub(super) struct BufferBuilder<'a> {
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
    alloc_id: Id<Allocation>,
    _phantom: std::marker::PhantomData<(T, K)>,
}

pub(super) struct ResolvedBuffer<K> {
    buffer: wgpu::Buffer,
    offset: u64,
    size: u64,
    _marker: std::marker::PhantomData<K>,
}

crate::create_marker!(StorageRead, StorageReadWrite, StorageVertex, Uniform, Index, Vertex);

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

impl BindableBufferKind for StorageVertex {
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

impl BufferKind for StorageVertex {
    const USAGE: wgpu::BufferUsages =
        wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::VERTEX);
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
    fn new(id: Id<Allocation>) -> Self {
        Buffer {
            alloc_id: id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn raw_region(&self) -> (wgpu::Buffer, std::ops::Range<u64>) {
        let arena = context::buffer().read().unwrap();
        let alloc = arena.allocations.get(self.alloc_id).unwrap();
        let buffer = arena.pages[alloc.page].buffer.clone();
        (buffer, alloc.offset..alloc.offset + alloc.size)
    }
}

impl<T, K: BindableBufferKind> Buffer<T, K> {
    pub(crate) fn resolve_binding(&self) -> impl Bindable + use<T, K> {
        let arena = context::buffer().read().unwrap();
        let allocation = arena.allocations.get(self.alloc_id).unwrap();
        let page = &arena.pages[allocation.page];

        ResolvedBuffer::<K> {
            buffer: page.buffer.clone(),
            offset: allocation.offset,
            size: allocation.size,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: BindableBufferKind> Bindable for ResolvedBuffer<K> {
    fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: K::BINDING_TYPE,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    fn binding_resource(&self) -> wgpu::BindingResource<'_> {
        let size = wgpu::BufferSize::new(self.size)
            .expect("Une région liée doit avoir une taille non nulle");

        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: self.offset,
            size: Some(size),
        })
    }
}

pub fn storage<T: bytemuck::Pod>(capacity: u64) -> Result<Buffer<T, StorageRead>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, StorageRead>(capacity)
}

pub fn storage_vertex<T: bytemuck::Pod>(
    capacity: u64,
) -> Result<Buffer<T, StorageVertex>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, StorageVertex>(capacity)
}

pub fn storage_rw<T: bytemuck::Pod>(
    capacity: u64,
) -> Result<Buffer<T, StorageReadWrite>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, StorageReadWrite>(capacity)
}

pub fn uniform<T: bytemuck::Pod>(capacity: u64) -> Result<Buffer<T, Uniform>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, Uniform>(capacity)
}

pub fn vertex<T: bytemuck::Pod>(capacity: u64) -> Result<Buffer<T, Vertex>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, Vertex>(capacity)
}

pub fn index<T: bytemuck::Pod>(capacity: u64) -> Result<Buffer<T, Index>, AllocError> {
    context::buffer()
        .write()
        .unwrap()
        .allocate::<T, Index>(capacity)
}

pub fn write_buffer<T, K>(buffer: &Buffer<T, K>, offset: u64, data: &[T])
where
    T: bytemuck::Pod,
    K: BufferKind,
{
    context::buffer()
        .read()
        .unwrap()
        .write(buffer, offset, data);
}

pub fn delete_buffer<T, K>(buffer: Buffer<T, K>)
where
    T: bytemuck::Pod,
    K: BufferKind,
{
    context::buffer().write().unwrap().deallocate(buffer);
}

#[derive(Debug)]
pub(super) struct BufferPage {
    pub buffer: wgpu::Buffer,
    pub size: u64,
    pub next_offset: u64,
    pub allocations: u64,
}

#[derive(Debug)]
pub(super) struct Allocation {
    pub page: usize,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug)]
pub enum AllocError {
    AllocationTooLarge { requested: u64, page_size: u64 },
    AlignmentOverflow { offset: u64, alignment: u64 },
    OffsetOverflow { offset: u64, size: u64 },
}

impl std::fmt::Display for AllocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationTooLarge {
                requested,
                page_size,
            } => write!(
                f,
                "Allocation de {requested} octets supérieure à la capacité d'une page de {page_size} octets"
            ),
            Self::AlignmentOverflow { offset, alignment } => write!(
                f,
                "Débordement lors de l'alignement de l'offset {offset} sur {alignment} octets"
            ),
            Self::OffsetOverflow { offset, size } => write!(
                f,
                "Débordement lors du calcul de fin d'allocation pour l'offset {offset} et la taille {size}"
            ),
        }
    }
}

impl std::error::Error for AllocError {}

#[derive(Debug)]
pub(super) struct BufferArena {
    allocations: Arena<Allocation>,
    pages: Vec<BufferPage>,
    free_pages: Vec<(usize, u64)>, // (indice de page, taille en octets)
}

impl BufferArena {
    const PAGE_SIZE: u64 = 1024 * 1024;

    pub fn new() -> Self {
        BufferArena {
            allocations: Arena::new(),
            pages: Vec::new(),
            free_pages: Vec::new(),
        }
    }

    pub fn allocate<T, K>(&mut self, capacity: u64) -> Result<Buffer<T, K>, AllocError>
    where
        T: bytemuck::Pod,
        K: BufferKind,
    {
        let size = std::mem::size_of::<T>() as u64 * capacity;

        if size > Self::PAGE_SIZE {
            return Err(AllocError::AllocationTooLarge {
                requested: size,
                page_size: Self::PAGE_SIZE,
            });
        }

        let limits = context::device().limits();
        let alignment = match K::USAGE {
            usage if usage.contains(wgpu::BufferUsages::UNIFORM) => {
                limits.min_uniform_buffer_offset_alignment
            }
            usage if usage.contains(wgpu::BufferUsages::STORAGE) => {
                limits.min_storage_buffer_offset_alignment
            }
            _ => 1, // Temp pour l'instant TODO: il faudra supporter les différentes contrainte pour
                    // les autres buffer
        };

        let create_page = || BufferPage {
            buffer: BufferBuilder::new()
                .with_usages(K::USAGE | wgpu::BufferUsages::COPY_DST)
                .with_size(Self::PAGE_SIZE)
                .build(),
            size: Self::PAGE_SIZE,
            next_offset: 0,
            allocations: 0,
        };

        let free_page_position = self.free_pages.iter().position(|&(idx, page_size)| {
            size <= page_size && self.pages[idx].buffer.usage().contains(K::USAGE)
        });
        let mut idx = free_page_position
            .map(|position| self.free_pages[position].0)
            .unwrap_or(0);

        let alloc = loop {
            match self.pages.get(idx) {
                Some(page) if page.buffer.usage().contains(K::USAGE) => {}
                Some(_) => {
                    idx += 1;
                    continue;
                }
                None => {
                    self.pages.push(create_page());
                }
            }
            let page = self.pages.get_mut(idx).unwrap();
            let start = align_up(page.next_offset, alignment as u64)?;
            let end = start.checked_add(size).ok_or(AllocError::OffsetOverflow {
                offset: start,
                size,
            })?;

            if end <= page.size {
                page.next_offset = end;
                page.allocations += 1;
                if let Some(position) = free_page_position {
                    self.free_pages.swap_remove(position);
                }
                break Allocation {
                    page: idx,
                    offset: start,
                    size,
                };
            }
            idx += 1;
        };

        let alloc_id = self.allocations.insert(alloc);
        Ok(Buffer::new(alloc_id))
    }

    pub fn deallocate<T, K>(&mut self, buffer: Buffer<T, K>) {
        // TODO: Utiliser pleinement les options (améliorer la gestion d'erreur)
        let alloc = self.allocations.get(buffer.alloc_id).unwrap();
        let page = self.pages.get_mut(alloc.page).unwrap();
        page.allocations -= 1;

        if page.allocations == 0 {
            page.next_offset = 0;
            self.free_pages.push((alloc.page, page.size));
        }
        self.allocations.remove(buffer.alloc_id);
    }

    pub fn write<T, K>(&self, buffer: &Buffer<T, K>, offset: u64, data: &[T])
    where
        T: bytemuck::Pod,
        K: BufferKind,
    {
        let alloc = self.allocations.get(buffer.alloc_id).unwrap();
        let page = self.pages.get(alloc.page).unwrap();
        let total_offset = alloc.offset.checked_add(offset).expect("u64 overflow");
        let data_size = std::mem::size_of_val(data);

        if total_offset >= page.size {
            panic!(
                "Page at {} have size of {}, tried to write {data_size} bytes",
                buffer.alloc_id, page.size
            );
        }

        if data_size as u64 > page.size {
            panic!(
                "Page at {} have size of {}, can't write {data_size} bytes",
                buffer.alloc_id, page.size
            );
        }

        if total_offset
            .checked_add(data_size as u64)
            .expect("u64 overflow")
            > page.size
        {
            panic!(
                "Buffer at {} have size of {}, can't write {data_size} bytes",
                buffer.alloc_id, page.size
            );
        }

        if offset.checked_add(data_size as u64).expect("u64 overflow") > alloc.size {
            panic!(
                "Buffer at {} have size of {}, can't write {data_size} bytes",
                buffer.alloc_id, alloc.size
            );
        }

        context::queue().write_buffer(&page.buffer, total_offset, bytemuck::cast_slice(data));
    }
}

fn align_up(offset: u64, alignment: u64) -> Result<u64, AllocError> {
    let remainder = offset % alignment;
    if remainder == 0 {
        Ok(offset)
    } else {
        offset
            .checked_add(alignment - remainder)
            .ok_or(AllocError::AlignmentOverflow { offset, alignment })
    }
}
