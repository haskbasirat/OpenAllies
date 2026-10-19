//! Custom battle setup: the players the original setup screen collects, and
//! building the starting world from the map (each start location gets its
//! nation's Corps HQ in place of `placeholder_corpsunpacked`, starting
//! resources from the rules). Owner: core.
use crate::data::GameData;
use crate::map::GameMap;
use crate::world::{Controller, World};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSetup {
    pub name: String,
    /// Original nation id: "usa", "gbr", "rus", "ger", "jpn".
    pub nation: String,
    pub team: u32,
    pub color: [u8; 3],
    pub controller: Controller,
    /// Index into `GameMap::start_locations`.
    pub start: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkirmishSetup {
    pub map_id: String,
    pub players: Vec<PlayerSetup>,
    /// Starting stockpile override; None uses the original rules default.
    pub starting_resources: Option<f32>,
    pub seed: u64,
}

impl World {
    /// Implemented by the core owner.
    pub fn new_skirmish(data: Arc<GameData>, map: Arc<GameMap>, setup: &SkirmishSetup) -> anyhow::Result<World> {
        let world = World::empty(data, map, setup.seed);
        let _ = setup;
        anyhow::bail!("World::new_skirmish is not implemented yet ({} kingdoms)", world.kingdoms.len())
    }
}
