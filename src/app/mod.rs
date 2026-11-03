mod events;
mod update;

use crate::{
    backend::renderer::Renderer, camera::Camera2DController, core::Resources, debug::DebugManager,
};
use std::{sync::Arc, time::Instant};
use winit::window::Window;

#[derive(Default)]
pub struct App {
    resources: Resources,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    debug: Option<DebugManager>,
    cam_controller: Option<Camera2DController>,
    last_frame: Option<Instant>,
}
