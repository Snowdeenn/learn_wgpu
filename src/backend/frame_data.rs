use super::{instance::InstanceStorage, text::Text};

/// Persistent CPU instances. GPU resources stay in their dedicated storages.
#[derive(Default)]
pub struct FrameData {
    pub instances: InstanceStorage,
    pub text: Vec<Text>,
}

impl FrameData {
    pub fn new() -> Self {
        Self::default()
    }
}
