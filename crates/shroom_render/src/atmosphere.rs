use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

pub const PARTICLE_POOL_SIZE: usize = 40;

/// Packed uniform struct — matches the WGSL `VignetteUniforms` struct exactly.
///
/// Layout: color (16 bytes) + intensity (4 bytes) + _padding (12 bytes) = 32 bytes.
#[derive(ShaderType, Debug, Clone)]
pub struct VignetteUniforms {
    pub color: LinearRgba,
    pub intensity: f32,
    // Pad to 16-byte boundary so the struct size is a multiple of the largest alignment.
    pub _padding: Vec3,
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct VignetteMaterial {
    #[uniform(0)]
    pub uniforms: VignetteUniforms,
}

impl Material2d for VignetteMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/vignette.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker component for the fullscreen vignette quad.
#[derive(Component)]
pub struct VignetteOverlay;

/// Startup system: spawn a large quad with `VignetteMaterial` at a high Z so it
/// composites over everything else.
pub fn spawn_vignette(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VignetteMaterial>>,
) {
    commands.spawn((
        VignetteOverlay,
        Mesh2d(meshes.add(Rectangle::new(4000.0, 4000.0))),
        MeshMaterial2d(materials.add(VignetteMaterial {
            uniforms: VignetteUniforms {
                color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
                intensity: 0.6,
                _padding: Vec3::ZERO,
            },
        })),
        Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
    ));
}

/// Marker component for individually drifting ambient dust particles.
#[derive(Component)]
pub struct AmbientParticle {
    pub velocity: Vec2,
}

/// Startup system: fill a fixed pool of ambient particles at random positions/velocities
/// using a deterministic LCG so there is no external RNG dependency.
pub fn spawn_particle_pool(mut commands: Commands) {
    let mut rng_seed: u32 = 42;
    for _ in 0..PARTICLE_POOL_SIZE {
        rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let x = (rng_seed as f32 / u32::MAX as f32) * 2000.0 - 1000.0;
        rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let y = (rng_seed as f32 / u32::MAX as f32) * 2000.0 - 1000.0;
        rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let vx = (rng_seed as f32 / u32::MAX as f32) * 4.0 - 2.0;
        rng_seed = rng_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let vy = (rng_seed as f32 / u32::MAX as f32) * 4.0 - 2.0;

        commands.spawn((
            AmbientParticle {
                velocity: Vec2::new(vx, vy),
            },
            Sprite {
                color: Color::srgba(0.8, 0.75, 0.6, 0.15),
                custom_size: Some(Vec2::splat(2.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(x, y, 50.0)),
        ));
    }
}

/// PostUpdate system: move each particle by its velocity and wrap it back onto the
/// visible viewport edge when it drifts out of bounds.
pub fn update_particles(
    time: Res<Time>,
    camera_q: Query<(&Transform, &Projection), With<Camera2d>>,
    mut particles: Query<(&mut Transform, &mut AmbientParticle), Without<Camera2d>>,
) {
    let dt = time.delta_secs();

    let (cam_pos, viewport_half) = if let Ok((cam_tf, proj)) = camera_q.single() {
        let half = match proj {
            Projection::Orthographic(ortho) => {
                Vec2::new(ortho.area.width() * 0.5, ortho.area.height() * 0.5)
            }
            // Fallback for perspective cameras (not typical for 2-D games).
            _ => Vec2::new(1000.0, 800.0),
        };
        (cam_tf.translation.truncate(), half)
    } else {
        (Vec2::ZERO, Vec2::new(1000.0, 800.0))
    };

    for (mut transform, mut particle) in particles.iter_mut() {
        transform.translation.x += particle.velocity.x * dt * 10.0;
        transform.translation.y += particle.velocity.y * dt * 10.0;

        let rel = transform.translation.truncate() - cam_pos;
        if rel.x.abs() > viewport_half.x * 1.2 || rel.y.abs() > viewport_half.y * 1.2 {
            // Re-seed position on a viewport edge using a hash of the current state.
            let seed = (transform.translation.x.to_bits())
                .wrapping_mul(1_664_525)
                .wrapping_add(time.elapsed_secs().to_bits());
            let t = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            if seed % 2 == 0 {
                transform.translation.x = cam_pos.x + viewport_half.x * t;
                transform.translation.y = cam_pos.y
                    + if seed % 4 < 2 {
                        viewport_half.y
                    } else {
                        -viewport_half.y
                    };
            } else {
                transform.translation.y = cam_pos.y + viewport_half.y * t;
                transform.translation.x = cam_pos.x
                    + if seed % 4 < 2 {
                        viewport_half.x
                    } else {
                        -viewport_half.x
                    };
            }
            let new_seed = seed.wrapping_mul(2_654_435_761);
            particle.velocity = Vec2::new(
                (new_seed as f32 / u32::MAX as f32) * 4.0 - 2.0,
                ((new_seed >> 16) as f32 / u32::MAX as f32) * 4.0 - 2.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_material_has_intensity() {
        let mat = VignetteMaterial {
            uniforms: VignetteUniforms {
                color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
                intensity: 0.6,
                _padding: Vec3::ZERO,
            },
        };
        assert_eq!(mat.uniforms.intensity, 0.6);
    }

    #[test]
    fn particle_pool_size_is_40() {
        assert_eq!(PARTICLE_POOL_SIZE, 40);
    }
}
