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
    let u = uv.x;
    let v = uv.y;
    let dist = abs(u);

    let edge_noise = noise2d(vec2<f32>(v * 12.0, u * 4.0) + material.time * 0.05) * 0.1;
    let distorted_dist = dist + edge_noise;

    let outer_edge = smoothstep(1.0, 0.5, distorted_dist);
    if outer_edge < 0.01 {
        discard;
    }

    let glow = clamp(material.biomass * 0.15, 0.05, 0.8);

    let fiber1 = noise2d(vec2<f32>(u * 3.0, v * 15.0) + vec2<f32>(0.0, material.time * 0.03));
    let fiber2 = noise2d(vec2<f32>(u * 6.0, v * 30.0));
    let fibers = fiber1 * 0.6 + fiber2 * 0.4;

    let core_strength = smoothstep(0.4, 0.0, distorted_dist) * glow;

    var color = material.body_color.rgb * (0.3 + fibers * 0.4);
    color = mix(color, material.core_color.rgb, core_strength);

    let edge_glow = smoothstep(0.9, 0.5, distorted_dist) * smoothstep(0.2, 0.5, distorted_dist);
    color = color + material.core_color.rgb * edge_glow * glow * 0.4;

    let tip_pulse = smoothstep(0.7, 1.0, v) * (sin(material.time * 3.0) * 0.5 + 0.5);
    color = color + material.core_color.rgb * tip_pulse * 0.3;

    let alpha = outer_edge * (0.4 + core_strength * 0.6);
    return vec4<f32>(color, alpha);
}
