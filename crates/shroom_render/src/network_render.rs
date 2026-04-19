use bevy::{
    asset::RenderAssetUsages,
    mesh::PrimitiveTopology,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

use crate::data_layer::{BranchGraph, RivalBranchGraph, TipPositions};
use crate::terrain_render::TILE_SIZE;

const SPLINE_SAMPLES: usize = 12;
const STRANDS_PER_EDGE: usize = 3;

#[derive(Component)]
pub struct NetworkPathSprite;

#[derive(Component)]
pub struct NetworkMesh;

#[derive(Component)]
pub struct JunctionMesh;

/// Packed uniform struct — matches the WGSL `NetworkUniforms` struct exactly.
#[derive(ShaderType, Debug, Clone)]
pub struct NetworkUniforms {
    pub core_color: LinearRgba, // vec4<f32> — 16 bytes
    pub body_color: LinearRgba, // vec4<f32> — 16 bytes
    pub biomass: f32,           // f32 — 4 bytes
    pub time: f32,              // f32 — 4 bytes
    pub _padding: Vec2,         // pad to 16-byte boundary — 8 bytes
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct NetworkMaterial {
    #[uniform(0)]
    pub uniforms: NetworkUniforms,
}

impl Material2d for NetworkMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/network.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Catmull-Rom spline segment: curve passes through p1..p2, with p0/p3 as tangent guides.
#[must_use]
pub fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Map a specialization to its display color (legacy, returns Color for existing tests).
#[cfg(test)]
#[must_use]
fn region_color(spec: Option<shroom_core::SpecializationType>) -> Color {
    use shroom_core::SpecializationType;
    match spec {
        Some(SpecializationType::Explorer) => Color::srgb(1.0, 0.9, 0.3),
        Some(SpecializationType::Parasite) => Color::srgb(0.8, 0.2, 0.2),
        Some(SpecializationType::Researcher) => Color::srgb(0.3, 0.5, 0.9),
        Some(SpecializationType::Hunter) => Color::srgb(0.6, 0.4, 0.1),
        Some(SpecializationType::Decomposer) => Color::srgb(0.2, 0.7, 0.3),
        Some(SpecializationType::Symbiont) => Color::srgb(0.3, 0.8, 0.8),
        Some(SpecializationType::Infiltrator) => Color::srgb(0.6, 0.3, 0.8),
        Some(SpecializationType::Transporter) => Color::srgb(0.9, 0.6, 0.2),
        None => Color::srgb(0.9, 0.85, 0.7),
    }
}

/// Map a specialization to its core color as `LinearRgba`.
#[must_use]
fn region_color_linear(spec: Option<shroom_core::SpecializationType>) -> LinearRgba {
    use shroom_core::SpecializationType;
    match spec {
        Some(SpecializationType::Explorer) => LinearRgba::new(1.0, 0.9, 0.3, 1.0),
        Some(SpecializationType::Parasite) => LinearRgba::new(0.8, 0.2, 0.2, 1.0),
        Some(SpecializationType::Researcher) => LinearRgba::new(0.3, 0.5, 0.9, 1.0),
        Some(SpecializationType::Hunter) => LinearRgba::new(0.6, 0.4, 0.1, 1.0),
        Some(SpecializationType::Decomposer) => LinearRgba::new(0.2, 0.7, 0.3, 1.0),
        Some(SpecializationType::Symbiont) => LinearRgba::new(0.3, 0.8, 0.8, 1.0),
        Some(SpecializationType::Infiltrator) => LinearRgba::new(0.6, 0.3, 0.8, 1.0),
        Some(SpecializationType::Transporter) => LinearRgba::new(0.9, 0.6, 0.2, 1.0),
        None => LinearRgba::new(0.9, 0.85, 0.7, 1.0),
    }
}

/// Derive a muted body color from a bright core color using luminance mixing.
#[must_use]
fn body_color_from_core(core: LinearRgba) -> LinearRgba {
    let gray = core.red * 0.299 + core.green * 0.587 + core.blue * 0.114;
    LinearRgba::new(
        (core.red * 0.4 + gray * 0.6) * 0.5,
        (core.green * 0.4 + gray * 0.6) * 0.5,
        (core.blue * 0.4 + gray * 0.6) * 0.5,
        0.7,
    )
}

/// Build a triangle-strip mesh for a Catmull-Rom spline between two endpoints.
///
/// Returns the mesh and the list of sampled centerline points (useful for testing).
/// UV_0: left vertex gets `[-1.0, v]`, right vertex gets `[1.0, v]` where v in [0, 1].
#[cfg(test)]
#[must_use]
fn build_spline_mesh(from: Vec2, to: Vec2, half_width: f32) -> (Mesh, Vec<Vec2>) {
    build_spline_mesh_inner(from, to, half_width, None)
}

/// Build a spline mesh with per-sample perpendicular noise wobble on interior points.
///
/// `seed` drives a deterministic hash so the same edge always produces the same shape.
/// Endpoints are never displaced to preserve junction alignment.
#[must_use]
fn build_spline_mesh_with_wobble(
    from: Vec2,
    to: Vec2,
    half_width: f32,
    seed: u32,
) -> (Mesh, Vec<Vec2>) {
    build_spline_mesh_inner(from, to, half_width, Some(seed))
}

/// Shared implementation — `wobble_seed = None` for straight, `Some(seed)` for wobble.
fn build_spline_mesh_inner(
    from: Vec2,
    to: Vec2,
    half_width: f32,
    wobble_seed: Option<u32>,
) -> (Mesh, Vec<Vec2>) {
    // Extrapolate control points for tangent continuity
    let dir = to - from;
    let p0 = from - dir;
    let p3 = to + dir;

    // Sample centerline
    let mut points = Vec::with_capacity(SPLINE_SAMPLES);
    for i in 0..SPLINE_SAMPLES {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / (SPLINE_SAMPLES - 1) as f32;
        points.push(catmull_rom(p0, from, to, p3, t));
    }

    // Apply perpendicular wobble to interior points when a seed is provided
    if let Some(seed) = wobble_seed {
        let branch_len = dir.length();
        let wobble_scale = (branch_len * 0.12).min(8.0);

        // Precompute tangents before mutating points
        let mut normals: Vec<Vec2> = Vec::with_capacity(SPLINE_SAMPLES);
        for i in 0..SPLINE_SAMPLES {
            let tangent = if i == 0 {
                points[1] - points[0]
            } else if i == SPLINE_SAMPLES - 1 {
                points[SPLINE_SAMPLES - 1] - points[SPLINE_SAMPLES - 2]
            } else {
                points[i + 1] - points[i - 1]
            };
            normals.push(Vec2::new(-tangent.y, tangent.x).normalize_or_zero());
        }

        for i in 1..(SPLINE_SAMPLES - 1) {
            #[allow(clippy::cast_possible_truncation)]
            let hash = seed
                .wrapping_mul(2_654_435_761)
                .wrapping_add(i as u32 * 73_856_093);
            #[allow(clippy::cast_precision_loss)]
            let noise = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
            points[i] += normals[i] * noise * wobble_scale;
        }
    }

    // Build triangle-strip vertices + UV_0
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(SPLINE_SAMPLES * 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(SPLINE_SAMPLES * 2);

    for i in 0..SPLINE_SAMPLES {
        let tangent = if i == 0 {
            points[1] - points[0]
        } else if i == SPLINE_SAMPLES - 1 {
            points[SPLINE_SAMPLES - 1] - points[SPLINE_SAMPLES - 2]
        } else {
            points[i + 1] - points[i - 1]
        };

        let normal = Vec2::new(-tangent.y, tangent.x).normalize_or_zero();
        let left = points[i] + normal * half_width;
        let right = points[i] - normal * half_width;

        positions.push([left.x, left.y, 0.0]);
        positions.push([right.x, right.y, 0.0]);

        #[allow(clippy::cast_precision_loss)]
        let v = i as f32 / (SPLINE_SAMPLES - 1) as f32;
        uvs.push([-1.0, v]);
        uvs.push([1.0, v]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleStrip,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    (mesh, points)
}

/// Count how many edges connect to each node for junction detection.
fn count_node_edges(graph: &BranchGraph) -> std::collections::HashMap<IVec2, usize> {
    let mut counts: std::collections::HashMap<IVec2, usize> = std::collections::HashMap::new();
    for edge in &graph.edges {
        *counts.entry(edge.from).or_default() += 1;
        *counts.entry(edge.to).or_default() += 1;
    }
    counts
}

pub fn network_render_system(
    mut commands: Commands,
    graph: Res<BranchGraph>,
    rival_graph: Res<RivalBranchGraph>,
    tip_positions: Res<TipPositions>,
    existing_meshes: Query<Entity, With<NetworkMesh>>,
    existing_junctions: Query<Entity, With<JunctionMesh>>,
    existing_sprites: Query<Entity, With<NetworkPathSprite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut net_materials: ResMut<Assets<NetworkMaterial>>,
    time: Res<Time>,
) {
    // Despawn all previous visuals
    for entity in existing_meshes.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_junctions.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_sprites.iter() {
        commands.entity(entity).despawn();
    }

    let elapsed = time.elapsed_secs();

    // Render each edge as multiple wobbled sub-strands
    for edge in &graph.edges {
        let from = edge.from.as_vec2() * TILE_SIZE;
        let to = edge.to.as_vec2() * TILE_SIZE;
        let total_width = (edge.thickness * 2.0).clamp(2.0, 8.0);
        let strand_width = total_width / STRANDS_PER_EDGE as f32;

        let spec = graph.nodes.get(&edge.from).and_then(|n| n.specialization);
        let core = region_color_linear(spec);
        let body = body_color_from_core(core);

        let base_seed =
            (edge.from.x.wrapping_mul(73_856_093) ^ edge.to.y.wrapping_mul(19_349_663)) as u32;

        // Perpendicular to edge direction — used to spread strands apart
        let edge_dir = (to - from).normalize_or_zero();
        let perp = Vec2::new(-edge_dir.y, edge_dir.x);

        for strand in 0..STRANDS_PER_EDGE {
            let strand_seed = base_seed.wrapping_add((strand as u32).wrapping_mul(2_654_435_761));

            // Fan strands out from the center line
            let spread = (strand as f32 - (STRANDS_PER_EDGE - 1) as f32 * 0.5) * strand_width * 0.8;
            let strand_from = from + perp * spread;
            let strand_to = to + perp * spread;

            let (mesh, _) = build_spline_mesh_with_wobble(
                strand_from,
                strand_to,
                strand_width * 0.5,
                strand_seed,
            );

            // Center strand is brighter
            let strand_biomass = if strand == 0 {
                edge.thickness
            } else {
                edge.thickness * 0.7
            };

            commands.spawn((
                NetworkMesh,
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(net_materials.add(NetworkMaterial {
                    uniforms: NetworkUniforms {
                        core_color: core,
                        body_color: body,
                        biomass: strand_biomass,
                        time: elapsed,
                        _padding: Vec2::ZERO,
                    },
                })),
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0 + strand as f32 * 0.01)),
            ));
        }
    }

    // Junction circles at branching nodes (3+ edges)
    let edge_counts = count_node_edges(&graph);
    for (&pos, &count) in &edge_counts {
        if count >= 3 {
            let spec = graph.nodes.get(&pos).and_then(|n| n.specialization);
            let core = region_color_linear(spec);
            let body = body_color_from_core(core);
            let biomass = graph.nodes.get(&pos).map_or(1.0, |n| n.biomass);
            let world_pos = pos.as_vec2() * TILE_SIZE;

            commands.spawn((
                JunctionMesh,
                Mesh2d(meshes.add(Circle::new(4.0))),
                MeshMaterial2d(net_materials.add(NetworkMaterial {
                    uniforms: NetworkUniforms {
                        core_color: core,
                        body_color: body,
                        biomass,
                        time: elapsed,
                        _padding: Vec2::ZERO,
                    },
                })),
                Transform::from_translation(world_pos.extend(1.5)),
            ));
        }
    }

    // Rival network — deep crimson
    let rival_core = LinearRgba::new(0.7, 0.1, 0.1, 1.0);
    let rival_body = LinearRgba::new(0.3, 0.05, 0.05, 0.7);

    for edge in &rival_graph.edges {
        let from = edge.from.as_vec2() * TILE_SIZE;
        let to = edge.to.as_vec2() * TILE_SIZE;
        let total_width = (edge.thickness * 2.0).clamp(2.0, 8.0);
        let strand_width = total_width / STRANDS_PER_EDGE as f32;
        let base_seed =
            (edge.from.x.wrapping_mul(73_856_093) ^ edge.to.y.wrapping_mul(19_349_663)) as u32;
        let edge_dir = (to - from).normalize_or_zero();
        let perp = Vec2::new(-edge_dir.y, edge_dir.x);

        for strand in 0..STRANDS_PER_EDGE {
            let strand_seed = base_seed.wrapping_add((strand as u32).wrapping_mul(2_654_435_761));
            let spread = (strand as f32 - (STRANDS_PER_EDGE - 1) as f32 * 0.5) * strand_width * 0.8;
            let (mesh, _) = build_spline_mesh_with_wobble(
                from + perp * spread,
                to + perp * spread,
                strand_width * 0.5,
                strand_seed,
            );

            commands.spawn((
                NetworkMesh,
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(net_materials.add(NetworkMaterial {
                    uniforms: NetworkUniforms {
                        core_color: rival_core,
                        body_color: rival_body,
                        biomass: edge.thickness * if strand == 0 { 1.0 } else { 0.7 },
                        time: elapsed,
                        _padding: Vec2::ZERO,
                    },
                })),
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0 + strand as f32 * 0.01)),
            ));
        }
    }

    // Tip glow circles
    for (pos, spec) in &tip_positions.tips {
        let core = region_color_linear(*spec);
        let world_pos = pos.as_vec2() * TILE_SIZE;
        let pulse = (elapsed * 3.0).sin() * 0.5 + 0.5;

        commands.spawn((
            NetworkMesh,
            Mesh2d(meshes.add(Circle::new(8.0 + pulse * 4.0))),
            MeshMaterial2d(net_materials.add(NetworkMaterial {
                uniforms: NetworkUniforms {
                    core_color: core,
                    body_color: body_color_from_core(core),
                    biomass: 3.0 + pulse * 2.0,
                    time: elapsed,
                    _padding: Vec2::ZERO,
                },
            })),
            Transform::from_translation(world_pos.extend(1.8)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::SpecializationType;

    #[test]
    fn catmull_rom_passes_through_control_points() {
        let p0 = Vec2::new(0.0, 0.0);
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(2.0, 1.0);
        let p3 = Vec2::new(3.0, 1.0);

        let at_start = catmull_rom(p0, p1, p2, p3, 0.0);
        let at_end = catmull_rom(p0, p1, p2, p3, 1.0);

        assert!((at_start - p1).length() < 0.001);
        assert!((at_end - p2).length() < 0.001);
    }

    #[test]
    fn catmull_rom_midpoint_is_between_control_points() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(2.0, 0.0);
        let p3 = Vec2::new(3.0, 0.0);

        let mid = catmull_rom(p0, p1, p2, p3, 0.5);

        assert!((mid - Vec2::new(1.5, 0.0)).length() < 0.001);
    }

    #[test]
    fn spline_mesh_has_correct_vertex_count() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _points) = build_spline_mesh(from, to, 2.0);

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("mesh should have positions");

        let count = match positions {
            bevy::mesh::VertexAttributeValues::Float32x3(v) => v.len(),
            _ => panic!("unexpected attribute format"),
        };

        assert_eq!(count, SPLINE_SAMPLES * 2);
    }

    #[test]
    fn spline_mesh_vertices_surround_centerline() {
        let from = Vec2::new(10.0, 20.0);
        let to = Vec2::new(110.0, 20.0);
        let half_width = 3.0;
        let (mesh, _points) = build_spline_mesh(from, to, half_width);

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("mesh should have positions");

        let verts = match positions {
            bevy::mesh::VertexAttributeValues::Float32x3(v) => v,
            _ => panic!("unexpected attribute format"),
        };

        let left = Vec2::new(verts[0][0], verts[0][1]);
        let right = Vec2::new(verts[1][0], verts[1][1]);
        let midpoint = (left + right) * 0.5;

        assert!(
            (midpoint - from).length() < 0.01,
            "midpoint {midpoint} should be near from {from}"
        );

        let separation = (left - right).length();
        assert!(
            (separation - half_width * 2.0).abs() < 0.01,
            "separation {separation} should be near {half_width_2}",
            half_width_2 = half_width * 2.0
        );
    }

    #[test]
    fn region_color_maps_specializations() {
        let explorer = region_color(Some(SpecializationType::Explorer));
        assert_eq!(explorer, Color::srgb(1.0, 0.9, 0.3));

        let parasite = region_color(Some(SpecializationType::Parasite));
        assert_eq!(parasite, Color::srgb(0.8, 0.2, 0.2));

        let none_color = region_color(None);
        assert_eq!(none_color, Color::srgb(0.9, 0.85, 0.7));

        let hunter = region_color(Some(SpecializationType::Hunter));
        assert_eq!(hunter, Color::srgb(0.6, 0.4, 0.1));
    }

    // --- Step 1: UV attribute tests ---

    #[test]
    fn spline_mesh_has_uv_attribute() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh(from, to, 4.0);

        let uvs = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .expect("mesh should have UV_0 attribute");

        let uv_count = match uvs {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.len(),
            _ => panic!("unexpected UV format"),
        };
        assert_eq!(uv_count, SPLINE_SAMPLES * 2);
    }

    #[test]
    fn spline_mesh_uv_range_is_symmetric() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh(from, to, 4.0);

        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap() {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.clone(),
            _ => panic!("unexpected UV format"),
        };

        // First left vertex: u = -1, v = 0
        assert!((uvs[0][0] - (-1.0)).abs() < 0.001, "left u should be -1");
        assert!((uvs[0][1]).abs() < 0.001, "first v should be 0");
        // First right vertex: u = 1, v = 0
        assert!((uvs[1][0] - 1.0).abs() < 0.001, "right u should be 1");
        // Last right vertex: v should be 1
        assert!(
            (uvs[SPLINE_SAMPLES * 2 - 1][1] - 1.0).abs() < 0.001,
            "last v should be 1"
        );
    }

    // --- Step 2: Wobble tests ---

    #[test]
    fn spline_mesh_with_wobble_differs_from_straight() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (_, straight_points) = build_spline_mesh(from, to, 4.0);
        let (_, wobble_points) = build_spline_mesh_with_wobble(from, to, 4.0, 42);

        let mut any_different = false;
        for i in 1..(SPLINE_SAMPLES - 1) {
            if (straight_points[i] - wobble_points[i]).length() > 0.01 {
                any_different = true;
                break;
            }
        }
        assert!(
            any_different,
            "wobble should displace at least one interior point"
        );
    }

    #[test]
    fn spline_mesh_wobble_preserves_endpoints() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (_, straight_points) = build_spline_mesh(from, to, 4.0);
        let (_, wobble_points) = build_spline_mesh_with_wobble(from, to, 4.0, 99);

        assert!(
            (straight_points[0] - wobble_points[0]).length() < 0.001,
            "start endpoint must not be displaced"
        );
        assert!(
            (straight_points[SPLINE_SAMPLES - 1] - wobble_points[SPLINE_SAMPLES - 1]).length()
                < 0.001,
            "end endpoint must not be displaced"
        );
    }

    #[test]
    fn spline_mesh_wobble_has_uv_attribute() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh_with_wobble(from, to, 4.0, 7);

        let uvs = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .expect("wobble mesh should have UV_0 attribute");

        let uv_count = match uvs {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.len(),
            _ => panic!("unexpected UV format"),
        };
        assert_eq!(uv_count, SPLINE_SAMPLES * 2);
    }

    // --- Step 3: NetworkMaterial tests ---

    #[test]
    fn network_material_stores_core_color_and_biomass() {
        let mat = NetworkMaterial {
            uniforms: NetworkUniforms {
                core_color: LinearRgba::new(1.0, 0.9, 0.3, 1.0),
                body_color: LinearRgba::new(0.5, 0.45, 0.15, 0.6),
                biomass: 5.0,
                time: 0.0,
                _padding: Vec2::ZERO,
            },
        };
        assert_eq!(mat.uniforms.biomass, 5.0);
    }

    #[test]
    fn body_color_from_core_is_muted() {
        let core = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
        let body = body_color_from_core(core);
        assert!(body.red < core.red, "body red should be less than core red");
        assert_eq!(body.alpha, 0.7);
    }
}
