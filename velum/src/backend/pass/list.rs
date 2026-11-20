use new_collection::{Arena, Id};

use crate::{api::scene::RenderData, backend::target};

use super::{PassContext, PassError, PassTarget, RenderPassBuilder};

/// A persistent, explicitly ordered list. No dependency graph or implicit passes.
pub struct PassList {
    passes: Arena<PassContext>,
    order: Vec<Id<PassContext>>,
}

impl PassList {
    pub fn ids(&self) -> &[Id<PassContext>] {
        &self.order
    }
    pub fn get(&self, id: Id<PassContext>) -> Option<&PassContext> {
        self.passes.get(id)
    }
    pub fn new() -> Self {
        PassList {
            passes: Arena::new(),
            order: Vec::new(),
        }
    }

    #[inline]
    pub fn insert(&mut self, pass: PassContext) -> Id<PassContext> {
        let id = self.passes.insert(pass);
        self.order.push(id);

        id
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id<PassContext>) -> Option<&mut PassContext> {
        self.passes.get_mut(id)
    }
}

/// Resource creation, resizing and execution belong to the render thread.
/// On error, discard the frame's encoder rather than submitting a partial frame.
pub fn execute(
    passes: &mut PassList,
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    frame: &mut RenderData,
) -> Result<(), PassError> {
    for context_id in &passes.order {
        let context = passes.passes.get_mut(*context_id).unwrap();
        execute_one(context, encoder, surface_view, frame)?;
    }
    Ok(())
}

pub fn execute_one(
    context: &mut PassContext,
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    frame: &mut RenderData,
) -> Result<(), PassError> {
    let start = std::time::Instant::now();
    let result = encode_one(context, encoder, surface_view, frame);
    crate::backend::report::record_pass(start.elapsed());
    if let Err(error) = &result {
        crate::backend::report::diagnostic("pass", error.to_string());
    }
    result
}

fn encode_one(
    context: &mut PassContext,
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    frame: &mut RenderData,
) -> Result<(), PassError> {
    match context {
        PassContext::Render {
            target,
            load,
            store,
            depth,
            operation,
            active,
        } => {
            if !*active {
                return Ok(());
            }

            let view = match *target {
                PassTarget::Surface => surface_view.clone(),
                PassTarget::Offscreen(id) => {
                    let target = target::get(id).ok_or(PassError::MissingTarget(id))?;
                    let texture = target.texture().as_raw();
                    if !texture
                        .usage()
                        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
                        || texture.format().is_depth_stencil_format()
                    {
                        return Err(PassError::InvalidColorTarget(id));
                    }
                    target.texture().view().clone()
                }
            };

            let depth_target = depth
                .map(|id| target::get(id).ok_or(PassError::MissingTarget(id)))
                .transpose()?;

            if let Some(depth_target) = &depth_target {
                let texture = depth_target.texture().as_raw();
                if texture.format() != crate::backend::depth::FORMAT
                    || !texture
                        .usage()
                        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
                    || texture.size() != view.texture().size()
                    || texture.sample_count() != view.texture().sample_count()
                {
                    return Err(PassError::InvalidDepthTarget(depth.unwrap()));
                }
            }

            let mut builder = RenderPassBuilder::new().with_color_attachment(&view, |attachment| {
                attachment.with_load_op(*load).with_store_op(*store)
            });

            if let Some(depth_target) = &depth_target {
                builder =
                    builder.with_depth_stencil_descriptor(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_target.texture().view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });
            }

            let _attachments = crate::backend::pipeline::AttachmentScope::new(
                crate::backend::pipeline::AttachmentSignature {
                    colors: vec![Some(view.texture().format())],
                    depth: depth_target
                        .as_ref()
                        .map(|target| target.texture().as_raw().format()),
                    samples: view.texture().sample_count(),
                },
            );
            let mut pass = builder.begin(encoder);
            operation.execute(&mut pass, frame)?;
        }
        PassContext::Compute { operation, active } => {
            if !*active {
                return Ok(());
            }

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute operation"),
                timestamp_writes: None,
            });
            crate::backend::report::compute_pass();
            operation.execute(&mut pass, frame)?;
        }
    }
    Ok(())
}
