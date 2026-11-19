use super::{
    BindingUse, Geometry, GeometryId, InstanceBinding, InstanceStream, Material, MaterialId,
    RenderError, RenderResult,
    bindings::{self, BindingResource, BindingSet, ResolvedGroup},
    material::{self, TemplateResource},
};
use crate::backend::{
    DrawSortKey, SortPolicy,
    pass::{PassError, RenderOperation},
};
use crate::{RenderData, RenderDataId};
use std::{ops::Range, sync::Arc, time::Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseId(pub u32);
impl PhaseId {
    pub const OPAQUE: Self = Self(0);
    pub const TRANSPARENT: Self = Self(1);
    pub const OVERLAY: Self = Self(2);
}
#[derive(Clone)]
pub struct RenderObject {
    pub geometry: GeometryId,
    pub material: MaterialId,
    pub instances: Range<u32>,
    pub instance_stream: Option<InstanceStream>,
    pub bindings: Vec<BindingUse>,
    pub phase: PhaseId,
    pub sort_key: DrawSortKey,
}
impl RenderObject {
    pub fn new(geometry: GeometryId, material: MaterialId) -> Self {
        Self {
            geometry,
            material,
            instances: 0..1,
            instance_stream: None,
            bindings: vec![],
            phase: PhaseId::OPAQUE,
            sort_key: DrawSortKey::default(),
        }
    }
    pub fn with_instances(mut self, stream: InstanceStream, range: Range<u32>) -> Self {
        self.instance_stream = Some(stream);
        self.instances = range;
        self
    }
    fn compatible(&self, other: &Self) -> bool {
        self.geometry == other.geometry
            && self.material == other.material
            && self.phase == other.phase
            && self.instances.end == other.instances.start
            && self
                .bindings
                .iter()
                .map(|b| (b.set, &b.dynamic_offsets, b.replace))
                .eq(other
                    .bindings
                    .iter()
                    .map(|b| (b.set, &b.dynamic_offsets, b.replace)))
            && match (&self.instance_stream, &other.instance_stream) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    a.region == b.region
                        && a.binding == b.binding
                        && a.count == b.count
                        && a.stride == b.stride
                }
                _ => false,
            }
    }
}

/// Owns resolved snapshots and all allocation leases through encoding.
pub struct PreparedDraw {
    geometry: Arc<Geometry>,
    _material: Arc<Material>,
    template: Arc<TemplateResource>,
    groups: Vec<ResolvedGroup>,
    stream: Option<InstanceStream>,
    instances: Range<u32>,
}
pub fn prepare(object: &RenderObject, common: &[BindingUse]) -> RenderResult<PreparedDraw> {
    let start = Instant::now();
    let result = prepare_inner(object, common);
    crate::backend::report::record_preparation(start.elapsed());
    if let Err(error) = &result {
        crate::backend::report::diagnostic("draw preparation", error.to_string());
    }
    result
}
fn prepare_inner(object: &RenderObject, common: &[BindingUse]) -> RenderResult<PreparedDraw> {
    let geometry = super::geometry::get(object.geometry)
        .ok_or_else(|| RenderError::new("render object", "missing geometry"))?;
    let material = material::get(object.material)
        .ok_or_else(|| RenderError::new("render object", "missing material"))?;
    let template = material::get(material.template.id())
        .ok_or_else(|| RenderError::new("render object", "missing template"))?;
    let desc = &template.description;
    if geometry.context != desc.context {
        return Err(RenderError::new("render object", "GPU context mismatch"));
    }
    geometry.validate()?;
    if object.instances.start > object.instances.end {
        return Err(RenderError::new("render object", "reversed instance range"));
    }
    let mut uses = common.to_vec();
    uses.extend(material.bindings.iter().map(|owner| owner.id().into()));
    uses.extend(object.bindings.iter().cloned());
    let mut instance_owner = None;
    if let Some(stream) = &object.instance_stream {
        if object.instances.end > stream.count
            || stream.count as u64 * stream.stride > stream.region.len()
        {
            return Err(RenderError::new(
                "instances",
                "draw range exceeds instance stream",
            ));
        }
        match &stream.binding {
            InstanceBinding::Storage { group, binding } => {
                stream
                    .region
                    .validate(desc.context, wgpu::BufferUsages::STORAGE)?;
                let entry = template.layouts.schema.entry(*group, *binding)?;
                if !matches!(
                    entry.ty,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        ..
                    }
                ) {
                    return Err(RenderError::new(
                        "instances",
                        "storage instances require a read-only non-dynamic storage entry",
                    ));
                }
                let owner = bindings::try_register(BindingSet {
                    context: desc.context,
                    group: *group,
                    entries: vec![(*binding, BindingResource::buffer(stream.region.clone()))],
                })?;
                uses.push(owner.id().into());
                instance_owner = Some(owner);
            }
            InstanceBinding::Vertex { slot, layout } => {
                stream
                    .region
                    .validate(desc.context, wgpu::BufferUsages::VERTEX)?;
                if desc
                    .vertex_layouts
                    .get(*slot as usize)
                    .and_then(Option::as_ref)
                    != Some(layout)
                    || geometry.vertices.iter().any(|v| v.slot == *slot)
                {
                    return Err(RenderError::new(
                        "instances",
                        "vertex instance layout mismatch or conflicting vertex slot",
                    ));
                }
            }
        }
    }
    for (slot, layout) in desc.vertex_layouts.iter().enumerate() {
        let Some(layout) = layout else {
            continue;
        };
        if let Some(stream) = &object.instance_stream
            && matches!(&stream.binding, InstanceBinding::Vertex { slot: s, .. } if *s as usize == slot)
        {
            continue;
        }
        let vertex = geometry
            .vertices
            .iter()
            .find(|v| v.slot as usize == slot)
            .ok_or_else(|| {
                RenderError::new("geometry", format!("missing vertex buffer at slot {slot}"))
            })?;
        let count = match layout.step_mode {
            wgpu::VertexStepMode::Instance => object.instances.end,
            wgpu::VertexStepMode::Vertex if geometry.indices.is_none() => geometry.elements.end,
            _ => 0,
        };
        // Last attribute, not trailing padding, determines the necessary bytes.
        let attribute_end = layout
            .attributes
            .iter()
            .map(|a| a.offset + a.format.size())
            .max()
            .unwrap_or(0);
        let required = if count == 0 {
            0
        } else {
            (count as u64 - 1)
                .checked_mul(layout.stride)
                .and_then(|n| n.checked_add(attribute_end))
                .ok_or_else(|| RenderError::new("geometry", "vertex range overflow"))?
        };
        if required > vertex.region.len() {
            return Err(RenderError::new(
                "geometry",
                "vertex range exceeds allocation",
            ));
        }
    }
    for vertex in &geometry.vertices {
        if desc
            .vertex_layouts
            .get(vertex.slot as usize)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(RenderError::new(
                "geometry",
                "vertex buffer has no template layout",
            ));
        }
    }
    let groups = bindings::resolve(desc.context, &template.layouts, &uses)?;
    drop(instance_owner);
    Ok(PreparedDraw {
        geometry,
        _material: material,
        template,
        groups,
        stream: object.instance_stream.clone(),
        instances: object.instances.clone(),
    })
}

impl PreparedDraw {
    pub fn encode(&self, pass: &mut wgpu::RenderPass<'_>) -> RenderResult<()> {
        if self.instances.is_empty() || self.geometry.elements.is_empty() {
            return Ok(());
        }
        super::pipeline::validate_current_attachments(&self.template.description)?;
        let pipeline = crate::backend::pipeline::get(self.template.pipeline.id())
            .ok_or_else(|| RenderError::new("draw", "missing pipeline context"))?;
        super::retain((
            self.geometry.clone(),
            self._material.clone(),
            self.template.clone(),
            self.stream.clone(),
            self.groups
                .iter()
                .flat_map(|g| g._resources.iter().cloned())
                .collect::<Vec<_>>(),
        ));
        pass.set_pipeline(&pipeline.pipeline);
        for group in &self.groups {
            pass.set_bind_group(group.index, &group.group, &group.offsets);
        }
        for vertex in &self.geometry.vertices {
            pass.set_vertex_buffer(
                vertex.slot,
                vertex.region.buffer.slice(vertex.region.bytes.clone()),
            );
        }
        if let Some(stream) = &self.stream
            && let InstanceBinding::Vertex { slot, .. } = stream.binding
        {
            pass.set_vertex_buffer(
                slot,
                stream.region.buffer.slice(stream.region.bytes.clone()),
            );
        }
        if let Some(index) = &self.geometry.indices {
            pass.set_index_buffer(
                index.region.buffer.slice(index.region.bytes.clone()),
                index.format,
            );
            pass.draw_indexed(
                self.geometry.elements.clone(),
                self.geometry.base_vertex,
                self.instances.clone(),
            );
        } else {
            pass.draw(self.geometry.elements.clone(), self.instances.clone());
        }
        crate::backend::metrics::draw();
        crate::backend::report::record_instances(
            (self.instances.end - self.instances.start) as u64,
        );
        Ok(())
    }
}
pub fn draw(
    object: &RenderObject,
    common: &[BindingUse],
    pass: &mut wgpu::RenderPass<'_>,
) -> RenderResult<()> {
    let draw = prepare(object, common)?;
    let start = Instant::now();
    let result = draw.encode(pass);
    crate::backend::report::record_encoding(start.elapsed());
    if let Err(error) = &result {
        crate::backend::report::diagnostic("draw encoding", error.to_string());
    }
    result
}

/// Stable Velum sorting with reusable storage. Only adjacent compatible ranges merge.
pub struct RenderPhase {
    pub id: PhaseId,
    pub policy: SortPolicy,
    pub common: Vec<BindingUse>,
    objects: Vec<RenderObject>,
    prepared: Vec<PreparedDraw>,
    batches: Vec<RenderObject>,
}
impl RenderPhase {
    pub fn new(id: PhaseId, policy: SortPolicy) -> Self {
        Self {
            id,
            policy,
            common: vec![],
            objects: vec![],
            prepared: vec![],
            batches: vec![],
        }
    }
    pub fn push(&mut self, object: RenderObject) {
        self.objects.push(object);
    }
    pub fn objects(&self) -> &[RenderObject] {
        &self.objects
    }
    pub fn clear(&mut self) {
        self.objects.clear();
        self.prepared.clear();
        self.batches.clear();
    }
    fn prepare(&mut self) -> RenderResult<()> {
        for object in &self.objects {
            if object.phase != self.id {
                return Err(RenderError::new(
                    "phase",
                    "object submitted to a different phase",
                ));
            }
            if matches!(
                self.policy,
                SortPolicy::FrontToBack | SortPolicy::BackToFront
            ) && !object.sort_key.depth.is_finite()
            {
                return Err(RenderError::new("phase", "depth must be finite"));
            }
        }
        match self.policy {
            SortPolicy::PreserveOrder => {}
            SortPolicy::DrawOrder => self.objects.sort_by_key(|o| o.sort_key.draw_order),
            SortPolicy::FrontToBack => self
                .objects
                .sort_by(|a, b| a.sort_key.depth.partial_cmp(&b.sort_key.depth).unwrap()),
            SortPolicy::BackToFront => self
                .objects
                .sort_by(|a, b| b.sort_key.depth.partial_cmp(&a.sort_key.depth).unwrap()),
        }
        self.batches.clear();
        for object in &self.objects {
            if let Some(last) = self.batches.last_mut()
                && last.compatible(object)
            {
                last.instances.end = object.instances.end;
            } else {
                self.batches.push(object.clone());
            }
        }
        self.prepared.clear();
        for (index, object) in self.batches.iter().enumerate() {
            self.prepared
                .push(prepare(object, &self.common).map_err(|error| {
                    RenderError::new(
                        format!("phase {} batch {index}, {}", self.id.0, error.context),
                        error.message,
                    )
                })?);
        }
        // Validate every attachment before encoding the first object in a phase.
        for draw in &self.prepared {
            super::pipeline::validate_current_attachments(&draw.template.description)?;
        }
        Ok(())
    }
    pub fn execute(&mut self, pass: &mut wgpu::RenderPass<'_>) -> RenderResult<()> {
        let result = self.prepare().and_then(|_| {
            let start = Instant::now();
            let result = self.prepared.iter().try_for_each(|draw| {
                if !draw.instances.is_empty() && !draw.geometry.elements.is_empty() {
                    crate::backend::metrics::batch();
                }
                draw.encode(pass)
            });
            crate::backend::report::record_encoding(start.elapsed());
            result
        });
        if let Err(error) = &result {
            crate::backend::report::diagnostic(format!("phase {}", self.id.0), error.to_string());
        }
        self.clear();
        result
    }
}
/// Existing `RenderSequence`, render-operation macros and scene schedules accept this.
pub struct PhaseOperation {
    pub phase: RenderDataId<RenderPhase>,
}
impl RenderOperation for PhaseOperation {
    fn execute(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        data: &mut RenderData,
    ) -> Result<(), PassError> {
        data.require_mut(self.phase)?
            .execute(pass)
            .map_err(PassError::from)
    }
}
