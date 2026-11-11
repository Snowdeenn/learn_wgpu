struct Dummy {
    color: vec4f,
    offset: vec2f,
    scale: vec2f,
    layer: u32,
    rotation: f32,
    _padding1: u32,
    _padding2: u32,
    uv_min: vec2f,
    uv_max: vec2f,
}

struct Vertex {
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
    @location(2) color: vec4f,
}

struct UniCamera {
    view: mat4x4f,
}

struct VsOuput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

@group(0) @binding(0)
var<storage, read> dummies: array<Dummy>;

@group(1) @binding(0)
var<uniform> cam: UniCamera;

@vertex
fn vs_main(
    vertex: Vertex,
    @builtin(instance_index) instance_index: u32,
) -> VsOuput {
    var ouput: VsOuput;

    let dummy = dummies[instance_index];
    let angle = dummy.rotation;
    let local = vertex.pos * dummy.scale;
    let rotated = vec2f(local.x * cos(angle) - local.y * sin(angle),
                        local.x * sin(angle) + local.y * cos(angle));
    ouput.position = cam.view * vec4f(rotated + dummy.offset, 0.0, 1.0);
    ouput.color = dummy.color;

    return ouput;
}

@fragment
fn fs_main(fs_input: VsOuput) -> @location(0) vec4f {
    return fs_input.color;
}
