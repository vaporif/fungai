use bevy::prelude::*;
use shroom_core::{
    FaunaAgent, FragmentAgent, FruitingBody, GridPos, MushroomEntity, NeutralFungusAgent,
    OrganismSpriteLink, PlantRootAgent, SpecializationType,
};

use crate::assets::EntitySprites;
use crate::data_layer::TipPositions;
use crate::terrain_render::TILE_SIZE;

#[derive(Component)]
pub struct TipSprite;

#[derive(Component)]
pub struct OrganismSprite;

#[must_use]
pub fn organism_sprite_size() -> Vec2 {
    Vec2::splat(TILE_SIZE * 0.7)
}

pub fn tip_render_system(
    mut commands: Commands,
    tip_positions: Res<TipPositions>,
    existing: Query<Entity, With<TipSprite>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    for (pos, spec) in &tip_positions.tips {
        let color = match spec {
            Some(SpecializationType::Explorer) => Color::srgb(1.0, 0.9, 0.3),
            Some(SpecializationType::Parasite) => Color::srgb(0.8, 0.2, 0.2),
            Some(SpecializationType::Researcher) => Color::srgb(0.3, 0.5, 0.9),
            Some(SpecializationType::Hunter) => Color::srgb(0.6, 0.4, 0.1),
            _ => Color::srgb(0.9, 0.9, 0.9),
        };

        let world_pos = Vec3::new(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, 2.0);

        commands.spawn((
            TipSprite,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.4)),
                ..default()
            },
            Transform::from_translation(world_pos),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn organism_render_system(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    linked_sprites: Query<(Entity, &OrganismSpriteLink), With<OrganismSprite>>,
    fragments: Query<(Entity, &GridPos, &FragmentAgent), Without<OrganismSprite>>,
    plants: Query<(Entity, &GridPos, &PlantRootAgent), Without<OrganismSprite>>,
    fauna: Query<(Entity, &GridPos, &FaunaAgent), Without<OrganismSprite>>,
    fruiting_bodies: Query<(Entity, &FruitingBody), Without<OrganismSprite>>,
    mushrooms: Query<(Entity, &MushroomEntity), Without<OrganismSprite>>,
    neutral_fungi: Query<(Entity, &GridPos, &NeutralFungusAgent), Without<OrganismSprite>>,
) {
    // Despawn sprites whose source entity no longer exists
    for (sprite_entity, link) in linked_sprites.iter() {
        if commands.get_entity(link.0).is_err() {
            commands.entity(sprite_entity).despawn();
        }
    }

    let size = organism_sprite_size();

    for (source, gpos, _fragment) in fragments.iter() {
        let world_pos = gpos.0.as_vec2() * TILE_SIZE;
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

    for (source, gpos, _plant) in plants.iter() {
        let world_pos = gpos.0.as_vec2() * TILE_SIZE;
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

    for (source, gpos, _fauna_agent) in fauna.iter() {
        let world_pos = gpos.0.as_vec2() * TILE_SIZE;
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

    for (source, body) in fruiting_bodies.iter() {
        let world_pos = body.column_top.as_vec2() * TILE_SIZE;
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

    for (source, mushroom) in mushrooms.iter() {
        let world_pos = mushroom.pos.as_vec2() * TILE_SIZE;
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

    for (source, gpos, _fungus) in neutral_fungi.iter() {
        let world_pos = gpos.0.as_vec2() * TILE_SIZE;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organism_sprite_size_is_proportional_to_tile() {
        let size = organism_sprite_size();
        assert!(size.x >= 48.0 * 0.6);
        assert!(size.x <= 48.0 * 0.75);
    }
}
