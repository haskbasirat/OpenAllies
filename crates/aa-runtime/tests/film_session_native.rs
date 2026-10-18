use aa_actors::ActorId;
use aa_runtime::film_session::{self, Output, Result, Roster};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

struct Players {
    rows: Vec<Value>,
    world: bool,
    reads: Rc<RefCell<Vec<usize>>>,
    names: Vec<Vec<u16>>,
}
impl Roster for Players {
    type Player = usize;
    fn count(&mut self) -> Result<usize> {
        Ok(self.rows.len())
    }
    fn player_at(&mut self, index: usize) -> Result<usize> {
        Ok(index)
    }
    fn id(&mut self, player: &usize) -> Result<u32> {
        Ok(self.rows[*player]["id"].as_u64().unwrap() as u32)
    }
    fn name_24(&mut self, player: &usize) -> Result<&[u16]> {
        Ok(&self.names[*player])
    }
    fn kingdom_28(&mut self, player: &usize) -> Result<Option<ActorId>> {
        self.reads.borrow_mut().push(*player);
        Ok((self.world && !self.rows[*player]["kingdom"].is_null())
            .then_some(ActorId::from_raw(100 + *player as u32)))
    }
    fn ai_implicit(&mut self, player: &usize) -> Result<(u8, u8)> {
        Ok((
            self.rows[*player]["ai"].as_u64().unwrap() as u8,
            self.rows[*player]["implicit"].as_u64().unwrap() as u8,
        ))
    }
}
struct Stream {
    rows: Vec<Value>,
    events: Vec<Value>,
    payload: Vec<u8>,
    compressed: u8,
    stop: Option<usize>,
    flushed: bool,
    attached: bool,
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
impl Stream {
    fn event(&mut self, value: Value) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err(value["name"].as_str().unwrap().into());
        }
        self.events.push(value);
        Ok(())
    }
}
impl Output for Stream {
    fn save_map_header_and_strings(&mut self) -> Result<()> {
        self.event(json!({"name":"header_strings"}))
    }
    fn compression_enabled(&mut self) -> Result<u8> {
        Ok(self.compressed)
    }
    fn open_chunk(&mut self, name: &str, compressed: u8) -> Result<()> {
        self.event(json!({"name":"chunk","chunk":name,"compressed":compressed}))
    }
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.event(json!({"name":"write","hex":hex(bytes)}))?;
        self.payload.extend_from_slice(bytes);
        Ok(())
    }
    fn kingdom_index_188(&mut self, kingdom: ActorId) -> Result<u32> {
        Ok(self.rows[(kingdom.raw() - 100) as usize]["kingdom"]
            .as_u64()
            .unwrap() as u32)
    }
    fn diagnostic_player(&mut self, ai: u8, implicit: u8, id: u32) -> Result<()> {
        self.event(json!({"name":"diagnostic","ai":ai,"implicit":implicit,"id":id}))
    }
    fn flush(&mut self) -> Result<()> {
        self.event(json!({"name":"flush"}))?;
        self.flushed = true;
        Ok(())
    }
    fn attach_recorder(&mut self) -> Result<()> {
        self.event(json!({"name":"attach"}))?;
        self.attached = true;
        Ok(())
    }
}

#[test]
fn original_v14_all_pc_roster_payload_and_required_service_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-session.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 123);
    for row in rows {
        let input = &row["input"];
        let players = input["players"].as_array().unwrap().clone();
        let reads = Rc::new(RefCell::new(Vec::new()));
        let mut roster = Players {
            rows: players.clone(),
            world: input["world_present"].as_bool().unwrap(),
            reads: Rc::clone(&reads),
            names: players
                .iter()
                .map(|p| {
                    p["name"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u16)
                        .collect()
                })
                .collect(),
        };
        let mut stream = Stream {
            rows: players,
            events: Vec::new(),
            payload: Vec::new(),
            compressed: input["compressed"].as_u64().unwrap() as u8,
            stop: input
                .get("stop")
                .and_then(Value::as_u64)
                .map(|n| n as usize),
            flushed: false,
            attached: false,
        };
        let stopped = film_session::open(&mut roster, &mut stream).err();
        let output = json!({"events":stream.events,"payload_hex":hex(&stream.payload),
            "kingdom_reads":*reads.borrow(),"flushed":stream.flushed,
            "attached":stream.attached,"stopped":stopped});
        assert_eq!(output, row["output"], "{}", row["pc"]);
    }
}
