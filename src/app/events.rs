use super::App;
use crate::{
    backend::renderer::Renderer,
    camera::Camera2DController,
    debug::DebugManager,
    scene::{self, resources::{Fps, FrameTimings}},
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{WindowAttributes, WindowId},
};

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.last_frame = None;
        if self.window.is_some() {
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title("Learn Wgpu")
            .with_resizable(true);
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("Impossible de creer la fenetre"),
        );
        let renderer = Renderer::new(window.clone());
        let debug = DebugManager::init(&window, renderer.conf.format);
        scene::setup::initialize(
            &mut self.resources,
            renderer.conf.format,
            [renderer.conf.width, renderer.conf.height],
        );
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.debug = Some(debug);
        self.cam_controller = Some(Camera2DController::new(250.0));

        self.resources.insert(Fps(0.0));
        self.resources.insert(FrameTimings::default());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        let Some(debug) = self.debug.as_mut() else {
            return;
        };
        debug.handle_event(window, &event);
        let Some(controller) = self.cam_controller.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                renderer.resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state.is_pressed() {
                        controller.pressed(key);
                    } else {
                        controller.released(key);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if matches!(
                    button,
                    winit::event::MouseButton::Left | winit::event::MouseButton::Right
                ) {
                    if state.is_pressed() {
                        controller.mouse_pressed(button);
                    } else {
                        controller.mouse_release(button);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                controller.set_mouse_position(position.x as f32, position.y as f32);
            }
            WindowEvent::RedrawRequested => {
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let Some(renderer) = self.renderer.as_ref() else {
                    return;
                };

                let acquire_start = std::time::Instant::now();
                let acquired = renderer.begin_frame(window);
                let acquire_elapsed = acquire_start.elapsed();
                match acquired {
                    Ok(Some(mut frame)) => {
                        self.resources.write_resource::<FrameTimings>().surface_acquire.record(acquire_elapsed);
                        crate::backend::metrics::reset();
                        debug.begin_frame(window);
                        scene::debug_panel::run(debug, &self.resources);

                        let size_in_pixels = frame.size_in_pixels();
                        if let Err(error) = scene::render::prepare_text(
                            &self.resources,
                            size_in_pixels,
                            window.scale_factor() as f32,
                        ) {
                            eprintln!("{error}");
                            event_loop.exit();
                            return;
                        }
                        if let Err(error) = scene::render::render(&self.resources, &mut frame) {
                            eprintln!("{error}");
                            event_loop.exit();
                            return;
                        }

                        debug.flush_into(window, &mut frame.encoder, &frame.view, size_in_pixels);

                        let measure_wait = self.resources.read_resource::<FrameTimings>().measure_gpu_wait;
                        let wait = frame.finish(measure_wait);
                        let (slots, reserved) = crate::backend::instances::cache::memory_stats();
                        let mut timings = self.resources.write_resource::<FrameTimings>();
                        timings.render = crate::backend::metrics::snapshot();
                        timings.instance_slots = slots;
                        timings.instance_reserved_bytes = reserved;
                        if let Some(wait) = wait {
                            timings.gpu_wait.record(wait);
                        } else if measure_wait {
                            timings.measure_gpu_wait = false;
                            eprintln!("Mesure d'attente GPU indisponible : poll a échoué");
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.update();
    }
}
