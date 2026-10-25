//! Defeat when a kingdom has no buildings and no forces left; the
//! winning team. Owner: core.
//!
//! Behaviour-faithful port of the GKingdom victory checks every kingdom's
//! economy tick runs once world time (GWorld+0xE0) >= 1.0 (0x4E9C40, called
//! from `crate::economy`):
//! - 0x4E9960: a kingdom's counted actors (+0x2F4, predicate 0x4E1540) are
//!   its living organizations, its living actors that can recruit
//!   (RecruitComponent), and its trucks or packable buildings whose other
//!   form is an organization or can recruit. With TeamVictory (SVars.tgi:
//!   true) the whole team's count decides (0x4E64C0): when it is zero every
//!   kingdom of the team is defeated (0x4E9720 status 2, event 0x34);
//! - 0x4E9A40: with the CityVictoryRatio world rule above zero (default 0),
//!   a kingdom whose team owns at least that percentage of the cities
//!   (`owned / available * 100` at PC53) wins (event 0x39, 0x4E9720
//!   status 1);
//! - when only one team has undefeated kingdoms left it has won (status 1,
//!   event 0x35).
//! Independent kingdoms (+0x190) never lose and never win. Not ported: the
//! scripted mission victory triggers and the World+28 / mission-start /
//! phase-6 guards (always open in a custom battle).
use crate::world::{Event, KingdomId, World};

/// Native 0x4E1540 for a definition.
fn counts(world: &World, def: crate::data::DefId) -> bool {
    let d = world.data.def(def);
    d.has_component("OrganizationComponent") || d.has_component("RecruitComponent")
}

/// Number of counted actors of each kingdom (index = KingdomId).
pub fn counted(world: &World) -> Vec<u32> {
    let mut out = vec![0u32; world.kingdoms.len()];
    for org in &world.orgs {
        if org.alive && (org.kingdom as usize) < out.len() {
            out[org.kingdom as usize] += 1;
        }
    }
    for a in &world.actors {
        let Some(k) = a.kingdom.map(|k| k as usize).filter(|&k| k < out.len()) else { continue };
        if !a.alive || a.removed || a.org.is_some() {
            continue;
        }
        let d = world.data.def(a.def);
        if counts(world, a.def) || d.unpacks_to.or(d.packs_to).is_some_and(|other| counts(world, other)) {
            out[k] += 1;
        }
    }
    out
}

fn team_victory(world: &World) -> bool {
    world.data.rule_ci("TeamVictory").is_none_or(|v| v != 0.0)
}

/// The kingdom and, with TeamVictory, its team mates (World+0x154 order).
fn side(world: &World, k: KingdomId) -> Vec<usize> {
    let team = world.kingdoms[k as usize].team;
    let tv = team_victory(world);
    (0..world.kingdoms.len())
        .filter(|&i| !world.kingdoms[i].independent && (i == k as usize || (tv && world.kingdoms[i].team == team)))
        .collect()
}

/// A result is open: the kingdom plays, is not defeated (+0x19C) and the
/// battle is not over.
fn open(world: &World, k: KingdomId) -> bool {
    let kingdom = &world.kingdoms[k as usize];
    world.winner.is_none() && !kingdom.independent && !kingdom.defeated
}

/// 0x4E9960 for kingdom `k`.
pub fn team_check(world: &mut World, k: KingdomId) {
    if (k as usize) >= world.kingdoms.len() || !open(world, k) {
        return;
    }
    let count = counted(world);
    let members = side(world, k);
    if members.iter().map(|&i| count[i]).sum::<u32>() > 0 {
        return;
    }
    for i in members {
        if !world.kingdoms[i].defeated {
            world.kingdoms[i].defeated = true;
            world.events.push(Event::KingdomDefeated { kingdom: i as u32 });
        }
    }
    resolve(world);
}

/// 0x4E9A40 for kingdom `k`.
pub fn city_check(world: &mut World, k: KingdomId) {
    if (k as usize) >= world.kingdoms.len() || !open(world, k) {
        return;
    }
    let ratio = world.data.rule_ci("CityVictoryRatio").map_or(0, |v| v as i32);
    if ratio <= 0 {
        return;
    }
    // 4F1B30: the cities of the world; 4E3AB0: those a kingdom owns.
    let cities: Vec<Option<KingdomId>> = world.actors.iter().filter(|a| a.alive && !a.removed && world.data.def(a.def).city).map(|a| a.kingdom).collect();
    let available = cities.len() as i32;
    if available <= 0 {
        return;
    }
    let members = side(world, k);
    let owned = cities.iter().filter(|c| c.is_some_and(|o| members.contains(&(o as usize)))).count() as i32;
    // fild owned; fidiv available; fmul 100.0f; fild ratio; fcompp (PC53).
    let percent = f64::from(owned) / f64::from(available) * 100.0;
    if f64::from(ratio) <= percent {
        let team = world.kingdoms[k as usize].team;
        world.winner = Some(team);
        world.events.push(Event::Victory { team });
    }
}

/// 0x4E9720's end of battle: one team left wins; nobody left ends it.
fn resolve(world: &mut World) {
    let mut all: Vec<u32> = world.kingdoms.iter().filter(|k| !k.independent).map(|k| k.team).collect();
    all.sort_unstable();
    all.dedup();
    let mut teams: Vec<u32> = world.kingdoms.iter().filter(|k| !k.defeated && !k.independent).map(|k| k.team).collect();
    teams.sort_unstable();
    teams.dedup();
    if all.len() < 2 || world.winner.is_some() {
        // A battle of one team has nobody to win against.
        return;
    }
    if teams.len() == 1 {
        world.winner = Some(teams[0]);
        world.events.push(Event::Victory { team: teams[0] });
    } else if teams.is_empty() {
        // Everybody fell in the same tick: no winner, the battle is over.
        world.winner = Some(u32::MAX);
    }
}

/// Both checks for every kingdom, as the economy ticks run them (for callers
/// that step systems by hand).
pub fn tick(world: &mut World) {
    if f32::from_bits(world.clock.begin.0) < 1.0 {
        return;
    }
    for k in 0..world.kingdoms.len() as KingdomId {
        team_check(world, k);
        city_check(world, k);
    }
}
