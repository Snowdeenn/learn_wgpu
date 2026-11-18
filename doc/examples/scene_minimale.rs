//! Scene construction and one-frame entry point; the host owns its winit loop.
use velum::backend::{
    pass::{self, PassError, PassTarget, RenderOperation},
    renderer::Renderer,
    target::{self, RenderTargetDescriptor, TargetSize},
};
use velum::effects::{
    RenderEffect,
    blur::BlurEffect,
    composite::{CompositeEffect, CompositionOperation},
};
use velum::{EffectId, EffectSource, RenderData, RenderDataId, Scene, SceneRenderer};

type Error = Box<dyn std::error::Error>;

struct ClearOnly;
impl RenderOperation for ClearOnly {
    fn execute(
        &mut self,
        _: &mut wgpu::RenderPass<'_>,
        _: &mut RenderData,
    ) -> Result<(), PassError> {
        Ok(())
    }
}

pub struct DemoHandles {
    pub blur: EffectId,
    presenter: RenderDataId<CompositeEffect>,
}

pub fn build_scene(
    format: wgpu::TextureFormat,
    resolution: [u32; 2],
) -> Result<(Scene, DemoHandles), Error> {
    let mut data = RenderData::new();
    let mut renderer = SceneRenderer::new();
    let source = target::register(RenderTargetDescriptor {
        label: "Documentation source",
        size: TargetSize::Window {
            width: resolution[0],
            height: resolution[1],
        },
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        sample_count: 1,
    });
    renderer.add_pass(pass::render(
        source.id(),
        wgpu::LoadOp::Clear(wgpu::Color {
            r: 0.1,
            g: 0.3,
            b: 0.6,
            a: 1.0,
        }),
        ClearOnly,
        true,
    ));
    let blur = BlurEffect::new(format, resolution);
    let presenter = CompositeEffect::new(blur.output(), format);
    let blur_id = renderer.add_effect(blur);
    renderer.connect(
        blur_id,
        BlurEffect::SOURCE,
        EffectSource::Target(source.id()),
    )?;
    renderer.append_effect(blur_id)?;
    renderer.add_pass(pass::render(
        PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        CompositionOperation {
            pipeline: presenter.pipeline.id(),
            source: presenter.source.id(),
        },
        true,
    ));
    data.insert(source);
    let presenter = data.insert(presenter);
    Ok((
        Scene { renderer, data },
        DemoHandles {
            blur: blur_id,
            presenter,
        },
    ))
}

/// Call on redraw after updating app data. Errors abandon the acquired frame.
pub fn draw_frame(
    gpu: &Renderer,
    window: &winit::window::Window,
    scene: &mut Scene,
    handles: &DemoHandles,
) -> Result<(), Error> {
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return Ok(());
    }
    // A local final pass is not an effect-graph consumer: follow bypass explicitly.
    let output = scene
        .renderer
        .resolve_source(EffectSource::Output(handles.blur))?;
    scene
        .data
        .require_mut(handles.presenter)?
        .set_source(output)?;
    let acquired = gpu
        .begin_frame(window)
        .map_err(|error| std::io::Error::other(error.to_owned()))?;
    if let Some(mut frame) = acquired {
        scene.renderer.execute(&mut frame, &mut scene.data)?;
        frame.finish(false);
    }
    Ok(())
}
