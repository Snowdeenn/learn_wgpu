use winit::event_loop::{ControlFlow, EventLoop};

mod app;
use velum::{api, backend, camera, core, effects, geometry, particle};
pub use velum::{bind_group, define_pipeline_context, pipeline_context, unique_resource};
pub use velum::{create_marker, define_compute_operation, define_render_operation};
mod debug;
mod scene;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = app::App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
