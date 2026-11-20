struct Dummy {
    color: vec4f,
    offset: vec2f,
    scale: vec2f,
    layer: u32,
    rotation: f32,
    depth: f32,
    _padding1: u32,
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
    @interpolate(flat)
    @location(1) layer: u32,
    @location(2) color: vec4f,
}

@group(0) @binding(0)
var<storage, read> dummies: array<Dummy>;

@group(1) @binding(0)
var<uniform> cam: UniCamera;

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VsOuput {
    var output: VsOuput;

    let dummy = dummies[instance_index];
    let angle = dummy.rotation;
    let local = vertex.pos * dummy.scale;
    let rotated = vec2f(
            local.x * cos(angle) - local.y * sin(angle),
            local.x * sin(angle) + local.y * cos(angle),
        );
    output.position = cam.view * vec4f(rotated + dummy.offset, 0.0, 1.0);
    output.position.z = dummy.depth * output.position.w;
    output.texcoord = vertex.uv;
    output.layer = dummy.layer;
    output.color = dummy.color;

    return output;
}

@group(2) @binding(0)
var tex: texture_2d_array<f32>;
@group(2) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_cutout(input: VsOuput) -> @location(0) vec4f {
    let color = textureSample(tex, tex_sampler, input.texcoord, i32(input.layer)) * input.color;

    if color.a < 0.5 {
        discard;
    }

    return vec4f(color.rgb, 1.0);
}

@fragment
fn fs_main(input: VsOuput) -> @location(0) vec4f {
    return textureSample(tex, tex_sampler, input.texcoord, i32(input.layer)) * input.color;
}
