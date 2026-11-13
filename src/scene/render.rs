use super::resources::*;
use crate::effects::{
    blur::{BlurHorizontalOperation, BlurVerticalOperation},
    composite::CompositionOperation,
    present::PresentOperation,
};
use crate::{
    backend::{self, Sprite, SpriteInstance, shape, text},
    camera::Camera2D,
    core::Resources,
    particle,
};

use super::instance::Dummy;
use crate::backend::{
    instances::InstanceId,
    pass::{self, PassError, PassTarget},
    pipeline,
    renderer::Frame,
};

crate::define_render_operation! {
    Scene {
        mode: ScenePipeline,
        pipelines: ScenePipelines,
        instances: InstanceId<Dummy>,
    }

    execute(this, pass, frame) {
        let mesh = pipeline::get(this.pipelines.mesh)
            .ok_or(PassError::MissingPipeline)?;

        let texture = pipeline::get(this.pipelines.texture)
            .ok_or(PassError::MissingPipeline)?;

        let texture_array = pipeline::get(this.pipelines.texture_array)
            .ok_or(PassError::MissingPipeline)?;

        let instances = frame.instances
            .get_mut(&this.instances)
            .ok_or(PassError::MissingInstances)?;

        match this.mode {
            ScenePipeline::Mesh => {
                backend::bind(&*mesh, pass);
                shape::draw_all(&mut instances.data, pass);
            }
            ScenePipeline::Texture => {
                draw_sprites(
                    &texture,
                    &texture_array,
                    pass,
                    &mut instances.data,
                );
            }
        }



        text::draw_all(pass)?;
        Ok(())
    }
}

crate::define_render_operation! {
    Particle {
        pipelines: ScenePipelines,
    }

    execute(this, pass, _frame) {
        let mesh = pipeline::get(this.pipelines.mesh)
            .ok_or(PassError::MissingPipeline)?;

        let texture = pipeline::get(this.pipelines.texture)
            .ok_or(PassError::MissingPipeline)?;

        let texture_array = pipeline::get(this.pipelines.texture_array)
            .ok_or(PassError::MissingPipeline)?;

        // Les particules de forme ont besoin du pipeline mesh,
        // quel que soit le mode choisi pour les objets de la scène.
        backend::bind(&*mesh, pass);

        particle::prepare_registered_effects();

        particle::for_each_prepared_effect(|instances| {
            shape::draw_all(instances.shape(), pass);
        });

        particle::for_each_prepared_effect(|instances| {
            draw_sprites(
                &texture,
                &texture_array,
                pass,
                instances.sprite(),
            );
        });
        Ok(())
    }
}

pub fn prepare_text(
    resources: &Resources,
    resolution: [u32; 2],
    scale_factor: f32,
) -> Result<(), text::TextError> {
    let camera = resources.read_resource::<Camera2D>();
    let rendering = resources.read_resource::<SceneRendering>();
    text::prepare(
        &rendering.data.text,
        &text::TextView {
            resolution,
            scale_factor,
            camera: &camera,
        },
    )
}

pub fn render(resources: &Resources, frame: &mut Frame<'_>) -> Result<(), PassError> {
    let mode = *resources.read_resource::<ScenePipeline>();
    let pipelines = *resources.read_resource::<ScenePipelines>();
    let mut rendering = resources.write_resource::<SceneRendering>();

    rendering.blur.prepare();

    let operation = SceneOperation {
        mode,
        pipelines,
        instances: rendering.instances,
    };

    let present_operation = PresentOperation {
        pipeline: rendering.present.pipeline,
        source: rendering.present.source,
    };

    let particle_op = ParticleOperation {
        pipelines,
    };

    let blur_vertical = BlurVerticalOperation {
        pipeline: rendering.blur.pipeline,
        source: rendering.blur.source_a,
        direction: rendering.blur.vertical_group.clone(),
    };

    let blur_horizontal = BlurHorizontalOperation {
        pipeline: rendering.blur.pipeline,
        source: rendering.blur.source_b,
        direction: rendering.blur.horizontal_group.clone(),
    };

    let composition = CompositionOperation {
        pipeline: rendering.composition.pipeline,
        source: rendering.composition.source,
    };

    let target = rendering.target;
    let blur_target_a = rendering.blur.target_a;
    let blur_target_b = rendering.blur.target_b;

    rendering.passes.clear();
    rendering.passes.push(pass::render(
        PassTarget::Offscreen(target),
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        operation,
    ));
    rendering.passes.push(pass::render(
        PassTarget::Offscreen(blur_target_a),
        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        particle_op,
    ));
    rendering.passes.push(pass::render(
        PassTarget::Offscreen(blur_target_b),
        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        blur_vertical,
    ));
    rendering.passes.push(pass::render(
        PassTarget::Offscreen(blur_target_a),
        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        blur_horizontal,
    ));
    rendering.passes.push(pass::render(
        PassTarget::Offscreen(target),
        wgpu::LoadOp::Load,
        composition,
    ));
    rendering.passes.push(pass::render(
        PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        present_operation,
    ));

    let SceneRendering { passes, data, .. } = &mut *rendering;
    frame.execute(passes, data)
}

fn draw_sprites<T: SpriteInstance>(
    texture: &TexturePipelineContext,
    texture_array: &TextureArrayPipelineContext,
    pass: &mut wgpu::RenderPass<'_>,
    instances: &mut [T],
) {
    let end = instances.len() as u32;

    for batch in backend::batch(instances, 0..end) {
        match instances[batch.range.start as usize].sprite() {
            Sprite::Separate { .. } | Sprite::Atlas { .. } => {
                backend::bind(texture, pass);
            }
            Sprite::Array { .. } | Sprite::Animated { .. } => {
                backend::bind(texture_array, pass);
            }
        }

        backend::draw(instances, &batch, pass);
    }
}
