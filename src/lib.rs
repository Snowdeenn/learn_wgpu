//! Scene orchestration and rendering tools for applications embedding Velum.
//!
//! ```
//! use velum::{scene, RenderData, Scene, Velum};
//!
//! let mut data = RenderData::new();
//! let position = data.insert([0.0_f32, 0.0]);
//! let renderer = scene::renderer().build();
//! let mut velum = Velum::new();
//! let world = velum.add_scene(Scene { renderer, data });
//! velum.activate(world)?;
//! velum.scene_mut(world).unwrap().data.require_mut(position)?[0] = 10.0;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
pub mod api;
pub mod backend;
pub mod camera;
pub mod core;
pub mod effects;
pub mod geometry;
pub mod particle;

pub use api::{SceneError, Velum, scene};
pub use api::scene::{RenderData, RenderDataError, RenderDataId, Scene, SceneId, SceneRenderer};
pub use core::{OwnedResource, ResourceHandle, SharedResource};
