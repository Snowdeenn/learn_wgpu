struct Dummy {
    color: vec4f,
    offset: vec2f,
    scale: vec2f,
    layer: u32,
    _padding0: u32,
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
    @location(0) texcoord: vec2f,
    @location(1) @interpolate(flat) layer: u32,
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
    ouput.position = cam.view * vec4f(vertex.pos * dummy.scale + dummy.offset, 0.0, 1.0);
    ouput.texcoord = vertex.uv;
    ouput.layer = dummy.layer;

    return ouput;
}

@group(2) @binding(0)
var tex: texture_2d_array<f32>;
@group(2) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(fs_input: VsOuput) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, fs_input.texcoord, fs_input.layer);
}
