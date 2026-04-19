#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct TerrainUniforms {
    base_color: vec4<f32>,
    terrain_type: u32,
    grid_x: u32, // axial q coordinate, used as noise seed
    grid_y: u32, // axial r coordinate, used as noise seed
    discovered: f32,
    time: f32,
    nutrient_level: f32,
    _padding: f32,
};

@group(2) @binding(0) var<uniform> material: TerrainUniforms;

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = dot(hash22(i + vec2<f32>(0.0, 0.0)) - 0.5, f - vec2<f32>(0.0, 0.0));
    let b = dot(hash22(i + vec2<f32>(1.0, 0.0)) - 0.5, f - vec2<f32>(1.0, 0.0));
    let c = dot(hash22(i + vec2<f32>(0.0, 1.0)) - 0.5, f - vec2<f32>(0.0, 1.0));
    let d = dot(hash22(i + vec2<f32>(1.0, 1.0)) - 0.5, f - vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) + 0.5;
}

fn voronoi(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    var min_dist = 1.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let point = hash22(i + neighbor);
            let diff = neighbor + point - f;
            let dist = length(diff);
            min_dist = min(min_dist, dist);
        }
    }
    return min_dist;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let world_uv = vec2<f32>(f32(material.grid_x), f32(material.grid_y)) + uv;

    var color = material.base_color.rgb;
    let t_type = material.terrain_type;

    if t_type == 0u {
        let coarse = noise2d(world_uv * 4.0);
        let fine = noise2d(world_uv * 16.0);
        color = color + (coarse - 0.5) * 0.06 + (fine - 0.5) * 0.03;
    }
    else if t_type == 1u {
        let v = voronoi(world_uv * 6.0);
        let crack = smoothstep(0.0, 0.08, v);
        color = color * (0.7 + crack * 0.3);
        color = color + noise2d(world_uv * 8.0) * 0.03;
    }
    else if t_type == 2u {
        let scroll = vec2<f32>(material.time * 0.05, material.time * 0.03);
        let wave = noise2d((world_uv + scroll) * 5.0);
        color = color + vec3<f32>(-0.02, 0.0, 0.06) * (wave - 0.5);
    }
    else if t_type == 3u {
        let grain = noise2d(world_uv * 12.0);
        color = color + (grain - 0.5) * 0.04;
    }
    else if t_type == 4u {
        let grid_noise = noise2d(world_uv * 8.0);
        let lines = step(0.45, fract(world_uv.x * 3.0)) * step(0.45, fract(world_uv.y * 3.0));
        color = color + grid_noise * 0.04 + lines * 0.03;
    }
    else if t_type == 5u {
        let vein = noise2d(world_uv * 10.0);
        let pulse = sin(material.time * 2.0) * 0.5 + 0.5;
        color = color + vec3<f32>(0.0, 0.06, -0.02) * vein * (0.6 + pulse * 0.4);
    }
    else if t_type == 6u {
        let grass = noise2d(world_uv * 14.0);
        let blades = noise2d(world_uv * 30.0);
        color = color + vec3<f32>(-0.01, 0.04, -0.01) * grass + blades * 0.02;
    }

    // Nutrient overlay: green tint for high nutrients, red tint for depleted
    let n = material.nutrient_level;
    if n > 0.6 {
        let strength = (n - 0.6) * 0.15;
        color = color + vec3<f32>(-0.02, strength, -0.02);
    } else if n < 0.3 {
        let strength = (0.3 - n) * 0.12;
        color = color + vec3<f32>(strength, -0.02, -0.02);
    }

    // Fog of war: noise-dithered two-stage reveal
    var disc = material.discovered;
    if disc < 1.0 {
        // Noise dithering to scatter the boundary into a dissolve pattern
        disc = disc + (noise2d(world_uv * 6.0) - 0.5) * 0.25;
        disc = clamp(disc, 0.0, 1.0);

        let gray = dot(color, vec3<f32>(0.299, 0.587, 0.114));
        let desat = vec3<f32>(gray);

        // Stage 1: black to dim desaturated (disc 0.0 to 0.4)
        let dim = mix(vec3<f32>(0.0), desat * 0.35, smoothstep(0.0, 0.4, disc));
        // Stage 2: dim to full color (disc 0.4 to 1.0)
        color = mix(dim, color, smoothstep(0.4, 1.0, disc));
    }

    return vec4<f32>(color, 1.0);
}
