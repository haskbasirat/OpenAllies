//! Existing original4F1E10 oracle, replayed through exclusively owned state.
use aa_economy::statistics::{KingdomStatistics, RelationStatistics, Statistics};
use aa_economy::world_start::{OwnedWorldStart, WorldStart, world_start_owned};
use aa_economy::{EconomyError, F32Bits};
use serde_json::{Value, json};

struct Kingdom {
    statistics: KingdomStatistics,
    timer: F32Bits,
    snapshots: bool,
}
struct World<'a> {
    case: &'a Value,
    started: bool,
    fields: Vec<Option<u32>>,
    kingdoms: Vec<Option<Kingdom>>,
    events: Vec<Value>,
}
impl WorldStart for World<'_> {
    fn objects(&mut self) -> Result<Vec<u32>, EconomyError> {
        Ok((1..=self.fields.len() as u32)
            .filter(|&i| self.fields[i as usize - 1].is_some())
            .collect())
    }
    fn object_active(&mut self, object: u32) -> Result<bool, EconomyError> {
        self.events.push(json!({"kind":"active","object":object}));
        Ok(self.case["objects"][object as usize - 1]["active"]
            .as_bool()
            .unwrap())
    }
    fn field_13c(&mut self, actor: u32) -> Result<u32, EconomyError> {
        Ok(self.fields[actor as usize - 1].unwrap())
    }
    fn set_field_13c(&mut self, actor: u32, value: u32) -> Result<(), EconomyError> {
        self.fields[actor as usize - 1] = Some(value);
        Ok(())
    }
    fn owner(&mut self, actor: u32) -> Result<Option<usize>, EconomyError> {
        self.events.push(json!({"kind":"owner","object":actor}));
        Ok(self.case["objects"][actor as usize - 1]["owner"]
            .as_u64()
            .map(|i| i as usize))
    }
    fn kingdom_word_190(&mut self, kingdom: usize) -> Result<u32, EconomyError> {
        Ok(self.case["kingdoms"][kingdom]["word_190"].as_u64().unwrap() as u32)
    }
    fn statistics_track(&mut self, kingdom: usize, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind":"track","kingdom":kingdom,"object":actor}));
        Ok(())
    }
    fn statistics_snapshot(&mut self, kingdom: usize) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind":"snapshot","kingdom":kingdom}));
        self.kingdoms[kingdom].as_mut().unwrap().snapshots = true;
        Ok(())
    }
}
impl OwnedWorldStart for World<'_> {
    fn started(&mut self) -> Result<bool, EconomyError> {
        Ok(self.started)
    }
    fn set_started(&mut self, value: bool) -> Result<(), EconomyError> {
        self.started = value;
        Ok(())
    }
    fn kingdoms(&mut self) -> Result<Vec<Option<usize>>, EconomyError> {
        Ok(self
            .kingdoms
            .iter()
            .enumerate()
            .map(|(i, k)| k.as_ref().map(|_| i))
            .collect())
    }
    fn promote_and_reset_statistics(
        &mut self,
        kingdom: usize,
        count: usize,
    ) -> Result<(), EconomyError> {
        self.kingdoms[kingdom]
            .as_mut()
            .unwrap()
            .statistics
            .start_world(count);
        Ok(())
    }
    fn has_snapshots(&mut self, kingdom: usize) -> Result<bool, EconomyError> {
        Ok(self.kingdoms[kingdom].as_ref().unwrap().snapshots)
    }
    fn reset_snapshot_timer(&mut self, kingdom: usize) -> Result<(), EconomyError> {
        self.kingdoms[kingdom].as_mut().unwrap().timer = F32Bits::ZERO;
        Ok(())
    }
}
fn stats(v: &Value) -> Option<Statistics> {
    (!v.is_null()).then(|| Statistics {
        production: vec![F32Bits::ZERO; 3],
        scores: serde_json::from_value(v.clone()).unwrap(),
    })
}

#[test]
fn owned_world_start_matches_all_existing_original_fixtures() {
    let doc: Value =
        serde_json::from_str(include_str!("fixtures/native-world-start-fixtures.json")).unwrap();
    let mut count = 0;
    for f in doc["fixtures"].as_array().unwrap() {
        let c = &f["case"];
        let mut w = World {
            case: c,
            started: c["started"].as_bool().unwrap(),
            events: vec![],
            fields: c["objects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o["field_13c"].as_u64().map(|i| i as u32))
                .collect(),
            kingdoms: c["kingdoms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    (!k.is_null()).then(|| Kingdom {
                        statistics: KingdomStatistics {
                            active: stats(&k["active"]),
                            loaded: stats(&k["loaded"]),
                            relations: (!k["relations"].is_null())
                                .then(RelationStatistics::default),
                            loaded_relations: (!k["loaded_relations"].is_null())
                                .then(RelationStatistics::default),
                        },
                        timer: F32Bits(k["timer"].as_u64().unwrap() as u32),
                        snapshots: k["has_snapshots"].as_bool().unwrap(),
                    })
                })
                .collect(),
        };
        world_start_owned(3, &mut w).unwrap();
        let e = &f["expected"];
        assert_eq!(json!(w.started), e["started"]);
        assert_eq!(json!(w.fields), e["fields_13c"]);
        assert_eq!(json!(w.events), e["events"]);
        for (i, k) in w.kingdoms.iter().enumerate() {
            let Some(k) = k else {
                assert!(e["kingdoms"][i].is_null());
                continue;
            };
            assert_eq!(json!(k.timer), e["kingdoms"][i]["timer"]);
            assert_eq!(json!(k.snapshots), e["kingdoms"][i]["has_snapshots"]);
            if c["started"] == false {
                assert_eq!(
                    json!(k.statistics.active.as_ref().unwrap().scores),
                    e["kingdoms"][i]["active_scores"]
                );
                assert!(k.statistics.loaded.is_none());
                assert!(k.statistics.loaded_relations.is_none());
                assert!(k.statistics.relations.is_some());
            }
        }
        let before = json!({"fields":w.fields,"events":w.events});
        world_start_owned(3, &mut w).unwrap();
        assert_eq!(before, json!({"fields":w.fields,"events":w.events}));
        count += 1;
    }
    assert_eq!(count, 360);
}
