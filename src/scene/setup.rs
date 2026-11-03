use super::{instance::Dummy, resources::*};
use crate::{
    backend::{self, buffer},
    camera::Camera2D,
    core::Resources,
    geometry::{mesh::Vertex, primitives::*},
};

pub fn initialize(resources: &mut Resources, format: wgpu::TextureFormat) {
    let texture_shader =
        backend::load_shader(Some("Texture Shader"), "assets/shaders/texture.wgsl").unwrap();
    let mesh_shader =
        backend::load_shader(Some("Mesh Shader"), "assets/shaders/mesh.wgsl").unwrap();

    let rand = |min: f32, max: f32| rand::random_range(min..max);

    let mut instance_data = Vec::new();

    let texture_a = backend::load_texture("Drole", "assets/textures/image2.webp").unwrap();
    let texture_b = backend::load_texture("Drole2", "assets/textures/thats-funny.webp").unwrap();

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

    let instance_buffer = buffer::storage::<backend::RenderInstance>(instance_data.len() as u64)
        .expect("Impossible d'allouer le buffer d'instances");

    let vertex_buffer = buffer::storage_vertex::<Vertex>(50)
        .expect("Impossible d'allouer le buffer de sommets");
    let index_buffer = buffer::index::<u32>(144)
        .expect("Impossible d'allouer le buffer d'indices");

    let cam_buff = buffer::uniform::<math::Mat4>(1)
        .expect("Impossible d'allouer le buffer de caméra");

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
            vertex_buffer: [(Vertex::layout(), &vertex_buffer)],
            index_buffer: Some(&index_buffer),
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
            index_buffer: Some(&index_buffer),
        }
    };

    let cam = Camera2D::new();

    resources.insert(InstanceBuffer(instance_buffer));
    resources.insert(IndexBuffer(index_buffer));
    resources.insert(VertexBuffer(vertex_buffer));
    resources.insert(ScenePipeline::default());
    resources.insert(texture_pipe_ctx);
    resources.insert(mesh_pipe_ctx);
    resources.insert(CamBuffer(cam_buff));
    resources.insert(cam);
    resources.insert(InstanceData(instance_data));
    resources.insert(IndexCount(0));
    resources.insert(RectMesh(rect_mesh));
    resources.insert(CircleMesh(circle_mesh));
    resources.insert(TriangleMesh(triangle_mesh));
}
