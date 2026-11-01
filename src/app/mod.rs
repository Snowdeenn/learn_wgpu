pub mod resource;
mod types;

use resource::Resources;
pub use types::*;

use std::{sync::Arc, time::Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::backend::renderer::Renderer;
use crate::backend::{self, PipelineContext};
use crate::backend::{buffer, mesh::*};

use crate::camera::{Camera2D, Camera2DController};

use crate::debug::{self, DebugManager};

pub struct Dummy {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub sprite: backend::Sprite,
}

crate::define_pipeline_context!(Mesh);
crate::define_pipeline_context!(Texture);

#[derive(Default)]
pub struct App {
    resources: Resources,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    debug: Option<DebugManager>,
    cam_controller: Option<Camera2DController>,
    last_frame: Option<Instant>,
}

impl App {
    fn initialize_scene(&mut self, format: wgpu::TextureFormat) {
        let texture_shader =
            backend::load_shader(Some("Texture Shader"), "src/shader/texture.wgsl").unwrap();
        let mesh_shader =
            backend::load_shader(Some("Mesh Shader"), "src/shader/mesh.wgsl").unwrap();

        let rand = |min: f32, max: f32| rand::random_range(min..max);

        let mut instance_data = Vec::new();

        let texture_a = backend::load_texture("Drole", "src/image2.webp").unwrap();
        let texture_b = backend::load_texture("Drole2", "src/thats-funny.webp").unwrap();

        let (id_a, lay_a) = backend::register_texture(texture_a);
        let (id_b, _) = backend::register_texture(texture_b);

        let (rect_mesh, _) = create_rect_vertices();
        let (circle_mesh, _) = create_circle_vertices(CircleOptions::default());
        let (triangle_mesh, _) = create_triangle_vertices();

        let inst_1 = Dummy {
            color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
            scale: [100.0, 100.0],
            offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
            sprite: backend::Sprite::Separate {
                texture: id_a,
                indices: 0..rect_mesh.indices().len() as u32,
            },
        };
        instance_data.push(inst_1);

        let inst_2 = Dummy {
            color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
            scale: [100.0, 100.0],
            offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
            sprite: backend::Sprite::Separate {
                texture: id_b,
                indices: 0..rect_mesh.indices().len() as u32,
            },
        };
        instance_data.push(inst_2);

        let instance_buffer = buffer::storage::<Dummy>("Dummy Buffer", 2);

        let vertex_buffer = buffer::storage_vertex::<Vertex>("Vertex Buffer", 50);
        let index_buffer = buffer::index::<u32>("Index Buffer", 144);

        let cam_buff = buffer::uniform::<math::Mat4>("Cam Uniform", 1);

        let (mesh_layout, mesh_group) = crate::bind_group! {
           wgpu::ShaderStages::VERTEX => vertex_buffer,
           wgpu::ShaderStages::VERTEX => instance_buffer,
        };

        let (instance_lay, _) = crate::bind_group! {
            wgpu::ShaderStages::VERTEX => instance_buffer,
        };

        let (cam_layout, cam_group) = crate::bind_group! {
            wgpu::ShaderStages::VERTEX_FRAGMENT => cam_buff
        };

        let texture_pipe_ctx = crate::pipeline_context! {
            Texture {
                shader: &texture_shader,
                format: format,
                fragment: "fs_main",
                bind_group: [
                    (instance_lay,),
                    (cam_layout, cam_group.clone()),
                    (lay_a,),
                ],
                vertex_buffer: [(Vertex::layout(), vertex_buffer.as_raw().clone())],
                index_buffer: Some(index_buffer.as_raw().clone()),
            }
        };

        let mesh_pipe_ctx = crate::pipeline_context! {
            Mesh {
                shader: &mesh_shader,
                format: format,
                fragment: "fs_main",
                bind_group: [
                    (mesh_layout, mesh_group.clone()),
                    (cam_layout, cam_group.clone())
                ],
                index_buffer: Some(index_buffer.as_raw().clone()),
            }
        };

        let cam = Camera2D::new();

        self.resources.insert(InstanceBuffer(instance_buffer));
        self.resources.insert(IndexBuffer(index_buffer));
        self.resources.insert(VertexBuffer(vertex_buffer));
        self.resources.insert(ScenePipeline::default());
        self.resources.insert(texture_pipe_ctx);
        self.resources.insert(mesh_pipe_ctx);
        self.resources.insert(CamBuffer(cam_buff));
        self.resources.insert(cam);
        self.resources.insert(InstanceData(instance_data));
        self.resources.insert(IndexCount(0));
        self.resources.insert(RectMesh(rect_mesh));
        self.resources.insert(CircleMesh(circle_mesh));
        self.resources.insert(TriangleMesh(triangle_mesh));
    }
}

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
        self.initialize_scene(renderer.conf.format);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.debug = Some(debug);
        self.cam_controller = Some(Camera2DController::new(250.0));

        self.resources.insert(Fps(0.0));
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

                match renderer.begin_frame(window) {
                    Ok(Some(mut frame)) => {
                        debug.begin_frame(window);
                        debug::run(debug, &self.resources);

                        let index_count = self.resources.read_resource::<IndexCount>();
                        let scene = *self.resources.read_resource::<ScenePipeline>();

                        frame.render(|pass| match scene {
                            ScenePipeline::Mesh => {
                                let ctx = self.resources.read_resource::<MeshPipelineContext>();
                                backend::bind(&*ctx, pass);
                                pass.draw_indexed(0..**index_count, 0, 0..1);
                            }
                            ScenePipeline::Texture => {
                                let ctx = self.resources.read_resource::<TexturePipelineContext>();
                                backend::bind(&*ctx, pass);

                                let mut instances = self.resources.write_resource::<InstanceData>();
                                let count = 0..instances.len() as u32;
                                backend::draw_all(&mut *instances, count, pass);
                            }
                        });

                        debug.flush_into(window, &mut frame.encoder, &frame.view);

                        frame.finish();
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
            backend::write_buffer(cam_buffer.as_raw(), 0, bytemuck::bytes_of(&view_proj));
            controller.end_frame();
        }
        window.request_redraw();
    }
}

struct CircleOptions {
    radius: f32,
    num_subdivisions: usize,
    inner_radius: f32,
    start_angle: f32,
    end_angle: f32,
}

impl Default for CircleOptions {
    fn default() -> Self {
        Self {
            radius: 1.0,
            num_subdivisions: 24,
            inner_radius: 0.0,
            start_angle: 0.0,
            end_angle: std::f32::consts::TAU,
        }
    }
}

fn create_circle_vertices(options: CircleOptions) -> (Mesh, usize) {
    let CircleOptions {
        radius,
        num_subdivisions,
        inner_radius,
        start_angle,
        end_angle,
    } = options;

    assert!(num_subdivisions > 0);

    let num_vertices = (num_subdivisions + 1) * 2;
    let mut mesh = Mesh::with_capacity(num_vertices * 2, num_subdivisions * 6);

    for i in 0..=num_subdivisions {
        let angle = start_angle + i as f32 * (end_angle - start_angle) / num_subdivisions as f32;

        let (s, c) = angle.sin_cos();

        mesh.extend_vertices_from_slice(&[
            Vertex {
                pos: [c * radius, s * radius],
                ..Default::default()
            },
            Vertex {
                pos: [c * inner_radius, s * inner_radius],
                ..Default::default()
            },
        ]);
    }

    for i in 0..num_subdivisions {
        let outer1 = (i * 2) as u32;
        let inner1 = outer1 + 1;
        let outer2 = outer1 + 2;
        let inner2 = outer1 + 3;

        mesh.extend_indices_from_slice(&[outer1, outer2, inner1, inner1, outer2, inner2]);
    }

    (mesh, num_vertices)
}

fn create_triangle_vertices() -> (Mesh, usize) {
    let pos = vec![
        Vertex {
            pos: [0.0, 0.5],
            ..Default::default()
        },
        Vertex {
            pos: [0.5, -0.5],
            ..Default::default()
        },
        Vertex {
            pos: [-0.5, -0.5],
            ..Default::default()
        },
    ];
    let mut mesh = Mesh::new();
    pos.into_iter().for_each(|v| {
        let idx = mesh.push_vertex(v);
        mesh.push_index(idx);
    });

    let num_vertices = 3;

    (mesh, num_vertices)
}

fn create_rect_vertices() -> (Mesh, usize) {
    let pos = vec![
        Vertex {
            pos: [-0.5, 0.5], // Bas gauche
            uv: [0.0, 1.0],
            ..Default::default()
        },
        Vertex {
            pos: [0.5, 0.5], // Bas droit
            uv: [1.0, 1.0],
            ..Default::default()
        },
        Vertex {
            pos: [0.5, -0.5], // Haut droit
            uv: [1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            pos: [-0.5, -0.5], // Haut gauche
            uv: [0.0, 0.0],
            ..Default::default()
        },
    ];

    let mut mesh = Mesh::new();
    pos.into_iter().for_each(|v| {
        let _ = mesh.push_vertex(v);
    });
    mesh.push_triangle(0, 1, 2);
    mesh.push_triangle(2, 3, 0);
    let num_vertices = 4;

    (mesh, num_vertices)
}
