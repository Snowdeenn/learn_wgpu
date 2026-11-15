use super::resources::*;
use crate::{
    api::{SceneError, scene::Scene},
    backend::{self, Sprite, SpriteInstance, shape, text},
    camera::Camera2D,
    core::Resources,
    particle,
};
use std::time::Instant;

use crate::backend::{pass::PassError, pipeline, renderer::Frame};

use crate::api::Velum;

crate::define_render_operation! {
    Scene {
        mode: crate::api::scene::RenderDataId<ScenePipeline>,
        pipelines: ScenePipelines,
        instances: crate::api::scene::RenderDataId<InstanceData>,
    }

    execute(this, pass, frame) {
        let mesh = pipeline::get(this.pipelines.mesh)
            .ok_or(PassError::MissingPipeline)?;

        let texture = pipeline::get(this.pipelines.texture)
            .ok_or(PassError::MissingPipeline)?;

        let texture_array = pipeline::get(this.pipelines.texture_array)
            .ok_or(PassError::MissingPipeline)?;

        let mode = *frame.get(this.mode).ok_or(PassError::MissingData)?;
        let instances = frame
            .get_mut(this.instances)
            .ok_or(PassError::MissingInstances)?;

        match mode {
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

crate::define_render_operation! {
    SceneText {}
    execute(_this, pass, _frame) {
        text::draw_all(pass)?;
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
    let velum = resources.read_resource::<Velum>();
    let Scene { data, .. } = &*velum.get(rendering.scene).unwrap();
    text::prepare(
        data.get(rendering.texts)
            .expect("Textes de la scène introuvables"),
        &text::TextView {
            resolution,
            scale_factor,
            camera: &camera,
        },
    )
}

pub fn render(
    resources: &Resources,
    frame: &mut Frame<'_>,
) -> Result<(), SceneError> {
    // Prepare before encoding so the two timings do not overlap.
    let start = Instant::now();
    particle::prepare_registered_effects();
    let elapsed = start.elapsed();
    resources
        .write_resource::<FrameTimings>()
        .preparation
        .record(elapsed);

    let mode = *resources.read_resource::<ScenePipeline>();
    let mut rendering = resources.write_resource::<SceneRendering>();
    let mut velum = resources.write_resource::<Velum>();
    let Scene { data, .. } = &mut *velum.get_mut(rendering.scene).unwrap();
    let mode_id = rendering.mode;
    *data.get_mut(mode_id).unwrap() = mode;

    rendering.blur.prepare();

    let start = Instant::now();
    let result = velum.render(frame);
    let elapsed = start.elapsed();

    resources
        .write_resource::<FrameTimings>()
        .render_encoding
        .record(elapsed);

    result
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
