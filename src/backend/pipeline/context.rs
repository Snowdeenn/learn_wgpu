pub fn bind<'a>(ctx: &'a impl PipelineContext, pass: &mut wgpu::RenderPass<'_>) {
    pass.set_pipeline(ctx.pipeline());

    for (i, group) in ctx.bind_groups().iter().enumerate() {
        if let Some(group) = group {
            pass.set_bind_group(i as u32, group, &[]);
        }
    }

    for (i, vb) in ctx.vertex_buffer().iter().enumerate() {
        if let Some((buffer, range)) = vb {
            pass.set_vertex_buffer(i as u32, buffer.slice(range.clone()));
        }
    }

    if let Some((buffer, range)) = ctx.index_buffer() {
        pass.set_index_buffer(buffer.slice(range.clone()), wgpu::IndexFormat::Uint32);
    }
}

pub trait PipelineContext {
    fn pipeline(&self) -> &wgpu::RenderPipeline;
    fn bind_groups(&self) -> &[Option<wgpu::BindGroup>];
    fn index_buffer(&self) -> Option<&(wgpu::Buffer, std::ops::Range<u64>)>;
    fn vertex_buffer(&self) -> &[Option<(wgpu::Buffer, std::ops::Range<u64>)>];
}

#[macro_export]
macro_rules! define_pipeline_context {
    ($name:ident) => {
        paste::paste! {
            pub struct [<$name PipelineContext>] {
                pub pipeline: wgpu::RenderPipeline,
                pub bind_groups: Vec<Option<wgpu::BindGroup>>,
                pub vertex_buffer: Vec<Option<(wgpu::Buffer, std::ops::Range<u64>)>>,
                pub index_buffer: Option<(wgpu::Buffer, std::ops::Range<u64>)>,
            }

            impl $crate::backend::PipelineContext for [<$name PipelineContext>] {
                fn pipeline(&self) -> &wgpu::RenderPipeline {
                    &self.pipeline
                }

                fn bind_groups(&self) -> &[Option<wgpu::BindGroup>] {
                    &self.bind_groups
                }

                fn index_buffer(&self) -> Option<&(wgpu::Buffer, std::ops::Range<u64>)> {
                    self.index_buffer.as_ref()
                }

                fn vertex_buffer(&self) -> &[Option<(wgpu::Buffer, std::ops::Range<u64>)>] {
                    self.vertex_buffer.as_ref()
                }
            }
        }
    };
}

/// Optional `blend: Option<wgpu::BlendState>` goes before `bind_group`.
/// Omitting it preserves the builder's default blending; `None` disables blending.
#[macro_export]
macro_rules! pipeline_context {
    (
        $name:ident {
            shader: $shader:expr,
            format: $format:expr,
            $(fragment: $fragment:expr,)?
            $(blend: $blend:expr,)?
            bind_group: [$(($layout:expr, $($group:expr)?)),* $(,)?],
            $(vertex_buffer: [$(($vb_layout:expr, $vb:expr)),* $(,)?],)?
            index_buffer: $ib:expr $(,)?
        }
    ) => {{
        let shader = $shader;

        let layouts = [$(Some(&$layout)),*];

        let pipeline_layout = $crate::backend::context::device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &layouts,
                immediate_size: 0,
            });

        let builder = $crate::backend::pipeline::RenderPipelineBuilder::from_layout(
            &pipeline_layout,
            shader,
        )
        .with_vertex_entry_point("vs_main")
        .with_color_format($format);

        $(let builder = builder.with_blend($blend);)?

        $(
            let builder = builder
                .with_fragment_shader(shader)
                .with_fragment_entry_point($fragment);
        )?

        $(
            $(let builder = builder.add_vertex_buffer_layout($vb_layout);)*
        )?

        let pipeline = builder.build();

        #[allow(unused)]
        let result = paste::paste! {
            [<$name PipelineContext>] {
                pipeline,
                bind_groups: vec![
                    $(
                        {
                            let group: Option<wgpu::BindGroup> = None;
                            $(let group = Some($group);)?
                            group
                        }
                    ),*
                ],
                index_buffer: {
                    let buffer: Option<&$crate::backend::buffer::Buffer<
                        u32,
                        $crate::backend::buffer::Index,
                    >> = $ib;
                    buffer.map(|buffer| buffer.raw_region())
                },
                vertex_buffer: vec![
                    $($(Some(($vb).raw_region())),*)?
                ],
            }
        };
        result
    }};
}
