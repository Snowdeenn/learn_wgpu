use scene::SceneId;

use crate::backend::pass::PassError;

pub mod scene;
mod velum;

pub use velum::Velum;

#[derive(Debug)]
pub enum SceneError {
    MissingScene(SceneId),
    Pass(PassError),
    NoActiveScene,
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::MissingScene(s) => write!(f, "Unknow or Missing scene: {s}"),
            SceneError::NoActiveScene => write!(f, "No scene has been set to active"),
            SceneError::Pass(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SceneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SceneError::Pass(e) => Some(e),
            _ => None
        }
    }
}
