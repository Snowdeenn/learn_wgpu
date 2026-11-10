use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod backend;
mod camera;
mod core;
mod debug;
mod geometry;
mod scene;
mod particle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = app::App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
