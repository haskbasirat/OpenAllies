//! Shared contract between the menus/HUD/input side and the battle renderer.
//! Shared file: change only by agreement (add, never repurpose).
use aa_play::{Command, KingdomId, SkirmishSetup, Unit, World};
use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    /// Original front-end menus (main menu, Single Player, Custom setup).
    #[default]
    Menu,
    /// Original loading screen while the battle is built.
    Loading,
    Battle,
}

/// The custom battle the setup screen asked for; the loader builds `Battle`.
/// Flow: the UI inserts this and sets AppState::Loading; the lead's loader
/// builds the battle in the background, updates `LoadProgress`, inserts
/// `Battle` and sets AppState::Battle (or AppState::Menu with `error` set).
#[derive(Resource, Clone, Debug)]
pub struct PendingSetup(pub SkirmishSetup);

/// Set together with `PendingSetup` when the player chose Random Map: the
/// loader generates the map from these original options instead of loading
/// `SkirmishSetup::map_id`.
#[derive(Resource, Clone, Debug)]
pub struct PendingRandomMap(pub aa_play::mapgen::RandomMapParams);

/// Loader progress for the original loading screen.
#[derive(Resource, Clone, Debug, Default)]
pub struct LoadProgress {
    /// 0..1.
    pub fraction: f32,
    /// Status text key or message.
    pub status: String,
    /// Set when loading failed; the UI shows it and returns to the menus.
    pub error: Option<String>,
}

/// The loader's current stage as the original's status key (Localization
/// `app_status_*`, set by the world load `0x4FC600` / `0x682EE0` in this
/// order: loading_data, generating_map / verifying_map for a random map,
/// loading_objects, loading_resources; `0x6E8C00` finished). Written by the
/// background loader thread, read by the loading screen. (Appended by the UI
/// owner.)
pub static LOAD_STATUS: std::sync::Mutex<&'static str> = std::sync::Mutex::new("app_status_loading_data");

/// Set the loader's status key (see `LOAD_STATUS`).
pub fn set_load_status(key: &'static str) {
    if let Ok(mut s) = LOAD_STATUS.lock() {
        *s = key;
    }
}

/// The loader's status key now.
pub fn load_status() -> &'static str {
    LOAD_STATUS.lock().map(|s| *s).unwrap_or("app_status_loading_data")
}

/// The running battle.
#[derive(Resource)]
pub struct Battle {
    pub world: World,
    /// The human player's kingdom.
    pub local: KingdomId,
    pub selection: Vec<Unit>,
    /// Commands from input and HUD, applied on the next tick.
    pub pending: Vec<Command>,
    pub paused: bool,
    /// Game speed multiplier (the original's game speed setting).
    pub speed: f32,
    accumulator: f32,
    /// Events of every tick since the last frame, for HUD/sound/effects.
    pub frame_events: Vec<aa_play::Event>,
}

impl Battle {
    pub fn new(world: World, local: KingdomId) -> Self {
        Self { world, local, selection: Vec::new(), pending: Vec::new(), paused: false, speed: 1.0, accumulator: 0.0, frame_events: Vec::new() }
    }
}

/// Fixed 16 Hz simulation, decoupled from the frame rate.
pub fn step_battle(time: Res<Time>, battle: Option<ResMut<Battle>>) {
    let Some(mut battle) = battle else { return };
    battle.frame_events.clear();
    if battle.paused {
        return;
    }
    battle.accumulator += time.delta_secs() * battle.speed;
    let mut steps = 0;
    while battle.accumulator >= aa_play::DT && steps < 8 {
        battle.accumulator -= aa_play::DT;
        let commands = std::mem::take(&mut battle.pending);
        battle.world.step(commands);
        let events = std::mem::take(&mut battle.world.events);
        battle.frame_events.extend(events.iter().cloned());
        battle.world.events = events;
        steps += 1;
    }
    if steps == 8 {
        battle.accumulator = 0.0;
    }
}

/// Fraction of the way to the next tick, for smooth drawing between ticks.
pub fn tick_fraction(battle: &Battle) -> f32 {
    (battle.accumulator / aa_play::DT).clamp(0.0, 1.0)
}

/// Set together with `PendingSetup` by the staging screen's launch: the
/// computer general (SAI profile `IDS`, Data/SAI/*.tgi, e.g. what the
/// slot's StagingSelectSAI picked or the random pick for the nation) per
/// computer player, as (index into `SkirmishSetup::players`, profile ids),
/// for `aa_play::ai::set_profile` on that player's kingdom.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingAiProfiles(pub Vec<(usize, String)>);

/// Object animations the FX system started on actors (FXC
/// PLAY_OBJECT_ANIMATION, e.g. a tank's `recoil1` when it fires): actor ->
/// (the animation IDS, lower case, and the battle time it started). Written
/// by the effects (effects/mod.rs); battle_view can play the rig clip of
/// that name (case-insensitive, e.g. "Recoil1") once from that time.
/// (Appended by the effects and sound owner.)
#[derive(Resource, Default)]
pub struct ObjectAnimations(pub std::collections::HashMap<aa_play::ActorId, (String, f32)>);
