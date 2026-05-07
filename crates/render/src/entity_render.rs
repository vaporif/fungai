use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use kingdom_core::{
    BIAS_MAGNITUDE_CAP, FaunaAgent, FragmentAgent, FruitingBody, GridPos, Hex, HexLayout,
    MushroomEntity, NeutralFungusAgent, OrganismSpriteLink, PlantRootAgent, Tile,
};

use crate::assets::EntitySprites;
use crate::data_layer::SelectedRegionTiles;

#[derive(Component)]
pub struct OrganismSprite;

/// Sprite size based on hex inner radius (apothem) at ~70% fill.
#[must_use]
pub fn organism_sprite_size(layout: &HexLayout) -> Vec2 {
    let inner_radius = layout.scale.x * 3.0_f32.sqrt() / 2.0;
    Vec2::splat(inner_radius * 1.4)
}

// One `Added<T>` query per organism so each component picks its own sprite/colour.
#[derive(SystemParam)]
pub struct NewOrganisms<'w, 's> {
    fragments: Query<'w, 's, (Entity, &'static GridPos), Added<FragmentAgent>>,
    plants: Query<'w, 's, (Entity, &'static GridPos), Added<PlantRootAgent>>,
    fauna: Query<'w, 's, (Entity, &'static GridPos), Added<FaunaAgent>>,
    fruiting: Query<'w, 's, (Entity, &'static FruitingBody), Added<FruitingBody>>,
    mushrooms: Query<'w, 's, (Entity, &'static MushroomEntity), Added<MushroomEntity>>,
    neutral_fungi: Query<'w, 's, (Entity, &'static GridPos), Added<NeutralFungusAgent>>,
}

pub fn spawn_organism_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_organisms: NewOrganisms,
) {
    let size = organism_sprite_size(&layout);

    for (source, gpos) in new_organisms.fragments.iter() {
        let world_pos = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.fragment.clone(),
                color: Color::srgb(0.9, 0.7, 1.0),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }

    for (source, gpos) in new_organisms.plants.iter() {
        let world_pos = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.plant_root.clone(),
                color: Color::srgb(0.2, 0.7, 0.3),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }

    for (source, gpos) in new_organisms.fauna.iter() {
        let world_pos = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.fauna.clone(),
                color: Color::srgb(0.7, 0.3, 0.2),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }

    for (source, body) in new_organisms.fruiting.iter() {
        let world_pos = layout.hex_to_world_pos(body.column_top);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.mushroom.clone(),
                color: Color::WHITE,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }

    for (source, mushroom) in new_organisms.mushrooms.iter() {
        let world_pos = layout.hex_to_world_pos(mushroom.pos);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.mushroom.clone(),
                color: Color::WHITE,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }

    for (source, gpos) in new_organisms.neutral_fungi.iter() {
        let world_pos = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.neutral_fungus.clone(),
                color: Color::srgb(0.5, 0.6, 0.4),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }
}

// One `RemovedComponents<T>` per organism — they can't be merged into one param.
#[derive(SystemParam)]
pub struct RemovedOrganisms<'w, 's> {
    fragments: RemovedComponents<'w, 's, FragmentAgent>,
    plants: RemovedComponents<'w, 's, PlantRootAgent>,
    fauna: RemovedComponents<'w, 's, FaunaAgent>,
    fruiting: RemovedComponents<'w, 's, FruitingBody>,
    mushrooms: RemovedComponents<'w, 's, MushroomEntity>,
    neutral_fungi: RemovedComponents<'w, 's, NeutralFungusAgent>,
}

pub fn despawn_orphaned_organism_sprites(
    mut commands: Commands,
    mut removed_organisms: RemovedOrganisms,
    linked_sprites: Query<(Entity, &OrganismSpriteLink), With<OrganismSprite>>,
) {
    let mut removed: HashSet<Entity> = HashSet::new();
    removed.extend(removed_organisms.fragments.read());
    removed.extend(removed_organisms.plants.read());
    removed.extend(removed_organisms.fauna.read());
    removed.extend(removed_organisms.fruiting.read());
    removed.extend(removed_organisms.mushrooms.read());
    removed.extend(removed_organisms.neutral_fungi.read());

    if removed.is_empty() {
        return;
    }
    for (sprite, link) in linked_sprites.iter() {
        if removed.contains(&link.0) {
            commands.entity(sprite).despawn();
        }
    }
}

#[derive(Component)]
pub struct BiasGlowMarker;

// Despawn-and-respawn each frame is the simplest correct implementation. On
// an 80x60 grid only owned + recently-painted tiles fire the glow path
// (typically <100 entities), so the archetype churn is fine. Switch to a
// diff-based update if profiling ever shows hitching here.
pub fn bias_glow_render_system(
    mut commands: Commands,
    layout: Res<HexLayout>,
    tiles: Query<(&GridPos, &Tile)>,
    existing: Query<Entity, With<BiasGlowMarker>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let quad_size = Vec2::splat(layout.scale.x * 1.6);

    for (gpos, tile) in tiles.iter() {
        let mag = tile.priority_bias.length();
        if mag < 0.05 {
            continue;
        }
        let alpha = (mag / BIAS_MAGNITUDE_CAP).min(1.0);
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            BiasGlowMarker,
            Sprite {
                color: Color::srgba(1.0, 0.7, 0.3, alpha),
                custom_size: Some(quad_size),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, 5.0),
        ));
    }
}

#[derive(Component)]
pub struct RegionHighlightSprite;

/// Build a triangle-list mesh of thin quads for the boundary edges of a hex region.
fn build_outline_mesh(tiles: &[Hex], layout: &HexLayout, half_width: f32) -> Option<Mesh> {
    let tile_set: HashSet<Hex> = tiles.iter().copied().collect();
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for &hex in tiles {
        let corners = layout.hex_corners(hex);
        let neighbors = hex.all_neighbors();
        for (i, neighbor) in neighbors.iter().enumerate() {
            if tile_set.contains(neighbor) {
                continue;
            }
            let a = corners[i];
            let b = corners[(i + 1) % 6];

            let dir = (b - a).normalize();
            let normal = Vec2::new(-dir.y, dir.x);
            let offset = normal * half_width;

            let base = positions.len() as u32;
            positions.push([(a - offset).x, (a - offset).y, 0.0]);
            positions.push([(a + offset).x, (a + offset).y, 0.0]);
            positions.push([(b + offset).x, (b + offset).y, 0.0]);
            positions.push([(b - offset).x, (b - offset).y, 0.0]);

            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    if positions.is_empty() {
        return None;
    }

    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; positions.len()];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; positions.len()];

    Some(
        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices)),
    )
}

pub fn region_highlight_render_system(
    mut commands: Commands,
    selected_tiles: Res<SelectedRegionTiles>,
    existing: Query<Entity, With<RegionHighlightSprite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    layout: Res<HexLayout>,
) {
    if !selected_tiles.is_changed() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    if selected_tiles.tiles.is_empty() {
        return;
    }

    if let Some(mesh) = build_outline_mesh(&selected_tiles.tiles, &layout, 1.5) {
        commands.spawn((
            RegionHighlightSprite,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(
                materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.9, 0.5, 0.8))),
            ),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::create_hex_layout;

    #[test]
    fn organism_sprite_size_is_proportional_to_hex() {
        let layout = create_hex_layout();
        let size = organism_sprite_size(&layout);
        // inner_radius = 28.0 * sqrt(3)/2 ~= 24.25, * 1.4 ~= 33.9
        assert!(size.x >= 30.0, "sprite too small: {}", size.x);
        assert!(size.x <= 40.0, "sprite too large: {}", size.x);
    }
}
