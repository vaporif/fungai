use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    GridWorld, Hex, HexLayout, Occupant, RegionStates, SelectedRegion, TerrainType, Tile,
    TileContents,
};
use kingdom_input::GameCamera;

const POPOVER_OFFSET_PX: Vec2 = Vec2::new(16.0, -16.0);
const POPOVER_MARGIN_PX: f32 = 8.0;

#[derive(Component)]
pub struct TilePopoverRoot;

#[derive(Component)]
pub struct TilePopoverText;

#[derive(SystemParam)]
pub struct TilePopoverInputs<'w, 's> {
    selected: Res<'w, SelectedRegion>,
    layout: Res<'w, HexLayout>,
    grid: Res<'w, GridWorld>,
    region_states: Res<'w, RegionStates>,
    tiles: Query<'w, 's, &'static Tile>,
    camera_q: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

/// Spawn the popover when a tile is selected, despawn it otherwise, and
/// reposition every frame so the box tracks the hex through camera pans.
pub fn update_tile_popover(
    mut commands: Commands,
    inputs: TilePopoverInputs,
    mut existing: Query<(Entity, &mut Node, &ComputedNode), With<TilePopoverRoot>>,
    mut text: Query<&mut Text, With<TilePopoverText>>,
) {
    let popover_size = existing
        .iter()
        .next()
        .map(|(_, _, c)| c.size)
        .unwrap_or(Vec2::ZERO);

    let Some(payload) = resolve_popover(&inputs, popover_size) else {
        for (entity, _, _) in existing.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    if let Ok((_, mut node, _)) = existing.single_mut() {
        node.left = Val::Px(payload.pos.x);
        node.top = Val::Px(payload.pos.y);
        if let Ok(mut t) = text.single_mut() {
            **t = payload.text;
        }
    } else {
        spawn_popover(&mut commands, payload);
    }
}

struct PopoverPayload {
    pos: Vec2,
    text: String,
}

fn resolve_popover(inputs: &TilePopoverInputs, popover_size: Vec2) -> Option<PopoverPayload> {
    let hex = inputs.selected.selected_pos?;
    let &entity = inputs.grid.tiles.get(&hex)?;
    let tile = inputs.tiles.get(entity).ok()?;
    let (camera, cam_transform) = inputs.camera_q.single().ok()?;
    let window = inputs.windows.single().ok()?;

    let world = inputs.layout.hex_to_world_pos(hex);
    let screen = camera
        .world_to_viewport(cam_transform, Vec3::new(world.x, world.y, 0.0))
        .ok()?;

    let win = Vec2::new(window.width(), window.height());
    let mut pos = screen + POPOVER_OFFSET_PX;
    pos.x = pos.x.clamp(
        POPOVER_MARGIN_PX,
        win.x - popover_size.x - POPOVER_MARGIN_PX,
    );
    pos.y = pos.y.clamp(
        POPOVER_MARGIN_PX,
        win.y - popover_size.y - POPOVER_MARGIN_PX,
    );

    Some(PopoverPayload {
        pos,
        text: format_tile(hex, tile, &inputs.region_states),
    })
}

fn spawn_popover(commands: &mut Commands, payload: PopoverPayload) {
    commands
        .spawn((
            TilePopoverRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(payload.pos.x),
                top: Val::Px(payload.pos.y),
                padding: UiRect::all(Val::Px(8.0)),
                max_width: Val::Px(260.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
        ))
        .with_children(|parent| {
            parent.spawn((
                TilePopoverText,
                Text::new(payload.text),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.92, 0.92)),
            ));
        });
}

fn format_tile(hex: Hex, tile: &Tile, region_states: &RegionStates) -> String {
    let terrain = match tile.terrain {
        TerrainType::Soil => "Soil",
        TerrainType::Rock => "Rock",
        TerrainType::Water => "Water",
        TerrainType::Root => "Root",
        TerrainType::Ruin => "Ruin",
        TerrainType::Toxic => "Toxic",
        TerrainType::Surface => "Surface",
    };
    let occupant = match tile.occupant {
        Occupant::Empty => "Empty".into(),
        Occupant::Player(rid) => format!("Player (region {})", rid.0),
        Occupant::Rival(rid) => format!("Rival {}", rid.0),
    };
    let contents = tile.contents.map(|c| match c {
        TileContents::OrganicMatter => "Organic matter".into(),
        TileContents::Mineral => "Mineral".into(),
        TileContents::Artifact => "Artifact".into(),
        TileContents::Fragment(id) => format!("Fragment #{}", id.0),
        TileContents::UniqueDecomposable(id) => format!("Unique decomposable #{id}"),
        TileContents::NeutralFungus(id) => format!("Neutral fungus #{id}"),
        TileContents::PlantRoot(id) => format!("Plant root #{id}"),
    });

    let mut out = format!(
        "({}, {})\nTerrain: {terrain}\nOccupant: {occupant}",
        hex.x, hex.y,
    );
    if let Some(c) = contents {
        out.push_str(&format!("\nContents: {c}"));
    }
    if !tile.discovered {
        out.push_str("\n(undiscovered)");
    }
    out.push_str(&format!(
        "\nN:{:.0} M:{:.0} B:{:.0}",
        tile.nutrient_level * 100.0,
        tile.moisture * 100.0,
        tile.biomass,
    ));

    if let Some(state) = tile
        .occupant
        .region_id()
        .and_then(|rid| region_states.get(rid))
    {
        out.push_str(&format!(
            "\nRegion {} | tiles {}",
            state.region_id.0, state.tile_count,
        ));
    }

    out
}
