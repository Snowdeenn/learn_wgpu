use super::instance::Dummy;
use crate::backend::buffer::{Buffer, Index, StorageRead, StorageVertex, Uniform};
use crate::backend::frame_data::FrameData;
use crate::backend::instances::InstanceId;
use crate::backend::pass::PassList;
use crate::backend::shape::MeshResource;
use crate::geometry::mesh::Vertex;
use math::Mat4;
use new_collection::Id;

use crate::backend::pipeline::PipelineId;
use crate::backend::target::RenderTargetId;

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

pub type InstanceData = crate::backend::instances::InstanceData<Dummy>;
crate::unique_resource!(IndexCount, u32);
crate::unique_resource!(RectMesh, Id<MeshResource>);
crate::unique_resource!(CircleMesh, Id<MeshResource>);
crate::unique_resource!(TriangleMesh, Id<MeshResource>);
crate::unique_resource!(Fps, f32);

impl std::fmt::Display for Fps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.0)
    }
}

crate::define_pipeline_context!(Mesh);
crate::define_pipeline_context!(Texture);
crate::define_pipeline_context!(TextureArray);

pub struct SceneRendering {
    pub data: FrameData,
    pub instances: InstanceId<Dummy>,
    pub passes: PassList,
    pub target: RenderTargetId,
    pub blur: crate::effects::blur::BlurEffect,
    pub present: crate::effects::present::PresentEffect,
    pub composition: crate::effects::composite::CompositeEffect,
}

impl SceneRendering {
    pub fn instances_mut(&mut self) -> &mut InstanceData {
        self.data
            .instances
            .get_mut(&self.instances)
            .expect("Instances de la scène introuvables")
    }
}

#[derive(Clone, Copy)]
pub struct ScenePipelines {
    pub mesh: PipelineId<MeshPipelineContext>,
    pub texture: PipelineId<TexturePipelineContext>,
    pub texture_array: PipelineId<TextureArrayPipelineContext>,
}
