use crate::backend::DrawSortKey;
use crate::core::ResourceHandle;
use crate::core::{OwnedResource, SharedResource};
use std::ops::Range;
use std::sync::{OnceLock, RwLock};

use new_collection::{Arena, Id};

use super::buffer::{self, Buffer, Index, Vertex};
use super::{RenderInstance, instances::cache};
use crate::backend;
use crate::backend::instances::Instance;
use crate::geometry::mesh::{Mesh, Vertex as MeshVertex};

pub struct MeshResource {
    vertices: SharedResource<Buffer<MeshVertex, Vertex>>,
    indices: SharedResource<Buffer<u32, Index>>,
    index_count: u32,
    geometry: SharedResource<super::GeometryId>,
}

static MESHES: OnceLock<RwLock<Arena<MeshResource>>> = OnceLock::new();

/// Removes a mesh and frees its vertex and index allocations.
pub fn remove(id: Id<MeshResource>) -> bool {
    let Some(storage) = MESHES.get() else {
        return false;
    };
    let mesh = {
        let mut meshes = storage.write().unwrap();
        if meshes.get(id).is_none() {
            return false;
        }
        meshes.remove(id)
    };
    let Some(mesh) = mesh else {
        return false;
    };
    drop(mesh.vertices);
    drop(mesh.indices);
    true
}

impl ResourceHandle for Id<MeshResource> {
    fn release(self) {
        remove(self);
    }
}

pub fn register(mesh: Mesh) -> OwnedResource<Id<MeshResource>> {
    assert!(!mesh.vertices().is_empty(), "A mesh must contain vertices");
    assert!(!mesh.indices().is_empty(), "A mesh must contain indices");
    assert!(
        mesh.indices()
            .iter()
            .all(|&index| (index as usize) < mesh.vertices().len()),
        "Mesh index out of bounds"
    );
    let index_count = u32::try_from(mesh.indices().len()).expect("Too many mesh indices");
    let vertices = buffer::vertex(mesh.vertices().len() as u64)
        .expect("Impossible d'allouer les sommets du mesh");
    let indices = match buffer::index(mesh.indices().len() as u64) {
        Ok(indices) => indices,
        Err(error) => {
            drop(vertices);
            panic!("Impossible d'allouer les indices du mesh: {error}");
        }
    };
    buffer::write_buffer(&vertices, 0, mesh.vertices());
    buffer::write_buffer(&indices, 0, mesh.indices());
    let vertices = vertices.share();
    let indices = indices.share();
    let geometry = super::geometry::try_register(super::Geometry {
        context: None,
        vertices: vec![super::VertexBuffer {
            slot: 0,
            region: super::BufferRegion::from_shared(vertices.clone())
                .expect("Valid mesh allocation"),
        }],
        indices: Some(super::IndexBuffer {
            region: super::BufferRegion::from_shared(indices.clone())
                .expect("Valid mesh allocation"),
            format: wgpu::IndexFormat::Uint32,
        }),
        elements: 0..index_count,
        base_vertex: 0,
    })
    .expect("Valid mesh geometry")
    .share();
    let id = MESHES
        .get_or_init(Default::default)
        .write()
        .unwrap()
        .insert(MeshResource {
            vertices,
            indices,
            index_count,
            geometry,
        });
    OwnedResource::new(id)
}

/// Shared geometry for both the historical shape API and described materials.
pub fn geometry(id: Id<MeshResource>) -> Option<SharedResource<super::GeometryId>> {
    Some(MESHES.get()?.read().unwrap().get(id)?.geometry.clone())
}

pub trait ShapeInstance: Instance {
    fn mesh(&self) -> Id<MeshResource>;
    fn render_instance(&self) -> RenderInstance;
    /// Override to supply an explicit order or avoid constructing a render instance.
    fn sort_key(&self) -> DrawSortKey {
        DrawSortKey {
            draw_order: 0,
            depth: self.render_instance().depth,
        }
    }
}

#[allow(
    dead_code,
    reason = "API du moteur conservee pour les autres scenes et modules."
)]
pub struct Shape {
    pub mesh: Id<MeshResource>,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [f32; 4],
    pub ground_y: f32,
}

impl Instance for Shape {}

impl ShapeInstance for Shape {
    fn mesh(&self) -> Id<MeshResource> {
        self.mesh
    }

    fn render_instance(&self) -> RenderInstance {
        RenderInstance {
            offset: self.position,
            scale: self.scale,
            color: self.color,
            layer: 0,
            rotation: 0.0,
            // TODO: Changer les valeurs min_y et max_y pour pas les hardcoder
            depth: backend::depth::depth_from_ground_y(self.ground_y, 0.0, 1080.0),
            _padding: 0,
            uv_min: [0.0; 2],
            uv_max: [1.0; 2],
        }
    }
}

pub struct Batch {
    pub mesh: Id<MeshResource>,
    pub range: Range<u32>,
}

pub fn batch<T: ShapeInstance>(instances: &mut [T], range: Range<u32>) -> Vec<Batch> {
    let mut batches = Vec::new();
    batch_into(instances, range, &mut batches);
    batches
}

/// Groups consecutive compatible shapes without changing their draw order.
pub fn batch_into<T: ShapeInstance>(instances: &[T], range: Range<u32>, batches: &mut Vec<Batch>) {
    let start = range.start;
    let slice = &instances[start as usize..range.end as usize];
    batches.clear();
    for (offset, instance) in slice.iter().enumerate() {
        let mesh = instance.mesh();
        let index = start + offset as u32;
        if let Some(last) = batches.last_mut()
            && last.mesh == mesh
        {
            last.range.end = index + 1;
            continue;
        }
        batches.push(Batch {
            mesh,
            range: index..index + 1,
        });
    }
}

pub(crate) fn contains(id: Id<MeshResource>) -> bool {
    MESHES
        .get()
        .is_some_and(|meshes| meshes.read().unwrap().get(id).is_some())
}

pub fn draw<T: ShapeInstance>(
    instances: &mut [T],
    batch: &Batch,
    pass: &mut wgpu::RenderPass<'_>,
) -> Result<(), crate::backend::instances::InstanceError> {
    if batch.range.start > batch.range.end || batch.range.end as usize > instances.len() {
        return Err(crate::backend::instances::InstanceError::InvalidRange);
    }
    if batch.range.is_empty() {
        return Ok(());
    }
    let Some(meshes) = MESHES.get() else {
        tracing::error!("No meshes registered");
        return Err(crate::backend::instances::InstanceError::MissingMesh);
    };
    let meshes = meshes.read().unwrap();
    let Some(mesh) = meshes.get(batch.mesh) else {
        tracing::error!("Unknown mesh {:?}", batch.mesh);
        return Err(crate::backend::instances::InstanceError::MissingMesh);
    };
    let (vertices, vertex_range) = mesh.vertices.id().raw_region();
    let (indices, index_range) = mesh.indices.id().raw_region();
    pass.set_vertex_buffer(0, vertices.slice(vertex_range));
    pass.set_index_buffer(indices.slice(index_range), wgpu::IndexFormat::Uint32);

    crate::backend::metrics::batch();
    for range in cache::chunk_ranges(batch.range.clone()) {
        let instance_count = range.end - range.start;
        let render_instances = instances[range.start as usize..range.end as usize]
            .iter()
            .map(ShapeInstance::render_instance);
        let group = cache::prepare_render_instances(render_instances, range)?;
        pass.set_bind_group(0, &group, &[]);
        crate::backend::metrics::draw();
        crate::backend::report::record_instances(instance_count as u64);
        pass.draw_indexed(0..mesh.index_count, 0, 0..instance_count);
    }
    Ok(())
}

pub fn draw_all<T: ShapeInstance>(
    instances: &mut [T],
    pass: &mut wgpu::RenderPass<'_>,
) -> Result<(), crate::backend::instances::InstanceError> {
    let end = u32::try_from(instances.len())
        .map_err(|_| crate::backend::instances::InstanceError::BatchTooLarge)?;
    for batch in batch(instances, 0..end) {
        draw(instances, &batch, pass)?;
    }
    Ok(())
}
