use std::{sync::Arc, vec};

use super::{instance::Dummy, resources::*};
use crate::backend::target::{self, RenderTargetDescriptor, TargetSize};
use crate::{
    backend::{self, buffer, frame_data::FrameData},
    camera::Camera2D,
    core::Resources,
    geometry::{mesh::Vertex, primitives::*},
    particle::{
        self, EffectDescription, EmitterSettings,
        modules::{
            ColorOverLifetime, ConeVelocity, Drag, Gravity, InitAngularVelocity, InitPosition,
            InitRotation, IntegratePosition, IntegrateRotation, NoiseForce, PositionShape,
            RandomLifetime, RenderShape, ScaleOverLifetime, SpawnRegion, UpdateAge, WithRotation,
        },
    },
};

pub fn initialize(resources: &mut Resources, format: wgpu::TextureFormat, resolution: [u32; 2]) {
    let texture_shader =
        backend::load_shader(Some("Texture Shader"), "assets/shaders/texture.wgsl").unwrap();
    let mesh_shader =
        backend::load_shader(Some("Mesh Shader"), "assets/shaders/mesh.wgsl").unwrap();

    let tex_array_shader =
        backend::load_shader(Some("Texture array"), "assets/shaders/texture_array.wgsl").unwrap();

    let rand = |min: f32, max: f32| rand::random_range(min..max);

    let mut instance_data = Vec::new();

    let texture_a = backend::load_texture("Drole", "assets/textures/image2.webp").unwrap();
    let texture_b = backend::load_texture("Drole2", "assets/textures/thats-funny.webp").unwrap();

    let (id_a, lay_a) = backend::register_texture(texture_a);
    let (id_b, _) = backend::register_texture(texture_b);

    let path = vec![
        "assets/textures/idle/IDLE0.png",
        "assets/textures/idle/IDLE1.png",
        "assets/textures/idle/IDLE2.png",
        "assets/textures/idle/IDLE3.png",
        "assets/textures/idle/IDLE4.png",
        "assets/textures/idle/IDLE5.png",
        "assets/textures/idle/IDLE6.png",
    ];
    let texture_array = backend::load_texture_array("text array", &path).unwrap();
    let (id_tex_array_a, tex_array_lay_a) = backend::register_texture_array(texture_array).unwrap();

    let path_b = vec![
        "assets/textures/run/RUN0.png",
        "assets/textures/run/RUN1.png",
        "assets/textures/run/RUN2.png",
        "assets/textures/run/RUN3.png",
        "assets/textures/run/RUN4.png",
        "assets/textures/run/RUN5.png",
        "assets/textures/run/RUN6.png",
        "assets/textures/run/RUN7.png",
    ];

    let tex_array_b = backend::load_texture_array("Run", &path_b).unwrap();
    let (id_tex_array_b, _) = backend::register_texture_array(tex_array_b).unwrap();

    let path_c = vec![
        "assets/textures/attack/ATTACK2_0.png",
        "assets/textures/attack/ATTACK2_1.png",
        "assets/textures/attack/ATTACK2_2.png",
        "assets/textures/attack/ATTACK2_3.png",
        "assets/textures/attack/ATTACK2_4.png",
    ];

    let tex_array_c = backend::load_texture_array("Attack", &path_c).unwrap();
    let (id_tex_array_c, _) = backend::register_texture_array(tex_array_c).unwrap();

    let (rect_mesh, _) = create_rect_vertices();
    let (circle_mesh, _) = create_circle_vertices(CircleOptions::default());
    let (triangle_mesh, _) = create_triangle_vertices();
    let rectangle = backend::shape::register(rect_mesh.clone());
    let circle = backend::shape::register(circle_mesh.clone());
    let triangle = backend::shape::register(triangle_mesh);

    let inst_1 = Dummy {
        mesh: rectangle,
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
        mesh: rectangle,
        color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
        scale: [100.0, 100.0],
        offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
        sprite: backend::Sprite::Separate {
            texture: id_b,
            indices: 0..rect_mesh.indices().len() as u32,
        },
    };
    instance_data.push(inst_2);

    let inst_3 = Dummy {
        mesh: rectangle,
        color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
        scale: [100.0, 100.0],
        offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
        sprite: backend::Sprite::Animated {
            texture: id_tex_array_a,
            layers: 0..7,
            current_layer: 0,
            elapsed: 0.0,
            indices: 0..rect_mesh.indices().len() as u32,
        },
    };
    instance_data.push(inst_3);

    let inst_4 = Dummy {
        mesh: rectangle,
        color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
        scale: [100.0, 100.0],
        offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
        sprite: backend::Sprite::Animated {
            texture: id_tex_array_b,
            layers: 0..8,
            current_layer: 0,
            elapsed: 0.0,
            indices: 0..rect_mesh.indices().len() as u32,
        },
    };
    instance_data.push(inst_4);

    let inst_5 = Dummy {
        mesh: rectangle,
        color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
        scale: [100.0, 100.0],
        offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
        sprite: backend::Sprite::Animated {
            texture: id_tex_array_c,
            layers: 0..5,
            current_layer: 0,
            elapsed: 0.0,
            indices: 0..rect_mesh.indices().len() as u32,
        },
    };
    instance_data.push(inst_5);

    let atlas = backend::load_texture("Atlas player", "assets/textures/spritesheet.png").unwrap();
    let regions = backend::build_atlas_region(
        "assets/textures/sprites.json",
        [atlas.width(), atlas.height()],
    )
    .unwrap();

    let (id_atlas, ids_regions) = backend::register_atlas(atlas, regions);

    let inst_6 = Dummy {
        mesh: rectangle,
        color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
        scale: [100.0, 100.0],
        offset: [rand(-300.0, 300.0), rand(-200.0, 200.0)],
        sprite: backend::Sprite::Atlas {
            texture: id_atlas,
            region: ids_regions[0],
            indices: 0..rect_mesh.indices().len() as u32,
        },
    };
    instance_data.push(inst_6);

    let instance_buffer = buffer::storage::<backend::RenderInstance>(instance_data.len() as u64)
        .expect("Impossible d'allouer le buffer d'instances");

    let vertex_buffer =
        buffer::storage_vertex::<Vertex>(50).expect("Impossible d'allouer le buffer de sommets");
    let index_buffer = buffer::index::<u32>(144).expect("Impossible d'allouer le buffer d'indices");

    let render_instances: Vec<_> = instance_data
        .iter()
        .map(backend::SpriteInstance::render_instance)
        .collect();

    backend::write_buffer(&instance_buffer, 0, &render_instances);
    backend::write_buffer(&vertex_buffer, 0, rect_mesh.vertices());
    backend::write_buffer(&index_buffer, 0, rect_mesh.indices());

    let index_count = rect_mesh.indices().len() as u32;

    let cam_buff =
        buffer::uniform::<math::Mat4>(1).expect("Impossible d'allouer le buffer de caméra");

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

    let tex_array_pipe_ctx = crate::pipeline_context! {
        TextureArray {
            shader: &tex_array_shader,
            format: format,
            fragment: "fs_main",
            bind_group: [
                (instance_lay,),
                (cam_layout, cam_group.clone()),
                (tex_array_lay_a,)
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
                (instance_lay,),
                (cam_layout, cam_group.clone())
            ],
            vertex_buffer: [(Vertex::layout(), &vertex_buffer)],
            index_buffer: None,
        }
    };

    let cam = Camera2D::new();

    resources.insert(InstanceBuffer(instance_buffer));
    resources.insert(IndexBuffer(index_buffer));
    resources.insert(VertexBuffer(vertex_buffer));
    resources.insert(ScenePipeline::default());
    resources.insert(ScenePipelines {
        mesh: backend::pipeline::register(mesh_pipe_ctx),
        texture: backend::pipeline::register(texture_pipe_ctx),
        texture_array: backend::pipeline::register(tex_array_pipe_ctx),
    });

    resources.insert(CamBuffer(cam_buff));
    resources.insert(cam);
    let mut data = FrameData::new();
    let instances = data.instances.insert(InstanceData {
        data: instance_data,
    });

    resources.insert(IndexCount(index_count));
    resources.insert(RectMesh(rectangle));
    resources.insert(CircleMesh(circle));
    resources.insert(TriangleMesh(triangle));

    let world_label = backend::text::register(backend::text::TextDescription {
        content: "Origine du monde",
        font_size: 18.0,
        line_height: 24.0,
        ..Default::default()
    })
    .expect("Impossible de creer le texte monde");

    let hud_label = backend::text::register(backend::text::TextDescription {
        content: "WASD : deplacer la camera",
        font_size: 18.0,
        line_height: 24.0,
        ..Default::default()
    })
    .expect("Impossible de creer le texte ecran");

    let mut world_text = backend::text::Text::new(world_label, backend::text::TextSpace::World);
    world_text.position = [0.0, -40.0];
    world_text.anchor = [0.5, 1.0];
    world_text.color = [1.0, 0.85, 0.3, 1.0];

    let mut hud_text = backend::text::Text::new(hud_label, backend::text::TextSpace::Screen);
    hud_text.position = [20.0, 20.0];

    data.text = vec![world_text, hud_text];

    let target = target::register(RenderTargetDescriptor {
        label: "Scene",
        size: TargetSize::Window {
            width: resolution[0],
            height: resolution[1],
        },
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        sample_count: 1,
    });

    let blur = crate::effects::blur::BlurEffect::new(format, resolution);
    let present = crate::effects::present::PresentEffect::new(target, format);
    let composition = crate::effects::composite::CompositeEffect::new(blur.target_a, format);

    resources.insert(SceneRendering {
        data,
        instances,
        passes: Vec::new(),
        target,
        blur,
        present,
        composition,
    });

    let bubble = backend::load_texture(
        "bubble",
        "assets/textures/Bubbles/Coloured/128x128/bubble_4.png",
    )
    .unwrap();

    let (_id_bubble, _) = backend::register_texture(bubble);

    particle::spawn_effect(
        Arc::new(EffectDescription {
            // Une seconde d'émission par bouffée, puis une pause avant la suivante.
            emitter: EmitterSettings::burst(1_000, 4.5),
            init_modules: vec![
                Box::new(InitPosition {
                    shape: PositionShape::Circle {
                        center: [0.0; 2],
                        radius: 18.0,
                    },
                    region: SpawnRegion::Volume,
                }),
                Box::new(ConeVelocity {
                    // L'axe Y de la caméra pointe vers le bas.
                    direction: -std::f32::consts::FRAC_PI_2,
                    spread: 0.45,
                    min_speed: 65.0,
                    max_speed: 110.0,
                }),
                Box::new(InitRotation {
                    min: -0.3,
                    max: 0.3,
                }),
                Box::new(InitAngularVelocity {
                    min: -0.4,
                    max: 0.4,
                }),
                Box::new(RandomLifetime { min: 2.2, max: 3.2 }),
            ],
            update_modules: vec![
                Box::new(UpdateAge),
                // Une poussée vers le haut et du bruit simulent la flottabilité.
                Box::new(Gravity {
                    acceleration: [0.0, -24.0],
                }),
                Box::new(NoiseForce {
                    strength: 28.0,
                    frequency: 0.025,
                    seed: 42,
                }),
                Box::new(Drag { coefficient: 0.35 }),
                Box::new(IntegratePosition),
                Box::new(IntegrateRotation),
                Box::new(ScaleOverLifetime {
                    start: [5.0; 2],
                    end: [15.0; 2],
                }),
                Box::new(ColorOverLifetime {
                    start: [1.0, 1.0, 1.0, 0.85],
                    end: [0.75, 0.9, 1.0, 0.0],
                }),
            ],
            render_modules: vec![Box::new(WithRotation(RenderShape {
                mesh: rectangle,
                indices: 0..index_count,
            }))],
        }),
        [30.0, 200.0],
    );
}
