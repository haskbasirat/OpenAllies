//! Replays research/phase8-economy/native_supply.py: original supply component
//! tick, provides/in-supply predicates and EconomyComponent::Tick.
use aa_economy::supply::*;
use aa_economy::{EconomyError, F32Bits, X87Precision};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn bits(v: &Value) -> F32Bits {
    F32Bits(v.as_u64().unwrap() as u32)
}
fn flag(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn opt_bool(v: &Value) -> Option<bool> {
    v.as_bool()
}

fn actor(v: &Value) -> SupplyActor {
    let d = &v["definition"];
    SupplyActor {
        gtid: v["gtid"].as_u64().unwrap() as u32,
        flags8: v["flags8"].as_u64().unwrap() as u32,
        states: v["states"].as_u64().unwrap(),
        definition: SupplyDefinition {
            relay: flag(&d["relay"]),
            stockpile_max: bits(&d["stockpile_max"]),
            economy_ticks: flag(&d["economy_ticks"]),
            byte_31d: flag(&d["byte_31d"]),
            byte_2f4: flag(&d["byte_2f4"]),
        },
        supply: (!v["supply"].is_null()).then(|| SupplyComponent {
            radius: bits(&v["supply"]["radius"]),
            dirty: flag(&v["supply"]["dirty"]),
            stockpile: bits(&v["supply"]["stockpile"]),
            relay: v["supply"]["relay"].as_u64().unwrap() as u32,
        }),
        element: (!v["element"].is_null()).then(|| v["element"][0].as_u64().map(|i| i as u32)),
        organization_component: opt_bool(&v["organization_component"]),
        properties_flag20: opt_bool(&v["properties_flag20"]),
        city_flag20: opt_bool(&v["city_flag20"]),
        word_118: flag(&v["word_118"]),
        word_128: flag(&v["word_128"]),
        word_d8: flag(&v["word_d8"]),
        gtid_d0: v["gtid_d0"].as_u64().unwrap() as u32,
        kingdom: v["kingdom"].as_u64().map(|k| k as u32),
        position: [bits(&v["position"][0]), bits(&v["position"][1])],
        tile: [
            v["tile"][0].as_u64().unwrap() as u32,
            v["tile"][1].as_u64().unwrap() as u32,
        ],
        zone_word8: if v["zone_word8"].is_string() {
            None
        } else {
            v["zone_word8"].as_u64().map(|w| F32Bits(w as u32))
        },
    }
}

struct World {
    actors: Vec<SupplyActor>,
    large: Vec<bool>,
    covered: Vec<Vec<u32>>,
    candidates: Vec<Vec<u32>>,
    gtids: BTreeMap<u32, (u16, Option<u32>)>,
    kingdoms: Vec<KingdomSupply>,
    suspended: bool,
    under_siege: bool,
    rate: F32Bits,
    width: u32,
    supplied: BTreeSet<u32>,
    events: Vec<Value>,
}
impl SupplyWorld for World {
    fn actor(&self, actor: u32) -> Result<&SupplyActor, EconomyError> {
        self.actors
            .get(actor as usize)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn actor_mut(&mut self, actor: u32) -> Result<&mut SupplyActor, EconomyError> {
        self.actors
            .get_mut(actor as usize)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn resolve(&self, gtid: u32) -> Option<u32> {
        let (generation, live) = self.gtids.get(&(gtid & 0xffff)).copied()?;
        if generation == (gtid >> 16) as u16 {
            live
        } else {
            None
        }
    }
    fn kingdom(&self, kingdom: u32) -> Result<KingdomSupply, EconomyError> {
        self.kingdoms
            .get(kingdom as usize)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn owner(&mut self, actor: u32) -> Result<Option<u32>, EconomyError> {
        self.events.push(json!({"kind": "owner", "actor": actor}));
        Ok(self.actors[actor as usize].kingdom)
    }
    fn world_suspended(&mut self) -> Result<bool, EconomyError> {
        Ok(self.suspended)
    }
    fn provide_under_siege(&self) -> bool {
        self.under_siege
    }
    fn recharge_rate(&self) -> F32Bits {
        self.rate
    }
    fn map_width(&self) -> u32 {
        self.width
    }
    fn large_actor(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.events
            .push(json!({"kind": "large_actor", "actor": actor}));
        Ok(self.large[actor as usize])
    }
    fn tile_supplied(
        &mut self,
        tile: u32,
        kingdom: Option<u32>,
        actor: u32,
    ) -> Result<bool, EconomyError> {
        self.events.push(
            json!({"kind": "tile_supplied", "tile": tile, "kingdom": kingdom, "actor": actor}),
        );
        Ok(self.supplied.contains(&tile))
    }
    fn covered_tiles(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError> {
        self.events
            .push(json!({"kind": "covered_tiles", "actor": actor}));
        Ok(self.covered[actor as usize].clone())
    }
    fn relay_candidates(
        &mut self,
        actor: u32,
        kingdom: Option<u32>,
    ) -> Result<Vec<u32>, EconomyError> {
        self.events
            .push(json!({"kind": "relay_candidates", "actor": actor, "kingdom": kingdom}));
        Ok(self.candidates[actor as usize].clone())
    }
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "set_state", "actor": actor, "state": state, "enabled": enabled}));
        let states = &mut self.actors[actor as usize].states;
        if enabled {
            *states |= 1 << state;
        } else {
            *states &= !(1 << state);
        }
        Ok(())
    }
    fn refresh_actor(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "refresh_actor", "actor": actor}));
        Ok(())
    }
    fn refresh_economy(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "refresh_economy", "actor": actor}));
        Ok(())
    }
    fn stop_economy_ticks(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "stop_economy_ticks", "actor": actor}));
        Ok(())
    }
    fn clear_zone(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "clear_zone", "actor": actor}));
        Ok(())
    }
    fn set_zone(
        &mut self,
        actor: u32,
        position: [F32Bits; 2],
        radius: F32Bits,
    ) -> Result<(), EconomyError> {
        self.events.push(json!({"kind": "set_zone", "actor": actor,
            "position": [position[0].0, position[1].0], "radius": radius.0}));
        Ok(())
    }
}

fn world(case: &Value) -> World {
    let w = &case["world"];
    let actors = case["actors"].as_array().unwrap();
    World {
        actors: actors.iter().map(actor).collect(),
        large: actors.iter().map(|a| flag(&a["large"])).collect(),
        covered: actors
            .iter()
            .map(|a| {
                a["covered"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t.as_u64().unwrap() as u32)
                    .collect()
            })
            .collect(),
        candidates: actors
            .iter()
            .map(|a| {
                a["candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t.as_u64().unwrap() as u32)
                    .collect()
            })
            .collect(),
        gtids: w["gtids"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(slot, v)| {
                (
                    slot.parse().unwrap(),
                    (
                        v[0].as_u64().unwrap() as u16,
                        v[1].as_u64().map(|i| i as u32),
                    ),
                )
            })
            .collect(),
        kingdoms: w["kingdoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| KingdomSupply {
                word_190: k["word_190"].as_u64().unwrap() as u32,
                shortage_ratio: bits(&k["shortage_ratio"]),
            })
            .collect(),
        suspended: flag(&w["suspended"]),
        under_siege: flag(&w["provide_under_siege"]),
        rate: bits(&w["recharge_rate"]),
        width: w["map_width"].as_u64().unwrap() as u32,
        supplied: w["supplied_tiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_u64().unwrap() as u32)
            .collect(),
        events: vec![],
    }
}

fn snapshot(world: &World) -> Value {
    Value::Array(
        world
            .actors
            .iter()
            .map(|a| {
                json!({"states": a.states, "supply": a.supply.as_ref().map(|s| json!({
                "radius": s.radius.0, "dirty": s.dirty, "stockpile": s.stockpile.0, "relay": s.relay}))})
            })
            .collect(),
    )
}

#[test]
fn supply_matches_original_x86_at_all_precisions() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/native-supply-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let precision: X87Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let mut world = world(&fixture["case"]);
        assert_eq!(snapshot(&world), fixture["initial"]);
        for step in fixture["steps"].as_array().unwrap() {
            let op = &step["op"];
            let label = format!("{} {:?} {op}", fixture["name"], precision);
            let actor = op["actor"].as_u64().unwrap() as u32;
            let seconds = bits(&op["seconds"]);
            world.events.clear();
            let result = match op["kind"].as_str().unwrap() {
                "supply_tick" => {
                    supply_tick(&mut world, actor, seconds, precision).expect(&label);
                    Value::Null
                }
                "economy_tick" => {
                    economy_component_tick(&mut world, actor).expect(&label);
                    Value::Null
                }
                "in_supply" => json!(in_supply(&mut world, actor).expect(&label)),
                "provides" => json!(provides_supply(&mut world, actor).expect(&label)),
                other => panic!("{other}"),
            };
            let actual =
                json!({"actors": snapshot(&world), "events": world.events, "result": result});
            assert_eq!(actual, step["expected"], "{label}");
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
