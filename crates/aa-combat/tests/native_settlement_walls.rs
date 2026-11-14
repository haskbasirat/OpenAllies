//! Replays research/phase8-combat/native-settlement-walls-fixtures.json:
//! original x86 execution of the Settlement wall piece helpers 5856F0,
//! 585BF0, UpgradeFullWall 585C60 and CreateFullWall 587210. Every recorded
//! callback with its arguments and every return value must match.
use aa_combat::Result;
use aa_combat::settlement_tick::{Ref, Settlement};
use aa_combat::settlement_walls::*;
use serde_json::{Value, json};

fn document() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-settlement-walls-fixtures.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}

struct World<'a> {
    c: &'a Value,
    keys: Vec<String>,
    events: Vec<Value>,
    created: usize,
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
        for k in c["actors"].as_object().unwrap().keys() {
            keys.push(k.clone());
            keys.push(format!("{k}:9c"));
        }
        Self { c, keys, events: Vec::new(), created: 0 }
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
}

impl WallWorld for World<'_> {
    fn definition(&mut self, actor: Ref) -> Result<Ref> {
        Ok(match self.name(actor) {
            "actor" => self.id("settlement_def"),
            k => self.opt(&self.c["actors"][k]["def"]),
        })
    }
    fn piece_count(&mut self) -> Result<i32> {
        Ok(self.c["pieces"].as_array().unwrap().len() as i32)
    }
    fn piece(&mut self, slot: i32) -> Result<Ref> {
        Ok(self.opt(&self.c["pieces"][usize::try_from(slot).unwrap()]))
    }
    fn definition_4a9(&mut self, definition: Ref) -> Result<u8> {
        assert_eq!(self.name(definition), "settlement_def");
        Ok(u(&self.c["byte_4a9"]) as u8)
    }
    fn style(&mut self, definition: Ref, index: i32) -> Result<Ref> {
        assert_eq!(self.name(definition), "settlement_def");
        Ok(self.id(&format!("style{index}")))
    }
    fn slot_kind(&mut self, definition: Ref, slot: i32) -> Result<u32> {
        assert_eq!(self.name(definition), "settlement_def");
        Ok(u(&self.c["slot_kinds"][usize::try_from(slot).unwrap()]))
    }
    fn style_definitions(&mut self, style: Ref) -> Result<[Ref; 3]> {
        let i: usize = self.name(style).strip_prefix("style").unwrap().parse().unwrap();
        let s = &self.c["styles"][i];
        Ok([self.opt(&s[0]), self.opt(&s[1]), self.opt(&s[2])])
    }
    fn component_9c(&mut self, actor: Ref) -> Result<Ref> {
        let k = self.name(actor).to_string();
        Ok(if self.c["actors"][&k]["component"].as_bool().unwrap() { self.id(&format!("{k}:9c")) } else { 0 })
    }
    fn upgrade(&mut self, component: Ref, definition: Ref) -> Result<()> {
        self.events.push(json!(["upgrade", self.key(component), self.key(definition)]));
        Ok(())
    }
    fn create_piece(&mut self, slot: i32, flag: u32, second: u32) -> Result<Ref> {
        self.events.push(json!(["create_piece", slot as u32, flag, second]));
        let r = self.opt(&self.c["responses"]["create_piece"][self.created]);
        self.created += 1;
        Ok(r)
    }
    fn report(&mut self, report: Report) -> Result<()> {
        let line = match report {
            Report::UpgradePiece => 0x3ae,
            Report::CreatePiece => 0x626,
        };
        self.events.push(json!(["report", line]));
        Ok(())
    }
}

#[test]
fn settlement_walls_match_native() {
    let d = document();
    for (k, n) in [("5856F0", 69), ("585BF0", 41), ("585C60", 36), ("587210", 50)] {
        assert_eq!(d["coverage"][k], n, "every instruction of {k} ran");
    }
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    for case in cases {
        let (c, o) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let mut w = World::new(c);
        let s = Settlement {
            actor: w.id("actor"),
            style_index: c["style_index"].as_i64().unwrap() as u32,
            last_periodic: 0.0,
            rotation: 0,
            forwards: 0,
        };
        let arg = c["arg"].as_i64().unwrap() as i32;
        let result = match c["operation"].as_str().unwrap() {
            "level" => piece_level(&s, arg, &mut w).map(|v| v as u32),
            "upgrade_piece" => upgrade_piece(&s, arg, &mut w).map(u32::from),
            "upgrade_full_wall" => upgrade_full_wall(&s, &mut w).map(u32::from),
            "create_full_wall" => create_full_wall(&s, arg as u32, &mut w).map(u32::from),
            other => panic!("{other}"),
        }
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(result, u(&o["result"]), "{name} result");
        assert_eq!(Value::Array(w.events), o["events"], "{name} events");
    }
}
