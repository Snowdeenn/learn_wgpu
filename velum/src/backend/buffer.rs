#![allow(unused)]
use super::bind_group::Bindable;
use super::context;
use crate::core::{OwnedResource, ResourceHandle};

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
        self.build_in(None)
    }

    pub fn build_in(self, context_id: Option<context::ContextId>) -> wgpu::Buffer {
        let device = context::device(context_id);

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
    context: Option<context::ContextId>,
    _phantom: std::marker::PhantomData<(T, K)>,
}

impl<T, K> Copy for Buffer<T, K> {}

impl<T, K> Clone for Buffer<T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: bytemuck::Pod + 'static, K: BufferKind + 'static> ResourceHandle for Buffer<T, K> {
    fn release(self) {
        delete_buffer(self);
    }
}

pub(super) struct ResolvedBuffer<K> {
    buffer: wgpu::Buffer,
    offset: u64,
    size: u64,
    _marker: std::marker::PhantomData<K>,
}

crate::create_marker!(
    StorageRead,
    StorageReadWrite,
    StorageVertex,
    Uniform,
    Index,
    Vertex
);

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
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::VERTEX);
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
    fn new(id: Id<Allocation>, context: Option<context::ContextId>) -> Self {
        Buffer {
            alloc_id: id,
            context,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn raw_region(&self) -> (wgpu::Buffer, std::ops::Range<u64>) {
        self.try_raw_region().expect("Missing buffer allocation")
    }

    pub fn context_id(&self) -> Option<context::ContextId> {
        self.context
    }

    pub fn try_raw_region(&self) -> Result<(wgpu::Buffer, std::ops::Range<u64>), RegionError> {
        let ctx = context::try_context(self.context).ok_or(RegionError::MissingContext)?;
        let storage_owner = &ctx.buffer;
        let arena = storage_owner.read().unwrap();
        let alloc = arena
            .allocations
            .get(self.alloc_id)
            .ok_or(BindingError::MissingAllocation)?;
        let buffer = arena.pages[alloc.page].buffer.clone();
        Ok((buffer, alloc.offset..alloc.offset + alloc.size))
    }
}

impl<T, K: BindableBufferKind> Buffer<T, K> {
    pub fn resolve_binding(&self) -> Result<impl Bindable + use<T, K>, BindingError> {
        let ctx = context::try_context(self.context).ok_or(BindingError::MissingAllocation)?;
        let storage_owner = &ctx.buffer;
        let arena = storage_owner.read().unwrap();
        let allocation = arena
            .allocations
            .get(self.alloc_id)
            .ok_or(BindingError::MissingAllocation)?;
        let page = arena
            .pages
            .get(allocation.page)
            .ok_or(BindingError::MissingPage)?;

        Ok(ResolvedBuffer::<K> {
            buffer: page.buffer.clone(),
            offset: allocation.offset,
            size: allocation.size,
            _marker: std::marker::PhantomData,
        })
    }
}

#[derive(Debug)]
pub enum BindingError {
    MissingAllocation,
    MissingPage,
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingAllocation => "Buffer allocation is missing",
            Self::MissingPage => "Buffer page is missing",
        })
    }
}
impl std::error::Error for BindingError {}

#[derive(Debug)]
pub enum RegionError {
    MissingContext,
    Binding(BindingError),
}
impl From<BindingError> for RegionError {
    fn from(error: BindingError) -> Self {
        Self::Binding(error)
    }
}
impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContext => f.write_str("GPU context is missing"),
            Self::Binding(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for RegionError {}

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

pub fn storage<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, StorageRead>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, StorageRead>(capacity, context_id)
}

pub fn storage_vertex<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, StorageVertex>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, StorageVertex>(capacity, context_id)
}

pub fn storage_rw<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, StorageReadWrite>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, StorageReadWrite>(capacity, context_id)
}

pub fn uniform<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, Uniform>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, Uniform>(capacity, context_id)
}

pub fn vertex<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, Vertex>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, Vertex>(capacity, context_id)
}

pub fn index<T: bytemuck::Pod>(
    context_id: Option<crate::backend::context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, Index>>, AllocError> {
    context::buffer(context_id)
        .write()
        .unwrap()
        .allocate_in::<T, Index>(capacity, context_id)
}

pub fn write_buffer<T, K>(buffer: &Buffer<T, K>, offset: u64, data: &[T])
where
    T: bytemuck::Pod,
    K: BufferKind + 'static,
{
    context::buffer(buffer.context)
        .read()
        .unwrap()
        .write(buffer, offset, data);
}

pub fn delete_buffer<T, K>(buffer: Buffer<T, K>)
where
    T: bytemuck::Pod,
    K: BufferKind + 'static,
{
    if let Some(ctx) = context::try_context(buffer.context) {
        ctx.buffer.write().unwrap().deallocate(buffer);
    }
}

#[derive(Debug)]
pub enum WriteError {
    Binding(RegionError),
    InvalidRange,
    UnalignedTransfer,
}
impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binding(e) => e.fmt(f),
            Self::InvalidRange => f.write_str("Write exceeds the allocated buffer region"),
            Self::UnalignedTransfer => {
                f.write_str("Buffer write offset and size must be multiples of four bytes")
            }
        }
    }
}
impl std::error::Error for WriteError {}
/// Fallible immediate upload, validating the allocation rather than the whole page.
pub fn try_write_buffer<T: bytemuck::Pod, K: BufferKind>(
    buffer: &Buffer<T, K>,
    offset: u64,
    values: &[T],
) -> Result<(), WriteError> {
    let (raw, range) = buffer.try_raw_region().map_err(WriteError::Binding)?;
    let bytes = bytemuck::cast_slice(values);
    if offset
        .checked_add(bytes.len() as u64)
        .is_none_or(|end| end > range.end - range.start)
    {
        return Err(WriteError::InvalidRange);
    }
    if offset % wgpu::COPY_BUFFER_ALIGNMENT != 0
        || bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT != 0
    {
        return Err(WriteError::UnalignedTransfer);
    }
    let ctx = context::try_context(buffer.context)
        .ok_or(WriteError::Binding(RegionError::MissingContext))?;
    ctx.queue.write_buffer(&raw, range.start + offset, bytes);
    super::report::record_upload(bytes.len() as u64);
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferMemoryStats {
    pub active_allocations: usize,
    pub active_bytes: u64,
    /// Allocator page capacity, including free space. Not physical GPU memory.
    pub page_capacity_bytes: u64,
}
pub fn memory_stats(context_id: Option<context::ContextId>) -> Option<BufferMemoryStats> {
    let ctx = context::try_context(context_id)?;
    let arena = ctx.buffer.read().unwrap();
    Some(BufferMemoryStats {
        active_allocations: arena.allocations.iter().count(),
        active_bytes: arena.allocations.iter().map(|a| a.size).sum(),
        page_capacity_bytes: arena.pages.iter().map(|p| p.size).sum(),
    })
}

/// Typed allocation on either the default or an explicitly registered context.
pub fn allocate_in<T: bytemuck::Pod, K: BufferKind + 'static>(
    context_id: Option<context::ContextId>,
    capacity: u64,
) -> Result<OwnedResource<Buffer<T, K>>, ContextAllocError> {
    let ctx = context::try_context(context_id).ok_or(ContextAllocError::MissingContext)?;
    ctx.buffer
        .write()
        .unwrap()
        .allocate_in::<T, K>(capacity, context_id)
        .map_err(ContextAllocError::Allocation)
}

#[derive(Debug)]
pub enum ContextAllocError {
    MissingContext,
    Allocation(AllocError),
}
impl std::fmt::Display for ContextAllocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContext => f.write_str("GPU context is missing"),
            Self::Allocation(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ContextAllocError {}

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
    InvalidBufferSize { size: u64 },
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
            Self::InvalidBufferSize { size } => write!(f, "Capacité donnée invalide {size}"),
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
    pub(super) const PAGE_SIZE: u64 = 1024 * 1024;

    pub fn new() -> Self {
        BufferArena {
            allocations: Arena::new(),
            pages: Vec::new(),
            free_pages: Vec::new(),
        }
    }

    pub fn allocate<T, K>(
        &mut self,
        capacity: u64,
    ) -> Result<OwnedResource<Buffer<T, K>>, AllocError>
    where
        T: bytemuck::Pod,
        K: BufferKind + 'static,
    {
        self.allocate_in::<T, K>(capacity, None)
    }

    fn allocate_in<T: bytemuck::Pod, K: BufferKind + 'static>(
        &mut self,
        capacity: u64,
        context_id: Option<context::ContextId>,
    ) -> Result<OwnedResource<Buffer<T, K>>, AllocError> {
        let size = (std::mem::size_of::<T>() as u64)
            .checked_mul(capacity)
            .ok_or(AllocError::InvalidBufferSize { size: capacity })?;

        if size > Self::PAGE_SIZE {
            return Err(AllocError::AllocationTooLarge {
                requested: size,
                page_size: Self::PAGE_SIZE,
            });
        }

        if size == 0 {
            return Err(AllocError::InvalidBufferSize { size });
        }

        let device = context::device(context_id);
        let limits = device.limits();
        let alignment = match K::USAGE {
            usage if usage.contains(wgpu::BufferUsages::UNIFORM) => {
                limits.min_uniform_buffer_offset_alignment
            }
            usage if usage.contains(wgpu::BufferUsages::STORAGE) => {
                limits.min_storage_buffer_offset_alignment
            }
            _ => wgpu::COPY_BUFFER_ALIGNMENT as u32,
        };

        let create_page = || BufferPage {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Velum typed allocation page"),
                size: Self::PAGE_SIZE,
                usage: K::USAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
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
        Ok(OwnedResource::new(Buffer::new(alloc_id, context_id)))
    }

    pub fn deallocate<T, K>(&mut self, buffer: Buffer<T, K>) {
        let Some(alloc) = self.allocations.get(buffer.alloc_id) else {
            tracing::error!("Invalid or unknow Id {}", buffer.alloc_id);
            return;
        };
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
        K: BufferKind + 'static,
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

        context::queue(buffer.context).write_buffer(
            &page.buffer,
            total_offset,
            bytemuck::cast_slice(data),
        );
        super::report::record_upload(data_size as u64);
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
