//! Replays research/phase8-zones/native-zone-queries.json (the original
//! zone-set query slots and GMap tile wrappers under Unicorn): return values,
//! native fault points and the ordered actor virtual / visitor / notify calls.
use aa_zones::query::{self, ActorList, DetectionVisitor, Kingdom, VisitWorld, ZoneActors};
use aa_zones::{Entry, Error, Result, SetRef, ZonePool};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-zones/native-zone-queries.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn opt(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

struct Actor {
    owner: Option<usize>,
    allied: Vec<usize>,
    enemy: Vec<usize>,
    reveal: Option<u32>,
    b0: u32,
    word4: u32,
    zones: Vec<u32>,
}

struct World {
    actors: Vec<Actor>,
    by_addr: HashMap<u32, usize>,
    kingdoms: Vec<Option<Kingdom>>,
    events: Rc<RefCell<Vec<Value>>>,
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
    fn kkey(&self, i: Option<usize>) -> u32 {
        i.and_then(|i| self.kingdoms[i]).map_or(0, |k| k.key)
    }
}

impl ZoneActors for World {
    fn owner(&mut self, actor: u32) -> Result<u32> {
        let i = self.a(actor);
        self.events.borrow_mut().push(json!(["owner", i]));
        Ok(self.kkey(self.actors[i].owner))
    }
    fn allied(&mut self, actor: u32, kingdom: u32) -> Result<bool> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.borrow_mut().push(json!(["allied", i, k]));
        Ok(k.as_u64().is_some_and(|k| self.actors[i].allied.contains(&(k as usize))))
    }
    fn enemy(&mut self, actor: u32, kingdom: u32) -> Result<bool> {
        let i = self.a(actor);
        let k = self.kidx(kingdom);
        self.events.borrow_mut().push(json!(["enemy", i, k]));
        Ok(k.as_u64().is_some_and(|k| self.actors[i].enemy.contains(&(k as usize))))
    }
    fn zone_slot(&mut self, actor: u32, index: u32) -> Result<u32> {
        let i = self.a(actor);
        Ok(self.actors[i].zones.get(index as usize).copied().unwrap_or(0))
    }
    fn word4(&mut self, actor: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].word4)
    }
    fn component_b0(&mut self, actor: u32) -> Result<u32> {
        Ok(self.actors[self.a(actor)].b0)
    }
    fn reveal_mask(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok(self.actors[self.a(actor)].reveal)
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

struct Visitor(Rc<RefCell<Vec<Value>>>, Vec<Option<Kingdom>>);
impl DetectionVisitor for Visitor {
    fn reveal(&mut self, kingdom: u32, explored: bool, visible: bool) -> Result<()> {
        let k = if kingdom == 0 {
            Value::Null
        } else {
            json!(self.1.iter().position(|k| k.map(|k| k.key) == Some(kingdom)).unwrap())
        };
        self.0.borrow_mut().push(json!(["visit", k, explored as u32, visible as u32]));
        Ok(())
    }
}

/// Build the tiles' sets in a pool in the recorded order (each list is
/// rebuilt from its last entry to its first with tile adds, which reproduces
/// the native list exactly unless the two-element swap applies; the test
/// checks the rebuilt order).
fn build(pool: &mut ZonePool, list: &[Entry]) -> Option<SetRef> {
    let mut slot = None;
    for e in list.iter().rev() {
        pool.tile_add(&mut slot, e.actor, e.flags).unwrap();
    }
    slot
}

#[test]
fn zone_queries_match_native() {
    let f = fixture();
    let mut n = 0;
    for case in f["cases"].as_array().unwrap() {
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
        let mut w = World { actors: vec![], by_addr: HashMap::new(), kingdoms: kingdoms.clone(), events: Rc::new(RefCell::new(vec![])) };
        let mut addrs = vec![];
        for (i, a) in case["actors"].as_array().unwrap().iter().enumerate() {
            addrs.push(u(&a["addr"]));
            w.by_addr.insert(u(&a["addr"]), i);
            w.actors.push(Actor {
                owner: opt(&a["owner"]),
                allied: a["allied"].as_array().unwrap().iter().map(|x| u(x) as usize).collect(),
                enemy: a["enemy"].as_array().unwrap().iter().map(|x| u(x) as usize).collect(),
                reveal: a["reveal_mask"].as_u64().map(|x| x as u32),
                b0: u(&a["b0"]),
                word4: u(&a["word4"]),
                zones: a["zones"].as_array().unwrap().iter().map(u).collect(),
            });
        }
        let mut pool = ZonePool::new();
        let lists: Vec<Vec<Entry>> = case["tiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                l.as_array()
                    .unwrap()
                    .iter()
                    .map(|e| Entry { actor: addrs[u(&e[0]) as usize], flags: u(&e[1]) as u16 })
                    .collect()
            })
            .collect();
        let sets: Vec<Option<SetRef>> = lists.iter().map(|l| build(&mut pool, l)).collect();
        for (s, l) in sets.iter().zip(&lists) {
            assert_eq!(&pool.entries(*s).unwrap(), l);
        }
        let supply: Vec<u16> = case["supply"].as_array().unwrap().iter().map(|x| u(x) as u16).collect();
        let kref = |q: &Value| opt(&q["kingdom"]).and_then(|i| kingdoms[i]);
        let akey = |v: &Value| opt(v).map_or(0, |i| addrs[i]);
        for q in case["queries"].as_array().unwrap() {
            let t = u(&q["tile"]) as usize;
            let set = sets[t];
            w.events.borrow_mut().clear();
            let kind = q["q"].as_str().unwrap();
            let r: Result<u32> = match kind {
                "54ef40" => {
                    let zone = (!q["zone"].is_null())
                        .then(|| (addrs[u(&q["zone"][0]) as usize], u(&q["zone"][1]) as u16));
                    query::tile_has_zone(&pool, set, zone).map(u32::from)
                }
                "54ef80" => query::tile_first_zone(&pool, set, &mut w, u(&q["type"]), akey(&q["exclude"]), u(&q["word4"])),
                "54efc0" => query::tile_enemy_zone(&pool, set, &mut w, kref(q).map_or(0, |k| k.key), u(&q["type"])),
                "54eff0" => query::tile_owned_zone(&pool, set, &mut w, kref(q).map_or(0, |k| k.key), u(&q["type"])),
                "54f020" => query::tile_enemy_discovery(&pool, set, &mut w, kref(q).map_or(0, |k| k.key)).map(u32::from),
                "54f870" => query::tile_own_supply(&pool, set, supply[t], &mut w, kref(q).as_ref(), akey(&q["exclude"])).map(u32::from),
                "54f8b0" => query::tile_friendly_supply(&pool, set, supply[t], &mut w, kref(q).as_ref(), akey(&q["exclude"])).map(u32::from),
                "54f8f0" => {
                    let mut list = ActorList {
                        items: q["list_before"].as_array().unwrap().iter().map(|x| addrs[u(x) as usize]).collect(),
                    };
                    let r = query::tile_collect_allied(&pool, set, &mut w, &mut list, kref(q).map_or(0, |k| k.key), u(&q["type"]), akey(&q["exclude"]));
                    if r.is_ok() {
                        let after: Vec<u32> = q["list_after"].as_array().unwrap().iter().map(|x| addrs[u(x) as usize]).collect();
                        assert_eq!(list.items, after, "{q}");
                    }
                    r.map(u32::from)
                }
                "has_actor" => query::has_actor(&pool, set.unwrap(), addrs[u(&q["actor"]) as usize]).map(u32::from),
                "leaf_id" | "part_id" => {
                    let (t2, depth, head) = (u(&q["part"][0]) as usize, u(&q["part"][1]), q["part"][2].as_bool().unwrap());
                    let mut obj = sets[t2].unwrap();
                    for _ in 0..depth {
                        obj = pool.tail(obj).unwrap();
                    }
                    if head {
                        obj = SetRef::Leaf(pool.entries(Some(obj)).unwrap()[0]);
                    }
                    if kind == "leaf_id" {
                        query::contains_leaf(&pool, set.unwrap(), obj).map(u32::from)
                    } else {
                        query::contains_part(&pool, set.unwrap(), obj).map(u32::from)
                    }
                }
                "detects" => {
                    let k = kingdoms[u(&q["kingdom"]) as usize].unwrap();
                    query::detects(&pool, set.unwrap(), &mut w, &k).map(u32::from)
                }
                "visit" => {
                    let mut v = Visitor(w.events.clone(), kingdoms.clone());
                    query::visit(&pool, set.unwrap(), &mut w, &mut v).map(|_| 0)
                }
                "notify" => {
                    let ty = q["type"].as_i64().unwrap() as i32 as u32;
                    let mut calls = vec![];
                    let r = query::notify_type(&pool, set.unwrap(), &mut w, ty, &mut |z| {
                        calls.push(json!(["notify", z]));
                        Ok(())
                    });
                    w.events.borrow_mut().extend(calls);
                    r.map(|_| 0)
                }
                other => panic!("{other}"),
            };
            match (&r, q["fault"].is_null()) {
                (Ok(v), true) => {
                    let eax = u(&q["eax"]);
                    let bool_result = matches!(kind, "54ef40" | "54f020" | "54f870" | "54f8b0" | "54f8f0" | "has_actor" | "leaf_id" | "part_id" | "detects");
                    if bool_result {
                        assert_eq!(*v, eax & 0xff, "seed {} {q}", case["seed"]);
                    } else if kind != "visit" && kind != "notify" {
                        assert_eq!(*v, eax, "seed {} {q}", case["seed"]);
                    }
                }
                (Err(Error::NativeNull(_)), false) => {}
                other => panic!("seed {} {q}: {other:?}", case["seed"]),
            }
            assert_eq!(Value::Array(w.events.borrow().clone()), q["events"], "seed {} {q}", case["seed"]);
            n += 1;
        }
    }
    assert_eq!(n, u(&f["query_count"]) as usize);
}

