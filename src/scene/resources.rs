use crate::api::scene_renderer::EffectId;
use super::instance::Dummy;
use crate::api::scene::{RenderDataId, SceneId};
use crate::backend::buffer::{Buffer, Index, StorageRead, StorageVertex, Uniform};
use crate::backend::shape::MeshResource;
use crate::geometry::mesh::Vertex;
use math::Mat4;
use new_collection::Id;

use crate::backend;
use crate::backend::pipeline::PipelineId;

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

/// CPU timings only: these do not measure GPU execution or presentation waits.
#[derive(Default)]
pub struct FrameTimings {
    pub simulation: CpuTiming,
    pub preparation: CpuTiming,
    pub render_encoding: CpuTiming,
    pub surface_acquire: CpuTiming,
    pub gpu_wait: CpuTiming,
    pub measure_gpu_wait: bool,
    pub render: backend::metrics::RenderMetrics,
    pub instance_slots: usize,
    pub instance_reserved_bytes: u64,
}

#[derive(Default)]
pub struct CpuTiming {
    pub last_ms: f64,
    pub average_ms: f64,
    initialized: bool,
}

impl CpuTiming {
    pub fn record(&mut self, duration: std::time::Duration) {
        self.last_ms = duration.as_secs_f64() * 1000.0;
        // Exponential moving average: 10% of the new sample, 90% of the history.
        self.average_ms = if self.initialized {
            self.average_ms + 0.1 * (self.last_ms - self.average_ms)
        } else {
            self.last_ms
        };
        self.initialized = true;
    }
}

impl std::fmt::Display for Fps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.0)
    }
}

crate::define_pipeline_context!(Mesh);
crate::define_pipeline_context!(Texture);
crate::define_pipeline_context!(TextureArray);

pub struct SceneRendering {
    pub scene: SceneId,
    pub mode: RenderDataId<ScenePipeline>,
    pub instances: RenderDataId<InstanceData>,
    pub texts: RenderDataId<Vec<backend::text::Text>>,
    pub blur: EffectId,
    pub composition: RenderDataId<crate::effects::composite::CompositeEffect>,
}

#[derive(Clone, Copy)]
pub struct ScenePipelines {
    pub mesh: PipelineId<MeshPipelineContext>,
    pub texture: PipelineId<TexturePipelineContext>,
    pub texture_array: PipelineId<TextureArrayPipelineContext>,
}
