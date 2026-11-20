//! GPU resources, materials and drawing operations.
//!
//! ```
//! use velum::backend::{
//!     geometry::Geometry,
//!     material::{Material, MaterialTemplate},
//!     object::{PhaseId, RenderObject, RenderPhase},
//!     SortPolicy,
//! };
//!
//! let geometry = Geometry::generated(None, 0..3);
//! let phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
//! ```

pub mod bind_group;
pub mod binding_set;
pub mod buffer;
pub mod context;
pub mod depth;
pub mod frame_data;
pub mod metrics;
pub mod pass;
pub mod pipeline;
pub mod renderer;
pub mod report;
pub mod sampler;
pub mod target;
pub mod texture;

pub mod instances;

pub mod bindings;
pub mod geometry;
pub mod material;
mod material_macros;
pub mod object;
pub mod phase;
mod render;
mod shader;
pub mod shape;
pub mod sort;
pub mod sprite;
pub mod standard;
mod storage;
pub mod text;

pub use bindings::{
    BindingEntry, BindingResource, BindingSchema, BindingSet, BindingSetId, BindingUse, GroupSchema,
};
pub use geometry::{BufferRegion, Geometry, GeometryId, IndexBuffer, VertexBuffer};
pub use instances::{GpuInstance, InstanceBinding, InstanceSource, InstanceStream, ObjectInstance};
pub use material::{
    Material, MaterialId, MaterialTemplate, MaterialTemplateId, ShaderEntry, VertexLayout,
};
pub use object::{PhaseId, RenderObject};
pub use phase::{DrawCommand, DrawError, DrawGeometry, RenderPhase};
pub use render::{RenderError, RenderResult, discard, submitted, try_shader};
pub(crate) use render::{checked, gpu, retain};
pub use sort::{DrawSortKey, SortPolicy};

pub use buffer::write_buffer;
pub use pipeline::{PipelineContext, bind};
pub use shader::load_shader;
#[allow(
    unused_imports,
    reason = "Reexport de l'API du moteur, non utilise par la scene actuelle."
)]
pub use sprite::{RenderInstance, Sprite, SpriteInstance, batch, draw, draw_all};
pub use texture::{
    build_atlas_region, load_texture, load_texture_array, register_atlas, register_texture,
    register_texture_array,
};

#[macro_export]
macro_rules! create_marker {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name;
        )*
    };
}
