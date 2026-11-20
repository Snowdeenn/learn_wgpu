//! Described geometry, materials and objects on Velum's frame/pass lifecycle.
//!
//! Resources are created with free functions and kept alive with `OwnedResource`
//! or `SharedResource`. No slots are reserved. Builders and macros share validation.
//! A failed draw is diagnosed and returned; discard the encoder on any pass error.
thread_local! {
    static FRAME_OWNERS: std::cell::RefCell<Vec<Box<dyn std::any::Any>>> = std::cell::RefCell::new(Vec::new());
}
pub(crate) fn retain(owner: impl std::any::Any) {
    FRAME_OWNERS.with_borrow_mut(|owners| owners.push(Box::new(owner)));
}
/// Call after submitting all commands referencing allocations. `Frame` and
/// `OffscreenFrame` handle this automatically. Raw encoder integrations must call it.
pub fn submitted() {
    FRAME_OWNERS.with_borrow_mut(Vec::clear);
}
/// Call after dropping every encoder/command buffer for an abandoned image.
pub fn discard() {
    FRAME_OWNERS.with_borrow_mut(Vec::clear);
}

pub type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    pub context: String,
    pub message: String,
}
impl RenderError {
    pub fn new(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.context, self.message)
    }
}
impl std::error::Error for RenderError {}

pub(crate) fn gpu(
    context: Option<super::context::ContextId>,
) -> RenderResult<std::sync::Arc<super::context::Context>> {
    super::context::try_context(context)
        .ok_or_else(|| RenderError::new("GPU context", "missing context"))
}

/// Error scopes also cover shader/layout/device validation in all `try_` builders.
pub(crate) fn checked<T>(
    device: &wgpu::Device,
    label: &str,
    build: impl FnOnce() -> T,
) -> RenderResult<T> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = build();
    if let Some(error) = pollster::block_on(scope.pop()) {
        Err(RenderError::new(label, error.to_string()))
    } else {
        Ok(value)
    }
}

pub fn try_shader(
    context: Option<super::context::ContextId>,
    label: &str,
    source: &str,
) -> RenderResult<wgpu::ShaderModule> {
    let gpu = gpu(context)?;
    checked(&gpu.device, label, || {
        gpu.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    })
}
