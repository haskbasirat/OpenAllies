//! Measurement only (AA_BEVY_FX_BENCH=<soldiers>): puts that many Russian and
//! German infantry (whole original regiments, half each side) in two lines
//! near the local HQ and orders them to attack-move at each other, so the
//! frame cost of the effects and sound can be measured with ~100 soldiers
//! firing. Everything after the spawn is the real game systems.
use crate::session::Battle;
use aa_play::{Command, Unit};
use bevy::prelude::*;

pub fn spawn(battle: Option<ResMut<Battle>>, ready: Option<Res<super::FxState>>, mut done: Local<bool>) {
    if *done || ready.is_none() {
        return;
    }
    let Some(soldiers) = std::env::var("AA_BEVY_FX_BENCH").ok().and_then(|v| v.parse::<usize>().ok()) else {
        *done = true;
        return;
    };
    let Some(mut battle) = battle else { return };
    *done = true;
    let world = &mut battle.world;
    // AA_BEVY_FX_BENCH_KIND: the regiment kind ("infantry", "tank_medium",
    // "infantry_artillery"...), the same for both nations.
    let kind = std::env::var("AA_BEVY_FX_BENCH_KIND").unwrap_or_else(|_| "infantry".into());
    // "<russian kind>/<german kind>" sets the two sides apart.
    let (rk, gk) = kind.split_once('/').unwrap_or((kind.as_str(), kind.as_str()));
    let (Some(rus), Some(ger)) = (world.data.find(&format!("regiment_rus_{rk}")), world.data.find(&format!("regiment_ger_{gk}"))) else {
        warn!("fx bench: no regiment_rus_{rk} / regiment_ger_{gk}");
        return;
    };
    if world.kingdoms.len() < 2 {
        return;
    }
    let hq = world
        .actors
        .iter()
        .find(|a| a.kingdom == Some(0) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
        .map(|a| a.pos.truncate())
        .unwrap_or(Vec2::new(100.0, 100.0));
    let a = hq + Vec2::new(-1.4, -18.4);
    let b = hq + Vec2::new(18.4, 1.4);
    let d = (b - a).normalize_or_zero();
    let perp = Vec2::new(-d.y, d.x);
    let per_side = soldiers.div_ceil(12).max(1);
    let mut orders = Vec::new();
    for i in 0..per_side {
        let off = perp * ((i as f32 - (per_side as f32 - 1.0) * 0.5) * 9.0) - d * 6.0;
        let ours = aa_play::production::spawn_organization(world, rus, 0, a + off, 0.0, None);
        let theirs = aa_play::production::spawn_organization(world, ger, 1, b + off + d * 12.0, std::f32::consts::PI, None);
        orders.push(Command::Move { kingdom: 0, units: vec![Unit::Org(ours)], to: b + off, attack_move: true });
        orders.push(Command::Move { kingdom: 1, units: vec![Unit::Org(theirs)], to: a + off, attack_move: true });
    }
    battle.pending.extend(orders);
    info!("fx bench: {} regiments a side ordered to attack-move", per_side);
}
