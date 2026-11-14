//! Replays research/phase8-combat/native-settlement-pieces-fixtures.json:
//! original x86 execution of the Settlement piece creation 586DB0, the
//! health copy 5850F0 and the 4CEF70 broadcast 585040 under PC24/53/64.
//! Every recorded callback with its arguments (the whole creation block
//! included) and every return value must match.
use aa_combat::settlement_tick::{Ref, Settlement};
use aa_combat::settlement_walls::*;
use aa_combat::{Error, Result};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn document() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-settlement-pieces-fixtures.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
}
fn f2(v: &Value) -> [f32; 2] {
    [f(&v[0]), f(&v[1])]
}

struct World<'a> {
    c: &'a Value,
    keys: Vec<String>,
    events: Vec<Value>,
}

impl<'a> World<'a> {
    fn new(c: &'a Value) -> Self {
        let mut keys = vec![String::new(), "actor".into(), "settlement_def".into()];
        for k in c["piece_defs"].as_array().unwrap() {
            keys.push(k.as_str().unwrap().into());
        }
        for i in 0..c["styles"].as_array().unwrap().len() {
            keys.push(format!("style{i}"));
        }
        for k in c["objects"].as_object().unwrap().keys() {
            keys.push(k.clone());
            keys.push(format!("{k}:body"));
        }
        Self { c, keys, events: Vec::new() }
    }
    fn id(&self, key: &str) -> Ref {
        self.keys.iter().position(|k| k == key).unwrap_or_else(|| panic!("{key}")) as Ref
    }
    fn opt(&self, v: &Value) -> Ref {
        v.as_str().map_or(0, |k| self.id(k))
    }
    fn key(&self, r: Ref) -> Value {
        if r == 0 { json!(0) } else { json!(self.keys[r as usize]) }
    }
    fn name(&self, r: Ref) -> &str {
        &self.keys[r as usize]
    }
    fn object(&self, r: Ref) -> &'a Value {
        &self.c["objects"][self.name(r)]
    }
}

impl WallWorld for World<'_> {
    fn definition(&mut self, actor: Ref) -> Result<Ref> {
        Ok(match self.name(actor) {
            "actor" => self.id("settlement_def"),
            _ => self.opt(&self.object(actor)["def"]),
        })
    }
    fn piece_count(&mut self) -> Result<i32> {
        Ok(self.c["pieces"].as_array().unwrap().len() as i32)
    }
    fn piece(&mut self, slot: i32) -> Result<Ref> {
        Ok(self.opt(&self.c["pieces"][usize::try_from(slot).unwrap()]))
    }
    fn definition_4a9(&mut self, _: Ref) -> Result<u8> {
        Ok(u(&self.c["byte_4a9"]) as u8)
    }
    fn style(&mut self, _: Ref, index: i32) -> Result<Ref> {
        Ok(self.id(&format!("style{index}")))
    }
    fn slot_kind(&mut self, _: Ref, slot: i32) -> Result<u32> {
        Ok(u(&self.c["slots"][usize::try_from(slot).unwrap()]["kind"]))
    }
    fn style_definitions(&mut self, style: Ref) -> Result<[Ref; 3]> {
        let i: usize = self.name(style).strip_prefix("style").unwrap().parse().unwrap();
        let s = &self.c["styles"][i];
        Ok([self.opt(&s[0]), self.opt(&s[1]), self.opt(&s[2])])
    }
    fn component_9c(&mut self, _: Ref) -> Result<Ref> {
        Err(Error::World("not read by these functions".into()))
    }
    fn upgrade(&mut self, _: Ref, _: Ref) -> Result<()> {
        Err(Error::World("not called by these functions".into()))
    }
    fn create_piece(&mut self, _: i32, _: u32, _: u32) -> Result<Ref> {
        Err(Error::World("not called by these functions".into()))
    }
    fn report(&mut self, _: Report) -> Result<()> {
        Err(Error::World("not called by these functions".into()))
    }
}

impl PieceWorld for World<'_> {
    fn slot_entry(&mut self, _: Ref, slot: i32) -> Result<SlotEntry> {
        let e = &self.c["slots"][usize::try_from(slot).unwrap()];
        Ok(SlotEntry { kind: u(&e["kind"]), offset: f2(&e["offset"]), facing: f2(&e["facing"]) })
    }
    fn position(&mut self, actor: Ref) -> Result<[f32; 3]> {
        assert_eq!(self.name(actor), "actor");
        let p = &self.c["position"];
        Ok([f(&p[0]), f(&p[1]), f(&p[2])])
    }
    fn kingdom(&mut self, _: Ref) -> Result<Ref> {
        self.events.push(json!(["kingdom"]));
        Ok(u(&self.c["kingdom"]))
    }
    fn handle_of(&mut self, actor: Ref) -> Result<u32> {
        assert_eq!(self.name(actor), "actor");
        Ok(u(&self.c["actor_handle"]))
    }
    fn create(&mut self, p: &PieceParams) -> Result<Ref> {
        let block = json!([
            self.key(p.definition),
            p.position[0].to_bits(), p.position[1].to_bits(), p.position[2].to_bits(),
            p.facing[0].to_bits(), p.facing[1].to_bits(),
            "empty", p.variant, 0, p.kingdom, 0, 0, p.word_34, p.mode, 0, 0, p.parent, 0, 0, 0, 0, 0
        ]);
        self.events.push(json!(["create", block]));
        Ok(self.opt(&self.c["created"]))
    }
    fn attach_piece(&mut self, slot: i32, piece: Ref, second: u32) -> Result<()> {
        self.events.push(json!(["attach_piece", slot as u32, self.key(piece), second]));
        Ok(())
    }
    fn definition_4a8(&mut self, _: Ref) -> Result<u8> {
        Ok(u(&self.c["byte_4a8"]) as u8)
    }
    fn body(&mut self, actor: Ref) -> Result<Option<Ref>> {
        let k = self.name(actor).to_string();
        Ok(if self.c["objects"][&k]["body"].is_null() { None } else { Some(self.id(&format!("{k}:body"))) })
    }
    fn body_health(&mut self, body: Ref) -> Result<u32> {
        let k = self.name(body).strip_suffix(":body").unwrap().to_string();
        Ok(u(&self.c["objects"][&k]["body"]))
    }
    fn set_health(&mut self, body: Ref, health: u32) -> Result<()> {
        self.events.push(json!(["set_health", self.key(body), health, 0, 0, 0]));
        Ok(())
    }
    fn holder(&mut self) -> Result<Ref> {
        Ok(self.opt(&self.c["holder"]))
    }
    fn member_count(&mut self) -> Result<i32> {
        Ok(self.c["members"].as_array().unwrap().len() as i32)
    }
    fn member(&mut self, index: i32) -> Result<Ref> {
        Ok(self.opt(&self.c["members"][usize::try_from(index).unwrap()]))
    }
    fn word_78(&mut self, object: Ref) -> Result<u32> {
        Ok(u(&self.object(object)["word_78"]))
    }
    fn notify(&mut self, object: Ref, value: u32, actor: Ref) -> Result<()> {
        self.events.push(json!(["notify", self.key(object), value, self.key(actor)]));
        Ok(())
    }
}

#[test]
fn settlement_pieces_match_native() {
    let d = document();
    // Unvisited: 586DB0's string-free path (4) and an alignment filler in 585040.
    for (k, n) in [("586DB0", 121), ("5850F0", 42), ("585040", 71)] {
        assert_eq!(d["coverage"][k], n, "{k}");
    }
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    let mut created = 0;
    for case in cases {
        let (c, o) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut w = World::new(c);
        let s = Settlement {
            actor: w.id("actor"),
            style_index: u(&c["style_index"]),
            last_periodic: 0.0,
            rotation: 0,
            forwards: 0,
        };
        match c["operation"].as_str().unwrap() {
            "create" => {
                let a = &c["args"];
                let piece = create_piece(&s, a[0].as_i64().unwrap() as i32, u(&a[1]) as u8, u(&a[2]), precision, &mut w)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(w.key(piece), o["result"], "{name} result");
                created += usize::from(piece != 0);
            }
            "share" => {
                let piece = w.id(c["piece"].as_str().unwrap());
                share_health(&s, piece, &mut w).unwrap_or_else(|e| panic!("{name}: {e}"));
            }
            "notify" => {
                notify_all(&s, u(&c["event_value"]), u(&c["event_handled"]) as u8, &mut w)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
            }
            other => panic!("{other}"),
        }
        assert_eq!(Value::Array(w.events), o["events"], "{name} events");
    }
    assert!(created > 30, "{created}");
}
