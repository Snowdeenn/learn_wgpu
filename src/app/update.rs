use super::App;
use crate::{
    backend::{self},
    camera::Camera2D,
    particle,
    scene::resources::{CamBuffer, Fps, InstanceData},
};
use std::time::Instant;

const ANIMATION_FPS: f32 = 8.0;

impl App {
    pub(super) fn update(&mut self) {
        let (Some(window), Some(controller)) = (self.window.as_ref(), self.cam_controller.as_mut())
        else {
            return;
        };
        let now = Instant::now();
        let dt = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32())
            .min(0.1);

        {
            let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
            self.resources.write_resource::<Fps>().0 = fps;

            let mut instances = self.resources.write_resource::<InstanceData>();
            backend::sprite::run_animated(&mut *instances, dt);
        }

        particle::update_effects(dt);

        {
            let mut cam = self.resources.write_resource::<Camera2D>();
            let cam_buffer = self.resources.read_resource::<CamBuffer>();
            controller.update(&mut cam, dt);

            let size = window.inner_size();
            let view_proj = cam.get_view_proj(size.width.max(1) as f32, size.height.max(1) as f32);
            backend::write_buffer(&**cam_buffer, 0, &[view_proj]);
            controller.end_frame();
        }
        window.request_redraw();
    }
}
