use std::ops::Range;
use std::sync::{OnceLock, RwLock};

use new_collection::{Arena, Id};

use super::buffer::{self, Buffer, Index, Vertex};
use super::{RenderInstance, instances::cache};
use crate::backend::instances::Instance;
use crate::geometry::mesh::{Mesh, Vertex as MeshVertex};

pub struct MeshResource {
    vertices: Buffer<MeshVertex, Vertex>,
    indices: Buffer<u32, Index>,
    index_count: u32,
}

static MESHES: OnceLock<RwLock<Arena<MeshResource>>> = OnceLock::new();

pub fn register(mesh: Mesh) -> Id<MeshResource> {
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
            buffer::delete_buffer(vertices);
            panic!("Impossible d'allouer les indices du mesh: {error}");
        }
    };
    buffer::write_buffer(&vertices, 0, mesh.vertices());
    buffer::write_buffer(&indices, 0, mesh.indices());
    MESHES
        .get_or_init(Default::default)
        .write()
        .unwrap()
        .insert(MeshResource {
            vertices,
            indices,
            index_count,
        })
}

pub trait ShapeInstance: Instance {
    fn mesh(&self) -> Id<MeshResource>;
    fn render_instance(&self) -> RenderInstance;
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
            _padding: [0; 2],
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
    let start = range.start;
    let slice = &mut instances[start as usize..range.end as usize];
    slice.sort_by_key(ShapeInstance::mesh);
    let mut batches: Vec<Batch> = Vec::new();
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
    batches
}

pub fn draw<T: ShapeInstance>(instances: &mut [T], batch: &Batch, pass: &mut wgpu::RenderPass<'_>) {
    if batch.range.is_empty() {
        return;
    }
    let Some(meshes) = MESHES.get() else {
        tracing::error!("No meshes registered");
        return;
    };
    let meshes = meshes.read().unwrap();
    let Some(mesh) = meshes.get(batch.mesh) else {
        tracing::error!("Unknown mesh {:?}", batch.mesh);
        return;
    };
    let (vertices, vertex_range) = mesh.vertices.raw_region();
    let (indices, index_range) = mesh.indices.raw_region();
    let render_instances = instances[batch.range.start as usize..batch.range.end as usize]
        .iter()
        .map(ShapeInstance::render_instance);
    let group = cache::prepare_render_instances(render_instances, batch.range.clone());
    pass.set_bind_group(0, &group, &[]);
    pass.set_vertex_buffer(0, vertices.slice(vertex_range));
    pass.set_index_buffer(indices.slice(index_range), wgpu::IndexFormat::Uint32);
    
    let instance_count = batch.range.end - batch.range.start;
    pass.draw_indexed(0..mesh.index_count, 0, 0..instance_count);
}

pub fn draw_all<T: ShapeInstance>(instances: &mut [T], pass: &mut wgpu::RenderPass<'_>) {
    let end = u32::try_from(instances.len()).expect("Too many shape instances");
    for batch in batch(instances, 0..end) {
        draw(instances, &batch, pass);
    }
}
