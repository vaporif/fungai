use std::collections::HashSet;

use bevy::prelude::*;
use fungai_core::{
    FaunaAgent, FragmentAgent, FruitingBody, GridPos, Hex, HexLayout, MushroomEntity,
    NeutralFungusAgent, OrganismSpriteLink, PlantRootAgent, SpecializationType,
};

use crate::assets::EntitySprites;
use crate::data_layer::{PriorityBiasMap, SelectedRegionTiles, TipPositions};

#[derive(Component)]
pub struct TipSprite;

#[derive(Component)]
pub struct OrganismSprite;

/// Sprite size based on hex inner radius (apothem) at ~70% fill.
#[must_use]
pub fn organism_sprite_size(layout: &HexLayout) -> Vec2 {
    let inner_radius = layout.scale.x * 3.0_f32.sqrt() / 2.0;
    Vec2::splat(inner_radius * 1.4)
}

pub fn tip_render_system(
    mut commands: Commands,
    tip_positions: Res<TipPositions>,
    existing: Query<Entity, With<TipSprite>>,
    layout: Res<HexLayout>,
) {
    if !tip_positions.is_changed() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let inner_radius = layout.scale.x * 3.0_f32.sqrt() / 2.0;
    let tip_size = Vec2::splat(inner_radius * 0.8);

    for (pos, spec) in &tip_positions.tips {
        let color = match spec {
            Some(SpecializationType::Decomposer) => Color::srgb(0.4, 0.7, 0.3),
            Some(SpecializationType::Explorer) => Color::srgb(1.0, 0.9, 0.3),
            Some(SpecializationType::Parasite) => Color::srgb(0.8, 0.2, 0.2),
            Some(SpecializationType::Researcher) => Color::srgb(0.3, 0.5, 0.9),
            Some(SpecializationType::Hunter) => Color::srgb(0.6, 0.4, 0.1),
            _ => Color::srgb(0.9, 0.9, 0.9),
        };

        let base_pos = layout.hex_to_world_pos(*pos);
        let world_pos = Vec3::new(base_pos.x, base_pos.y, 2.0);

        commands.spawn((
            TipSprite,
            Sprite {
                color,
                custom_size: Some(tip_size),
                ..default()
            },
            Transform::from_translation(world_pos),
        ));
    }
}

// One query per organism component is the cleanest expression of the
// `Added<T>` reaction; collapsing them via `Or<...>` filters loses the
// per-type sprite/colour selection that the body relies on.
#[allow(clippy::too_many_arguments)]
pub fn spawn_organism_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_fragments: Query<(Entity, &GridPos), Added<FragmentAgent>>,
    new_plants: Query<(Entity, &GridPos), Added<PlantRootAgent>>,
    new_fauna: Query<(Entity, &GridPos), Added<FaunaAgent>>,
    new_fruiting: Query<(Entity, &FruitingBody), Added<FruitingBody>>,
    new_mushrooms: Query<(Entity, &MushroomEntity), Added<MushroomEntity>>,
    new_neutral_fungi: Query<(Entity, &GridPos), Added<NeutralFungusAgent>>,
) {
    let size = organism_sprite_size(&layout);

    for (source, gpos) in new_fragments.iter() {
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

    for (source, gpos) in new_plants.iter() {
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

    for (source, gpos) in new_fauna.iter() {
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

    for (source, body) in new_fruiting.iter() {
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

    for (source, mushroom) in new_mushrooms.iter() {
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

    for (source, gpos) in new_neutral_fungi.iter() {
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

// Each `RemovedComponents<T>` is a distinct system param; merging them is not possible.
#[allow(clippy::too_many_arguments)]
pub fn despawn_orphaned_organism_sprites(
    mut commands: Commands,
    mut removed_fragments: RemovedComponents<FragmentAgent>,
    mut removed_plants: RemovedComponents<PlantRootAgent>,
    mut removed_fauna: RemovedComponents<FaunaAgent>,
    mut removed_fruiting: RemovedComponents<FruitingBody>,
    mut removed_mushrooms: RemovedComponents<MushroomEntity>,
    mut removed_neutral_fungi: RemovedComponents<NeutralFungusAgent>,
    linked_sprites: Query<(Entity, &OrganismSpriteLink), With<OrganismSprite>>,
) {
    let mut removed: HashSet<Entity> = HashSet::new();
    removed.extend(removed_fragments.read());
    removed.extend(removed_plants.read());
    removed.extend(removed_fauna.read());
    removed.extend(removed_fruiting.read());
    removed.extend(removed_mushrooms.read());
    removed.extend(removed_neutral_fungi.read());

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
pub struct PriorityArrowSprite;

pub fn priority_arrow_render_system(
    mut commands: Commands,
    bias_map: Res<PriorityBiasMap>,
    existing: Query<Entity, With<PriorityArrowSprite>>,
    layout: Res<HexLayout>,
) {
    if !bias_map.is_changed() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let inner_radius = layout.scale.x * 3.0_f32.sqrt() / 2.0;
    let arrow_size = Vec2::new(inner_radius * 0.5, inner_radius * 0.15);

    for (hex, bias) in &bias_map.biases {
        let angle = bias.y.atan2(bias.x);
        let base_pos = layout.hex_to_world_pos(*hex);
        let offset = *bias * inner_radius * 0.3;
        let world_pos = Vec3::new(base_pos.x + offset.x, base_pos.y + offset.y, 3.0);

        commands.spawn((
            PriorityArrowSprite,
            Sprite {
                color: Color::srgba(0.2, 1.0, 0.6, 0.6),
                custom_size: Some(arrow_size),
                ..default()
            },
            Transform::from_translation(world_pos).with_rotation(Quat::from_rotation_z(angle)),
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
    use fungai_core::create_hex_layout;

    #[test]
    fn organism_sprite_size_is_proportional_to_hex() {
        let layout = create_hex_layout();
        let size = organism_sprite_size(&layout);
        // inner_radius = 28.0 * sqrt(3)/2 ~= 24.25, * 1.4 ~= 33.9
        assert!(size.x >= 30.0, "sprite too small: {}", size.x);
        assert!(size.x <= 40.0, "sprite too large: {}", size.x);
    }
}
