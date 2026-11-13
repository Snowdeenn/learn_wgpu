use crate::backend::frame_data::FrameData;

use super::PassError;

pub trait RenderOperation {
    fn execute(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &mut FrameData,
    ) -> Result<(), PassError>;
}

pub trait ComputeOperation {
    fn execute(
        &mut self,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &mut FrameData,
    ) -> Result<(), PassError>;
}

/// Each operation binds its own state before drawing into the same attachment.
#[derive(Default)]
#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub struct RenderSequence {
    pub operations: Vec<Box<dyn RenderOperation>>,
}

impl RenderSequence {
    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn new(operations: Vec<Box<dyn RenderOperation>>) -> Self {
        Self { operations }
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn push(&mut self, operation: impl RenderOperation + 'static) {
        self.operations.push(Box::new(operation));
    }
}

impl RenderOperation for RenderSequence {
    fn execute(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &mut FrameData,
    ) -> Result<(), PassError> {
        for operation in &mut self.operations {
            operation.execute(pass, frame)?;
        }
        Ok(())
    }
}
