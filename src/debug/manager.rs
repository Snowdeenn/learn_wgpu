use crate::backend::context;

use super::WidgetBuilder;

pub struct DebugManager {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pointer_position: Option<egui::Pos2>,
}


impl DebugManager {
    pub fn init(window: &winit::window::Window, format: wgpu::TextureFormat) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            context::device(),
            format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: false,
                predictable_texture_filtering: false,
            },
        );

        Self {
            ctx,
            state,
            renderer,
            pointer_position: None,
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        match event {
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let scale = self.ctx.pixels_per_point();
                self.pointer_position = Some(egui::pos2(
                    position.x as f32 / scale,
                    position.y as f32 / scale,
                ));
            }
            winit::event::WindowEvent::CursorLeft { .. } => self.pointer_position = None,
            _ => (),
        }
        let response = self.state.on_window_event(window, event);
        match event {
            winit::event::WindowEvent::MouseInput { .. } => self
                .pointer_position
                .is_some_and(|pos| pointer_over_debug_window(&self.ctx, pos)),
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                // P always remains available to hide/cycle the debug panel.
                event.physical_key
                    != winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyP)
                    && response.consumed
                    && self.ctx.egui_wants_keyboard_input()
            }
            _ => false,
        }
    }

    pub fn create_widget<'a>(&'a self, title: impl Into<String>) -> WidgetBuilder<'a> {
        WidgetBuilder::new_window(&self.ctx, title)
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.state.take_egui_input(window);
        self.ctx.begin_pass(raw_input);
    }

    pub fn end_frame(&mut self) -> egui::FullOutput {
        self.ctx.end_pass()
    }

    pub fn flush_into(
        &mut self,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
    ) {
        let context = context::context();

        let mut full_output = self.end_frame();
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let paint_jobs = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&context.device, &context.queue, *id, delta);
            }
        }

        let size = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        self.renderer.update_buffers(
            &context.device,
            &context.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let renderer = &mut self.renderer;
        {
            let debug_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Debug RenderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: surface_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

            let mut debug_pass = debug_pass.forget_lifetime();
            renderer.render(&mut debug_pass, &paint_jobs, &screen_descriptor);
            drop(debug_pass);
        }

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        full_output.textures_delta.clear();
    }
}

fn pointer_over_debug_window(ctx: &egui::Context, position: egui::Pos2) -> bool {
    ctx.layer_id_at(position)
        .is_some_and(|layer| layer.order != egui::Order::Background)
}
