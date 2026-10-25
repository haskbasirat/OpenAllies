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
fn neutral_actor(data: &GameData, def: crate::data::DefId) -> bool {
    let d = data.def(def);
    d.actor_data && (d.category.eq_ignore_ascii_case("BUILDING") || d.city || d.has_component("CityComponent") || (d.has_economy && d.production_all.iter().any(|v| *v > 0.0)))
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
        // The saved queue 0 (0x50B8B0): the kingdom economy Updatables in
        // their saved order; a destroyed kingdom's entry leaves with it. A
        // map without saved queues admits its kingdoms (crate::scheduler).
        if let Some(saved) = &map.queue0_kingdoms {
            let order: Vec<u32> = saved.iter().filter_map(|&s| slot_kingdom.get(s).copied().flatten()).collect();
            crate::scheduler::restore_kingdoms(&mut world, &order);
        }
        crate::scheduler::admit_kingdoms(&mut world);
        // The map's things in record order keep their saved ids; the
        // placeholders of player kingdoms are resolved afterwards (4FDA70).
        let mut replacements: Vec<(bool, u16, usize, crate::data::DefId, u32)> = Vec::new();
        for (index, p) in map.placed.iter().enumerate() {
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
            map.placed.iter().enumerate().filter(|(_, p)| p.serialized_id != 0).map(|(i, p)| (crate::ids::Gtid::from_raw(p.serialized_id).slot(), i)).collect();
        restored.sort_unstable();
        for (_, i) in restored {
            if let Some(def) = data.find(&map.placed[i].thing_id) {
                crate::creation::restored(&mut world, def);
            }
        }
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
            if actor {
                world.ids.replace_placed(index);
                spawn_placed(&mut world, def, Some(k), glam::Vec2::new(p.position[0], p.position[1]), p.yaw, index);
                world.ids.drop_request();
            }
            // A replacement object (not modelled by aa-play) keeps the id as
            // the placed thing.
        }
        // Supply zones and the economy as they stand at the start.
        crate::supply::tick(&mut world);
        crate::economy::refresh(&mut world);
        for k in 0..world.kingdoms.len() as u32 {
            crate::economy::publish(&mut world, k);
        }
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
fn relations(world: &World, map: &GameMap, slot_kingdom: &[Option<u32>], players: usize) -> Vec<Vec<u8>> {
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
                if world.kingdoms[a].team == world.kingdoms[b].team { FRIEND } else { ENEMY }
            } else if a < players {
                // The player regards the independent as the independent
                // regards the player's team (the saved tables are symmetric).
                saved(b, a).unwrap_or(ENEMY)
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

fn spawn_placed(world: &mut World, def: crate::data::DefId, kingdom: Option<u32>, pos: glam::Vec2, facing: f32, index: usize) {
    let data = world.data.clone();
    if data.def(def).class == Some(crate::data::DefClass::Organization) {
        if let Some(k) = kingdom {
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
