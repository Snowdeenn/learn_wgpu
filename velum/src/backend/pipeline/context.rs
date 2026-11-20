pub fn bind(ctx: &(impl PipelineContext + ?Sized), pass: &mut wgpu::RenderPass<'_>) {
    bind_resolved(
        ctx.pipeline(),
        ctx.bind_groups()
            .iter()
            .enumerate()
            .filter_map(|(slot, group)| {
                group
                    .as_ref()
                    .map(|group| (slot as u32, group, &[] as &[u32]))
            }),
        ctx.vertex_buffer()
            .iter()
            .enumerate()
            .filter_map(|(slot, vertex)| {
                vertex
                    .as_ref()
                    .map(|(buffer, range)| (slot as u32, buffer.slice(range.clone())))
            }),
        ctx.index_buffer()
            .map(|(buffer, range)| (buffer.slice(range.clone()), wgpu::IndexFormat::Uint32)),
        pass,
    );
}

/// Binds already-resolved graphics state without allocating temporary collections.
/// Slots, dynamic offsets and bounded slices are supplied by the caller, after validation.
/// Shared by historical contexts and prepared render objects; does not issue a draw.
pub fn bind_resolved<'a>(
    pipeline: &wgpu::RenderPipeline,
    groups: impl IntoIterator<Item = (u32, &'a wgpu::BindGroup, &'a [u32])>,
    vertices: impl IntoIterator<Item = (u32, wgpu::BufferSlice<'a>)>,
    indices: Option<(wgpu::BufferSlice<'a>, wgpu::IndexFormat)>,
    pass: &mut wgpu::RenderPass<'_>,
) {
    pass.set_pipeline(pipeline);
    for (slot, group, offsets) in groups {
        pass.set_bind_group(slot, group, offsets);
    }
    for (slot, buffer) in vertices {
        pass.set_vertex_buffer(slot, buffer);
    }
    if let Some((buffer, format)) = indices {
        pass.set_index_buffer(buffer, format);
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
    (template: $template:expr $(,)?) => {
        $crate::try_pipeline_context!(template: $template).expect("Invalid material pipeline context")
    };
    (
        $name:ident {
            shader: $shader:expr,
            format: $format:expr,
            $(fragment: $fragment:expr,)?
            $(blend: $blend:expr,)?
            $(depth: $depth:expr,)?
            bind_group: [$(($layout:expr, $($group:expr)?)),* $(,)?],
            $(vertex_buffer: [$(($vb_layout:expr, $vb:expr)),* $(,)?],)?
            index_buffer: $ib:expr $(,)?
        }
    ) => {{
        let shader = $shader;

        let layouts = [$(Some(&$layout)),*];

        let pipeline_layout = $crate::backend::context::device(None)
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
        $(let builder = builder.with_depth_stencil($depth);)?

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

/// Described context path; shares the template's validated shaders/layout/raster state.
#[macro_export]
macro_rules! try_pipeline_context {
    (template: $template:expr $(,)?) => {
        $crate::backend::pipeline::try_context($template)
    };
}
