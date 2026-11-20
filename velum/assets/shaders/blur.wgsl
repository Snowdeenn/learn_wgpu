struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    let position = positions[index];

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = vec2<f32>(
        (position.x + 1.0) * 0.5,
        (1.0 - position.y) * 0.5,
    );
    return output;
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

struct BlurParams {
    direction: vec2<f32>,
    spacing: f32,
    radius: u32,
    weights: array<vec4<f32>, 17>,
};

@group(1) @binding(0) var<uniform> params: BlurParams;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Noyau gaussien de rayon 2, sigma ≈ 1, normalisé.

    let dimensions = vec2<f32>(textureDimensions(source_texture));

    // Déplacement d'un pixel en coordonnées UV.
    let step = params.direction * params.spacing / dimensions;

    // Contribution du pixel central.
    var color = textureSample(
        source_texture,
        source_sampler,
        input.uv,
    ) * params.weights[0].x;

    // Contributions symétriques des voisins.
    for (var i = 1u; i <= min(params.radius, 16u); i++) {
        let offset = step * f32(i);

        color += textureSample(
            source_texture,
            source_sampler,
            input.uv + offset,
        ) * params.weights[i].x;

        color += textureSample(
            source_texture,
            source_sampler,
            input.uv - offset,
        ) * params.weights[i].x;
    }

    return color;
}
