use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
}

// -- Grid coordinate system --

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

// -- Terrain --

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Terrain {
    Soil,
    Rock,
    Water,
    Ruins,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Tile {
    pub terrain: Terrain,
    pub nutrient: f32,
    pub moisture: f32,
    pub discovered: bool,
}

// -- Ownership --

#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PlayerOwned;

#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct RivalOwned;

// -- Resources --

#[derive(Resource, Default, Debug, Reflect)]
pub struct GameState {
    pub turn: u32,
    pub paused: bool,
}
