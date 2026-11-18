use std::fmt;

use crate::backend::target::RenderTargetId;

#[derive(Debug)]
pub enum EffectError {
    UnknownInput(String),
    MissingInput(String),
    MissingTarget(RenderTargetId),
    IncompatibleInput { name: String, reason: String },
}

impl fmt::Display for EffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownInput(name) => write!(f, "Unknown effect input: {name}"),
            Self::MissingInput(name) => write!(f, "Missing required effect input: {name}"),
            Self::MissingTarget(id) => write!(f, "Missing effect target: {id}"),
            Self::IncompatibleInput { name, reason } => {
                write!(f, "Incompatible effect input {name}: {reason}")
            }
        }
    }
}

impl std::error::Error for EffectError {}
