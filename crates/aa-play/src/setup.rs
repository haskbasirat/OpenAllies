//! Custom battle setup: the players the original setup screen collects, and
//! building the starting world from the map (each start location gets its
//! nation's Corps HQ in place of `placeholder_corpsunpacked`, starting
//! resources from the rules). Owner: core.
//!
//! Behaviour-faithful reading of the original custom-game start:
//! - each player takes the map kingdom slot of its start location
//!   (`GameMap::player_slots`); kingdoms of unused slots are removed with all
//!   their placed things (Kingdom destruction 4F53D0/4E4130);
//! - a placed thing owned by a used slot is replaced through the player's
//!   nation placeholder table (Data/Nations/<nation>.tgi `[Placeholder]`,
//!   e.g. placeholder_corpsunpacked -> rus_corps, placeholder_bunker ->
//!   rus_bunker, placeholder_depot_supply -> rus_truck_supply), other owned
//!   ActorData things (start_loc...) stay that kingdom's actors; the
//!   replacements are created after every kept map thing, in the order of
//!   GWorld::ResolvePlaceholdersAndRandomThings 0x4FDA70 / 0x4FC4A0 (objects,
//!   then actors, each list in ascending placeholder slot), which is also the
//!   order their native ids are given in (`crate::ids`);
//! - independent-owned buildings (cities, farms, city blocks: ActorData of
//!   category BUILDING or with a City/Economy component) become neutral
//!   actors; unowned things (doodads, roads, trees) stay scenery;
//! - start mission 0x4E6DB0: the stockpile is reset to the starting value
//!   (ResourceStockpileDefault, 500 in world_rules_aa.tgi) unless overridden.
use crate::data::GameData;
use crate::map::GameMap;
use crate::world::{Controller, Kingdom, World};
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

/// Neutral placed things that matter to play (become actors without owner).
pub(crate) fn neutral_actor(data: &GameData, def: crate::data::DefId) -> bool {
    let d = data.def(def);
    // An organization or unit an independent kingdom holds is one too: a mission's
    // script gives it to the player (`TransferControl`, Actor virtual +160).
    let unit = matches!(d.class, Some(crate::data::DefClass::Organization | crate::data::DefClass::Member | crate::data::DefClass::Truck));
    d.actor_data && (unit || d.category.eq_ignore_ascii_case("BUILDING") || d.city || d.has_component("CityComponent") || (d.has_economy && d.production_all.iter().any(|v| *v > 0.0)))
}

impl World {
    /// Implemented by the core owner.
    pub fn new_skirmish(data: Arc<GameData>, map: Arc<GameMap>, setup: &SkirmishSetup) -> anyhow::Result<World> {
        Self::new_skirmish_with(data, map, setup, &[])
    }

    /// `new_skirmish` with the staging screen's random picks, one per player
    /// (missing: none). WorldParameters 0x50E8B0 resolves them during map
    /// restore, in the creator's kingdom (map slot) order, with the world
    /// RNG: a random nation replaces the player's, a random general is kept
    /// for `special_ops::set_general(world, kingdom, None)`.
    pub fn new_skirmish_with(data: Arc<GameData>, map: Arc<GameMap>, setup: &SkirmishSetup, picks: &[crate::creation::StartPick]) -> anyhow::Result<World> {
        let mut world = World::empty(data.clone(), map.clone(), setup.seed);
        if setup.players.is_empty() {
            anyhow::bail!("a custom battle needs at least one player");
        }
        let start = setup.starting_resources.or_else(|| data.rule_ci("ResourceStockpileDefault")).unwrap_or(0.0);
        // StartMission 0x4E6DB0 (aa-economy kingdom_start): every stockpile
        // row of every kingdom is reset to the starting value (+0 unless
        // positive), at most EconomyStockpileResourceMax.
        let start_max = data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
        let start_value = if start > 0.0 { start } else { 0.0 };
        let start_row = if start_max < start_value { start_max } else { start_value };
        // 0x50E8B0 per player kingdom in creator (map slot) order.
        let mut order: Vec<usize> = (0..setup.players.len()).collect();
        order.sort_by_key(|&i| map.player_slots.get(setup.players[i].start).copied().unwrap_or(usize::MAX));
        let mut nations: Vec<String> = setup.players.iter().map(|p| p.nation.clone()).collect();
        let mut generals: Vec<Option<String>> = vec![None; setup.players.len()];
        for &i in &order {
            let pick = picks.get(i).copied().unwrap_or_default();
            if pick.random_nation
                && let Some(n) = crate::creation::random_nation(&mut world)
            {
                nations[i] = n;
            }
            if pick.random_general {
                generals[i] = crate::creation::random_general(&mut world, &nations[i]);
            }
        }
        let mut slot_kingdom: Vec<Option<u32>> = vec![None; map.slots.len()];
        for (i, p) in setup.players.iter().enumerate() {
            let Some(&slot) = map.player_slots.get(p.start) else {
                anyhow::bail!("player {} start {} is not one of the map's {} start locations", p.name, p.start, map.start_locations.len());
            };
            if slot_kingdom[slot].is_some() {
                anyhow::bail!("two players share start location {}", p.start);
            }
            let nation = data.nation(&nations[i]).ok_or_else(|| anyhow::anyhow!("unknown nation {}", nations[i]))?;
            slot_kingdom[slot] = Some(i as u32);
            world.kingdoms.push(Kingdom {
                id: i as u32,
                name: p.name.clone(),
                nation: nation.id.clone(),
                team: p.team,
                color: p.color,
                controller: p.controller,
                stockpile: [0.0; 3],
                income: [0.0; 3],
                unit_limit: 0.0,
                units_used: 0.0,
                defeated: false,
                independent: false,
            });
            // Difficulty handicap (Game/handicaps.tgi in difficulty order).
            let handicap = match p.controller {
                Controller::Computer(level) => data.handicaps.get(level as usize).and_then(|h| h.1.clone()),
                _ => None,
            };
            let kb = world.books.kingdom_mut(i as u32, data.resources.len());
            kb.stockpiled.iter_mut().for_each(|s| *s = start_row);
            kb.handicap = handicap;
            kb.general = generals[i].clone();
        }
        // The map's independent kingdoms (header "independent_N", native
        // Kingdom +0x190 = 1) own the cities and the other neutral buildings.
        let players = world.kingdoms.len();
        for (s, slot) in map.slots.iter().enumerate().filter(|(_, s)| s.independent) {
            let id = world.kingdoms.len() as u32;
            slot_kingdom[s] = Some(id);
            let rgb = slot.color.and_then(|c| data.kingdom_colors.get(c as usize)).map_or([1.0, 1.0, 1.0], |c| c.1);
            world.kingdoms.push(Kingdom {
                id,
                name: slot.id.clone(),
                nation: String::new(),
                team: INDEPENDENT_TEAM | s as u32,
                color: rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8),
                controller: Controller::Independent,
                stockpile: [0.0; 3],
                income: [0.0; 3],
                unit_limit: 0.0,
                units_used: 0.0,
                defeated: false,
                independent: true,
            });
            world.books.kingdom_mut(id, data.resources.len()).stockpiled.iter_mut().for_each(|s| *s = start_row);
        }
        world.books.relations = relations(&world, &map, &slot_kingdom, players);
        // Native ids (crate::ids): every kingdom takes its map slot's id; the
        // kingdoms of unused slots are destroyed with their things.
        for (s, k) in slot_kingdom.iter().enumerate() {
            match k {
                Some(k) => world.ids.bind_kingdom(*k, s),
                None => world.ids.destroy_map_kingdom(&map, s),
            }
        }
        // The map's things in record order keep their saved ids; the
        // placeholders of player kingdoms are resolved afterwards (4FDA70).
        let mut replacements: Vec<(bool, u16, usize, crate::data::DefId, u32)> = Vec::new();
        for (index, p) in map.placed.iter().enumerate() {
            // A member stands with its organization's record.
            if p.load_fails || p.member_of.is_some() || map.generated.as_ref().is_some_and(|g| g.trailers.iter().any(|t| t.1 == index)) {
                continue;
            }
            let owner = p.owner_slot.map(|s| s as usize).filter(|&s| s < map.slots.len());
            let thing = p.thing_id.to_ascii_lowercase();
            let placeholder = data.nations.iter().any(|n| n.placeholders.contains_key(&thing));
            let pos = glam::Vec2::new(p.position[0], p.position[1]);
            match owner {
                Some(s) if !map.slots[s].independent => {
                    let Some(k) = slot_kingdom[s] else { continue };
                    let nation = data.nation(&world.kingdoms[k as usize].nation);
                    if placeholder {
                        // A missing substitution (GWorld.cpp 0x138E) leaves
                        // the placeholder as it is.
                        if let Some(def) = nation.and_then(|n| n.placeholders.get(&thing).copied()) {
                            let slot = crate::ids::Gtid::from_raw(p.serialized_id).slot();
                            replacements.push((data.def(def).actor_data, slot, index, def, k));
                        }
                        continue;
                    }
                    match data.find(&thing).filter(|&d| data.def(d).actor_data) {
                        Some(def) => keep_placed(&mut world, def, Some(k), pos, p.yaw, index),
                        None => world.books.scenery.push(index),
                    }
                }
                Some(s) => match data.find(&thing).filter(|&d| neutral_actor(&data, d)) {
                    Some(def) => keep_placed(&mut world, def, slot_kingdom[s], pos, p.yaw, index),
                    None if placeholder => {}
                    None => world.books.scenery.push(index),
                },
                None if placeholder => {}
                None => world.books.scenery.push(index),
            }
        }
        // The restored things' InitExisting draws (4FC600), in ascending GTID
        // slot order: a DynamicDoodad component each (5904A0 / 58F960), a
        // Supply component registering on queue 1 (58B0E0). All saved things
        // are restored, also those of the kingdoms the battle leaves out.
        let mut restored: Vec<(u16, usize)> =
            map.placed.iter().enumerate().filter(|(_, p)| p.serialized_id != 0 && !p.load_fails).map(|(i, p)| (crate::ids::Gtid::from_raw(p.serialized_id).slot(), i)).collect();
        restored.sort_unstable();
        for &(_, i) in restored.iter().filter(|_| map.generated.is_none()) {
            if map.placed[i].member_of.is_some() {
                continue;
            }
            if let Some(def) = data.find(&map.placed[i].thing_id)
                && !crate::creation::is_converted(&map.placed[i], data.def(def))
            {
                match &map.placed[i].organization {
                    Some(organization) => crate::creation::restored_organization(&mut world, def, organization),
                    None => crate::creation::restored(&mut world, def),
                }
            }
        }
        // The queue Load 0x50B8B0 (crate::scheduler): the saved periodic
        // queues replace the world's (no draws); a thing destroyed with its
        // kingdom leaves its entries. A map without saved queues admits its
        // kingdoms and its kept things' queue-1 components.
        let mut kept: std::collections::BTreeMap<usize, crate::scheduler::Kept> = std::collections::BTreeMap::new();
        for a in 0..world.actors.len() as u32 {
            let Some(i) = world.books.actor(a).and_then(|b| b.placed) else { continue };
            let organization = data.find(&map.placed[i].thing_id).is_some_and(|d| data.def(d).class == Some(crate::data::DefClass::Organization));
            match (organization, world.actors[a as usize].org) {
                (true, Some(o)) => kept.entry(i).or_insert(crate::scheduler::Kept::Org(o)),
                _ => kept.entry(i).or_insert(crate::scheduler::Kept::Actor(a)),
            };
        }
        // A placed organization with its members' own records: the organization is
        // no actor's `placed`.
        for (i, p) in map.placed.iter().enumerate() {
            if let Some(o) = p.organization.as_ref().and_then(|o| o.members.first()).and_then(|&(_, m)| kept.get(&m)).and_then(|k| match k {
                crate::scheduler::Kept::Actor(a) => world.actors[*a as usize].org,
                crate::scheduler::Kept::Org(_) => None,
            }) {
                kept.insert(i, crate::scheduler::Kept::Org(o));
            }
        }
        crate::movement::restore_saved_minds(&mut world, &kept);
        match &map.saved_queues {
            _ if map.generated.is_some() => {}
            Some(saved) => crate::scheduler::restore_saved(&mut world, saved, &crate::scheduler::Restoring { things: &kept, slot_kingdom: &slot_kingdom }),
            None => {
                let order: Vec<crate::scheduler::Kept> = restored.iter().filter_map(|&(_, i)| kept.get(&i).copied()).collect();
                crate::scheduler::admit_restored(&mut world, &order);
            }
        }
        if let Some(saved) = &map.saved_deferred {
            crate::scheduler::restore_deferred(&mut world, saved, &crate::scheduler::Restoring { things: &kept, slot_kingdom: &slot_kingdom });
        }
        // The PostLoad pass 0x4FD3B3 over what the queue Load left out.
        if map.saved_queues.is_some() {
            let order: Vec<usize> = restored.iter().map(|&(_, i)| i).collect();
            crate::scheduler::post_load(&mut world, &map, &order, &crate::scheduler::Restoring { things: &kept, slot_kingdom: &slot_kingdom });
        }
        // The last step of the map load 0x4FD47C: the records of a class that
        // changed, created fresh in record order.
        crate::scheduler::sync_clock(&mut world);
        if let Some(generated) = &map.generated {
            crate::scheduler::replay_generated(&mut world, generated, &crate::scheduler::Restoring { things: &kept, slot_kingdom: &slot_kingdom });
        }
        for (i, p) in map.placed.iter().enumerate().filter(|_| map.generated.is_none()) {
            if let Some(def) = data.find(&p.thing_id)
                && crate::creation::is_converted(p, data.def(def))
            {
                crate::creation::converted(&mut world, def, i as u32, p.variant);
            }
        }
        crate::scheduler::admit_kingdoms(&mut world);
        // StartMission 0x4E6DB0 draws nothing.
        // 4FDA70 queues the placeholders in ascending slot order (4EFBE0),
        // objects and actors apart; 4FC4A0 runs the objects, then the actors;
        // 4FBA50 destroys each placeholder, then creates the replacement with
        // the placeholder's own GTID (4FD7F0 / 4FD8C0 capture the source id
        // as the requested id), so the replacement keeps the saved id and
        // the next-fit cursor does not move (film evidence: Berlin's computer
        // players order their Corps at the placeholders' slots).
        replacements.sort_by_key(|&(actor, slot, ..)| (actor, slot));
        for (actor, _, index, def, k) in replacements {
            let p = &map.placed[index];
            // The placeholder's own Updatables leave the queues with it.
            crate::scheduler::release_placed(&mut world, index);
            if actor {
                world.ids.replace_placed(index);
                spawn_placed(&mut world, def, Some(k), glam::Vec2::new(p.position[0], p.position[1]), p.yaw, index);
                world.ids.drop_request();
            }
            // A replacement object (not modelled by aa-play) keeps the id as
            // the placed thing.
        }
        // 0x4F1E10: the kingdoms' statistics.
        crate::statistics::start_world(&mut world);
        // Supply zones and the economy as they stand at the start.
        crate::supply::tick(&mut world);
        crate::economy::refresh(&mut world);
        for k in 0..world.kingdoms.len() as u32 {
            crate::economy::publish(&mut world, k);
        }
        crate::statistics::start_snapshots(&mut world);
        // World sync dump and session launch (one draw, S reset).
        world.begin_battle();
        Ok(world)
    }
}

/// Team number of an independent kingdom (its own team, never a player's):
/// this bit or-ed with its map slot index.
pub const INDEPENDENT_TEAM: u32 = 0x8000_0000;

/// Relation table (native 4E52D0 values) for the custom battle: players on
/// one team are friends, players on different teams enemies; a relation with
/// an independent kingdom is the map's saved one (its +0x1DC entry for the
/// player's map team, else its +0x1D0 entry for the player's map kingdom),
/// independents among themselves use their saved +0x1D0 entries.
pub(crate) fn relations(world: &World, map: &GameMap, slot_kingdom: &[Option<u32>], players: usize) -> Vec<Vec<u8>> {
    use crate::books::{ENEMY, FRIEND, NEUTRAL, SELF};
    let n = world.kingdoms.len();
    let slot_of: Vec<Option<usize>> = (0..n).map(|k| slot_kingdom.iter().position(|s| *s == Some(k as u32))).collect();
    let saved = |from: usize, to: usize| -> Option<u8> {
        let (fs, ts) = (slot_of[from]?, slot_of[to]?);
        let f = &map.slots[fs];
        let t = &map.slots[ts];
        let by_team = (!t.independent).then(|| f.relations_to_teams.get(t.team as usize)).flatten();
        let by_kingdom = t.kingdom_index.and_then(|i| f.relations_to_kingdoms.get(i as usize));
        by_team.or(by_kingdom).map(|v| (*v).min(3) as u8)
    };
    let mut out = vec![vec![NEUTRAL; n]; n];
    for a in 0..n {
        for b in 0..n {
            out[a][b] = if a == b {
                SELF
            } else if a < players && b < players {
                // (orders) 0x4E52D0: a kingdom on no team (+1EC null, a team
                // name 0x4F4980 does not find) reads its own saved tables: +1DC
                // for the other's team, +1D0 by the other's +188 otherwise.
                let no_team = |k: usize| world.kingdoms[k].team as usize >= map.teams.len();
                let own = slot_of[a].map(|s| &map.slots[s]);
                let entry = if !no_team(a) {
                    None
                } else if no_team(b) {
                    let index = slot_of[b].and_then(|s| map.slots[s].kingdom_index);
                    own.zip(index).and_then(|(f, i)| f.relations_to_kingdoms.get(i as usize))
                } else {
                    own.and_then(|f| f.relations_to_teams.get(world.kingdoms[b].team as usize))
                };
                match entry {
                    Some(v) => (*v).min(3) as u8,
                    None if world.kingdoms[a].team == world.kingdoms[b].team => FRIEND,
                    None => ENEMY,
                }
            } else if a < players {
                // (orders) 0x4E52D0 for a kingdom on no team: its own +1D0 by
                // the independent's +188.
                let no_team = world.kingdoms[a].team as usize >= map.teams.len();
                let own = slot_of[a].map(|s| &map.slots[s]);
                let index = slot_of[b].and_then(|s| map.slots[s].kingdom_index);
                let entry = own.zip(index).filter(|_| no_team).and_then(|(f, i)| f.relations_to_kingdoms.get(i as usize));
                match entry {
                    Some(v) => (*v).min(3) as u8,
                    // The player regards the independent as the independent
                    // regards the player's team (the saved tables are symmetric).
                    None => saved(b, a).unwrap_or(ENEMY),
                }
            } else {
                saved(a, b).unwrap_or(if b < players { ENEMY } else { NEUTRAL })
            };
        }
    }
    out
}

/// How kingdom `a` regards kingdom `b` in `world` (native 4E52D0 values).
pub fn relation(world: &World, a: Option<u32>, b: Option<u32>) -> u8 {
    world.books.relation(&world.kingdoms, a, b)
}

/// A map thing aa-play turns into an actor or organization: it keeps its
/// saved id.
fn keep_placed(world: &mut World, def: crate::data::DefId, kingdom: Option<u32>, pos: glam::Vec2, facing: f32, index: usize) {
    world.ids.request_placed(index);
    // Restored, not created: no creation draws (crate::creation).
    world.ids.restore_scope = true;
    spawn_placed(world, def, kingdom, pos, facing, index);
    world.ids.restore_scope = false;
    world.ids.clear_request(index);
}

/// A placed organization and the members its record holds, each under its saved id
/// (57D130 -> 4FC200 -> 4FAE80) where it was saved, no creation draws: those of the
/// Load are `creation::restored_organization`, in the record pass.
fn restore_organization(world: &mut World, def: crate::data::DefId, kingdom: u32, pos: glam::Vec2, facing: f32, saved: &crate::map::PlacedOrganization) {
    let data = world.data.clone();
    let o = crate::production::new_organization(world, def, kingdom, pos, facing, None);
    let mut morale = 0.0f32;
    for &(slot, m) in &saved.members {
        let Some(member) = world.map.placed.get(m).cloned() else { continue };
        let Some(member_def) = data.find(&member.thing_id) else { continue };
        world.ids.request_placed(m);
        let a = crate::production::spawn_single(world, member_def, Some(kingdom), glam::Vec2::new(member.position[0], member.position[1]), member.yaw);
        world.ids.clear_request(m);
        world.actors[a as usize].pos = glam::Vec3::from(member.position);
        world.actors[a as usize].org = Some(o);
        world.actors[a as usize].slot = Some(slot as u16);
        world.orgs[o as usize].members.push(a);
        world.books.actor_mut(a).placed = Some(m);
        morale += data.def(member_def).morale_value;
    }
    world.orgs[o as usize].morale = morale * f32::from_bits(saved.morale_bits);
    world.books.org_mut(o).progress = vec![0.0; data.def(def).slots.len()];
}

fn spawn_placed(world: &mut World, def: crate::data::DefId, kingdom: Option<u32>, pos: glam::Vec2, facing: f32, index: usize) {
    let data = world.data.clone();
    if data.def(def).class == Some(crate::data::DefClass::Organization) {
        if let Some(k) = kingdom {
            if let Some(saved) = world.map.placed[index].organization.clone() {
                restore_organization(world, def, k, pos, facing, &saved);
                return;
            }
            let o = crate::production::spawn_organization(world, def, k, pos, facing, None);
            let properties = world.map.placed[index].properties.clone();
            for m in world.orgs[o as usize].members.clone() {
                world.books.actor_mut(m).placed = Some(index);
                world.books.actor_mut(m).properties = properties.clone();
            }
        }
        return;
    }
    let a = crate::production::spawn_single(world, def, kingdom, pos, facing);
    world.books.actor_mut(a).placed = Some(index);
    world.books.actor_mut(a).properties = world.map.placed[index].properties.clone();
}
