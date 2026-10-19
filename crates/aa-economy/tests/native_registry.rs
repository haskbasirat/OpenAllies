//! Replays research/phase8-economy/native_registry.py: original GKingdom
//! AddActor/RemoveActor and the recruit item table.
use aa_economy::kingdom_registry::*;
use aa_economy::statistics::Statistics;
use aa_economy::tick::RecruitSingle;
use aa_economy::{EconomyError, F32Bits, X87Precision};
use serde_json::{Value, json};

struct World<'a> {
    case: &'a Value,
    actors: Vec<RegistryActor>,
    statistics: Statistics,
    precision: X87Precision,
    events: Vec<Value>,
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl World<'_> {
    fn a(&self, actor: u32) -> &Value {
        &self.case["actors"][actor as usize]
    }
    fn item_of(&self, definition: u32) -> usize {
        self.case["items"]
            .as_array()
            .unwrap()
            .iter()
            .position(|i| u(&i["definition"]) == definition)
            .unwrap()
    }
    fn resolve(gtid: u32) -> Option<u32> {
        let slot = gtid & 0xffff;
        // Lazy: a null origin (0, slot 0) must not evaluate slot - 1.
        (gtid >> 16 == 5 && (1..=8).contains(&slot)).then(|| slot - 1)
    }
}
impl RegistryWorld for World<'_> {
    fn actor(&self, actor: u32) -> Result<RegistryActor, EconomyError> {
        Ok(self.actors[actor as usize].clone())
    }
    fn set_flags8(&mut self, actor: u32, flags: u32) -> Result<(), EconomyError> {
        self.actors[actor as usize].flags8 = flags;
        Ok(())
    }
    fn structure_excluded(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.events
            .push(json!({"kind": "structure_excluded", "actor": actor}));
        Ok(b(&self.a(actor)["excluded"]))
    }
    fn economy_add(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "economy_add", "actor": actor}));
        Ok(())
    }
    fn economy_remove(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "economy_remove", "actor": actor}));
        Ok(())
    }
    fn refresh_organization(&mut self, organization: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "refresh_organization", "actor": organization}));
        Ok(())
    }
    fn counted(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.events.push(json!({"kind": "counted", "actor": actor}));
        Ok(b(&self.a(actor)["counted"]))
    }
    fn statistics_present(&mut self) -> Result<bool, EconomyError> {
        Ok(b(&self.case["world"]["statistics"]))
    }
    fn statistics_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "statistics_track", "actor": actor, "add": add}));
        Ok(())
    }
    fn statistics_classified(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.events
            .push(json!({"kind": "statistics_classified", "actor": actor}));
        Ok(b(&self.a(actor)["classified"]))
    }
    fn add_score(&mut self, index: usize, amount_bits: u32) -> Result<(), EconomyError> {
        self.statistics
            .add_score(index, F32Bits(amount_bits), self.precision)
    }
    fn ownership_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "ownership_track", "actor": actor, "add": add}));
        Ok(())
    }
    fn property_records(&mut self) -> Result<Vec<PropertyApplication>, EconomyError> {
        Ok(self.case["world"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| PropertyApplication {
                definition: u(&r["definition"]),
                source: u(&r["source"]),
                origin: Self::resolve(u(&r["origin"])),
            })
            .collect())
    }
    fn apply_property(&mut self, actor: u32, r: PropertyApplication) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "apply_property", "actor": actor,
            "definition": r.definition, "source": r.source, "origin": r.origin}));
        Ok(())
    }
    fn remove_property_effect(
        &mut self,
        actor: u32,
        r: PropertyApplication,
    ) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "remove_property_effect", "actor": actor,
            "definition": r.definition, "origin": r.origin}));
        Ok(())
    }
    fn structure_properties(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError> {
        Ok(self.a(actor)["research"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect())
    }
    fn add_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError> {
        self.events.push(
            json!({"kind": "add_kingdom_property", "definition": definition, "actor": actor}),
        );
        Ok(())
    }
    fn remove_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError> {
        self.events.push(
            json!({"kind": "remove_kingdom_property", "definition": definition, "actor": actor}),
        );
        Ok(())
    }
    fn max_flares(&mut self) -> Result<i32, EconomyError> {
        Ok(self.case["world"]["max_flares"].as_i64().unwrap() as i32)
    }
    fn flare_time(&mut self, actor: u32) -> Result<u32, EconomyError> {
        Ok(u(&self.a(actor)["flare_time"]))
    }
    fn destroy_actor(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "destroy_actor", "actor": actor}));
        Ok(())
    }
}
impl RecruitAi for World<'_> {
    fn ai_enabled(&mut self) -> Result<bool, EconomyError> {
        Ok(b(&self.case["world"]["ai"]))
    }
    fn ai_listeners(&mut self) -> Result<Vec<u32>, EconomyError> {
        Ok((0..u(&self.case["world"]["listeners"])).collect())
    }
    fn notify_ai(
        &mut self,
        listener: u32,
        single: &RecruitSingle,
        added: bool,
    ) -> Result<(), EconomyError> {
        let item = self.item_of(single.definition);
        self.events
            .push(json!({"kind": "notify_ai", "listener": listener, "item": item, "added": added}));
        Ok(())
    }
}

fn registry_json(r: &KingdomRegistry, world: &World) -> Value {
    let classes: serde_json::Map<String, Value> = r
        .class_lists
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.to_string(), json!(v)))
        .collect();
    json!({"class_lists": classes, "supply": r.supply, "structures": r.structures,
        "settlements": r.settlements, "hqs": r.hqs, "organizations": r.organizations,
        "counted": r.counted, "flares": r.flares, "capital": r.capital, "word_190": r.word_190,
        "flags8": world.actors.iter().map(|a| a.flags8).collect::<Vec<_>>(),
        "score14": world.statistics.scores[14]})
}

fn table_json(t: &RecruitItemTable, world: &World) -> Value {
    let mut entries: Vec<(u32, bool)> = t.entries().collect();
    entries.sort();
    let singles: Vec<Value> = t
        .singles
        .iter()
        .map(|s| {
            json!({"definition": s.definition, "available": s.available, "actor": s.actor,
            "organization": s.organization, "progress": s.progress,
            "item": world.item_of(s.definition)})
        })
        .collect();
    json!({"entries": entries, "singles": singles, "changes": t.changes})
}

#[test]
fn registry_matches_original_x86() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/native-registry-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let precision: X87Precision = serde_json::from_value(fixture["precision"].clone()).unwrap();
        let actors = case["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| RegistryActor {
                class_index: u(&a["class_index"]),
                class_kind: u(&a["class_kind"]),
                flags8: u(&a["flags8"]),
                states: a["states"].as_u64().unwrap(),
                economy: b(&a["economy"]),
                structure: b(&a["structure"]),
                settlement: b(&a["settlement"]),
                hq: b(&a["hq"]),
                organization: b(&a["organization"]),
                supply: b(&a["supply"]),
                properties: b(&a["properties"]),
                component_ac: b(&a["component_ac"]),
                flare: b(&a["flare"]),
            })
            .collect();
        let mut world = World {
            case,
            actors,
            statistics: Statistics::new(0),
            precision,
            events: vec![],
        };
        let mut registry = KingdomRegistry {
            word_190: u(&case["world"]["word_190"]),
            ..KingdomRegistry::default()
        };
        let mut table = RecruitItemTable::default();
        assert_eq!(
            registry_json(&registry, &world),
            fixture["initial"]["registry"]
        );
        for step in fixture["steps"].as_array().unwrap() {
            let op = &step["op"];
            let label = format!("{} {:?} {op}", fixture["name"], precision);
            world.events.clear();
            let mut result = Value::Null;
            match op["kind"].as_str().unwrap() {
                "add_actor" => add_actor(&mut registry, u(&op["actor"]), &mut world).expect(&label),
                "remove_actor" => {
                    remove_actor(&mut registry, u(&op["actor"]), &mut world).expect(&label)
                }
                "add_item" => {
                    let item = &case["items"][op["item"].as_u64().unwrap() as usize];
                    if b(&item["single"]) {
                        let single = RecruitSingle {
                            definition: u(&item["definition"]),
                            available: b(&item["available"]),
                            actor: u(&item["actor"]),
                            organization: u(&item["organization"]),
                            progress: F32Bits(u(&item["progress"])),
                            regeneration_rate: F32Bits::ZERO,
                        };
                        table.add_single(single, &mut world).expect(&label);
                    } else {
                        table.add_plain(u(&item["definition"]));
                    }
                }
                "remove_item" => {
                    let item = &case["items"][op["item"].as_u64().unwrap() as usize];
                    let index = table
                        .singles
                        .iter()
                        .position(|s| s.definition == u(&item["definition"]))
                        .unwrap();
                    table
                        .remove_single(index, b(&op["force"]), &mut world)
                        .expect(&label);
                }
                "find_single" => {
                    result = match table.find_single(u(&op["definition"]), b(&op["available_only"]))
                    {
                        Some(index) => json!(world.item_of(table.singles[index].definition)),
                        None => Value::Null,
                    };
                }
                other => panic!("{other}"),
            }
            let actual = json!({"registry": registry_json(&registry, &world),
                "table": table_json(&table, &world), "events": world.events, "result": result});
            assert_eq!(actual, step["expected"], "{label}");
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
