//! Shared fixture world for the zone/fog oracle replays: implements every
//! service trait from the recorded synthetic world and records events in the
//! producers' format (research/phase8-zones/zones_world.py).
#![allow(dead_code)]
use aa_zones::query::{Kingdom, VisitWorld, ZoneActors};
use aa_zones::component::{ActorZones, DetectionComponent, DetectionComponentWorld};
use aa_zones::flood::FloodWorld;
use aa_zones::tile::{AllianceWorld, ComponentEffect, MapGeometry, SaiHook, TileZones, ZoneMap, ZoneWorld};
use aa_zones::zone::{EnterWorld, Zone, ZoneObjectWorld};
use aa_zones::{Entry, Error, Result, SetRef};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn fixture(name: &str) -> Value {
    let path = format!("{}/../../research/phase8-zones/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
pub fn u(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("not a u32: {v}")) as u32
}
pub fn opt(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

pub struct Actor {
    pub addr: u32,
    pub owner: Option<usize>,
    pub allied: Vec<usize>,
    pub enemy: Vec<usize>,
    pub flags: u32,
    pub reveal: Option<u32>,
    pub b4: bool,
    pub components: HashMap<u32, u32>,
    pub redirect: Option<(usize, u8)>,
    pub tile: (i32, i32),
    pub multi: bool,
    pub id: u32,
    // detection-component oracle fields
    pub pos: [u32; 2],
    pub v100: u32,
    pub v158: bool,
    pub c80: Option<(Option<usize>, u8)>,
    pub data: HashMap<u32, u32>,
    pub sources: u32,
    pub members: Option<Vec<Option<usize>>>,
    // actor-entered oracle fields
    pub size: u32,
    pub marker: bool,
    pub explored_deep: bool,
    pub has_comp: bool,
    pub discovery: Vec<usize>,
}

pub struct World {
    pub kingdoms: Vec<Option<Kingdom>>,
    pub actors: Vec<Actor>,
    pub by_addr: HashMap<u32, usize>,
    pub occupants: Vec<Vec<usize>>,
    pub width: i32,
    pub fog: bool,
    pub sai: bool,
    pub effects_gate: bool,
    pub fast: bool,
    pub solvers: (bool, bool),
    pub tile_flags: Vec<u32>,
    pub tile_blockers: Vec<u32>,
    pub events: Vec<Value>,
    pub zone_vectors: HashMap<usize, ActorZones>,
    pub dcomps: HashMap<usize, DetectionComponent>,
    pub kingdom_info: Vec<(Vec<usize>, Option<u8>)>,
    pub next_zone_key: u32,
    pub enter_zones: HashMap<(usize, u32), Zone>,
    pub flag48: bool,
    pub alliances: Vec<Vec<bool>>,
}

impl World {
    pub fn from_case(case: &Value, width: i32) -> Self {
        let kingdoms: Vec<Option<Kingdom>> = case["kingdoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                (!k.is_null()).then(|| Kingdom {
                    key: u(&k["addr"]),
                    index: u(&k["index"]),
                    nonmajor: u(&k["nonmajor"]),
                    shared_mask: u(&k["shared"]),
                    supply_mask: u(&k["supply"]) as u16,
                })
            })
            .collect();
        let mut actors = vec![];
        let mut by_addr = HashMap::new();
        for (i, a) in case["actors"].as_array().unwrap().iter().enumerate() {
            by_addr.insert(u(&a["addr"]), i);
            let comps = a["components"]
                .as_object()
                .map(|o| o.iter().map(|(k, v)| (k.parse::<u32>().unwrap(), u(v))).collect())
                .unwrap_or_default();
            actors.push(Actor {
                addr: u(&a["addr"]),
                owner: opt(&a["owner"]),
                allied: a["allied"].as_array().map_or(vec![], |v| v.iter().map(|x| u(x) as usize).collect()),
                enemy: a["enemy"].as_array().map_or(vec![], |v| v.iter().map(|x| u(x) as usize).collect()),
                flags: a["flags"].as_u64().unwrap_or(0) as u32,
                reveal: a["reveal_mask"].as_u64().map(|x| x as u32),
                b4: a["b4"].as_u64().unwrap_or(1) != 0,
                components: comps,
                redirect: a.get("redirect").and_then(|r| {
                    (!r.is_null()).then(|| (u(&r[0]) as usize, u(&r[1]) as u8))
                }),
                tile: a.get("tile").map_or((0, 0), |t| (u(&t[0]) as i32, u(&t[1]) as i32)),
                multi: a["multi"].as_u64().unwrap_or(0) != 0,
                id: 1000 + i as u32,
                pos: a.get("pos").map_or([0, 0], |p| [u(&p[0]), u(&p[1])]),
                v100: a["v100"].as_u64().unwrap_or(0) as u32,
                v158: a["v158"].as_u64().unwrap_or(0) != 0,
                c80: a.get("c80").and_then(|c| {
                    (!c.is_null()).then(|| (opt(&a["c80_target"]), u(&c[1]) as u8))
                }),
                data: [(0x1ccu32, "zod"), (0x1d0, "remote"), (0x430, "d430")]
                    .iter()
                    .filter_map(|(o, k)| a[*k].as_u64().map(|v| (*o, v as u32)))
                    .collect(),
                sources: a["sources"].as_u64().unwrap_or(0) as u32,
                members: a["members"].as_array().map(|m| m.iter().map(opt).collect()),
                size: a["size"].as_u64().unwrap_or(0) as u32,
                marker: a["marker"].as_u64().unwrap_or(0) != 0,
                explored_deep: a["explored_deep"].as_u64().unwrap_or(0) != 0,
                has_comp: a["has_comp"].as_u64().unwrap_or(0) != 0,
                discovery: a["discovery"].as_array().map_or(vec![], |v| v.iter().map(|x| u(x) as usize).collect()),
            });
            let last = actors.last_mut().unwrap();
            for (o, k) in [(0x100u32, "w100"), (0x138, "w138"), (0x94, "w94"), (0x90, "w90")] {
                if let Some(v) = a[k].as_u64() {
                    last.components.insert(o, v as u32);
                }
            }
        }
        let occupants = case["occupants"]
            .as_array()
            .map(|o| {
                o.iter()
                    .map(|l| l.as_array().unwrap().iter().map(|x| u(x) as usize).collect())
                    .collect()
            })
            .unwrap_or_default();
        World {
            kingdoms,
            actors,
            by_addr,
            occupants,
            width,
            fog: case["fog"].as_u64().unwrap_or(1) != 0,
            sai: case["sai"].as_u64().unwrap_or(0) != 0,
            effects_gate: case["effects_gate"].as_bool().unwrap_or(true),
            fast: case["fast"].as_u64().unwrap_or(1) != 0,
            solvers: (
                case["solvers"].get(0).is_none_or(|v| v.as_u64().unwrap() != 0),
                case["solvers"].get(1).is_none_or(|v| v.as_u64().unwrap() != 0),
            ),
            tile_flags: case["tile_flags"]
                .as_array()
                .map(|a| a.iter().map(u).collect())
                .unwrap_or_default(),
            tile_blockers: case["tile_blockers"]
                .as_array()
                .map(|a| a.iter().map(u).collect())
                .unwrap_or_default(),
            events: vec![],
            zone_vectors: HashMap::new(),
            dcomps: HashMap::new(),
            kingdom_info: case["kingdom_info"]
                .as_array()
                .map(|v| {
                    v.iter()
                        .map(|k| {
                            (
                                k["enemy"].as_array().unwrap().iter().map(|x| u(x) as usize).collect(),
                                k["byte"].as_u64().map(|b| b as u8),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            next_zone_key: 0x4000_0000,
            enter_zones: HashMap::new(),
            flag48: case["flag48"].as_u64().unwrap_or(0) != 0,
            alliances: case["allied"]
                .as_array()
                .map(|r| r.iter().map(|row| row.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect()).collect())
                .unwrap_or_default(),
        }
    }
    pub fn a(&self, key: u32) -> usize {
        *self.by_addr.get(&key).unwrap_or_else(|| panic!("actor {key:#x}"))
    }
    pub fn kidx(&self, key: u32) -> Value {
        if key == 0 {
            return Value::Null;
        }
        json!(self.kingdoms.iter().position(|k| k.map(|k| k.key) == Some(key)).unwrap())
    }
    pub fn kkey(&self, i: Option<usize>) -> u32 {
        i.and_then(|i| self.kingdoms[i]).map_or(0, |k| k.key)
    }
}

impl ZoneActors for World {
    fn owner(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["owner", i]));
        Ok(self.kkey(self.actors[i].owner))
    }
    fn allied(&mut self, actor: u32, kingdom: u32) -> Result<bool> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["allied", i, k]));
        Ok(k.as_u64().is_some_and(|k| self.actors[i].allied.contains(&(k as usize))))
    }
    fn enemy(&mut self, actor: u32, kingdom: u32) -> Result<bool> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["enemy", i, k]));
        Ok(k.as_u64().is_some_and(|k| self.actors[i].enemy.contains(&(k as usize))))
    }
    fn zone_slot(&mut self, _actor: u32, _index: u32) -> Result<u32> {
        Err(Error::World("zone_slot not used here".into()))
    }
    fn word4(&mut self, _actor: u32) -> Result<u32> {
        Err(Error::World("word4 not used here".into()))
    }
    fn component_b0(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        Ok(self.actors[i].components.get(&0xb0).copied().unwrap_or(0))
    }
    fn reveal_mask(&mut self, actor: u32) -> Result<Option<u32>> {
        let i = self.a(actor);
        if let Some(c) = self.dcomps.get(&i) {
            return Ok(Some(c.reveal_mask));
        }
        Ok(self.actors[i].reveal)
    }
}

impl VisitWorld for World {
    fn kingdom(&mut self, key: u32) -> Result<Kingdom> {
        Ok(self.kingdoms.iter().flatten().find(|k| k.key == key).copied().unwrap())
    }
    fn kingdom_slot(&mut self, index: u32) -> Result<u32> {
        Ok(self.kingdoms.get(index as usize).copied().flatten().map_or(0, |k| k.key))
    }
}

fn hook_name(h: SaiHook) -> &'static str {
    match h {
        SaiHook::OccupiedInc => "sai_occupied_inc",
        SaiHook::OccupiedDec => "sai_occupied_dec",
        SaiHook::ExploredInc => "sai_explored_inc",
        SaiHook::VisibleInc => "sai_visible_inc",
        SaiHook::VisibleDec => "sai_visible_dec",
        SaiHook::ExploredDec => "sai_explored_dec",
    }
}

impl ZoneWorld for World {
    fn fog_enabled(&mut self) -> Result<bool> {
        Ok(self.fog)
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        Ok(self.sai)
    }
    fn sai(&mut self, hook: SaiHook, tile: usize, kingdom: u32) -> Result<()> {
        let k = self.kidx(kingdom);
        self.events.push(json!([hook_name(hook), tile, k]));
        Ok(())
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(self.kingdoms.iter().map(|k| k.map_or(0, |k| k.key)).collect())
    }
    fn tile_actors(&mut self, tile: usize) -> Result<Vec<u32>> {
        Ok(self
            .occupants
            .get(tile)
            .map(|l| l.iter().map(|&i| self.actors[i].addr).collect())
            .unwrap_or_default())
    }
    fn actor_reveal(&mut self, actor: u32, kingdom: u32, explored: bool, visible: bool) -> Result<()> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["reveal", i, k, explored as u32, visible as u32]));
        Ok(())
    }
    fn actor_hide(&mut self, actor: u32, kingdom: u32) -> Result<()> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["hide", i, k]));
        Ok(())
    }
    fn actor_unexplore(&mut self, actor: u32, kingdom: u32) -> Result<()> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["unexplore", i, k]));
        Ok(())
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        // [B36550]+0xE4 != 3 (effects_gate) || GWorld+0x48 (flag48)
        Ok(self.effects_gate || self.flag48)
    }
    fn actor_b4(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["b4", i]));
        Ok(self.actors[i].b4)
    }
    fn actor_multi_tile(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["multi", i]));
        Ok(self.actors[i].multi)
    }
    fn actor_tile(&mut self, actor: u32) -> Result<i64> {
        let i = self.a(actor);
        self.events.push(json!(["actor_tile", i]));
        let (x, y) = self.actors[i].tile;
        Ok(i64::from(y) * i64::from(self.width) + i64::from(x))
    }
    fn actor_flags(&mut self, actor: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].flags)
    }
    fn actor_redirect(&mut self, actor: u32) -> Result<Option<u32>> {
        let i = self.a(actor);
        Ok(match self.actors[i].redirect {
            Some((t, b)) if b != 0 => Some(self.actors[t].addr),
            _ => None,
        })
    }
    fn component(&mut self, actor: u32, offset: u32) -> Result<u32> {
        let i = self.a(actor);
        Ok(self.actors[i].components.get(&offset).copied().unwrap_or(0))
    }
    fn component_effect(&mut self, effect: ComponentEffect, component: u32, target: u32) -> Result<()> {
        let t = self.a(target);
        self.events.push(json!(["effect", format!("{effect:?}"), component, t]));
        Ok(())
    }
}

impl ZoneObjectWorld for World {
    fn solver_present(&mut self, release: bool) -> Result<bool> {
        Ok(if release { self.solvers.0 } else { self.solvers.1 })
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        Ok(self.fast)
    }
    fn presentation(&mut self, owner: u32, x: u32, y: u32, r: u32, flags: u16) -> Result<()> {
        let k = self.kidx(owner);
        self.events.push(json!(["present", k, x, y, r, flags]));
        Ok(())
    }
    fn log_implicit_clear(&mut self, id: u32, flags: u16) -> Result<()> {
        self.events.push(json!(["log", id, flags]));
        Ok(())
    }
    fn actor_id(&mut self, actor: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].id)
    }
}

impl FloodWorld for World {
    fn tile_flags(&mut self, tile: usize) -> Result<u32> {
        Ok(self.tile_flags.get(tile).copied().unwrap_or(0))
    }
    fn tile_blocker(&mut self, tile: usize) -> Result<u32> {
        Ok(self.tile_blockers.get(tile).copied().unwrap_or(0))
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        self.events.push(json!(["assert_blocked_start"]));
        Ok(false)
    }
}

/// Native tile state `[list, fog, supply]` -> TileZones (set rebuilt in `pool`).
pub fn tile_state(map: &ZoneMap, t: usize, actors: &World) -> Value {
    let tz: &TileZones = &map.tiles[t];
    let list: Vec<Value> = map
        .pool
        .entries(tz.set)
        .unwrap()
        .iter()
        .map(|e| json!([actors.a(e.actor), e.flags]))
        .collect();
    json!([list, tz.fog, tz.supply])
}

/// Rebuild a recorded list into the pool (reverse adds reproduce the order
/// when no two-element swap intervenes; callers assert the result).
pub fn build(map: &mut ZoneMap, list: &Value, addrs: &[u32]) -> Option<SetRef> {
    let mut slot = None;
    let entries: Vec<Entry> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|e| Entry { actor: addrs[u(&e[0]) as usize], flags: u(&e[1]) as u16 })
        .collect();
    for e in entries.iter().rev() {
        map.pool.tile_add(&mut slot, e.actor, e.flags).unwrap();
    }
    slot
}

pub fn geometry(width: i32, height: i32, shift: u32, tile_bits: u32) -> MapGeometry {
    let ext = |n: i32| (n as f32 * f32::from_bits(tile_bits)).to_bits();
    MapGeometry { shift, tile_size_bits: tile_bits, extent_bits: [ext(width), ext(height)], width, height }
}

impl DetectionComponentWorld for World {
    fn detection_component(&mut self, actor: u32) -> Result<&mut DetectionComponent> {
        let i = self.a(actor);
        self.dcomps.get_mut(&i).ok_or(Error::NativeNull("actor+0xB8"))
    }
    fn zones(&mut self, actor: u32) -> Result<&mut ActorZones> {
        let i = self.a(actor);
        Ok(self.zone_vectors.entry(i).or_default())
    }
    fn new_zone_key(&mut self, _actor: u32, _index: u32) -> Result<u32> {
        self.next_zone_key += 0x20;
        Ok(self.next_zone_key)
    }
    fn log_unexpected_zone(&mut self, actor: u32, index: u32) -> Result<()> {
        let id = self.actors[self.a(actor)].id;
        self.events.push(json!(["log", id, index]));
        Ok(())
    }
    fn actor_position(&mut self, actor: u32) -> Result<[u32; 2]> {
        Ok(self.actors[self.a(actor)].pos)
    }
    fn actor_virtual_100(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["v100", i]));
        Ok(self.actors[i].v100)
    }
    fn actor_virtual_158(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["v158", i]));
        Ok(self.actors[i].v158)
    }
    fn actor_80(&mut self, actor: u32) -> Result<(u32, u32, u8)> {
        let i = self.a(actor);
        Ok(match self.actors[i].c80 {
            None => (0, 0, 0),
            Some((t, b)) => (1, t.map_or(0, |t| self.actors[t].addr), b),
        })
    }
    fn data_word(&mut self, actor: u32, offset: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].data.get(&offset).copied().unwrap_or(0))
    }
    fn modified_detection_range(&mut self, _actor: u32, _context: u32) -> Result<u32> {
        Err(Error::World("modifier context not used".into()))
    }
    fn reveal_sources(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["sources", i]));
        Ok(self.actors[i].sources)
    }
    fn organization_slots(&mut self, actor: u32) -> Result<Option<Vec<u32>>> {
        let i = self.a(actor);
        Ok(self.actors[i].members.clone().map(|m| m.iter().map(|x| x.map_or(0, |x| self.actors[x].addr)).collect()))
    }
    fn kingdom_enemy(&mut self, kingdom: u32, owner: u32) -> Result<bool> {
        let k = self.kidx(kingdom).as_u64().unwrap() as usize;
        let o = self.kidx(owner);
        self.events.push(json!(["kingdom_enemy", k, o]));
        Ok(o.as_u64().is_some_and(|o| self.kingdom_info[k].0.contains(&(o as usize))))
    }
    fn kingdom_reveal_byte(&mut self, kingdom: u32) -> Result<Option<u8>> {
        let k = self.kidx(kingdom).as_u64().unwrap() as usize;
        Ok(self.kingdom_info[k].1)
    }
    fn schedule_tick(&mut self, actor: u32, delay_bits: u32) -> Result<()> {
        let i = self.a(actor);
        self.events.push(json!(["schedule", i, delay_bits]));
        Ok(())
    }
}

impl EnterWorld for World {
    fn zone_object(&mut self, actor: u32, index: u32) -> Result<Option<Zone>> {
        let i = self.a(actor);
        Ok(self.enter_zones.get(&(i, index)).copied())
    }
    fn has_detection_component(&mut self, actor: u32) -> Result<bool> {
        Ok(self.actors[self.a(actor)].has_comp)
    }
    fn data_is_marker(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["marker", i]));
        Ok(self.actors[i].marker)
    }
    fn virtual_explored(&mut self, actor: u32, kingdom: u32, deep: bool) -> Result<bool> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.push(json!(["explored_deep", i, k, deep as u32]));
        Ok(self.actors[i].explored_deep)
    }
    fn world_flag_48(&mut self) -> Result<bool> {
        Ok(self.flag48)
    }
    fn marker_reveal_padding_bits(&mut self) -> Result<u32> {
        Ok(4.0f32.to_bits())
    }
    fn actor_size(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["size", i]));
        Ok(self.actors[i].size)
    }
    fn discovery_tiles(&mut self, actor: u32, _radius_bits: u32) -> Result<Vec<usize>> {
        Ok(self.actors[self.a(actor)].discovery.clone())
    }
    fn set_actor_flags(&mut self, actor: u32, flags: u32) -> Result<()> {
        let i = self.a(actor);
        self.actors[i].flags = flags;
        Ok(())
    }
}

impl AllianceWorld for World {
    fn kingdom_allied(&mut self, kingdom: u32, other: u32) -> Result<bool> {
        let k = self.kidx(kingdom).as_u64().unwrap() as usize;
        let o = self.kidx(other).as_u64().unwrap() as usize;
        self.events.push(json!(["allied", k, o]));
        Ok(self.alliances[k][o])
    }
}
