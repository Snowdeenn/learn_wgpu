use super::instance::Dummy;
use crate::backend::buffer::{Buffer, Index, StorageRead, StorageVertex, Uniform};
use crate::geometry::mesh::{Mesh, Vertex};
use math::Mat4;

crate::unique_resource!(CamBuffer, Buffer<Mat4, Uniform>);
crate::unique_resource!(InstanceBuffer, Buffer<crate::backend::RenderInstance, StorageRead>);
crate::unique_resource!(VertexBuffer, Buffer<Vertex, StorageVertex>);
crate::unique_resource!(IndexBuffer, Buffer<u32, Index>);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScenePipeline {
    #[default]
    Mesh,
    Texture,
}

crate::unique_resource!(InstanceData, Vec<Dummy>);
crate::unique_resource!(IndexCount, u32);
crate::unique_resource!(RectMesh, Mesh);
crate::unique_resource!(CircleMesh, Mesh);
crate::unique_resource!(TriangleMesh, Mesh);
crate::unique_resource!(Fps, f32);

impl std::fmt::Display for Fps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.0)
    }
}

crate::define_pipeline_context!(Mesh);
crate::define_pipeline_context!(Texture);
