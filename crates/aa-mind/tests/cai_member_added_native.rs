//! Replays research/phase8-ai/native-member-added.json (original 524B80 with
//! 4D0900 under Unicorn) against aa_mind::cai_member_added.
use aa_mind::{
    Arena, Mind, MindId, Result,
    cai_member_added::{MemberAddedWorld, member_added},
};
use serde_json::{Value, json};

struct Host {
    c: Value,
    list: Vec<u32>,
    events: Vec<Value>,
}

impl MemberAddedWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        // The oracle registers the objects in order: each writes its serial
        // at its index, and its pointer only when live.
        let (index, serial) = (gtid & 0xffff, gtid >> 16);
        let (mut serial_at, mut object) = (None, None);
        for (k, v) in self.c["gtids"].as_object().unwrap().values().enumerate() {
            if v[0].as_u64().unwrap() as u32 == index {
                serial_at = Some(v[1].as_u64().unwrap() as u32 & 0xffff);
                if v[2].as_bool().unwrap() {
                    object = Some(k as u32 + 1);
                }
            }
        }
        Ok(if serial_at == Some(serial) {
            object
        } else {
            None
        })
    }
    fn mind_word(&mut self, _mind: MindId, offset: u32, _arena: &Arena) -> Result<u32> {
        Ok(match offset {
            0x24 => self.c["word24"].as_u64().unwrap() as u32,
            0x28 => self.c["home"].as_u64().unwrap() as u32,
            _ => unreachable!(),
        })
    }
    fn member_mind_3c(&mut self, _member: u32, value: u32, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"op": "mind_3c", "value": value}));
        Ok(())
    }
    fn member_mind_40(
        &mut self,
        _member: u32,
        object: Option<u32>,
        _arena: &mut Arena,
    ) -> Result<()> {
        let names: Vec<&String> = self.c["gtids"].as_object().unwrap().keys().collect();
        let name = object.map(|k| names[k as usize - 1].clone());
        self.events.push(json!({"op": "mind_40", "object": name}));
        Ok(())
    }
    fn helper_522890(
        &mut self,
        _mind: MindId,
        _member: u32,
        flag: u32,
        _arena: &mut Arena,
    ) -> Result<()> {
        self.events.push(json!({"op": "522890", "flag": flag}));
        Ok(())
    }
    fn game_mode(&mut self) -> Result<u32> {
        Ok(self.c["game_mode"].as_u64().unwrap() as u32)
    }
    fn actor_gtid(&mut self, _actor: u32) -> Result<u32> {
        Ok(self.c["member_gtid"].as_u64().unwrap() as u32)
    }
    fn arrival_push_front(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<()> {
        self.list.insert(0, gtid);
        Ok(())
    }
}

#[test]
fn native_member_added_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-member-added.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let mut host = Host {
            c: c.clone(),
            list: c["list"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect(),
            events: vec![],
        };
        let mut arena = Arena::default();
        arena.minds.insert(MindId(1), Mind::base(1));
        member_added(&mut host, MindId(1), 7, &mut arena).unwrap();
        assert_eq!(json!(host.events), o["events"], "case {i}");
        assert_eq!(json!(host.list), o["list"], "case {i}");
    }
}
