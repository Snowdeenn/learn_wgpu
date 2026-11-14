use crate::{api::scene::RenderData, backend::target};

use super::{PassContext, PassError, PassTarget, RenderPassBuilder};

/// A persistent, explicitly ordered list. No dependency graph or implicit passes.
pub type PassList = Vec<PassContext>;

/// Resource creation, resizing and execution belong to the render thread.
/// On error, discard the frame's encoder rather than submitting a partial frame.
pub fn execute(
    passes: &mut [PassContext],
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    frame: &mut RenderData,
) -> Result<(), PassError> {
    for context in passes {
        match context {
            PassContext::Render {
                target,
                load,
                store,
                depth,
                operation,
            } => {
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

                let mut builder = RenderPassBuilder::new()
                    .with_color_attachment(&view, |attachment| {
                        attachment.with_load_op(*load).with_store_op(*store)
                    });

                if let Some(depth_target) = &depth_target {
                    builder = builder.with_depth_stencil_descriptor(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: depth_target.texture().view(),
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        },
                    );
                }

                let mut pass = builder.begin(encoder);
                operation.execute(&mut pass, frame)?;
            }
            PassContext::Compute { operation } => {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute operation"),
                    timestamp_writes: None,
                });
                operation.execute(&mut pass, frame)?;
            }
        }
    }
    Ok(())
}
