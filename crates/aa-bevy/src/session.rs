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
#[derive(Resource, Clone, Debug)]
pub struct PendingSetup(pub SkirmishSetup);

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
