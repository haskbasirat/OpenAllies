//! Display test world until the real skirmish setup lands: a Russian HQ,
//! soldiers and a tank as aa-play actors, moved on scripted loops so the
//! battle view can be checked. Not gameplay; the numbers here only drive the
//! display. Owner: lead. Delete once World::new_skirmish works.
use crate::session::Battle;
use aa_play::{Activity, Controller, Def, DefClass, GameData, GameMap, Kingdom, World};
use bevy::prelude::*;
use std::sync::Arc;

/// Actors on a loop: (actor, centre, radius, speed m/s, start angle).
#[derive(Resource, Default)]
pub struct Loops(Vec<(u32, Vec2, f32, f32, f32)>);

fn def(id: &str, class: DefClass, model: &str) -> Def {
    Def { id: id.into(), class: Some(class), health: 1.0, model: Some(model.into()), ..Default::default() }
}

/// `heights` are the map's corner heights; `hq` is the HQ in render X/Z.
pub fn build(width: u32, height: u32, cell: f32, heights: Vec<f32>, hq: Vec2, team: [u8; 3]) -> (Battle, Loops) {
    let mut data = GameData::default();
    for d in [
        def("rus_corps", DefClass::Building, "Buildings/RUS/RUSCorps/RUSCorps.nif"),
        def("rus_infantry", DefClass::Member, "Units/RUS/RUSInfantry/RUSInfantry.nif"),
        def("rus_tank_medium", DefClass::Member, "Units/RUS/RUSTankMedium/RUSTankMedium.nif"),
    ] {
        data.by_id.insert(d.id.clone(), data.defs.len() as u32);
        data.defs.push(d);
    }
    let map = GameMap { id: "display".into(), width, height, cell_size: cell, heights, ..Default::default() };
    let mut world = World::empty(Arc::new(data), Arc::new(map), 1);
    world.kingdoms.push(Kingdom {
        id: 0,
        name: "Player".into(),
        nation: "rus".into(),
        team: 0,
        color: team,
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
    });
    // Native ground point of the HQ (render z = -native y).
    let centre = Vec2::new(hq.x, -hq.y);
    let at = |w: &World, p: Vec2| w.ground(p);
    let p = at(&world, centre);
    world.spawn_actor(0, Some(0), p, -std::f32::consts::FRAC_PI_2);
    let mut loops = Loops::default();
    for row in 0..2 {
        for i in 0..6 {
            let pos = centre + Vec2::new(-6.0 + i as f32 * 2.0, -12.0 - row as f32 * 1.6);
            let p = at(&world, pos);
            let id = world.spawn_actor(1, Some(0), p, -std::f32::consts::FRAC_PI_2);
            world.set_activity(id, if row == 0 { Activity::Idle } else { Activity::Attacking });
        }
    }
    for i in 0..8 {
        let id = world.spawn_actor(1, Some(0), p, 0.0);
        world.set_activity(id, Activity::Moving);
        loops.0.push((id, centre, 16.0 + (i % 2) as f32 * 1.6, 3.5, -(i / 2) as f32 * 0.12));
    }
    for i in 0..2 {
        let pos = centre + Vec2::new(8.0 + i as f32 * 2.0, -10.0);
        let p = at(&world, pos);
        let id = world.spawn_actor(1, Some(0), p, 0.5);
        world.set_activity(id, Activity::Dying);
    }
    let tank = world.spawn_actor(2, Some(0), p, 0.0);
    world.set_activity(tank, Activity::Moving);
    loops.0.push((tank, centre, 26.0, 6.0, 2.0));
    (Battle::new(world, 0), loops)
}

/// Place looping actors from the battle time (scripted display motion).
pub fn drive(battle: Option<ResMut<Battle>>, loops: Option<Res<Loops>>) {
    let (Some(mut battle), Some(loops)) = (battle, loops) else { return };
    let time = battle.world.time;
    for &(id, centre, radius, speed, start) in &loops.0 {
        let angle = start + speed / radius * time;
        let p = centre + Vec2::new(angle.cos(), angle.sin()) * radius;
        let ground = battle.world.ground(p);
        if let Some(actor) = battle.world.actor_mut(id) {
            actor.pos = ground;
            // Tangent of a counter-clockwise loop.
            actor.facing = angle + std::f32::consts::FRAC_PI_2;
        }
    }
}
