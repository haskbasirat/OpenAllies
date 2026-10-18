use aa_actors::ActorId;
use aa_mapworld::header::{HeaderEntry, MapHeader, WorldParameters};
use aa_runtime::film_header_entries::{self, Players, Result, World};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
struct Events {
    rows: Vec<Value>,
    stop: Option<usize>,
}
impl Events {
    fn event(&mut self, name: &str) -> Result<()> {
        if self.stop == Some(self.rows.len()) {
            return Err(name.into());
        }
        self.rows.push(json!({"name":name}));
        Ok(())
    }
}
struct Roster {
    input: Value,
    events: Rc<RefCell<Events>>,
    names: Vec<Vec<u16>>,
    display: Vec<Vec<u16>>,
}
impl Roster {
    fn event(&self, name: &str) -> Result<()> {
        self.events.borrow_mut().event(name)
    }
}
impl Players for Roster {
    type Player = usize;
    fn first(&mut self) -> Result<Option<usize>> {
        self.event("first")?;
        Ok((!self.names.is_empty()).then_some(0))
    }
    fn next(&mut self, p: &usize) -> Result<Option<usize>> {
        self.event("next")?;
        Ok((p + 1 < self.names.len()).then_some(p + 1))
    }
    fn ai_byte(&mut self, p: &usize) -> Result<u8> {
        self.event("ai")?;
        Ok(self.input["players"][*p]["ai"].as_u64().unwrap() as u8)
    }
    fn kingdom_765e00(&mut self, p: &usize) -> Result<Option<ActorId>> {
        self.event("kingdom")?;
        Ok((self.input["world"].as_bool().unwrap()
            && !self.input["players"][*p]["kingdom"].is_null())
        .then_some(ActorId::from_raw(100 + *p as u32)))
    }
    fn name_24(&mut self, p: &usize) -> Result<&[u16]> {
        self.event("name")?;
        Ok(&self.names[*p])
    }
    fn display_name_08(&mut self, p: &usize) -> Result<&[u16]> {
        self.event("display")?;
        Ok(&self.display[*p])
    }
    fn is_local(&mut self, p: &usize) -> Result<bool> {
        self.event("local")?;
        Ok(self.input["local"].as_u64() == Some(*p as u64))
    }
}
struct Current {
    input: Value,
    events: Rc<RefCell<Events>>,
}
impl World for Current {
    fn kingdom_name_18(&mut self, id: ActorId) -> Result<Vec<u16>> {
        self.events.borrow_mut().event("kingdom_name")?;
        Ok(self.input["players"][(id.raw() - 100) as usize]["kingdom"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect())
    }
    fn ai_profile_ids(&mut self, id: Option<ActorId>) -> Result<Option<Vec<u16>>> {
        self.events.borrow_mut().event("profile")?;
        let Some(id) = id else { return Ok(None) };
        Ok(self.input["players"][(id.raw() - 100) as usize]["profiles"]
            .as_array()
            .unwrap()
            .first()
            .and_then(Value::as_str)
            .map(|s| s.encode_utf16().collect()))
    }
}
#[test]
fn original_v14_all_pc_actual_entries_ai_first_match_and_failure_prefixes() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-entries.json"
    ))
    .unwrap();
    let seed: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-header.json"
    ))
    .unwrap();
    let template: MapHeader = serde_json::from_value(seed["cases"][3]["input"].clone()).unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 84);
    for row in cases {
        let input = &row["input"];
        let rows = input["players"].as_array().unwrap();
        let mut parameters = WorldParameters {
            version: 13,
            teams: vec![],
            kingdoms: vec![],
        };
        for player in rows {
            if !player["kingdom"].is_null() && player["parameter"].as_bool().unwrap() {
                let mut param = template.parameters.kingdoms[0].clone();
                param.id = player["kingdom"].as_str().unwrap().encode_utf16().collect();
                param.string_2c = "old_profile".encode_utf16().collect();
                parameters.kingdoms.push(param);
            }
        }
        let events = Rc::new(RefCell::new(Events {
            rows: vec![],
            stop: input["stop"].as_u64().map(|n| n as usize),
        }));
        let mut players = Roster {
            input: input.clone(),
            events: events.clone(),
            names: rows
                .iter()
                .map(|p| p["name"].as_str().unwrap().encode_utf16().collect())
                .collect(),
            display: rows
                .iter()
                .map(|p| p["display"].as_str().unwrap().encode_utf16().collect())
                .collect(),
        };
        let mut world = Current {
            input: input.clone(),
            events: events.clone(),
        };
        let mut entries = vec![HeaderEntry {
            flag: 0xff,
            string_20: vec![],
            string_18: vec![],
            flag_v5: Some(0xff),
        }];
        let result =
            film_header_entries::capture(&mut parameters, &mut entries, &mut players, &mut world);
        let entries:Vec<_>=entries.iter().map(|e|json!({"flag":e.flag,"string_20":String::from_utf16(&e.string_20).unwrap(),"string_18":String::from_utf16(&e.string_18).unwrap(),"local":e.flag_v5.unwrap()})).collect();
        let profiles: Vec<_> = parameters
            .kingdoms
            .iter()
            .map(|p| String::from_utf16(&p.string_2c).unwrap())
            .collect();
        assert_eq!(
            json!({"events":events.borrow().rows,"stopped":result.err(),"entries":entries,"profiles":profiles}),
            row["output"],
            "{} {}",
            row["input"],
            row["pc"]
        );
    }
}
