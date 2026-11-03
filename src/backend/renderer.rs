use std::sync::Arc;

use winit::window::Window;

use crate::backend::pass::RenderPassBuilder;

use super::context;

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub conf: wgpu::SurfaceConfiguration,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::debugging() | wgpu::InstanceFlags::VALIDATION,
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // La surface possede un clone de l'Arc : sa duree de vie peut etre 'static.
        let surface = instance
            .create_surface(window.clone())
            .expect("Impossible de creer la surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Aucun adaptateur graphique compatible");

        context::init(&adapter);

        let size = window.inner_size();
        let mut conf = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("Aucune configuration de surface compatible");
        conf.present_mode = wgpu::PresentMode::Fifo;
        conf.format = wgpu::TextureFormat::Bgra8UnormSrgb;
        surface.configure(context::device(), &conf);

        Self { surface, conf }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.conf.width = size.width;
        self.conf.height = size.height;
        self.surface.configure(context::device(), &self.conf);
    }

    pub fn begin_frame<'a>(&'a self, window: &'a Window) -> Result<Option<Frame<'a>>, &'a str> {
        let encoder = context::device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main encoder"),
        });

        let surface = &self.surface;
        let conf = &self.conf;

        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Libérer la texture acquise avant de reconfigurer la surface.
                drop(frame);
                surface.configure(context::device(), conf);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(context::device(), conf);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface perdue ou erreur de validation wgpu");
            }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Frame view"),
            ..Default::default()
        });

        Ok(Some(Frame {
            window: window,
            texture_surface: frame,
            view,
            encoder,
        }))
    }
}

pub struct Frame<'a> {
    window: &'a Window,
    texture_surface: wgpu::SurfaceTexture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) encoder: wgpu::CommandEncoder,
}

impl<'a> Frame<'a> {
    pub(crate) fn size_in_pixels(&self) -> [u32; 2] {
        let size = self.texture_surface.texture.size();
        [size.width, size.height]
    }

    pub fn render(&mut self, draw: impl FnOnce(&mut wgpu::RenderPass<'_>)) {
        let mut pass = RenderPassBuilder::new()
            .with_label("Test Pass")
            .with_color_attachment_descriptor(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
            .begin(&mut self.encoder);

        draw(&mut pass);
    }

    pub fn finish(self) {
        context::queue().submit([self.encoder.finish()]);
        self.window.pre_present_notify();
        context::queue().present(self.texture_surface);
    }
}
