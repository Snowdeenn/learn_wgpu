use super::storage;
pub use super::storage::{get, remove};

pub type ComputePipelineId<T> = storage::PipelineId<T>;

pub trait ComputePipelineContext {
    fn pipeline(&self) -> &wgpu::ComputePipeline;
    fn bind_groups(&self) -> &[Option<wgpu::BindGroup>];
}

pub fn register<T: ComputePipelineContext + Send + Sync + 'static>(
    context: T,
) -> ComputePipelineId<T> {
    storage::insert(context)
}

pub fn bind(context: &impl ComputePipelineContext, pass: &mut wgpu::ComputePass<'_>) {
    pass.set_pipeline(context.pipeline());
    for (slot, group) in context.bind_groups().iter().enumerate() {
        if let Some(group) = group {
            pass.set_bind_group(slot as u32, group, &[]);
        }
    }
}

pub struct ComputePipelineBuilder<'a> {
    layout: &'a wgpu::PipelineLayout,
    shader: &'a wgpu::ShaderModule,
    entry_point: &'a str,
    label: Option<&'a str>,
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn from_layout(layout: &'a wgpu::PipelineLayout, shader: &'a wgpu::ShaderModule) -> Self {
        Self {
            layout,
            shader,
            entry_point: "cs_main",
            label: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_entry_point(mut self, entry_point: &'a str) -> Self {
        self.entry_point = entry_point;
        self
    }

    pub fn build(self) -> wgpu::ComputePipeline {
        crate::backend::context::device().create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: self.label,
                layout: Some(self.layout),
                module: self.shader,
                entry_point: Some(self.entry_point),
                compilation_options: Default::default(),
                cache: None,
            },
        )
    }
}

#[macro_export]
macro_rules! define_compute_pipeline_context {
    ($name:ident) => {
        paste::paste! {
            pub struct [<$name ComputePipelineContext>] {
                pub pipeline: wgpu::ComputePipeline,
                pub bind_groups: Vec<Option<wgpu::BindGroup>>,
            }

            impl $crate::backend::pipeline::compute::ComputePipelineContext for [<$name ComputePipelineContext>] {
                fn pipeline(&self) -> &wgpu::ComputePipeline { &self.pipeline }
                fn bind_groups(&self) -> &[Option<wgpu::BindGroup>] { &self.bind_groups }
            }
        }
    };
}
