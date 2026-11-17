use crate::api::scene::RenderData;
use std::sync::Arc;

use winit::window::Window;

use super::pass::RenderPassBuilder;

use super::context;

#[derive(Debug)]
pub enum RendererInitError {
    Surface(wgpu::CreateSurfaceError),
    Adapter(wgpu::RequestAdapterError),
    Context(context::InitError),
    SurfaceConfiguration,
    Text(super::text::InitError),
}
impl std::fmt::Display for RendererInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(e) => write!(f, "Surface creation failed: {e}"),
            Self::Adapter(e) => write!(f, "Adapter request failed: {e}"),
            Self::Context(e) => write!(f, "Context initialization failed: {e}"),
            Self::SurfaceConfiguration => f.write_str("No compatible surface configuration"),
            Self::Text(e) => write!(f, "Text initialization failed: {e}"),
        }
    }
}
impl std::error::Error for RendererInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Surface(e) => Some(e),
            Self::Adapter(e) => Some(e),
            Self::Context(e) => Some(e),
            Self::Text(e) => Some(e),
            _ => None,
        }
    }
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub conf: wgpu::SurfaceConfiguration,
}

fn select_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| formats.first().copied())
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self, RendererInitError> {
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
            .map_err(RendererInitError::Surface)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(RendererInitError::Adapter)?;

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = select_surface_format(&capabilities.formats)
            .ok_or(RendererInitError::SurfaceConfiguration)?;

        let mut conf = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(RendererInitError::SurfaceConfiguration)?;

        conf.present_mode = wgpu::PresentMode::Fifo;
        conf.format = format;

        context::init(&adapter).map_err(RendererInitError::Context)?;
        surface.configure(&context::device(None), &conf);

        super::text::init(conf.format).map_err(RendererInitError::Text)?;

        Ok(Self { surface, conf })
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.conf.width = size.width;
        self.conf.height = size.height;
        self.surface.configure(&context::device(None), &self.conf);
        super::target::resize_window_targets([size.width, size.height]);
    }

    pub fn begin_frame<'a>(&'a self, window: &'a Window) -> Result<Option<Frame<'a>>, &'a str> {
        let encoder =
            context::device(None).create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main encoder"),
            });

        let surface = &self.surface;
        let conf = &self.conf;

        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Libérer la texture acquise avant de reconfigurer la surface.
                drop(frame);
                surface.configure(&context::device(None), conf);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&context::device(None), conf);
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
            window,
            texture_surface: frame,
            view,
            encoder,
            _cleanup: FrameCleanup,
        }))
    }
}

/// Runs after the frame's encoder is discarded or submitted.
struct FrameCleanup;

impl Drop for FrameCleanup {
    fn drop(&mut self) {
        super::instances::reset_frame();
        super::text::reset_frame();
    }
}

pub struct Frame<'a> {
    window: &'a Window,
    texture_surface: wgpu::SurfaceTexture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) encoder: wgpu::CommandEncoder,
    // Keep this after the encoder: discard its commands before reusing frame resources.
    _cleanup: FrameCleanup,
}

impl<'a> Frame<'a> {
    pub fn execute(
        &mut self,
        passes: &mut [super::pass::PassContext],
        data: &mut RenderData,
    ) -> Result<(), super::pass::PassError> {
        super::pass::execute(passes, &mut self.encoder, &self.view, data)
    }

    pub fn size_in_pixels(&self) -> [u32; 2] {
        let size = self.texture_surface.texture.size();
        [size.width, size.height]
    }

    /// Accesses the encoder and output view for custom rendering integrations.
    pub fn render_parts(&mut self) -> (&mut wgpu::CommandEncoder, &wgpu::TextureView) {
        (&mut self.encoder, &self.view)
    }

    #[deprecated(note = "Utiliser Frame::execute à la place")]
    #[allow(unused)]
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

    pub fn finish(self, measure_gpu_wait: bool) -> Option<std::time::Duration> {
        let submission = context::queue(None).submit([self.encoder.finish()]);
        let gpu_wait = measure_gpu_wait
            .then(|| {
                let start = std::time::Instant::now();
                context::device(None)
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(std::time::Duration::from_secs(10)),
                    })
                    .map(|_| start.elapsed())
            })
            .and_then(Result::ok);
        super::instances::submitted();
        super::text::submitted();
        self.window.pre_present_notify();
        context::queue(None).present(self.texture_surface);
        gpu_wait
    }
}

#[cfg(test)]
mod tests {
    use super::select_surface_format;
    use wgpu::TextureFormat::*;

    #[test]
    fn surface_format_prefers_an_advertised_srgb_format() {
        assert_eq!(
            select_surface_format(&[Rgba8Unorm, Rgba8UnormSrgb, Bgra8UnormSrgb]),
            Some(Rgba8UnormSrgb)
        );
    }

    #[test]
    fn surface_format_falls_back_to_the_first_advertised_format() {
        assert_eq!(
            select_surface_format(&[Rgba8Unorm, Bgra8Unorm]),
            Some(Rgba8Unorm)
        );
    }

    #[test]
    fn empty_surface_formats_have_no_configuration() {
        assert_eq!(select_surface_format(&[]), None);
    }
}
