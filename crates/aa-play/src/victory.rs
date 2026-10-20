//! Defeat when a kingdom has no buildings and no forces left; the
//! winning team. Owner: opponent.
//!
//! Behaviour-faithful port of the GKingdom victory checks the economy tick
//! runs once world time >= 1.0 (0x4E9C40 -> 0x4E9960): a kingdom's counted
//! actors (+0x2F4, predicate 0x4E1540) are its living organizations, its
//! living actors that can recruit (RecruitComponent), and its trucks or
//! packable buildings whose other form is an organization or can recruit.
//! With TeamVictory (SVars.tgi: true) the whole team's count decides
//! (0x4E64C0): when it reaches zero every kingdom of the team is defeated
//! (0x4E9720 status 2, event 0x34). When only one team has undefeated
//! kingdoms left it has won (status 1, event 0x35).
//! Not ported: the percentage victory check 0x4E9A40 (a world rule that is
//! zero in custom battles) and the scripted mission victory triggers.
use crate::world::{Event, World};

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

pub fn tick(world: &mut World) {
    if world.time < 1.0 || world.winner.is_some() || world.kingdoms.is_empty() {
        return;
    }
    let team_victory = world.data.rule_ci("TeamVictory").is_none_or(|v| v != 0.0);
    let count = counted(world);
    let team_count = |team: u32| -> u32 {
        world.kingdoms.iter().zip(&count).filter(|(k, _)| k.team == team && !k.independent).map(|(_, c)| *c).sum()
    };
    let mut defeated = Vec::new();
    for (i, k) in world.kingdoms.iter().enumerate() {
        // Independent kingdoms never lose (and never win).
        if k.defeated || k.independent {
            continue;
        }
        let alive = if team_victory { team_count(k.team) } else { count[i] };
        if alive == 0 {
            defeated.push(i);
        }
    }
    for i in defeated {
        world.kingdoms[i].defeated = true;
        world.events.push(Event::KingdomDefeated { kingdom: i as u32 });
    }
    let mut all: Vec<u32> = world.kingdoms.iter().filter(|k| !k.independent).map(|k| k.team).collect();
    all.sort_unstable();
    all.dedup();
    let mut teams: Vec<u32> = world.kingdoms.iter().filter(|k| !k.defeated && !k.independent).map(|k| k.team).collect();
    teams.sort_unstable();
    teams.dedup();
    if all.len() < 2 {
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
