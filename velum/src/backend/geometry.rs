use super::{
    RenderError, RenderResult,
    storage::{self, ResourceId},
};
use crate::backend::{
    buffer::{self, Buffer, BufferKind},
    context::ContextId,
};
use crate::{OwnedResource, SharedResource};
use std::{ops::Range, sync::Arc};

/// A bounded view of a Velum allocation. The lease prevents page reuse while live.
/// Subregions are relative to this allocation, never to the underlying GPU page.
#[derive(Clone)]
pub struct BufferRegion {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bytes: Range<u64>,
    pub(crate) context: Option<ContextId>,
    lease: Arc<dyn Fn() -> RenderResult<()> + Send + Sync>,
}
impl BufferRegion {
    pub fn from_shared<
        T: bytemuck::Pod + Send + Sync + 'static,
        K: BufferKind + Send + Sync + 'static,
    >(
        owner: SharedResource<Buffer<T, K>>,
    ) -> RenderResult<Self> {
        let (buffer, bytes) = owner
            .id()
            .try_raw_region()
            .map_err(|e| RenderError::new("buffer region", e.to_string()))?;
        let context = owner.id().context_id();
        Ok(Self {
            buffer,
            bytes,
            context,
            lease: Arc::new(move || {
                owner
                    .id()
                    .try_raw_region()
                    .map(|_| ())
                    .map_err(|e| RenderError::new("buffer region", e.to_string()))
            }),
        })
    }
    /// Import a dedicated GPU buffer. Holds a cloned GPU handle; caller must not
    /// destroy it or recycle its storage during use. Velum allocations use `from_shared`.
    pub fn import(
        context: Option<ContextId>,
        buffer: wgpu::Buffer,
        bytes: Range<u64>,
    ) -> RenderResult<Self> {
        super::gpu(context)?;
        if bytes.start >= bytes.end || bytes.end > buffer.size() {
            return Err(RenderError::new("buffer region", "range outside buffer"));
        }
        Ok(Self {
            buffer,
            bytes,
            context,
            lease: Arc::new(|| Ok(())),
        })
    }
    pub fn subregion(&self, bytes: Range<u64>) -> RenderResult<Self> {
        if bytes.start >= bytes.end || bytes.end > self.len() {
            return Err(RenderError::new(
                "buffer region",
                "range outside allocation",
            ));
        }
        let mut result = self.clone();
        result.bytes = self.bytes.start + bytes.start..self.bytes.start + bytes.end;
        Ok(result)
    }
    pub fn len(&self) -> u64 {
        self.bytes.end - self.bytes.start
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn raw_region(&self) -> (&wgpu::Buffer, Range<u64>) {
        (&self.buffer, self.bytes.clone())
    }
    pub(crate) fn validate(
        &self,
        context: Option<ContextId>,
        usage: wgpu::BufferUsages,
    ) -> RenderResult<()> {
        (self.lease)()?;
        if self.context != context {
            return Err(RenderError::new("buffer region", "GPU context mismatch"));
        }
        if !self.buffer.usage().contains(usage) {
            return Err(RenderError::new("buffer region", "missing buffer usage"));
        }
        Ok(())
    }
}
impl PartialEq for BufferRegion {
    fn eq(&self, rhs: &Self) -> bool {
        self.context == rhs.context && self.buffer == rhs.buffer && self.bytes == rhs.bytes
    }
}

#[derive(Clone)]
pub struct VertexBuffer {
    pub slot: u32,
    pub region: BufferRegion,
}
#[derive(Clone)]
pub struct IndexBuffer {
    pub region: BufferRegion,
    pub format: wgpu::IndexFormat,
}
#[derive(Clone)]
pub struct Geometry {
    pub context: Option<ContextId>,
    pub vertices: Vec<VertexBuffer>,
    pub indices: Option<IndexBuffer>,
    pub elements: Range<u32>,
    pub base_vertex: i32,
}
pub type GeometryId = ResourceId<Geometry>;
storage::stored!(Geometry);
pub use storage::{get, remove};

impl Geometry {
    pub fn generated(context: Option<ContextId>, elements: Range<u32>) -> Self {
        Self {
            context,
            vertices: vec![],
            indices: None,
            elements,
            base_vertex: 0,
        }
    }
    pub fn validate(&self) -> RenderResult<()> {
        let device = super::gpu(self.context)?.device.clone();
        if self.elements.start > self.elements.end {
            return Err(RenderError::new("geometry", "reversed element range"));
        }
        let mut slots = std::collections::BTreeSet::new();
        for vertex in &self.vertices {
            vertex
                .region
                .validate(self.context, wgpu::BufferUsages::VERTEX)?;
            if vertex.slot >= device.limits().max_vertex_buffers
                || !slots.insert(vertex.slot)
                || vertex.region.bytes.start % 4 != 0
            {
                return Err(RenderError::new(
                    "geometry",
                    "duplicate, unaligned or out-of-bounds vertex slot",
                ));
            }
        }
        if let Some(index) = &self.indices {
            index
                .region
                .validate(self.context, wgpu::BufferUsages::INDEX)?;
            let stride = match index.format {
                wgpu::IndexFormat::Uint16 => 2,
                wgpu::IndexFormat::Uint32 => 4,
            };
            if index.region.bytes.start % stride != 0
                || self.elements.end as u64 * stride > index.region.len()
            {
                return Err(RenderError::new(
                    "geometry",
                    "index range exceeds allocation or is unaligned",
                ));
            }
        } else if self.base_vertex != 0 {
            return Err(RenderError::new(
                "geometry",
                "base vertex requires indexed drawing",
            ));
        }
        Ok(())
    }
}
pub fn try_register(geometry: Geometry) -> RenderResult<OwnedResource<GeometryId>> {
    geometry.validate()?;
    Ok(storage::insert(geometry))
}
pub fn try_update(id: GeometryId, geometry: Geometry) -> RenderResult<()> {
    geometry.validate()?;
    if !storage::replace(id, geometry) {
        return Err(RenderError::new("geometry", "missing resource"));
    }
    Ok(())
}

/// Meshes use Velum's established vertex format, shared by sprites and tessellation.
pub fn try_mesh(
    context: Option<ContextId>,
    mesh: &crate::geometry::Mesh,
) -> RenderResult<OwnedResource<GeometryId>> {
    if mesh.vertices().is_empty()
        || mesh.indices().is_empty()
        || mesh
            .indices()
            .iter()
            .any(|&i| i as usize >= mesh.vertices().len())
    {
        return Err(RenderError::new(
            "mesh",
            "empty mesh or invalid vertex index",
        ));
    }
    let vertices = buffer::allocate_in::<crate::geometry::Vertex, buffer::Vertex>(
        context,
        mesh.vertices().len() as u64,
    )
    .map_err(|e| RenderError::new("mesh vertices", e.to_string()))?;
    let indices = buffer::allocate_in::<u32, buffer::Index>(context, mesh.indices().len() as u64)
        .map_err(|e| RenderError::new("mesh indices", e.to_string()))?;
    buffer::write_buffer(&vertices, 0, mesh.vertices());
    buffer::write_buffer(&indices, 0, mesh.indices());
    try_register(Geometry {
        context,
        vertices: vec![VertexBuffer {
            slot: 0,
            region: BufferRegion::from_shared(vertices.share())?,
        }],
        indices: Some(IndexBuffer {
            region: BufferRegion::from_shared(indices.share())?,
            format: wgpu::IndexFormat::Uint32,
        }),
        elements: 0..u32::try_from(mesh.indices().len())
            .map_err(|_| RenderError::new("mesh", "too many indices"))?,
        base_vertex: 0,
    })
}
