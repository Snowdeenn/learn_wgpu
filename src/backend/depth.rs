use crate::core::OwnedResource;
use super::target::{self, RenderTargetDescriptor, RenderTargetId, TargetSize};
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub fn depth_from_ground_y(y: f32, min_y: f32, max_y: f32) -> f32 {
    assert!(y.is_finite() && min_y.is_finite() && max_y.is_finite() && max_y > min_y);
    let t = ((y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
    0.99 - t * 0.98
}
/// Shares the scene color target's window resize policy.
pub struct DepthTarget {
    id: OwnedResource<RenderTargetId>,
}
impl DepthTarget {
    pub fn new([width, height]: [u32; 2]) -> Self {
        Self {
            id: target::register(RenderTargetDescriptor {
                label: "Scene depth",
                size: TargetSize::Window { width, height },
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                sample_count: 1,
            }),
        }
    }
    pub fn id(&self) -> RenderTargetId {
        self.id.id()
    }
}
pub fn opaque_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: Default::default(),
        bias: Default::default(),
    }
}
