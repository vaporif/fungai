use std::collections::HashMap;

use bevy::prelude::*;

pub use hexx::{Hex, HexLayout, HexOrientation, OffsetHexMode};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos(pub Hex);

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct GridWorld {
    #[reflect(ignore)]
    pub tiles: HashMap<Hex, Entity>,
    pub width: i32,
    pub height: i32,
}

impl GridWorld {
    pub fn neighbors(&self, pos: Hex) -> impl Iterator<Item = (Hex, Entity)> + '_ {
        pos.all_neighbors()
            .into_iter()
            .filter_map(move |n| self.tiles.get(&n).map(|&e| (n, e)))
    }
}

#[must_use]
pub fn create_hex_layout() -> HexLayout {
    HexLayout {
        orientation: HexOrientation::Pointy,
        origin: hexx::Vec2::ZERO,
        scale: hexx::Vec2::splat(28.0),
    }
}
