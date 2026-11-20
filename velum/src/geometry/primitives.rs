use super::mesh::{Mesh, Vertex};

pub struct CircleOptions {
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

pub fn create_circle_vertices(options: CircleOptions) -> (Mesh, usize) {
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

pub fn create_triangle_vertices() -> (Mesh, usize) {
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

pub fn create_rect_vertices() -> (Mesh, usize) {
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
