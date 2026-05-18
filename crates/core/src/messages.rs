use bevy::ecs::message::Message;

use crate::grid::Hex;
use crate::region::RegionId;
use crate::tile::{FragmentId, TileContents};

#[derive(Message)]
pub struct TurnAdvanced;

#[derive(Message)]
pub struct TileDiscovered {
    pub pos: Hex,
    pub contents: Option<TileContents>,
}

#[derive(Message)]
pub struct DecompositionComplete {
    pub pos: Hex,
    pub was_unique: bool,
}

#[derive(Message)]
pub struct FragmentFused {
    pub fragment_id: FragmentId,
}

#[derive(Message)]
pub struct HiveCaptured {
    pub hive_pos: Hex,
    pub region_id: RegionId,
}

#[derive(Message)]
pub struct NetworkFounded {
    pub region_id: RegionId,
    pub seed: Hex,
}
