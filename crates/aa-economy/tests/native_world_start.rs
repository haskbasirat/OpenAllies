//! Replays research/phase8-economy/native_world_start.py: the economy parts
//! of the original world start 0x4F1E10 (0x4E3690, 0x58EA00, 0x4EFBE0 walk,
//! 0x4CEEC0/0x4CEED0, 0x58F010, forced 0x4E37D0 snapshot).
use aa_economy::statistics::{KingdomStatistics, RelationStatistics, Statistics};
use aa_economy::world_start::*;
use aa_economy::{EconomyError, F32Bits};
use serde_json::{Value, json};

struct World<'a> {
    case: &'a Value,
    fields: Vec<Option<u32>>,
    events: Vec<Value>,
}
impl WorldStart for World<'_> {
    fn objects(&mut self) -> Result<Vec<u32>, EconomyError> {
        Ok((1..=self.fields.len() as u32)
            .filter(|&j| self.fields[j as usize - 1].is_some())
            .collect())
    }
    fn object_active(&mut self, object: u32) -> Result<bool, EconomyError> {
        self.events
            .push(json!({"kind": "active", "object": object}));
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
        self.events.push(json!({"kind": "owner", "object": actor}));
        Ok(self.case["objects"][actor as usize - 1]["owner"]
            .as_u64()
            .map(|k| k as usize))
    }
    fn kingdom_word_190(&mut self, kingdom: usize) -> Result<u32, EconomyError> {
        Ok(self.case["kingdoms"][kingdom]["word_190"].as_u64().unwrap() as u32)
    }
    fn statistics_track(&mut self, kingdom: usize, actor: u32) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "track", "kingdom": kingdom, "object": actor}));
        Ok(())
    }
    fn statistics_snapshot(&mut self, kingdom: usize) -> Result<(), EconomyError> {
        self.events
            .push(json!({"kind": "snapshot", "kingdom": kingdom}));
        Ok(())
    }
}

fn statistics(v: &Value) -> Option<Statistics> {
    (!v.is_null()).then(|| {
        let mut s = Statistics::new(3);
        s.scores = serde_json::from_value(v.clone()).unwrap();
        s
    })
}

#[test]
fn world_start_matches_original_x86() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/native-world-start-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let label = format!("{} {}", fixture["name"], fixture["precision"]);
        let source = case["kingdoms"].as_array().unwrap();
        let mut stats: Vec<Option<KingdomStatistics>> = source
            .iter()
            .map(|k| {
                (!k.is_null()).then(|| KingdomStatistics {
                    active: statistics(&k["active"]),
                    loaded: statistics(&k["loaded"]),
                    relations: (!k["relations"].is_null()).then(RelationStatistics::default),
                    loaded_relations: (!k["loaded_relations"].is_null())
                        .then(RelationStatistics::default),
                })
            })
            .collect();
        let mut timers: Vec<F32Bits> = source
            .iter()
            .map(|k| F32Bits(k["timer"].as_u64().unwrap_or(0) as u32))
            .collect();
        let mut kingdoms: Vec<Option<StartingKingdom<'_>>> = stats
            .iter_mut()
            .zip(timers.iter_mut())
            .zip(source)
            .map(|((s, t), k)| {
                s.as_mut().map(|statistics| StartingKingdom {
                    statistics,
                    snapshot_timer: t,
                    has_snapshots: k["has_snapshots"].as_bool().unwrap(),
                })
            })
            .collect();
        let mut world = World {
            case,
            fields: case["objects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o["field_13c"].as_u64().map(|f| f as u32))
                .collect(),
            events: vec![],
        };
        let mut started = case["started"].as_bool().unwrap();
        let was_started = started;
        world_start(&mut started, &mut kingdoms, 3, &mut world).expect(&label);
        let snapshots: Vec<Option<bool>> = kingdoms
            .iter()
            .map(|k| k.as_ref().map(|k| k.has_snapshots))
            .collect();
        drop(kingdoms);
        let expected = &fixture["expected"];
        assert_eq!(started, expected["started"].as_bool().unwrap(), "{label}");
        assert_eq!(json!(world.events), expected["events"], "{label}");
        assert_eq!(json!(world.fields), expected["fields_13c"], "{label}");
        for (i, native) in expected["kingdoms"].as_array().unwrap().iter().enumerate() {
            let Some(k) = &stats[i] else {
                assert!(native.is_null());
                continue;
            };
            assert_eq!(json!(timers[i].0), native["timer"], "{label}");
            assert_eq!(
                json!(snapshots[i].unwrap()),
                native["has_snapshots"],
                "{label}"
            );
            if was_started {
                continue; // the run returned before touching the objects
            }
            assert_eq!(
                json!(k.active.as_ref().unwrap().scores),
                native["active_scores"],
                "{label}"
            );
            assert!(k.loaded.is_none() && native["loaded"] == 0, "{label}");
            assert!(
                k.relations.is_some() && native["relations"] == true,
                "{label}"
            );
            assert!(k.loaded_relations.is_none() && native["loaded_relations"] == 0);
        }
        executions += 1;
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
