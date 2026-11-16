//! Deferred 2D drawing using the existing shape/sprite batchers and upload cache.
//! Phases execute sequentially inside an already opened GPU pass. They do not
//! choose blending, depth state, camera or attachments; pipelines and passes do.
//!
//! Keep the phase in your operation and submit new commands for each image:
//! ```no_run
//! use velum::backend::{DrawCommand, RenderPhase, SortPolicy, PipelineContext};
//! use velum::backend::{pipeline::PipelineId, shape::ShapeInstance, phase::DrawError};
//!
//! fn render<P: PipelineContext + Send + Sync + 'static>(
//!     phase: &mut RenderPhase,
//!     pipeline: PipelineId<P>,
//!     objects: &[impl ShapeInstance],
//!     pass: &mut wgpu::RenderPass<'_>,
//! ) -> Result<(), DrawError> {
//!     for object in objects {
//!         phase.push(DrawCommand::shape(
//!             pipeline, object.mesh(), object.render_instance(), object.sort_key(),
//!         ));
//!     }
//!     phase.execute(pass) // Sorts, batches, draws, then clears pending commands.
//! }
//! let phase = RenderPhase::new(SortPolicy::BackToFront);
//! ```
use new_collection::Id;
use std::fmt;

use super::{
    bind, context,
    instances::Instance,
    pipeline::DrawPipeline,
    shape::{self, MeshResource, ShapeInstance},
    sort::{DrawSortKey, SortPolicy},
    sprite::{self, RenderInstance, Sprite, SpriteInstance},
    texture::TextureResource,
};

#[derive(Clone)]
pub enum DrawGeometry {
    Shape(Id<MeshResource>),
    Sprite(Sprite),
}

/// Owned CPU draw description. IDs do not keep global resources alive.
#[derive(Clone)]
pub struct DrawCommand {
    pub pipeline: DrawPipeline,
    pub geometry: DrawGeometry,
    pub instance: RenderInstance,
    pub sort_key: DrawSortKey,
    // Overrides for camera/material bindings. Instance and sprite texture slots
    // remain controlled by the existing backend drawing conventions.
    bindings: Vec<(u32, wgpu::BindGroup)>,
}

impl DrawCommand {
    pub fn shape(
        pipeline: impl Into<DrawPipeline>,
        mesh: Id<MeshResource>,
        instance: RenderInstance,
        sort_key: DrawSortKey,
    ) -> Self {
        Self {
            pipeline: pipeline.into(),
            geometry: DrawGeometry::Shape(mesh),
            instance,
            sort_key,
            bindings: Vec::new(),
        }
    }

    pub fn sprite(
        pipeline: impl Into<DrawPipeline>,
        sprite: Sprite,
        instance: RenderInstance,
        sort_key: DrawSortKey,
    ) -> Self {
        Self {
            pipeline: pipeline.into(),
            geometry: DrawGeometry::Sprite(sprite),
            instance,
            sort_key,
            bindings: Vec::new(),
        }
    }

    pub fn with_bind_group(mut self, slot: u32, group: wgpu::BindGroup) -> Self {
        if let Some(entry) = self.bindings.iter_mut().find(|entry| entry.0 == slot) {
            entry.1 = group;
        } else {
            self.bindings.push((slot, group));
            self.bindings.sort_by_key(|entry| entry.0);
        }
        self
    }

    fn same_state(&self, other: &Self) -> bool {
        self.pipeline == other.pipeline
            && self.bindings == other.bindings
            && std::mem::discriminant(&self.geometry) == std::mem::discriminant(&other.geometry)
    }
}

#[derive(Debug)]
pub struct DrawError {
    pub command: usize,
    pub reason: &'static str,
}
impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Draw command {}: {}", self.command, self.reason)
    }
}
impl std::error::Error for DrawError {}

struct PreparedShape {
    mesh: Id<MeshResource>,
    instance: RenderInstance,
}
impl Instance for PreparedShape {}
impl ShapeInstance for PreparedShape {
    fn mesh(&self) -> Id<MeshResource> {
        self.mesh
    }
    fn render_instance(&self) -> RenderInstance {
        self.instance
    }
}
struct PreparedSprite {
    sprite: Sprite,
    instance: RenderInstance,
}
impl Instance for PreparedSprite {}
impl SpriteInstance for PreparedSprite {
    fn sprite(&self) -> &Sprite {
        &self.sprite
    }
    fn sprite_mut(&mut self) -> &mut Sprite {
        &mut self.sprite
    }
    fn render_instance(&self) -> RenderInstance {
        self.instance
    }
}

/// Persistent phase configuration with reusable per-image command storage.
/// `execute` consumes pending commands (including on error) but retains capacity.
/// Call `clear` to abandon submissions before executing. Sources are never sorted.
pub struct RenderPhase {
    policy: SortPolicy,
    commands: Vec<DrawCommand>,
    shapes: Vec<PreparedShape>,
    sprites: Vec<PreparedSprite>,
    shape_batches: Vec<shape::Batch>,
    sprite_batches: Vec<sprite::Batch>,
}

impl RenderPhase {
    pub fn new(policy: SortPolicy) -> Self {
        Self {
            policy,
            commands: Vec::new(),
            shapes: Vec::new(),
            sprites: Vec::new(),
            shape_batches: Vec::new(),
            sprite_batches: Vec::new(),
        }
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Orders submissions without resolving GPU resources or encoding commands.
    pub fn prepare(&mut self) -> Result<(), DrawError> {
        if self.commands.len() > u32::MAX as usize {
            return Err(DrawError {
                command: 0,
                reason: "Too many instances",
            });
        }
        if matches!(
            self.policy,
            SortPolicy::FrontToBack | SortPolicy::BackToFront
        ) {
            for (command, draw) in self.commands.iter().enumerate() {
                if !draw.sort_key.depth.is_finite() {
                    return Err(DrawError {
                        command,
                        reason: "Depth must be finite",
                    });
                }
            }
        }
        match self.policy {
            SortPolicy::PreserveOrder => {}
            SortPolicy::DrawOrder => self.commands.sort_by_key(|draw| draw.sort_key.draw_order),
            SortPolicy::FrontToBack => self
                .commands
                .sort_by(|a, b| a.sort_key.depth.partial_cmp(&b.sort_key.depth).unwrap()),
            SortPolicy::BackToFront => self
                .commands
                .sort_by(|a, b| b.sort_key.depth.partial_cmp(&a.sort_key.depth).unwrap()),
        }
        Ok(())
    }

    /// Encodes using the same instance cache and batch drawing functions as direct rendering.
    pub fn execute(&mut self, pass: &mut wgpu::RenderPass<'_>) -> Result<(), DrawError> {
        let result = self.encode(pass);
        self.commands.clear();
        self.shapes.clear();
        self.sprites.clear();
        result
    }

    fn encode(&mut self, pass: &mut wgpu::RenderPass<'_>) -> Result<(), DrawError> {
        self.prepare()?;
        // Validate handles before encoding any part of this phase.
        for (index, command) in self.commands.iter().enumerate() {
            let error = |reason| DrawError {
                command: index,
                reason,
            };
            let pipeline = command
                .pipeline
                .get()
                .ok_or_else(|| error("Missing pipeline"))?;
            let reserved = |slot| {
                slot == 0 || (slot == 2 && matches!(command.geometry, DrawGeometry::Sprite(_)))
            };
            if command.bindings.iter().any(|(slot, _)| reserved(*slot)) {
                return Err(error("Binding override uses a reserved draw slot"));
            }
            if command
                .bindings
                .iter()
                .any(|(slot, _)| *slot as usize >= pipeline.bind_groups().len())
            {
                return Err(error("Binding override is outside the pipeline context"));
            }
            for (slot, group) in pipeline.bind_groups().iter().enumerate() {
                if group.is_none()
                    && !reserved(slot as u32)
                    && !command.bindings.iter().any(|entry| entry.0 == slot as u32)
                {
                    return Err(error("Missing per-command binding"));
                }
            }
            match &command.geometry {
                DrawGeometry::Shape(mesh) if !shape::contains(*mesh) => {
                    return Err(error("Missing mesh"));
                }
                DrawGeometry::Sprite(sprite) => validate_sprite(sprite).map_err(error)?,
                _ => {}
            }
        }
        let mut start = 0;
        while start < self.commands.len() {
            let first = &self.commands[start];
            let mut end = start + 1;
            while end < self.commands.len() && first.same_state(&self.commands[end]) {
                end += 1;
            }
            let pipeline = first.pipeline.get().ok_or(DrawError {
                command: start,
                reason: "Missing pipeline",
            })?;
            bind(&*pipeline, pass);
            for (slot, group) in &first.bindings {
                pass.set_bind_group(*slot, group, &[]);
            }
            match first.geometry {
                DrawGeometry::Shape(_) => {
                    self.shapes.clear();
                    for draw in &self.commands[start..end] {
                        if let DrawGeometry::Shape(mesh) = draw.geometry {
                            self.shapes.push(PreparedShape {
                                mesh,
                                instance: draw.instance,
                            });
                        }
                    }
                    shape::batch_into(
                        &self.shapes,
                        0..self.shapes.len() as u32,
                        &mut self.shape_batches,
                    );
                    for batch in &self.shape_batches {
                        shape::draw(&mut self.shapes, batch, pass);
                    }
                }
                DrawGeometry::Sprite(_) => {
                    self.sprites.clear();
                    for draw in &self.commands[start..end] {
                        if let DrawGeometry::Sprite(sprite) = &draw.geometry {
                            self.sprites.push(PreparedSprite {
                                sprite: sprite.clone(),
                                instance: draw.instance,
                            });
                        }
                    }
                    sprite::batch_into(
                        &self.sprites,
                        0..self.sprites.len() as u32,
                        &mut self.sprite_batches,
                    );
                    for batch in &self.sprite_batches {
                        sprite::draw(&mut self.sprites, batch, pass);
                    }
                }
            }
            start = end;
        }
        Ok(())
    }
}

fn validate_sprite(sprite: &Sprite) -> Result<(), &'static str> {
    let texture = match sprite {
        Sprite::Separate { texture, .. }
        | Sprite::Array { texture, .. }
        | Sprite::Animated { texture, .. }
        | Sprite::Atlas { texture, .. } => *texture,
    };
    let storage_owner = context::texture_storage(None);
    let storage = storage_owner.read().unwrap();
    let resource = storage.get(texture).ok_or("Missing texture")?;
    match (sprite, resource) {
        (Sprite::Separate { .. }, TextureResource::Separate(_)) => Ok(()),
        (Sprite::Array { layer, .. }, TextureResource::Array(resource))
        | (
            Sprite::Animated {
                current_layer: layer,
                ..
            },
            TextureResource::Array(resource),
        ) => {
            if *layer >= resource.texture.as_raw().depth_or_array_layers() {
                Err("Invalid texture array layer")
            } else {
                Ok(())
            }
        }
        (Sprite::Atlas { region, .. }, TextureResource::Atlas { regions, .. }) => regions
            .get(*region)
            .ok_or("Missing atlas region")
            .map(|_| ()),
        _ => Err("Texture does not match sprite kind"),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use tests::gpu_regression;
