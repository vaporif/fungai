#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct VignetteUniforms {
    color: vec4<f32>,
    intensity: f32,
    _padding: vec3<f32>,
};

@group(2) @binding(0) var<uniform> material: VignetteUniforms;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center) * 2.0;
    let vignette = smoothstep(0.3, 1.2, dist) * material.intensity;
    return vec4<f32>(material.color.rgb, vignette);
}
