use aa_actors::ActorId;
use aa_runtime::live_registration::{self, Result, State, World};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Storage {
    case: Value,
    current: u32,
    records: BTreeMap<u32, Value>,
    flags: BTreeMap<u32, u32>,
    trace: Vec<Value>,
}
fn w(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
impl Storage {
    fn new(case: &Value) -> Self {
        let records: BTreeMap<_, _> = case["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (w(&r["id"]), r.clone()))
            .collect();
        let flags = records
            .iter()
            .map(|(&id, r)| (id, r.get("flags").map(w).unwrap_or(0x8100_0000)))
            .collect();
        Self {
            case: case.clone(),
            current: 0,
            records,
            flags,
            trace: vec![],
        }
    }
    fn event(&mut self, label: &str, arg: Option<u32>) -> Result<()> {
        self.trace
            .push(arg.map_or_else(|| json!([label]), |arg| json!([label, arg])));
        if self.case["stop"].as_u64() == Some(self.trace.len() as u64) {
            return Err("stopped".into());
        }
        Ok(())
    }
    fn row(&self) -> &Value {
        &self.records[&self.current]
    }
    fn required(&self) -> Option<u32> {
        let id = self.row()["required"].as_u64().unwrap_or(0) as u32;
        self.records
            .get(&id)
            .filter(|r| r["generation"].as_u64().unwrap_or((id >> 16) as u64) == (id >> 16) as u64)
            .map(|_| id)
    }
    fn output(&self, state: &State, outcome: &str) -> Value {
        let mut names = BTreeMap::new();
        for r in self.records.values() {
            let name = r["name"].as_str().unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let entries = state
                .names()
                .entries(&name.encode_utf16().collect::<Vec<_>>())
                .unwrap();
            if !entries.is_empty() {
                names.insert(name.to_ascii_uppercase(),entries.iter().map(|entry|json!({"name":String::from_utf16(&entry.name).unwrap(),"id":entry.id.raw()})).collect::<Vec<_>>());
            }
        }
        json!({"outcome":outcome,"trace":self.trace,
            "flags":self.case["records"].as_array().unwrap().iter().map(|r|json!([r["id"],self.flags[&w(&r["id"])]] )).collect::<Vec<_>>(),
            "names":names,"areas":state.areas().iter().map(|id|id.raw()).collect::<Vec<_>>(),
            "cameras":state.cameras().iter().map(|id|id.raw()).collect::<Vec<_>>(),
            "teams":state.teams().iter().map(|id|id.map(ActorId::raw)).collect::<Vec<_>>(),
            "available":state.available_cities().iter().map(|id|id.raw()).collect::<Vec<_>>(),"latch":state.duplicate_assert_latch()})
    }
}
impl World for Storage {
    fn capture_name(&mut self) -> Result<Vec<u16>> {
        self.event("name", None)?;
        Ok(self.row()["name"]
            .as_str()
            .unwrap_or("")
            .encode_utf16()
            .collect())
    }
    fn clear_loading_flag(&mut self) -> Result<()> {
        self.event("clear", None)?;
        *self.flags.get_mut(&self.current).unwrap() &= 0x7fff_ffff;
        Ok(())
    }
    fn id(&mut self) -> Result<ActorId> {
        Ok(ActorId::from_raw(self.current))
    }
    fn predicate(&mut self, offset: u32) -> Result<bool> {
        self.event("predicate", Some(offset))?;
        live_registration::class_predicate(w(&self.row()["class"]), offset)
    }
    fn team_slot(&mut self) -> Result<i32> {
        self.event("team", None)?;
        Ok(self.row()["team"].as_i64().unwrap_or(0) as i32)
    }
    fn register_actor(&mut self) -> Result<()> {
        self.event("actor", None)?;
        if let Some(changes) = self.row()["after_actor"].as_object().cloned() {
            for (key, value) in changes {
                self.records.get_mut(&self.current).unwrap()[key] = value;
            }
        }
        Ok(())
    }
    fn settlement_space(&mut self) -> Result<bool> {
        self.event("source", None)?;
        Ok(self.row()["space"].as_bool().unwrap_or(false))
    }
    fn city_present(&mut self) -> Result<bool> {
        self.event("city", None)?;
        Ok(self.row()["city"].as_bool().unwrap_or(false))
    }
    fn required_city_live(&mut self) -> Result<bool> {
        self.event("required_live", None)?;
        Ok(self.required().is_some())
    }
    fn required_city_settlement_space(&mut self) -> Result<bool> {
        self.event("required_source", None)?;
        Ok(self.records[&self.required().unwrap()]["space"]
            .as_bool()
            .unwrap_or(false))
    }
    fn assert_duplicate_available_city(&mut self) -> Result<bool> {
        self.event("assert", None)?;
        Ok(self.case["assert_return"].as_bool().unwrap_or(false))
    }
}

#[test]
fn registration_order_names_classes_cities_and_retained_prefix_match_native() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-registration/native-registration.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 414);
    for row in cases {
        let case = &row["input"];
        let slots = case["teams"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().map(|v| ActorId::from_raw(v as u32)))
            .collect();
        let mut state = State::from_world_constructor(slots, case["latch"].as_bool().unwrap());
        let mut world = Storage::new(case);
        let mut outcome = "ok";
        for id in case["operations"].as_array().unwrap() {
            world.current = w(id);
            if let Err(error) = state.register(&mut world) {
                outcome = if error == "stopped" {
                    "stopped"
                } else if error.contains("m_all_teams") {
                    "duplicate_team"
                } else {
                    panic!("{error}")
                };
                break;
            }
        }
        assert_eq!(
            world.output(&state, outcome),
            row["output"],
            "{} / {} / {}",
            case["name"],
            row["image"],
            row["precision"]
        );
    }
}

#[test]
fn original_class_leaf_pins_are_exact() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-registration/native-registration.json"
    ))
    .unwrap();
    for pin in fixture["class_pins"].as_array().unwrap() {
        let value =
            live_registration::class_predicate(w(&pin["class"]), w(&pin["offset"])).unwrap();
        assert_eq!(value, pin["bytes"] == "b001c3");
    }
    assert!(live_registration::class_predicate(8, 0xbc).is_err());
}

#[test]
fn serialized_team_array_initialization_matches_native_without_capacity_cap() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-registration/native-registration.json"
    ))
    .unwrap();
    let rows = fixture["team_initialization"].as_array().unwrap();
    assert_eq!(rows.len(), 36);
    for row in rows {
        let state = State::from_saved_team_count(w(&row["count"]), true).unwrap();
        assert_eq!(
            state
                .teams()
                .iter()
                .map(|id| id.map_or(0, ActorId::raw))
                .collect::<Vec<_>>(),
            serde_json::from_value::<Vec<u32>>(row["initialized"].clone()).unwrap()
        );
        assert!(state.duplicate_assert_latch());
    }
    assert!(State::from_saved_team_count(u32::MAX, false).is_err());
}
