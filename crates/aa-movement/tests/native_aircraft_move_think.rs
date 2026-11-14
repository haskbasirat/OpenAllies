//! 51A720 against original x86 executions (research/aircraft-move-state).
use aa_movement::aircraft_move_state::{MoveThinkWorld, think};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-move-state/native-aircraft-move-think.json"
    ))
    .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn words<const N: usize>(v: &Value) -> [u32; N] {
    std::array::from_fn(|i| word(&v[i]))
}

#[derive(Clone, Copy, PartialEq)]
enum Actor {
    Owner,
    Other,
}

struct State<'a> {
    doc: &'a Value,
    input: &'a Value,
    calls: Vec<Value>,
    destination: [u32; 3],
    commanded: [u32; 2],
    flight_14: u32,
    flight_84: u32,
}
impl State<'_> {
    fn flag(&self, name: &str) -> u8 {
        self.input[name].as_u64().unwrap() as u8
    }
    fn kind(&self, name: &str) -> &str {
        self.input[name].as_str().unwrap()
    }
}
impl MoveThinkWorld for State<'_> {
    type Error = ();
    type Group = ();
    type Actor = Actor;
    fn think_base(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["think_base"]));
        Ok(())
    }
    fn owner(&mut self) -> Result<Actor, ()> {
        Ok(Actor::Owner)
    }
    fn formation_byte(&mut self) -> Result<u8, ()> {
        Ok(self.flag("formation"))
    }
    fn mind_group(&mut self) -> Result<Option<()>, ()> {
        Ok(self.input["group"].as_bool().unwrap().then_some(()))
    }
    fn group_leader(&mut self, _: ()) -> Result<Option<Actor>, ()> {
        self.calls.push(json!(["leader"]));
        Ok(match self.kind("leader") {
            "self" => Some(Actor::Owner),
            "other" => Some(Actor::Other),
            _ => None,
        })
    }
    fn followed_gtid(&mut self) -> Result<u32, ()> {
        let gtid = word(&self.doc["gtid"]);
        Ok(match self.kind("gtid") {
            "zero" => 0,
            "stale" => gtid ^ 0x1_0000,
            _ => gtid,
        })
    }
    fn resolve_followed(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, ()> {
        Ok(gtid == word(&self.doc["gtid"]) && self.kind("gtid") != "null")
    }
    fn follow_target(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["follow_target"]));
        let [x, y] = words::<2>(&self.doc["followed"]);
        self.destination[0] = x;
        self.destination[1] = y;
        Ok(())
    }
    fn destination(&mut self) -> Result<[u32; 3], ()> {
        Ok(self.destination)
    }
    fn set_commanded(&mut self, xy: [u32; 2]) -> Result<(), ()> {
        self.commanded = xy;
        Ok(())
    }
    fn flight_byte_90(&mut self, actor: Actor) -> Result<u8, ()> {
        assert!(actor == Actor::Owner);
        Ok(self.flag("flight_90"))
    }
    fn flight_word_34(&mut self, actor: Actor) -> Result<u32, ()> {
        assert!(actor == Actor::Owner);
        Ok(u32::from(self.flag("flight_34")))
    }
    fn actor_gtid(&mut self, actor: Actor) -> Result<u32, ()> {
        assert!(actor == Actor::Owner);
        Ok(word(&self.doc["actor_gtid"]))
    }
    fn formation_offset(&mut self, _: (), gtid: u32) -> Result<[u32; 3], ()> {
        self.calls.push(json!(["offset", gtid]));
        Ok(words::<3>(&self.doc["offset"]))
    }
    fn has_direction(&mut self) -> Result<u8, ()> {
        Ok(self.flag("has_direction"))
    }
    fn fly_formation(&mut self, actor: Actor, offset: [u32; 3], destination: [u32; 3], with_direction: bool) -> Result<(), ()> {
        assert!(actor == Actor::Owner);
        self.calls.push(json!(["fly_formation", offset, destination, with_direction]));
        Ok(())
    }
    fn has_destination(&mut self) -> Result<u8, ()> {
        Ok(self.flag("has_destination"))
    }
    fn cruise(&mut self) -> Result<u32, ()> {
        Ok(word(&self.doc["cruise"]))
    }
    fn set_flight_speeds(&mut self, actor: Actor, bits: u32) -> Result<(), ()> {
        assert!(actor == Actor::Owner);
        self.flight_14 = bits;
        self.flight_84 = bits;
        Ok(())
    }
    fn fly_to(&mut self, actor: Actor, destination: [u32; 3], with_direction: bool) -> Result<(), ()> {
        assert!(actor == Actor::Owner);
        self.calls.push(json!(["fly_to", destination, with_direction, self.flight_14, self.flight_84]));
        self.destination[0] = word(&self.doc["flown"]);
        Ok(())
    }
}

#[test]
fn formation_follow_and_fly_paths_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4608);
    for row in rows {
        let mut s = State {
            doc: &doc,
            input: &row["input"],
            calls: vec![],
            destination: words::<3>(&doc["destination"]),
            commanded: [0x4444, 0x5555],
            flight_14: 0x1111,
            flight_84: 0x2222,
        };
        think(&mut s).unwrap();
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(s.commanded, words::<2>(&row["commanded"]), "{row}");
        assert_eq!(s.flight_14, word(&row["flight_14"]), "{row}");
        assert_eq!(s.flight_84, word(&row["flight_84"]), "{row}");
    }
}
