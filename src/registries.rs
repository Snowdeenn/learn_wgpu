//! Shared resource registries for applications embedding Velum.
//!
//! These are the backend's own modules: registration through this API and
//! rendering through the backend use the same global storages. Registries are
//! shared by all scenes and all `Velum` instances in this process.
//!
//! Initialize [`context::init`] before creating GPU resources. Keep the returned
//! [`crate::OwnedResource`] (or a shared owner) alive while its ID is in use.
//! Dropping an owner removes its entry; copying an ID does not retain it.
//!
//! ```no_run
//! use velum::registries::{context, target};
//!
//! fn initialize(adapter: &wgpu::Adapter, descriptor: target::RenderTargetDescriptor)
//!     -> Result<velum::OwnedResource<target::RenderTargetId>, context::InitError>
//! {
//!     context::init(adapter)?;
//!     let owner = target::register(descriptor);
//!     let target = target::get(owner.id()).expect("registered target");
//!     Ok(owner)
//! }
//! ```

pub use crate::backend::pipeline;
pub use crate::backend::pipeline::compute;
pub use crate::backend::{binding_set, buffer, context, shape, target, text, texture};
pub use crate::particle;
