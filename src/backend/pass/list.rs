use crate::backend::{frame_data::FrameData, target};

use super::{PassContext, PassError, PassTarget, RenderPassBuilder};

/// A persistent, explicitly ordered list. No dependency graph or implicit passes.
pub type PassList = Vec<PassContext>;

/// Resource creation, resizing and execution belong to the render thread.
/// On error, discard the frame's encoder rather than submitting a partial frame.
pub fn execute(
    passes: &mut [PassContext],
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    frame: &mut FrameData,
) -> Result<(), PassError> {
    for context in passes {
        match context {
            PassContext::Render {
                target,
                load,
                store,
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
                let mut pass = RenderPassBuilder::new()
                    .with_color_attachment(&view, |attachment| {
                        attachment.with_load_op(*load).with_store_op(*store)
                    })
                    .begin(encoder);
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
