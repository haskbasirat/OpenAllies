//! Debug only (AA_BEVY_SKIRMISH_TEST=1): put a Russian and a German infantry
//! regiment near the local HQ and order them to attack-move at each other, so
//! movement, combat and the animations can be watched before selection and
//! orders work in the HUD. Everything after the spawn is the real game
//! systems. Owner: lead.
use crate::session::Battle;
use aa_play::{Command, Unit};
use bevy::prelude::*;

pub fn spawn(battle: Option<ResMut<Battle>>, mut done: Local<bool>) {
    if *done || std::env::var_os("AA_BEVY_SKIRMISH_TEST").is_none() {
        return;
    }
    let Some(mut battle) = battle else { return };
    *done = true;
    let world = &mut battle.world;
    let (Some(rus), Some(ger)) = (world.data.find("regiment_rus_infantry"), world.data.find("regiment_ger_infantry")) else {
        warn!("skirmish test: infantry regiments not in the data");
        return;
    };
    if world.kingdoms.len() < 2 {
        warn!("skirmish test needs two kingdoms");
        return;
    }
    // Near the local player's first building.
    let hq = world
        .actors
        .iter()
        .find(|a| a.kingdom == Some(0) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
        .map(|a| a.pos.truncate())
        .unwrap_or(Vec2::new(100.0, 100.0));
    // In view of the original camera (yaw -45 deg): either side of the HQ,
    // 28 m apart, in front of it.
    let a = hq + Vec2::new(-1.4, -18.4);
    let b = hq + Vec2::new(18.4, 1.4);
    let ours = aa_play::production::spawn_organization(world, rus, 0, a, 0.0, None);
    let theirs = aa_play::production::spawn_organization(world, ger, 1, b, std::f32::consts::PI, None);
    battle.pending.push(Command::Move { kingdom: 0, units: vec![Unit::Org(ours)], to: b, attack_move: true });
    battle.pending.push(Command::Move { kingdom: 1, units: vec![Unit::Org(theirs)], to: a, attack_move: true });
    info!("skirmish test: regiment {ours} and {theirs} ordered to attack-move");
}
