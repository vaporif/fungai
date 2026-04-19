#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct NetworkUniforms {
    core_color: vec4<f32>,
    body_color: vec4<f32>,
    biomass: f32,
    time: f32,
    _padding: vec2<f32>,
};

@group(2) @binding(0) var<uniform> material: NetworkUniforms;

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center) * 2.0;

    // Noise-distorted edge for organic shape
    let edge_noise = noise2d(uv * 8.0 + material.time * 0.1) * 0.15;
    let fine_noise = noise2d(uv * 20.0) * 0.08;
    let distorted_dist = dist + edge_noise + fine_noise;

    // Soft radial falloff — discard fully transparent pixels
    let outer_edge = smoothstep(1.0, 0.65, distorted_dist);
    if outer_edge < 0.01 {
        discard;
    }

    let glow = clamp(material.biomass * 0.15, 0.05, 0.8);

    // Internal fibrous texture
    let fiber1 = noise2d(uv * 12.0 + vec2<f32>(material.time * 0.05, 0.0));
    let fiber2 = noise2d(uv * 25.0 + vec2<f32>(0.0, material.time * 0.03));
    let fibers = fiber1 * 0.6 + fiber2 * 0.4;

    // Core glow at center
    let core_strength = smoothstep(0.5, 0.0, distorted_dist) * glow;

    // Compose layers
    var color = material.body_color.rgb * (0.3 + fibers * 0.4);
    color = mix(color, material.core_color.rgb, core_strength);

    // Edge glow ring
    let edge_glow = smoothstep(0.9, 0.6, distorted_dist) * smoothstep(0.4, 0.7, distorted_dist);
    color = color + material.core_color.rgb * edge_glow * glow * 0.5;

    // Pulsing veins
    let vein = smoothstep(0.48, 0.5, noise2d(uv * 15.0 + material.time * 0.08));
    color = color + material.core_color.rgb * vein * 0.15 * glow;

    let alpha = outer_edge * (0.5 + core_strength * 0.5);
    return vec4<f32>(color, alpha);
}
