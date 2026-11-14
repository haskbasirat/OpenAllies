//! Replays research/phase8-combat/native-settlement-tick-fixtures.json:
//! original x86 execution of the Settlement component Tick 587290 under
//! PC24/53/64. Every recorded callback with its arguments and the stored
//! Settlement +48 must match.
use aa_combat::Result;
use aa_combat::settlement_tick::*;
use aa_numeric::Precision;
use serde_json::{Value, json};

fn document() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-settlement-tick-fixtures.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
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
        for k in c["actors"].as_object().unwrap().keys() {
            keys.push(k.clone());
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
    fn resp(&self, k: &str) -> &'a Value {
        &self.c["responses"][k]
    }
    fn list(&self, k: &str, i: i32) -> Ref {
        self.opt(&self.c[k][usize::try_from(i).unwrap()])
    }
}

impl SettlementWorld for World<'_> {
    fn flags(&mut self, actor: Ref) -> Result<u32> {
        Ok(match self.name(actor) {
            "actor" => u(&self.c["actor_flags"]),
            k => u(&self.c["actors"][k]["flags"]),
        })
    }
    fn definition(&mut self, actor: Ref) -> Result<Ref> {
        Ok(match self.name(actor) {
            "actor" => self.id("settlement_def"),
            k => self.opt(&self.c["actors"][k]["def"]),
        })
    }
    fn health(&mut self, actor: Ref) -> Result<Option<[f32; 2]>> {
        let h = &self.c["actors"][self.name(actor)]["health"];
        Ok(h.as_array().map(|h| [f(&h[0]), f(&h[1])]))
    }
    fn member_count(&mut self) -> Result<i32> {
        Ok(self.c["members"].as_array().unwrap().len() as i32)
    }
    fn member(&mut self, index: i32) -> Result<Ref> {
        Ok(self.list("members", index))
    }
    fn piece_count(&mut self) -> Result<i32> {
        Ok(self.c["pieces"].as_array().unwrap().len() as i32)
    }
    fn piece(&mut self, slot: i32) -> Result<Ref> {
        Ok(self.list("pieces", slot))
    }
    fn owner_1f0(&mut self, owner: Owner) -> Result<f32> {
        let k = match owner {
            Owner::Component14 => "component14",
            Owner::Actor => "actor",
        };
        self.events.push(json!(["owner_1f0", k]));
        Ok(f(&self.c["owner_1f0"][k]))
    }
    fn definition_4a9(&mut self, definition: Ref) -> Result<u8> {
        assert_eq!(self.name(definition), "settlement_def");
        Ok(u(&self.c["byte_4a9"]) as u8)
    }
    fn definition_4a8(&mut self, definition: Ref) -> Result<u8> {
        assert_eq!(self.name(definition), "settlement_def");
        Ok(u(&self.c["byte_4a8"]) as u8)
    }
    fn style(&mut self, definition: Ref, index: u32) -> Result<Ref> {
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
    fn periodic(&mut self) -> Result<()> {
        self.events.push(json!(["periodic", "controller"]));
        Ok(())
    }
    fn start(&mut self, kind: u32, target: Ref) -> Result<()> {
        self.events.push(json!(["start", kind, self.key(target)]));
        Ok(())
    }
    fn idle(&mut self) -> Result<bool> {
        self.events.push(json!(["idle"]));
        Ok(self.resp("idle").as_bool().unwrap())
    }
    fn admits(&mut self, kind: u32) -> Result<bool> {
        self.events.push(json!(["admits", kind]));
        Ok(self.resp("admits")[kind.to_string()].as_bool().unwrap())
    }
    fn create_full_wall(&mut self) -> Result<bool> {
        self.events.push(json!(["create_full_wall", 1]));
        Ok(self.resp("create_full_wall").as_bool().unwrap())
    }
    fn create_piece(&mut self, slot: i32) -> Result<Ref> {
        self.events.push(json!(["create_piece", slot as u32, 1, 1]));
        Ok(self.opt(self.resp("create_piece")))
    }
    fn upgrade_full_wall(&mut self) -> Result<bool> {
        self.events.push(json!(["upgrade_full_wall"]));
        Ok(self.resp("upgrade_full_wall").as_bool().unwrap())
    }
    fn upgrade_piece(&mut self, slot: i32) -> Result<bool> {
        self.events.push(json!(["upgrade_piece", slot as u32]));
        Ok(self.resp("upgrade_piece").as_bool().unwrap())
    }
}

#[test]
fn settlement_tick_matches_native() {
    let d = document();
    assert_eq!(d["coverage"]["587290"], 249, "every instruction of 587290 ran");
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    let mut periodic = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for case in cases {
        let (c, o) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut w = World::new(c);
        let mut s = Settlement {
            actor: w.id("actor"),
            style_index: u(&c["style_index"]),
            last_periodic: f(&c["last"]),
            rotation: c["rotation"].as_i64().unwrap() as i32,
            forwards: u(&c["forwards"]) as u8,
        };
        tick(&mut s, f(&c["time"]), precision, &mut w).unwrap_or_else(|e| panic!("{name}: {e}"));
        // The controller is the Tick argument; the fixture records it raw.
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| if e[0] == "periodic" { json!(["periodic", "controller"]) } else { e.clone() })
            .collect();
        assert_eq!(w.events, native, "{name} events");
        assert_eq!(s.last_periodic.to_bits(), u(&o["last"]), "{name} +48");
        for e in &w.events {
            if e[0] == "periodic" {
                periodic += 1;
            }
            if e[0] == "start" {
                kinds.insert(e[1].as_u64().unwrap());
            }
        }
    }
    assert!(periodic > 100, "{periodic}");
    assert_eq!(kinds.into_iter().collect::<Vec<_>>(), vec![2, 4, 5, 6, 7]);
}
