struct Dummy {
    color: vec4f,
    offset: vec2f,
    scale: vec2f,
}

struct Vertex {
    pos: vec2f,
    uv: vec2f,
    color: vec4f,
}

struct UniCamera {
    view: mat4x4f,
}

struct VsOuput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

@group(0) @binding(0)
var<storage, read> vertices: array<Vertex>;
@group(0) @binding(1)
var<storage, read> dummies: array<Dummy>;

@group(1) @binding(0)
var<uniform> cam: UniCamera;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VsOuput {
    var ouput: VsOuput;

    let vertex = vertices[vertex_index];
    let dummy = dummies[instance_index];
    ouput.position = cam.view * vec4f(vertex.pos * dummy.scale + dummy.offset, 0.0, 1.0);
    ouput.color = dummy.color;

    return ouput;
}

@fragment
fn fs_main(fs_input: VsOuput) -> @location(0) vec4f {
    return fs_input.color;
}
