mod builder;
mod list;
mod macros;
pub mod operation;
#[cfg(test)]
mod tests;

pub use crate::backend::target::PassTarget;
pub use builder::*;
pub use list::{PassList, execute};
#[allow(unused_imports, reason = "Reexport de l'API du moteur, non utilise par la scene actuelle.")]
pub use operation::{ComputeOperation, RenderOperation, RenderSequence};

use crate::backend::{binding_set::BindingSetError, target::RenderTargetId, text::TextError};

pub enum PassContext {
    Render {
        target: PassTarget,
        load: wgpu::LoadOp<wgpu::Color>,
        store: wgpu::StoreOp,
        operation: Box<dyn RenderOperation>,
    },
    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    Compute {
        operation: Box<dyn ComputeOperation>,
    },
}

pub fn render(
    target: impl Into<PassTarget>,
    load: wgpu::LoadOp<wgpu::Color>,
    operation: impl RenderOperation + 'static,
) -> PassContext {
    PassContext::Render {
        target: target.into(),
        load,
        store: wgpu::StoreOp::Store,
        operation: Box::new(operation),
    }
}

#[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
pub fn compute(operation: impl ComputeOperation + 'static) -> PassContext {
    PassContext::Compute {
        operation: Box::new(operation),
    }
}

#[derive(Debug)]
pub enum PassError {
    MissingTarget(RenderTargetId),
    InvalidColorTarget(RenderTargetId),
    MissingPipeline,
    MissingInstances,
    BindingSet(BindingSetError),
    Text(TextError),
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTarget(id) => write!(f, "Unknown pass target: {id}"),
            Self::InvalidColorTarget(id) => {
                write!(f, "Target is not a color render attachment: {id}")
            }
            Self::MissingPipeline => f.write_str("Unknown operation pipeline"),
            Self::MissingInstances => f.write_str("Unknown operation instances"),
            Self::BindingSet(error) => error.fmt(f),
            Self::Text(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PassError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BindingSet(error) => Some(error),
            Self::Text(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BindingSetError> for PassError {
    fn from(error: BindingSetError) -> Self {
        Self::BindingSet(error)
    }
}

impl From<TextError> for PassError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}
