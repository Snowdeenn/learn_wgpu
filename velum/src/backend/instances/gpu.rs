use crate::backend::{BufferRegion, RenderError, RenderResult, VertexLayout};
use crate::backend::{buffer, context::ContextId, instances::Instance};

/// GPU-transferable application data. `Pod` makes padding and byte-copy rules explicit.
/// The associated binding is checked against the material's schema/layout at draw time.
pub trait GpuInstance: Instance + bytemuck::Pod + Send + Sync {
    fn binding() -> InstanceBinding;
}

/// CPU application object with an associated GPU representation, without a fixed
/// `RenderInstance` conversion. Metadata and IDs can stay outside its Pod payload.
pub trait InstanceSource: Instance {
    type Gpu: bytemuck::Pod + Send + Sync;
    fn gpu_data(&self) -> Self::Gpu;
    fn binding() -> InstanceBinding;
}
pub trait ObjectInstance: InstanceSource {
    fn geometry(&self) -> crate::backend::GeometryId;
    fn material(&self) -> crate::backend::MaterialId;
    fn sort_key(&self) -> crate::backend::DrawSortKey {
        Default::default()
    }
}
/// Shapes, sprites and custom application objects all submit the same draw model.
pub fn submit<T: ObjectInstance>(
    context: Option<ContextId>,
    instances: &[T],
    phase: &mut crate::backend::object::RenderPhase,
) -> RenderResult<()> {
    let stride = std::mem::size_of::<T::Gpu>();
    if stride == 0 || stride > buffer::BufferArena::PAGE_SIZE as usize {
        return Err(RenderError::new("instances", "invalid GPU stride"));
    }
    let mut pending = vec![];
    for chunk in instances.chunks(buffer::BufferArena::PAGE_SIZE as usize / stride) {
        let values: Vec<_> = chunk.iter().map(InstanceSource::gpu_data).collect();
        let stream = InstanceStream::upload_as(context, &values, T::binding())?;
        for (i, instance) in chunk.iter().enumerate() {
            let mut object =
                crate::backend::RenderObject::new(instance.geometry(), instance.material())
                    .with_instances(stream.clone(), i as u32..i as u32 + 1);
            object.sort_key = instance.sort_key();
            object.phase = phase.id;
            pending.push(object);
        }
    }
    for object in pending {
        phase.push(object);
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceBinding {
    Storage { group: u32, binding: u32 },
    Vertex { slot: u32, layout: VertexLayout },
}
#[derive(Clone)]
pub struct InstanceStream {
    pub(crate) region: BufferRegion,
    pub(crate) count: u32,
    pub(crate) stride: u64,
    pub(crate) binding: InstanceBinding,
}
impl InstanceStream {
    pub fn upload<T: GpuInstance>(context: Option<ContextId>, values: &[T]) -> RenderResult<Self> {
        Self::upload_as(context, values, T::binding())
    }
    /// Allows the application's layout to place instances at different locations.
    pub fn upload_as<T: bytemuck::Pod + Send + Sync + 'static>(
        context: Option<ContextId>,
        values: &[T],
        binding: InstanceBinding,
    ) -> RenderResult<Self> {
        if values.is_empty() || std::mem::size_of::<T>() == 0 {
            return Err(RenderError::new("instances", "empty or zero-sized stream"));
        }
        let count = u32::try_from(values.len())
            .map_err(|_| RenderError::new("instances", "too many instances"))?;
        let stride = std::mem::size_of::<T>() as u64;
        if let InstanceBinding::Vertex { layout, .. } = &binding {
            if layout.stride != stride || layout.step_mode != wgpu::VertexStepMode::Instance {
                return Err(RenderError::new(
                    "instances",
                    "instance type differs from vertex stride or step mode",
                ));
            }
        }
        let owner = buffer::allocate_in::<T, buffer::StorageVertex>(context, count as u64)
            .map_err(|e| RenderError::new("instances", e.to_string()))?;
        buffer::try_write_buffer(&owner, 0, values)
            .map_err(|e| RenderError::new("instances", e.to_string()))?;
        crate::backend::metrics::allocation();
        Ok(Self {
            region: BufferRegion::from_shared(owner.share())?,
            count,
            stride,
            binding,
        })
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn binding(&self) -> &InstanceBinding {
        &self.binding
    }
    pub fn region(&self) -> &BufferRegion {
        &self.region
    }
}
