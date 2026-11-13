use super::App;
use crate::{
    backend::{self}, camera::Camera2D, particle, scene::resources::{CamBuffer, Fps, FrameTimings, SceneRendering},
};
use std::time::Instant;

impl App {
    pub(super) fn update(&mut self) {
        let (Some(window), Some(controller)) = (self.window.as_ref(), self.cam_controller.as_mut())
        else {
            return;
        };
        let now = Instant::now();
        let frame_dt = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        let dt = frame_dt.min(0.1);

        {
            if frame_dt > 0.0 {
                let current_fps = 1.0 / frame_dt;
                let mut fps = self.resources.write_resource::<Fps>();
                // Seed with the first valid sample, then smooth subsequent values.
                fps.0 = if fps.0 > 0.0 {
                    fps.0 + 0.1 * (current_fps - fps.0)
                } else {
                    current_fps
                };
            }

            let mut rendering = self.resources.write_resource::<SceneRendering>();
            backend::sprite::run_animated(rendering.instances_mut(), dt);
        }

        let start = Instant::now();
        particle::update_effects(dt);
        let elapsed = start.elapsed();
        self.resources.write_resource::<FrameTimings>().simulation.record(elapsed);

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
