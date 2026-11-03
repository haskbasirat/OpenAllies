//! Restoring a saved game's world (the world load 0x4FC600 on a `savegame` stream,
//! read by `aa_mapworld::saved_world`), so a film of a saved game has the world it
//! was recorded on. A Load Game can start from [`restore`] too; this module has no
//! UI of its own.
//!
//! The restore is labelled partial ([`Restored::notes`]): what a save holds and
//! `aa-play` has no place for yet is listed there, never dropped silently.
//!
//! Restored from the original's words: the clock, the world stream with the draws of
//! the load's record pass (exact against native traces of two films), the GTIDs, the
//! kingdoms with their stockpiles, relations, generals' experience and properties, the
//! things with their position, facing, health, owner and organization, each
//! organization's members, leader, morale, experience, attack efficiency and layout,
//! the Supply's stock, the production queues and the world queues (UPDATABLES).
//!
//! The Minds go on as saved ([`apply_saved_minds`]): stacks, pending and current commands,
//! the Move state with its path, the Mover's speed and destination, the Character's mode.
//! A rendition, said so in the notes: a Mind on a state `aa-play` does not host yet (kill,
//! guard, retreat, explore) has its saved command given again as a new order.
use crate::data::{DefId, GameData};
use crate::map::{GameMap, SavedOwner, SavedPeriodic};
use crate::scheduler::{DEFERRED, Kept, Owner, THINK};
use crate::special_ops::GeneralData;
use crate::world::{Controller, Kingdom, KingdomId, World};
use aa_mapworld::kingdoms::KingdomRecord;
use aa_mapworld::saved_components::SavedComponent;
use aa_mapworld::saved_world::{self, MindDecoder, SavedThing};
use aa_mapworld::{MapArchive, WorldInventory};
use aa_mind::construct::Kind;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The concrete Mind a definition's component mask makes (0x4CF480 case 4 takes
/// ActorData+0x174 bit 7, then 23, then 8), read by aa-mind's 536340 and 528C20.
pub(crate) struct Minds;
impl MindDecoder for Minds {
    fn decode_mind(&mut self, mask: u32, bytes: &[u8]) -> aa_mapworld::Result<(usize, Option<i32>)> {
        let kind = if mask & (1 << 7) != 0 {
            Kind::Regiment
        } else if mask & (1 << 23) != 0 {
            Kind::Aircraft
        } else {
            Kind::Element
        };
        let (saved, consumed) = aa_mind::saved_load::decode(kind, bytes).map_err(|e| aa_mapworld::Error::Format(format!("{kind:?}: {e}")))?;
        Ok((consumed, saved.updatable().and_then(|u| u.serialized_id)))
    }
}

/// The things of a saved game in `GameMap::placed` order: the top-level records in
/// index order, then the groups, then every organization's members.
pub struct SavedThings {
    pub entries: Vec<SavedEntry>,
}

pub struct SavedEntry {
    pub thing: SavedThing,
    /// The `entries` index of the organization this member belongs to, and its slot.
    pub organization: Option<(usize, i32)>,
}

impl SavedThings {
    /// Every record of `inventory`, walked.
    pub fn read(inventory: &WorldInventory) -> Self {
        let version = inventory.world.version;
        let stream = u32::from(inventory.is_saved_game());
        let mut minds = Minds;
        let mut entries = Vec::new();
        let walk = |p: aa_mapworld::Placement, minds: &mut Minds| -> SavedThing {
            let backup = p.clone();
            saved_world::walk(p, version, stream, minds).unwrap_or_else(|e| SavedThing {
                record: backup,
                tail: None,
                components: Vec::new(),
                organization: None,
                mind: None,
                properties: Vec::new(),
                element_history: Vec::new(),
                queue: Vec::new(),
                denizen_commands: Vec::new(),
                aircraft: None,
                group_members: Vec::new(),
                sightings: Vec::new(),
                updatables: Vec::new(),
                partial: Some(e.to_string()),
                consumed: 0,
            })
        };
        for p in &inventory.dynamic_things.placements {
            entries.push(SavedEntry { thing: walk(p.clone(), &mut minds), organization: None });
        }
        for u in &inventory.dynamic_things.unsupported {
            if let Ok(p) = aa_mapworld::inventory::decode_group_record(&u.raw, u.record_ordinal) {
                entries.push(SavedEntry { thing: walk(p, &mut minds), organization: None });
            }
        }
        // Members after everything else: a top-level record keeps its placement index.
        for i in 0..entries.len() {
            let members = entries[i].thing.organization.as_mut().map(|o| std::mem::take(&mut o.members)).unwrap_or_default();
            for (slot, thing) in members {
                entries.push(SavedEntry { thing, organization: Some((i, slot)) });
            }
        }
        Self { entries }
    }
}

/// What the world's map holds for a saved game beyond a map's: the extra placed
/// things (groups, members) and the Updatable owners, for `map.rs`.
pub(crate) fn extra_placed(entry: &SavedEntry) -> crate::map::PlacedThing {
    let p = &entry.thing.record;
    let position = p.thing.position_bits.map(f32::from_bits);
    let facing = p.thing.facing_bits.map(f32::from_bits);
    let yaw = if facing[0] != 0.0 || facing[1] != 0.0 { facing[1].atan2(facing[0]) } else { -std::f32::consts::FRAC_PI_2 };
    let scale = p.object.as_ref().map(|o| f32::from_bits(o.scale_multiplier_bits)).filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0);
    crate::map::PlacedThing {
        thing_id: String::from_utf16_lossy(&p.thing_id_utf16),
        position,
        yaw,
        owner_slot: None,
        facing,
        scale,
        variant: p.object.as_ref().and_then(|o| o.variant_index).unwrap_or(-1),
        serialized_id: p.serialized_id,
        properties: Vec::new(),
        class_code: p.class_code,
        breakable: None,
        load_fails: false,
        saved_city: false,
        organization: None,
        member_of: None,
        components: Vec::new(),
        current_component: None,
        mind: None,
    }
}

/// What `crate::movement::restore_saved_minds` reads of a saved thing: its Mind, its
/// other components (the Mover, the Character) and, of an organization, the words its
/// Move state goes by.
pub(crate) fn saved_mind_parts(placed: &mut crate::map::PlacedThing, index: usize, things: &SavedThings) {
    let thing = &things.entries[index].thing;
    placed.mind = thing.mind.clone();
    placed.components = thing.components.clone();
    placed.current_component = thing.tail.as_ref().map(|t| t.current_component);
    if let Some(o) = &thing.organization {
        placed.organization = Some(crate::map::PlacedOrganization {
            members: things.entries.iter().enumerate().filter_map(|(m, e)| e.organization.filter(|(of, _)| *of == index).map(|(_, slot)| (slot, m))).collect(),
            leader: o.leader,
            morale_bits: o.morale_bits,
            formation: o.formation,
            position_bits: o.position_bits,
            offsets: o.offsets.clone(),
        });
    }
}

/// A restored world and what it does not hold.
pub struct Restored {
    pub world: World,
    pub notes: Vec<String>,
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

/// The map of a saved game: `GameMap::load_file` on its own bytes reads its terrain
/// and things, and the map id is what the caller calls it.
pub fn load_map(map_id: &str, bytes: Vec<u8>, data: &GameData, name: Option<String>) -> anyhow::Result<GameMap> {
    GameMap::load_file(map_id, bytes, data, name)
}

/// The actor state word's bits used here (the 0x477FC0 names, in order).
const STATE_OUT_OF_SUPPLY: u32 = 1 << 19;
const STATE_UNDER_SIEGE: u32 = 1 << 20;
const STATE_CONSTRUCTION: u32 = 1 << 21;
const STATE_DEAD: u32 = 1 << 23;
const STATE_FALLING: u32 = 1 << 24;
const STATE_MIND_EXHAUSTED: u32 = 1 << 5;
const STATE_ENTRENCHED: u32 = 1 << 22;

/// A regiment Mind's embedded states (`aa_mind::regiment_load::STATE_LIST`).
const CAI_SLEEP: u32 = 0x34;
const CAI_RAMPAGE: u32 = 0x54;
const CAI_MOVE: u32 = 0x68;
const CAI_KILL: u32 = 0xc0;
const CAI_CHASING: u32 = 0x120;
const CAI_REFORM: u32 = 0x1b8;
const CAI_ENGAGE: u32 = 0x1cc;
const CAI_ROUT: u32 = 0x20c;
const CAI_EXHAUSTED: u32 = 0x270;
const CAI_UNPACK: u32 = 0x280;

/// Build the world a saved game holds. `controller` says who plays a kingdom, by its
/// map slot id; a film has no computer player attached, and a kingdom's name is its
/// saved text.
pub fn restore(
    bytes: &[u8],
    data: Arc<GameData>,
    generals: Arc<GeneralData>,
    map: Arc<GameMap>,
    controller: &dyn Fn(&str) -> Controller,
) -> anyhow::Result<Restored> {
    let mut notes = Vec::new();
    let archive = MapArchive::from_bytes(bytes.to_vec())?;
    let inventory = WorldInventory::decode(&archive, 1 << 30)?;
    if !inventory.is_saved_game() {
        anyhow::bail!("not a saved game");
    }
    let kingdoms = inventory
        .kingdoms
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("KINGDOMS: {}", inventory.kingdoms_error.clone().unwrap_or_default()))?;
    let things = SavedThings::read(&inventory);
    let mut world = World::empty(data.clone(), map.clone(), 0);
    let stream = world.native_rng;
    // The GWORLD chunk's clock (0x4FC6E0..): tick, begin, end, step.
    let w = &inventory.world;
    world.clock.tick = w.word_dc;
    world.clock.begin = aa_simulation::FloatBits(w.begin_time_bits);
    world.clock.end = aa_simulation::FloatBits(w.word_e4);
    world.clock.step = aa_simulation::FloatBits(w.delta_bits);
    world.tick = u64::from(w.word_dc);
    world.time = f32::from_bits(w.word_e4);
    crate::scheduler::sync_clock(&mut world);

    // The kingdoms: every saved one, the majors first as a battle has them.
    let records: Vec<&KingdomRecord> = kingdoms.slots.iter().filter_map(|s| s.record.as_ref()).collect();
    let mut order: Vec<&KingdomRecord> = records.iter().copied().filter(|r| r.payload.word_190 == 0).collect();
    let players = order.len();
    order.extend(records.iter().copied().filter(|r| r.payload.word_190 != 0));
    let mut slot_kingdom: Vec<Option<KingdomId>> = vec![None; map.slots.len()];
    let resources = data.resources.len();
    for (k, record) in order.iter().enumerate() {
        let k = k as KingdomId;
        let id = utf16(&record.thing.name_utf16);
        let Some(slot) = map.slots.iter().position(|s| s.id.eq_ignore_ascii_case(&id)) else {
            anyhow::bail!("kingdom {id} has no slot in the saved header");
        };
        slot_kingdom[slot] = Some(k);
        let independent = record.payload.word_190 != 0;
        let nation_raw = record.payload.nation_reference_utf16.as_deref().map(utf16).unwrap_or_default();
        let nation = nation_raw.trim_start_matches("nation_").to_ascii_lowercase();
        if !independent && !nation.is_empty() && data.nation(&nation).is_none() {
            anyhow::bail!("kingdom {id}: unknown nation {nation_raw:?}");
        }
        let color_id = utf16(&record.payload.color_reference_utf16);
        let color = data
            .kingdom_colors
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(&color_id))
            .map_or([1.0, 1.0, 1.0], |c| c.1)
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        let team = if independent {
            crate::setup::INDEPENDENT_TEAM | slot as u32
        } else {
            let saved = record.payload.team_handle.unwrap_or(0);
            map.team_ids.iter().position(|t| *t == saved).map_or(map.teams.len() as u32 + k, |t| t as u32)
        };
        world.kingdoms.push(Kingdom {
            id: k,
            name: record.payload.display_text_utf16.as_deref().map_or_else(|| utf16(&record.payload.display_key_utf16), utf16),
            nation,
            team,
            color,
            controller: if independent { Controller::Independent } else { controller(&id) },
            stockpile: [0.0; 3],
            income: [0.0; 3],
            unit_limit: 0.0,
            units_used: 0.0,
            // 4E4930: a kingdom out of the game stays in the list.
            defeated: record.payload.bytes_23b_23c.is_some_and(|b| b[0] != 0),
            independent,
        });
        let kb = world.books.kingdom_mut(k, resources);
        for r in &record.payload.resources_1c4.values {
            let name = utf16(&r.reference_utf16);
            if let Some(i) = data.resource_index(&name) {
                kb.stockpiled[i] = f32::from_bits(r.word);
            } else {
                notes.push(format!("kingdom {id}: stockpile of unknown resource {name}"));
            }
        }
        if let Some(properties) = &record.payload.properties {
            for p in &properties.values {
                let name = utf16(&p.reference_utf16);
                if data.property(&name).is_some() {
                    kb.technologies.push(name);
                }
            }
        }
    }
    world.books.relations = crate::setup::relations(&world, &map, &slot_kingdom, players);
    for (s, k) in slot_kingdom.iter().enumerate() {
        if let Some(k) = k {
            world.ids.bind_kingdom(*k, s);
        }
    }

    // The things.
    let owner_of = |thing: &SavedThing| -> Option<usize> {
        let handle = thing.record.actor.as_ref()?.owner_handle;
        if handle == 0 {
            return None;
        }
        map.slots.iter().position(|s| s.serialized_id == Some(handle))
    };
    let mut kept: BTreeMap<usize, Kept> = BTreeMap::new();
    let mut scenery = 0usize;
    let (mut projectiles, mut aircraft, mut trailers, mut dead, mut unknown, mut partial) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let trailer_defs: std::collections::BTreeSet<DefId> = data.defs.iter().filter_map(|d| d.trailer).collect();
    for index in 0..things.entries.len() {
        let entry = &things.entries[index];
        let thing = &entry.thing;
        if thing.partial.is_some() {
            partial += 1;
        }
        if entry.organization.is_some() {
            continue; // its organization's turn below
        }
        let id = utf16(&thing.record.thing_id_utf16);
        let Some(def) = data.find(&id) else {
            unknown += 1;
            continue;
        };
        let d = data.def(def);
        if thing.record.class_code == 8 {
            continue; // a GGroup: see the notes
        }
        if thing.record.actor.is_none() || !d.actor_data {
            world.books.scenery.push(index);
            scenery += 1;
            continue;
        }
        if trailer_defs.contains(&def) {
            trailers += 1;
            continue;
        }
        if d.category.eq_ignore_ascii_case("PROJECTILE") || thing.partial.as_deref().is_some_and(|p| p.contains("component 1:")) {
            projectiles += 1;
            continue;
        }
        if thing.aircraft.is_some() {
            aircraft += 1;
            continue;
        }
        let state = thing.record.actor.as_ref().map_or(0, |a| a.state_low);
        if state & (STATE_DEAD | STATE_FALLING) != 0 {
            dead += 1;
            continue;
        }
        let slot = owner_of(thing);
        let kingdom = slot.and_then(|s| slot_kingdom[s]);
        if slot.is_none_or(|s| map.slots[s].independent) && !crate::setup::neutral_actor(&data, def) {
            world.books.scenery.push(index);
            scenery += 1;
            continue;
        }
        let position = thing.record.thing.position_bits.map(f32::from_bits);
        let at = glam::Vec2::new(position[0], position[1]);
        let yaw = map.placed[index].yaw;
        if let Some(org) = &thing.organization {
            let Some(k) = kingdom else {
                notes.push(format!("organization {id} without a kingdom"));
                continue;
            };
            world.ids.request_placed(index);
            world.ids.restore_scope = true;
            let o = crate::production::new_organization(&mut world, def, k, at, yaw, None);
            kept.insert(index, Kept::Org(o));
            world.orgs[o as usize].anchor = at;
            let member_entries: Vec<usize> = (0..things.entries.len()).filter(|&m| things.entries[m].organization.is_some_and(|(of, _)| of == index)).collect();
            for m in member_entries {
                let (_, member_slot) = things.entries[m].organization.unwrap_or((0, 0));
                let member = &things.entries[m].thing;
                let member_id = utf16(&member.record.thing_id_utf16);
                let Some(member_def) = data.find(&member_id) else {
                    unknown += 1;
                    continue;
                };
                let member_state = member.record.actor.as_ref().map_or(0, |a| a.state_low);
                if member_state & (STATE_DEAD | STATE_FALLING) != 0 {
                    dead += 1;
                    continue;
                }
                world.ids.request_placed(m);
                let pos = member.record.thing.position_bits.map(f32::from_bits);
                let a = crate::production::spawn_single(&mut world, member_def, Some(k), glam::Vec2::new(pos[0], pos[1]), map.placed[m].yaw);
                world.ids.clear_request(m);
                apply_actor(&mut world, a, member, m, &mut notes);
                world.actors[a as usize].org = Some(o);
                world.actors[a as usize].slot = Some(member_slot as u16);
                world.orgs[o as usize].members.push(a);
                kept.insert(m, Kept::Actor(a));
                restore_queue(&mut world, a, member, &mut notes);
            }
            world.ids.restore_scope = false;
            world.ids.clear_request(index);
            world.books.org_mut(o).saved_techs = Some(world.books.kingdom(k).map_or(0, |b| b.technologies.len()));
            for (slot, component) in &thing.components {
                if *slot == 17
                    && let SavedComponent::Experience { word_08, .. } = component
                {
                    world.orgs[o as usize].experience = f32::from_bits(*word_08);
                }
            }
            let slots = d.slots.len();
            world.books.org_mut(o).progress = vec![0.0; slots];
        } else {
            world.ids.request_placed(index);
            world.ids.restore_scope = true;
            let a = crate::production::spawn_single(&mut world, def, kingdom, at, yaw);
            world.ids.restore_scope = false;
            world.ids.clear_request(index);
            apply_actor(&mut world, a, thing, index, &mut notes);
            kept.insert(index, Kept::Actor(a));
            restore_queue(&mut world, a, thing, &mut notes);
        }
    }

    // The zones as the things stand (what this admits to the cities is the save's to
    // say: the lists below replace its registrations), then the record pass's draws on
    // the stream the GWORLD chunk holds.
    crate::supply::tick(&mut world);
    saved_component_values(&mut world, &things, &kept);
    world.native_rng = stream;
    record_pass_draws(&mut world, &things, &kept);

    // The queue Load 0x50B8B0 and 0x50BFF0: every list as it was saved. An entry whose
    // thing did not become an actor stays on its list, delivered without effect.
    if let Some(saved) = &map.saved_queues {
        restore_lists(&mut world, saved, &[0, 1, 2], aa_world::queues::Kind::Periodic, &kept, &slot_kingdom, &things);
    }
    if let Some(saved) = &map.saved_deferred {
        restore_lists(&mut world, saved, &[DEFERRED, THINK], aa_world::queues::Kind::Deferred, &kept, &slot_kingdom, &things);
    }
    crate::scheduler::sync_clock(&mut world);

    crate::special_ops::attach(&mut world, generals);
    let mut order_index = 0u32;
    for record in &order {
        let k = order_index;
        order_index += 1;
        let Some(commander) = &record.payload.commander else { continue };
        let Some(general) = commander.reference_utf16.as_deref().map(utf16).filter(|g| !g.is_empty()) else { continue };
        crate::special_ops::set_general(&mut world, k, Some(&general));
        if let Some(state) = &commander.state {
            crate::special_ops::restore_commander(&mut world, k, f32::from_bits(state.word_0c), state.abilities.values.iter().map(|a| f32::from_bits(a.word_10)).collect());
        }
    }

    // The economy as it stands; 0x4F1E10's statistics start over (see the notes).
    crate::statistics::start_world(&mut world);
    crate::economy::refresh(&mut world);
    for k in 0..world.kingdoms.len() as u32 {
        crate::economy::publish(&mut world, k);
    }
    let resumed = apply_saved_minds(&mut world, &things, &kept);
    world.begin_battle();

    if resumed > 0 {
        notes.push(format!(
            "a rendition: {resumed} Minds on a state aa-play does not host (a kill, a guard, a retreat, an explore) had their saved commands given again as new orders"
        ));
    }
    notes.push(format!(
        "a saved world, restored in part at tick {}: {} kingdoms, {} actors, {} organizations, {} scenery, {} list entries kept",
        w.word_dc,
        world.kingdoms.len(),
        world.actors.len(),
        world.orgs.len(),
        scenery,
        kept.len()
    ));
    for (count, what) in [
        (aircraft, "aircraft in flight (their Minds, 51C3C0, are not ported)"),
        (projectiles, "projectiles in flight"),
        (trailers, "trailers (aa-play draws none)"),
        (dead, "corpses"),
        (unknown, "things of definitions this data lacks"),
        (partial, "records read in part"),
    ] {
        if count > 0 {
            notes.push(format!("not restored: {count} {what}"));
        }
    }
    notes.push(
        "not restored, aa-play has no place for them yet: a Mobile Deployment under way, fog memories (GObject's sightings), \
         kingdom statistics, the AI's state, recruit lists, GGroups, trigger state, denizen groups in flight"
            .to_owned(),
    );
    Ok(Restored { world, notes })
}

/// The Minds' Loads over what the record pass made (`crate::movement::restore_saved_minds`:
/// 0x536340 and 0x528C20 with the Mover and the Character), then what `crate::combat`
/// holds of a regiment's Mind, by the state on top of its saved stack: the engage list
/// (+1D8's records through 51FE60), the rout (+21C), the reform (+1C8), the exhausted
/// state, and the order a Move state walks. No draws.
///
/// A rendition, said so in the notes: a state `aa-play` does not host as a Mind (a kill,
/// a guard, a retreat, an explore) is entered by giving its saved command again as a new
/// order. Natively nothing is ordered here; the stream stays the load's. Returns how many.
fn apply_saved_minds(world: &mut World, things: &SavedThings, kept: &BTreeMap<usize, Kept>) -> u32 {
    use aa_mind::saved_load::{Saved, decode};
    use crate::world::{Order, Unit};
    crate::movement::restore_saved_minds(world, kept);
    let stream = world.native_rng;
    let now = world.time;
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let mut again = Vec::new();
    for (&index, &k) in kept {
        let entry = &things.entries[index];
        if entry.organization.is_some() {
            continue;
        }
        let Some(bytes) = entry.thing.mind.as_deref() else { continue };
        match k {
            Kept::Org(o) => {
                let Ok((Saved::Regiment(r), _)) = decode(Kind::Regiment, bytes) else { continue };
                let Some(base) = r.base.as_ref() else { continue };
                let state = entry.thing.record.actor.as_ref().map_or(0, |a| a.state_low);
                let i = o as usize;
                st.orgs[i].exhausted = state & STATE_MIND_EXHAUSTED != 0;
                st.orgs[i].entrenched = state & STATE_ENTRENCHED != 0;
                for &[priority, gtid] in r.engage_records.iter().flatten() {
                    let target = match world.ids.thing(gtid) {
                        Some(crate::ids::Thing::Actor(a)) => crate::combat::TargetRef::Actor(a),
                        Some(crate::ids::Thing::Org(t)) => crate::combat::TargetRef::Org(t),
                        _ => continue,
                    };
                    crate::combat::insert_engage(&mut st, o, target, priority as i32);
                }
                let word = |offset: u32| {
                    r.stores.iter().rev().find_map(|s| match *s {
                        aa_mind::saved_load::Store::Word { offset: of, value } if of == offset => Some(value),
                        _ => None,
                    })
                };
                let walks = saved_walk(base);
                let top = base.stack.as_ref().and_then(|s| s.first().copied());
                // The Mind holds the fighting states on the ports (`movement::regiment::fight`)
                // unless its stack has a state that it does not.
                let on_mind = crate::movement::fight_state(world, o).is_some();
                let named = |offset: u32| match word(offset).and_then(|g| world.ids.thing(g)) {
                    Some(crate::ids::Thing::Actor(a)) => Some(crate::combat::TargetRef::Actor(a)),
                    Some(crate::ids::Thing::Org(t)) => Some(crate::combat::TargetRef::Org(t)),
                    _ => None,
                };
                match top {
                    Some(CAI_KILL) if on_mind => {
                        if let Some(t) = named(CAI_KILL + 0x10)
                            && let Some(rep) = crate::combat::representative(world, t)
                        {
                            let order = Order::Attack { target: rep };
                            st.orgs[i].explicit = Some(t);
                            st.orgs[i].seen_order = Some(order.clone());
                            crate::movement::restored_order(world, Unit::Org(o), order);
                        }
                    }
                    Some(CAI_CHASING) if on_mind => {
                        if let Some(rep) = named(CAI_CHASING + 0x10).and_then(|t| crate::combat::representative(world, t)) {
                            let order = Order::Attack { target: rep };
                            st.orgs[i].saved_order = Some(walks.unwrap_or(Order::Idle));
                            st.orgs[i].auto_order = Some(order.clone());
                            st.orgs[i].seen_order = Some(order.clone());
                            crate::movement::restored_order(world, Unit::Org(o), order);
                        }
                    }
                    Some(CAI_REFORM) if on_mind => {}
                    Some(CAI_ENGAGE) => {
                        if let Some(rep) = st.orgs[i].current_target().and_then(|t| crate::combat::representative(world, t)) {
                            let order = Order::Attack { target: rep };
                            st.orgs[i].saved_order = Some(walks.unwrap_or(Order::Idle));
                            st.orgs[i].auto_order = Some(order.clone());
                            st.orgs[i].seen_order = Some(order.clone());
                            crate::movement::restored_order(world, Unit::Org(o), order);
                        }
                    }
                    Some(CAI_ROUT) => {
                        let since = word(CAI_ROUT + 0x10).map_or(now, f32::from_bits);
                        st.orgs[i].routing = Some(crate::combat::Rout { since, destination: None, next_think: now });
                    }
                    Some(CAI_REFORM) => {
                        let since = word(CAI_REFORM + 0x10).map_or(now, f32::from_bits);
                        st.orgs[i].reform = Some(crate::combat::Reform { since, next_think: now });
                    }
                    Some(CAI_EXHAUSTED) => st.orgs[i].exhausted_state = true,
                    Some(CAI_MOVE) => {
                        if let Some(order) = walks {
                            st.orgs[i].seen_order = Some(order.clone());
                            crate::movement::restored_order(world, Unit::Org(o), order);
                        }
                    }
                    Some(CAI_SLEEP | CAI_RAMPAGE | CAI_UNPACK) | None => {}
                    Some(_) => again.push(index),
                }
            }
            Kept::Actor(a) => {
                if !data_has_element_mind(world, a) {
                    continue;
                }
                let Ok((Saved::Element(e), _)) = decode(Kind::Element, bytes) else { continue };
                let Some(base) = e.base.as_ref() else { continue };
                use aa_mind::uai_thinks::element as offsets;
                match base.stack.as_ref().and_then(|s| s.first().copied()) {
                    Some(offsets::MOVE) => {
                        if let Some(order) = saved_walk(base) {
                            crate::movement::restored_order(world, Unit::Actor(a), order);
                        }
                    }
                    Some(offsets::SLEEP | offsets::DEPLOY | offsets::UNPACK | offsets::REATTACH) | None => {}
                    Some(offsets::ENGAGE | offsets::KILL) if world.actors[a as usize].org.is_some() => {}
                    Some(_) => again.push(index),
                }
            }
        }
    }
    world.combat = st;
    let mut resumed = 0;
    for index in again {
        let entry = &things.entries[index];
        let Some(bytes) = entry.thing.mind.as_deref() else { continue };
        let (kind, unit, kingdom) = match kept[&index] {
            Kept::Org(o) => (Kind::Regiment, Unit::Org(o), world.orgs[o as usize].kingdom),
            Kept::Actor(a) => match world.actors[a as usize].kingdom {
                Some(kingdom) => (Kind::Element, Unit::Actor(a), kingdom),
                None => continue,
            },
        };
        let Ok((saved, _)) = decode(kind, bytes) else { continue };
        let Some(base) = saved.base() else { continue };
        let Some(c) = base.current_command.as_ref().or_else(|| base.commands.first()) else { continue };
        let name = c.definition.as_deref().map_or_else(String::new, utf16).to_ascii_lowercase();
        let at = glam::Vec2::new(f32::from_bits(c.position_bits[0]), f32::from_bits(c.position_bits[1]));
        let target = (c.flags & 1 != 0).then(|| world.ids.thing(c.word10)).flatten();
        let target_actor = match target {
            Some(crate::ids::Thing::Actor(a)) => Some(a),
            Some(crate::ids::Thing::Org(o)) => world.orgs[o as usize].members.first().copied(),
            _ => None,
        };
        let target_unit = match target {
            Some(crate::ids::Thing::Actor(a)) => Some(Unit::Actor(a)),
            Some(crate::ids::Thing::Org(o)) => Some(Unit::Org(o)),
            _ => None,
        };
        let order = |order| crate::world::Command::Order { kingdom, units: vec![unit], order };
        let command = match (name.as_str(), target_actor) {
            ("kill", Some(target)) => crate::world::Command::Attack { kingdom, units: vec![unit], target },
            // 0x527E30 -> 0x51D920: a move on a hostile target is the Kill state.
            ("move", Some(target)) => {
                world.apply(crate::world::Command::Order { kingdom, units: vec![unit], order: crate::orders::ActorOrder::MoveOnto });
                crate::world::Command::Attack { kingdom, units: vec![unit], target }
            }
            ("kill", None) if c.flags & 4 != 0 => crate::world::Command::Move { kingdom, units: vec![unit], to: at, attack_move: true },
            ("guard", _) => order(crate::orders::ActorOrder::Guard { target: target_unit, at, aux: None }),
            ("retreat", _) if c.flags & 4 != 0 => order(crate::orders::ActorOrder::Retreat { target: target_actor, at }),
            ("explore", _) if c.flags & 4 != 0 => order(crate::orders::ActorOrder::Explore { at, scalar: None }),
            _ => continue,
        };
        world.apply(command);
        resumed += 1;
    }
    world.native_rng = stream;
    resumed
}

/// The walk a saved Mind's command asks for: a `move` or a `kill` at a point.
fn saved_walk(base: &aa_mind::saved_load::Base) -> Option<crate::world::Order> {
    let c = base.current_command.as_ref().or_else(|| base.commands.first())?;
    if c.flags & 4 == 0 {
        return None;
    }
    let name = c.definition.as_deref().map_or_else(String::new, utf16).to_ascii_lowercase();
    let to = glam::Vec2::new(f32::from_bits(c.position_bits[0]), f32::from_bits(c.position_bits[1]));
    match name.as_str() {
        "move" => Some(crate::world::Order::Move { to, attack_move: false }),
        "kill" => Some(crate::world::Order::Move { to, attack_move: true }),
        _ => None,
    }
}

fn data_has_element_mind(world: &World, a: u32) -> bool {
    let d = world.data.def(world.actors[a as usize].def);
    d.components & (1 << 8) != 0 && d.components & (1 << 7) == 0 && d.components & (1 << 23) == 0
}

/// What the saved components hold beyond the thing's own words, once the state they
/// are kept in exists: the morale and the health are saved as fractions of their maximum
/// (59E300, 561D10), which the organization's level and the kingdom's properties decide;
/// the Supply's stock as a fraction of its stockpile (58B0B0).
fn saved_component_values(world: &mut World, things: &SavedThings, kept: &BTreeMap<usize, Kept>) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    // The organization's Load 57D130 reads its morale before the InitExisting 0x4D7230
    // applies the kingdom's properties to its members (0x4D1350), and +60 stands until
    // a change of modifiers refreshes it: on the traces the technologies' MORALE is in
    // it once, from the properties the organization itself holds, not its members',
    // unless a level above the first has refreshed them (the Experience's Load):
    // 45 + 1 at level 0, 45 + 10 * 1.5 at level 2.
    let techs: Vec<Vec<String>> = (0..world.kingdoms.len() as u32).map(|k| world.books.kingdom(k).map_or_else(Vec::new, |b| b.technologies.clone())).collect();
    let resources = world.data.resources.len();
    for k in 0..world.kingdoms.len() as u32 {
        world.books.kingdom_mut(k, resources).technologies.clear();
    }
    let clamp = world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0);
    for (&index, k) in kept {
        let Kept::Org(o) = *k else { continue };
        let own: f32 = things.entries[index]
            .thing
            .properties
            .iter()
            .filter(|p| p.state.byte_10.unwrap_or(0) == 0)
            .filter_map(|p| world.data.property(&p.id()))
            .flat_map(|p| p.modifiers.iter())
            .filter(|m| m.kind.eq_ignore_ascii_case("ADD") && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("MORALE")))
            .map(|m| m.amount)
            .sum();
        let sum = if st.orgs[o as usize].level == 0 { crate::morale::morale_maximum(world, &st, o) } else { st.orgs[o as usize].morale_max };
        st.orgs[o as usize].morale_max = crate::zones::clamp_add(sum, own, clamp);
        st.orgs[o as usize].saved_morale_living = Some(st.orgs[o as usize].living);
    }
    for (k, list) in techs.into_iter().enumerate() {
        world.books.kingdom_mut(k as u32, resources).technologies = list;
    }
    for (&index, k) in kept {
        let thing = &things.entries[index].thing;
        match *k {
            Kept::Org(o) => {
                let i = o as usize;
                if let Some(org) = &thing.organization {
                    world.orgs[i].morale = st.orgs[i].morale_max * f32::from_bits(org.morale_bits);
                    if let Some(w) = org.word_84 {
                        st.orgs[i].attack_efficiency = f32::from_bits(w);
                    }
                    if let Some(w) = org.word_88 {
                        st.orgs[i].attack_efficiency_desired = f32::from_bits(w);
                    }
                }
            }
            Kept::Actor(a) => {
                let i = a as usize;
                for (slot, component) in &thing.components {
                    match component {
                        SavedComponent::Body(body) if *slot == 0 => {
                            // The Load's fraction of the definition's maximum, then what
                            // 0x561D40 does when the maximum changes: old / max * new, in
                            // 53 bits.
                            let base = world.data.def(world.actors[i].def).health;
                            let max = st.actors[i].max_health;
                            let loaded = (f64::from(f32::from_bits(body.health_multiplier_bits)) * f64::from(base)) as f32;
                            world.actors[i].health = if max == base { loaded } else { (f64::from(loaded) / f64::from(base) * f64::from(max)) as f32 };
                        }
                        SavedComponent::Supply(supply) => {
                            let max = world.data.def(world.actors[i].def).supply_stockpile_max;
                            if let Some(w) = supply.amount_multiplier_bits {
                                world.books.actor_mut(a).supply_stock = max * f32::from_bits(w);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    world.combat = st;
}

/// The draws of the world load's record pass (4FAE80 over DYNAMIC_THINGS), in the order
/// of the records, an organization's embedded members (57D130) inside its own Load.
/// Each record's are [`crate::creation::saved_record`], then its publish 4FB514 ->
/// 4F9510 -> 4CFEA0 walks the zones of its tile: a city loaded before it, whose zone 8
/// covers it, admits it (0x564300) and, on no queue yet (they load after the records),
/// registers on queue 1 with one draw.
fn record_pass_draws(world: &mut World, things: &SavedThings, kept: &BTreeMap<usize, Kept>) {
    use crate::books::CityCandidateRef as Ref;
    let data = world.data.clone();
    let cities: Vec<(usize, u32)> = kept
        .iter()
        .filter_map(|(&index, k)| match k {
            Kept::Actor(a) if data.def(world.actors[*a as usize].def).city => Some((index, *a)),
            _ => None,
        })
        .collect();
    let mut registered = std::collections::BTreeSet::new();
    let mut publish = |world: &mut World, at_index: usize, record: usize| {
        let who = match kept.get(&record) {
            Some(Kept::Actor(a)) => Ref::Actor(*a),
            Some(Kept::Org(o)) => Ref::Org(*o),
            None => return,
        };
        if !crate::supply::admitted_at_load(world, who) {
            return;
        }
        let position = things.entries[record].thing.record.thing.position_bits.map(f32::from_bits);
        let xy = glam::Vec2::new(position[0], position[1]);
        for &(index, city) in &cities {
            if index >= at_index || registered.contains(&city) || who == Ref::Actor(city) {
                continue;
            }
            if crate::fog::engine(world).is_some_and(|e| e.city_zone_covers(world, xy, city)) {
                registered.insert(city);
                world.random_u32_at(crate::trace::sites::CITY_ADMISSION);
            }
        }
    };
    for (index, entry) in things.entries.iter().enumerate() {
        if entry.organization.is_some() {
            continue;
        }
        for (m, member) in things.entries.iter().enumerate() {
            if member.organization.is_some_and(|(of, _)| of == index)
                && let Some(def) = data.find(&utf16(&member.thing.record.thing_id_utf16))
            {
                crate::creation::saved_record(world, def);
                publish(world, index, m);
            }
        }
        if let Some(def) = data.find(&utf16(&entry.thing.record.thing_id_utf16)) {
            crate::creation::saved_record(world, def);
            publish(world, index, index);
        }
    }
}

/// A thing's saved words on its actor: where it stands, its health and states.
fn apply_actor(world: &mut World, a: u32, thing: &SavedThing, _index: usize, _notes: &mut Vec<String>) {
    let position = thing.record.thing.position_bits.map(f32::from_bits);
    let def = world.actors[a as usize].def;
    let kingdom = world.actors[a as usize].kingdom;
    let max = crate::production::max_health(world, kingdom, def);
    // A thing's height after the load is the ground under it, whatever word the save holds
    // (it lags, 0x4EEB40): on the traces of two films, every thing that stands.
    let z = world.map.height_at(glam::Vec2::new(position[0], position[1]));
    let actor = &mut world.actors[a as usize];
    actor.pos = glam::Vec3::new(position[0], position[1], z);
    let state = thing.record.actor.as_ref().map_or(0, |p| p.state_low);
    for (slot, component) in &thing.components {
        if *slot == 0
            && let SavedComponent::Body(body) = component
        {
            // 0x561D10: the saved multiplier of the maximum (level and all: see
            // `saved_component_values`).
            actor.health = max * f32::from_bits(body.health_multiplier_bits);
        }
    }
    // The saved list also holds what the kingdom's properties gave the thing (0x4E3AD0
    // -> 0x4CEF10), which aa-play applies from the kingdom's own list.
    let of_kingdom: Vec<String> = kingdom.and_then(|k| world.books.kingdom(k)).map_or_else(Vec::new, |b| b.technologies.clone());
    let books = world.books.actor_mut(a);
    books.saved_techs = Some(of_kingdom.len());
    books.saved_siege = state & STATE_UNDER_SIEGE != 0;
    books.out_of_supply = state & STATE_OUT_OF_SUPPLY != 0;
    books.constructing = state & STATE_CONSTRUCTION != 0;
    books.properties = thing
        .properties
        .iter()
        .filter(|p| p.state.byte_10.unwrap_or(0) == 0)
        .map(|p| p.id())
        .filter(|id| !of_kingdom.iter().any(|t| t.eq_ignore_ascii_case(id)))
        .collect();
}

/// The Structure's queue (58A470) and its words: +20 the deadline, +2C the duration,
/// +24 when it paused, +28 why.
fn restore_queue(world: &mut World, a: u32, thing: &SavedThing, notes: &mut Vec<String>) {
    let structure = thing.components.iter().find_map(|(_, c)| if let SavedComponent::Structure(s) = c { Some(s) } else { None });
    let Some(structure) = structure else { return };
    let resources = world.data.resources.len();
    let words = |v: &[u32]| -> Vec<f32> {
        let mut out: Vec<f32> = v.iter().map(|b| f32::from_bits(*b)).collect();
        out.resize(resources, 0.0);
        out
    };
    let data = world.data.clone();
    let own_def = world.actors[a as usize].def;
    let mut items = Vec::new();
    for item in &thing.queue {
        let def = match &item.actor {
            Some(id) => match data.find(id) {
                Some(d) => d,
                None => {
                    notes.push(format!("queue item of unknown definition {id}"));
                    continue;
                }
            },
            None => own_def,
        };
        let id = item.id.unwrap_or(world.books.next_item);
        world.books.next_item = world.books.next_item.max(id.saturating_add(1));
        items.push(crate::books::QueueItem {
            id,
            def,
            paid: words(&item.paid),
            requested_upkeep: words(&item.requested),
            current_upkeep: words(&item.current),
            attaches: false,
            research: item.research.clone(),
            organization_slot: 0,
            elements: None,
            location: item.location.map(|l| l.map(f32::from_bits)),
        });
    }
    let books = world.books.actor_mut(a);
    if !items.is_empty() {
        books.queue.items = items;
        books.queue.scheduled = true;
    }
    books.queue.deadline = structure.word_20.map_or(0.0, f32::from_bits);
    books.queue.duration = structure.word_2c.map_or(0.0, f32::from_bits);
    // 0x7F7FFFFF and -1 stand for not paused in the words these come from.
    let paused = structure.word_24.map(f32::from_bits).filter(|t| *t >= 0.0);
    books.queue.paused_at = paused;
    books.queue.pause_reason = match structure.word_28.map(|w| w as i32) {
        Some(0) if paused.is_some() => Some(crate::books::PauseReason::LimitedResource),
        Some(1) if paused.is_some() => Some(crate::books::PauseReason::Supply),
        Some(2) if paused.is_some() => Some(crate::books::PauseReason::Disabled),
        _ => None,
    };
}

/// A saved list's owner on this world: the Updatable of the component it names.
fn owner_of_entry(world: &World, owner: SavedOwner, kept: &BTreeMap<usize, Kept>, slot_kingdom: &[Option<KingdomId>]) -> Option<Owner> {
    match owner {
        SavedOwner::Placed { index, slot } => match kept.get(&index) {
            Some(Kept::Actor(a)) => Some(match slot {
                3 => Owner::Mover(*a),
                4 => Owner::Mind(*a),
                13 => Owner::Structure(*a),
                _ => Owner::Component(*a, slot),
            }),
            Some(Kept::Org(o)) => Some(match slot {
                4 => Owner::OrgMind(*o),
                _ => Owner::OrgComponent(*o, slot),
            }),
            None => world.ids.placed(index).is_some().then_some(Owner::Placed(index as u32, slot)),
        },
        SavedOwner::Doodad { index, slot } => world.ids.placed(index).is_some().then_some(Owner::Doodad(index as u32, slot)),
        SavedOwner::Kingdom { slot } => slot_kingdom.get(slot).copied().flatten().map(Owner::Kingdom),
        SavedOwner::KingdomComponent { slot, component } => slot_kingdom.get(slot).copied().flatten().map(|k| Owner::KingdomComponent(k, component)),
        SavedOwner::Unknown(id) => u32::try_from(id).ok().map(Owner::Saved),
    }
}

/// `saved[i]` onto queue `ids[i]`, no draws (0x50B8B0, 0x50BFF0).
fn restore_lists(
    world: &mut World,
    saved: &[SavedPeriodic],
    ids: &[u8],
    kind: aa_world::queues::Kind,
    kept: &BTreeMap<usize, Kept>,
    slot_kingdom: &[Option<KingdomId>],
    _things: &SavedThings,
) {
    for (q, &id) in saved.iter().zip(ids) {
        let buckets: Vec<Vec<(u64, u32)>> = q
            .buckets
            .iter()
            .map(|b| b.iter().filter_map(|&(o, data)| owner_of_entry(world, o, kept, slot_kingdom).map(|o| (o.key(), data))).collect())
            .collect();
        let on_queue_zero: Vec<KingdomId> = buckets
            .iter()
            .flatten()
            .filter_map(|&(key, _)| if let Some(Owner::Kingdom(k)) = Owner::from_key(key) { Some(k) } else { None })
            .collect();
        let queue = aa_world::queues::restoration::SavedQueue { id, kind, current: q.current, buckets };
        crate::scheduler::restore_saved_list(world, queue, on_queue_zero);
    }
}
