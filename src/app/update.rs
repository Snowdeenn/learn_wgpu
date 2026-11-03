use super::App;
use crate::{
    backend,
    camera::Camera2D,
    scene::resources::{CamBuffer, Fps},
};
use std::time::Instant;

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
            let fps = 1.0 / dt;
            self.resources.write_resource::<Fps>().0 = fps;
        }

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
