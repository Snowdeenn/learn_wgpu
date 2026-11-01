#![allow(unused)]

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn with_capacity(verts: usize, index: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(verts),
            indices: Vec::with_capacity(index),
        }
    }

    pub fn push_vertex(&mut self, v: Vertex) -> u32 {
        let idx = self.vertices.len();
        self.vertices.push(v);
        idx as u32
    }

    pub fn extend_vertices_from_slice(&mut self, v: &[Vertex]) {
        self.vertices.extend_from_slice(v);
    }

    pub fn push_index(&mut self, idx: u32) {
        self.indices.push(idx);
    }

    pub fn extend_indices_from_slice(&mut self, i: &[u32]) {
        self.indices.extend_from_slice(i);
    }

    pub fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    pub fn clear(&mut self) {
        self.indices.clear();
        self.vertices.clear();
    }

    pub fn vertices_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn indices_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn append(&mut self, other: &Mesh) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(other.vertices());
        self.indices
            .extend(other.indices().iter().map(|i| i + offset));
    }
}
