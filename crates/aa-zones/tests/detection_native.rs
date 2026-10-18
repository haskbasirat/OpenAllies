//! Replays research/phase8-zones/native-detection.json: the original actor
//! detection routines (0x4EC010, 0x4EC1B0, 0x4EA0C0, 0x4EA0D0, 0x4EA1D0,
//! 0x4D3B80, 0x4D3C50, 0x4EBC70, 0x4EBFB0, 0x4EBFD0) at PC24/PC53/PC64:
//! results, faults, ordered virtual calls / seams and every actor's +0x54
//! word and memory slots after each operation.
use aa_numeric::Precision;
use aa_zones::detection::{self, ActorDetectionWorld};
use aa_zones::query::{Kingdom, VisitWorld, ZoneActors};
use aa_zones::tile::{ComponentEffect, MapGeometry, SaiHook, ZoneMap, ZoneWorld};
use aa_zones::Entry;
use aa_zones::{Error, Result};
use serde_json::{Value, json};
use std::collections::HashMap;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-zones/native-detection.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("not a u32: {v}")) as u32
}
fn opt(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

struct Actor {
    addr: u32,
    size: u32,
    keeps: bool,
    eligible: bool,
    b4: bool,
    container: Option<usize>,
    flags: u32,
    announce: bool,
    carrier: u32,
    carrier_visible: bool,
    members: Option<Vec<usize>>,
    target: Option<(usize, u8)>,
    word: u32,
    slots: Vec<u32>,
    count: usize,
    shape: Vec<usize>,
    owner: Option<usize>,
    reveal: Option<u32>,
}

struct World {
    kingdoms: Vec<Option<Kingdom>>,
    actors: Vec<Actor>,
    by_addr: HashMap<u32, usize>,
    fog: bool,
    tile_fog: Vec<u32>,
    events: Vec<Value>,
    p: Precision,
    geometry: MapGeometry,
}

impl World {
    fn a(&self, key: u32) -> usize {
        self.by_addr[&key]
    }
    fn kidx(&self, key: u32) -> Value {
        if key == 0 {
            return Value::Null;
        }
        json!(self.kingdoms.iter().position(|k| k.map(|k| k.key) == Some(key)).unwrap())
    }
}

fn unused<T>() -> Result<T> {
    Err(Error::World("not used by the detection oracle".into()))
}

impl ZoneActors for World {
    fn owner(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["owner", i]));
        Ok(self.actors[i].owner.and_then(|k| self.kingdoms[k]).map_or(0, |k| k.key))
    }
    fn allied(&mut self, _: u32, _: u32) -> Result<bool> { unused() }
    fn enemy(&mut self, _: u32, _: u32) -> Result<bool> { unused() }
    fn zone_slot(&mut self, _: u32, _: u32) -> Result<u32> { unused() }
    fn word4(&mut self, _: u32) -> Result<u32> { unused() }
    fn component_b0(&mut self, _: u32) -> Result<u32> { unused() }
    fn reveal_mask(&mut self, actor: u32) -> Result<Option<u32>> { Ok(self.actors[self.a(actor)].reveal) }
}

impl VisitWorld for World {
    fn kingdom(&mut self, key: u32) -> Result<Kingdom> {
        Ok(self.kingdoms.iter().flatten().find(|k| k.key == key).copied().unwrap())
    }
    fn kingdom_slot(&mut self, index: u32) -> Result<u32> {
        Ok(self.kingdoms.get(index as usize).copied().flatten().map_or(0, |k| k.key))
    }
}

impl ZoneWorld for World {
    fn fog_enabled(&mut self) -> Result<bool> { Ok(self.fog) }
    fn sai_enabled(&mut self) -> Result<bool> { unused() }
    fn sai(&mut self, _: SaiHook, _: usize, _: u32) -> Result<()> { unused() }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(self.kingdoms.iter().map(|k| k.map_or(0, |k| k.key)).collect())
    }
    fn tile_actors(&mut self, _: usize) -> Result<Vec<u32>> { unused() }
    fn actor_reveal(&mut self, actor: u32, kingdom: u32, explored: bool, visible: bool) -> Result<()> {
        // the container is a GActor too: its +0x64 is 0x4EC010
        detection::reveal(self, actor, kingdom, explored, visible)
    }
    fn actor_hide(&mut self, _: u32, _: u32) -> Result<()> { unused() }
    fn actor_unexplore(&mut self, _: u32, _: u32) -> Result<()> { unused() }
    fn zone_effects_enabled(&mut self) -> Result<bool> { unused() }
    fn actor_b4(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["b4", i]));
        Ok(self.actors[i].b4)
    }
    fn actor_multi_tile(&mut self, _: u32) -> Result<bool> { unused() }
    fn actor_tile(&mut self, _: u32) -> Result<i64> { unused() }
    fn actor_flags(&mut self, actor: u32) -> Result<u32> { Ok(self.actors[self.a(actor)].flags) }
    fn actor_redirect(&mut self, _: u32) -> Result<Option<u32>> { unused() }
    fn component(&mut self, _: u32, _: u32) -> Result<u32> { unused() }
    fn component_effect(&mut self, _: ComponentEffect, _: u32, _: u32) -> Result<()> { unused() }
}

impl ActorDetectionWorld for World {
    fn detection_word(&mut self, actor: u32) -> Result<u32> { Ok(self.actors[self.a(actor)].word) }
    fn set_detection_word(&mut self, actor: u32, word: u32) -> Result<()> {
        let i = self.a(actor);
        self.actors[i].word = word;
        Ok(())
    }
    fn actor_container(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["container", i]));
        Ok(self.actors[i].container.map_or(0, |c| self.actors[c].addr))
    }
    fn actor_size_bits(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.push(json!(["size", i]));
        Ok(self.actors[i].size)
    }
    fn shape_tiles(&mut self, actor: u32) -> Result<Vec<usize>> { Ok(self.actors[self.a(actor)].shape.clone()) }
    fn tile_fog(&mut self, tile: usize) -> Result<u32> { Ok(self.tile_fog[tile]) }
    fn occupancy_tiles(&mut self, actor: u32) -> Result<Vec<usize>> { Ok(self.actors[self.a(actor)].shape.clone()) }
    fn keeps_memory(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["keeps", i]));
        Ok(self.actors[i].keeps)
    }
    fn memory_eligible(&mut self, actor: u32) -> Result<bool> {
        let i = self.a(actor);
        self.events.push(json!(["eligible", i]));
        Ok(self.actors[i].eligible)
    }
    fn memory_slot(&mut self, actor: u32, index: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].slots[index as usize])
    }
    fn memory_slots(&mut self, actor: u32) -> Result<Vec<u32>> {
        let a = &self.actors[self.a(actor)];
        Ok(a.slots[..a.count].to_vec())
    }
    fn create_memory(&mut self, actor: u32, kingdom: u32) -> Result<()> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        let index = self.kingdom(kingdom)?.index as usize;
        self.events.push(json!(["create", i, k]));
        self.actors[i].slots[index] = 0x7700;
        Ok(())
    }
    fn delete_memory(&mut self, actor: u32, kingdom: u32) -> Result<()> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        let index = self.kingdom(kingdom)?.index as usize;
        self.events.push(json!(["delete", i, k]));
        self.actors[i].slots[index] = 0;
        Ok(())
    }
    fn clear_memory(&mut self, actor: u32) -> Result<()> {
        let i = self.a(actor);
        self.events.push(json!(["clear_memory", i]));
        Ok(())
    }
    fn announces_first_sighting(&mut self, actor: u32) -> Result<bool> { Ok(self.actors[self.a(actor)].announce) }
    fn first_sighting(&mut self, actor: u32, _kingdom: u32) -> Result<()> {
        let i = self.a(actor);
        self.events.push(json!(["first_sighting", i]));
        Ok(())
    }
    fn carrier(&mut self, actor: u32) -> Result<u32> { Ok(self.actors[self.a(actor)].carrier) }
    fn carrier_visible(&mut self, carrier: u32, kingdom: u32) -> Result<bool> {
        let k = self.kidx(kingdom);
        self.events.push(json!(["carrier_visible", carrier, k]));
        Ok(self.actors[(carrier - 0x7000_0000) as usize].carrier_visible)
    }
    fn organization_members(&mut self, actor: u32) -> Result<Option<Vec<u32>>> {
        let i = self.a(actor);
        Ok(self.actors[i].members.clone().map(|m| m.iter().map(|&x| self.actors[x].addr).collect()))
    }
    fn component_80_target(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        Ok(self.actors[i].target.map_or(0, |(t, _)| self.actors[t].addr))
    }
    fn virtual_seen(&mut self, actor: u32, kingdom: u32, deep: bool, explored: bool) -> Result<bool> {
        if explored {
            detection::explored_by(self, actor, kingdom, deep)
        } else {
            detection::visible_to(self, actor, kingdom, deep)
        }
    }
}

#[test]
fn detection_matches_native() {
    let f = fixture();
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let p = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
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
            let init = &case["initial"][i];
            actors.push(Actor {
                addr: u(&a["addr"]),
                size: u(&a["size"]),
                keeps: u(&a["keeps"]) != 0,
                eligible: u(&a["eligible"]) != 0,
                b4: u(&a["b4"]) != 0,
                container: opt(&a["container"]),
                flags: u(&a["flags"]),
                announce: u(&a["announce"]) != 0,
                carrier: u(&a["carrier"]),
                carrier_visible: u(&a["carrier_visible"]) != 0,
                members: a["members"].as_array().map(|m| m.iter().map(|x| u(x) as usize).collect()),
                target: opt(&a["target"]).map(|t| (t, u(&a["target_byte"]) as u8)),
                word: u(&init["word"]),
                slots: init["slots"].as_array().unwrap().iter().map(u).collect(),
                count: a["slots"].as_array().unwrap().len(),
                shape: a["shape"].as_array().unwrap().iter().map(|x| u(x) as usize).collect(),
                owner: opt(&a["owner"]),
                reveal: a["reveal_mask"].as_u64().map(|x| x as u32),
            });
        }
        let tile = u(&case["tile"]);
        let addrs: Vec<u32> = actors.iter().map(|a| a.addr).collect();
        let mut map = ZoneMap::new(MapGeometry { shift: 3, tile_size_bits: tile, extent_bits: [0, 0], width: 6, height: 4 }).unwrap();
        for (t, l) in case["tile_sets"].as_array().unwrap().iter().enumerate() {
            let mut slot = None;
            let entries: Vec<Entry> = l
                .as_array()
                .unwrap()
                .iter()
                .map(|e| Entry { actor: addrs[u(&e[0]) as usize], flags: u(&e[1]) as u16 })
                .collect();
            for e in entries.iter().rev() {
                map.pool.tile_add(&mut slot, e.actor, e.flags).unwrap();
            }
            assert_eq!(map.pool.entries(slot).unwrap(), entries);
            map.tiles[t].set = slot;
        }
        let mut w = World {
            kingdoms: kingdoms.clone(),
            actors,
            by_addr,
            fog: u(&case["fog"]) != 0,
            tile_fog: case["tile_fog"].as_array().unwrap().iter().map(u).collect(),
            events: vec![],
            p,
            geometry: MapGeometry { shift: 3, tile_size_bits: tile, extent_bits: [0, 0], width: 6, height: 4 },
        };
        for op in case["ops"].as_array().unwrap() {
            let a = w.actors[u(&op["actor"]) as usize].addr;
            let k = kingdoms[u(&op["kingdom"]) as usize].map_or(0, |k| k.key);
            w.events.clear();
            let ctx = format!("seed {} {} {}", case["seed"], case["precision"], op);
            let g = w.geometry;
            let pp = w.p;
            let r: Result<Option<bool>> = match op["op"].as_str().unwrap() {
                "reveal" => detection::reveal(&mut w, a, k, u(&op["explored"]) != 0, u(&op["visible"]) != 0).map(|_| None),
                "hide" => detection::hide(&mut w, &g, a, k, pp).map(|_| None),
                "unexplore" => detection::unexplore(&mut w, a, k).map(|_| None),
                "visible_to" => detection::visible_to(&mut w, a, k, u(&op["deep"]) != 0).map(Some),
                "explored_by" => detection::explored_by(&mut w, a, k, u(&op["deep"]) != 0).map(Some),
                "shared_explored" => detection::shared_explored(&mut w, a, k).map(Some),
                "shared_visible" => detection::shared_visible(&mut w, a, k).map(Some),
                "refresh" => detection::refresh_memory(&mut w, a).map(|_| None),
                "all_explored" => detection::set_all_explored(&mut w, a).map(|_| None),
                "tile_changed" => detection::tile_changed(&map, &mut w, a).map(|_| None),
                "refresh_tiles" => detection::refresh_from_tiles(&map, &mut w, a).map(|_| None),
                "all_visible" => detection::set_all_visible(&mut w, a).map(|_| None),
                other => panic!("{other}"),
            };
            match (&r, op["fault"].is_null()) {
                (Ok(v), true) => {
                    if let Some(v) = v {
                        assert_eq!(u32::from(*v), u(&op["result"]), "{ctx}");
                    }
                }
                (Err(Error::NativeNull(_)), false) => faults += 1,
                other => panic!("{ctx}: {other:?}"),
            }
            assert_eq!(Value::Array(w.events.clone()), op["events"], "{ctx}");
            for (i, s) in op["state"].as_array().unwrap().iter().enumerate() {
                assert_eq!(w.actors[i].word, u(&s["word"]), "{ctx} actor {i} word");
                let slots: Vec<u32> = s["slots"].as_array().unwrap().iter().map(u).collect();
                assert_eq!(w.actors[i].slots, slots, "{ctx} actor {i} slots");
            }
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
