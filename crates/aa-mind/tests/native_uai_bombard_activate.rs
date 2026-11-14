//! Replays research/phase8-ai/native-uai-bombard-activate.json (original
//! 538120 KKC_UAIStateBombard::Activate with 52A000 and 529C30 under
//! Unicorn) against aa_mind::uai_thinks::uai_bombard_activate: the bool, the
//! bombard point and every seam call (4D4FF0, 537AF0, 52A810) in order.
use aa_mind::{
    Result,
    uai_thinks::uai_bombard_activate::{BombardActivateWorld, activate},
};
use serde_json::{Value, json};

const TARGET: u32 = 1;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-bombard-activate.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
    point: [u32; 2],
}
impl BombardActivateWorld for Host<'_> {
    fn is_current(&mut self) -> Result<bool> {
        Ok(self.c["stack"] == "self")
    }
    fn target_gtid(&mut self) -> Result<u32> {
        Ok(u(&self.c["state_gtid"]))
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.c["target"]["gtid"]))
    }
    fn has_locomotion(&mut self) -> Result<bool> {
        Ok(self.c["locomotion"].as_bool().unwrap())
    }
    fn relation(&mut self, object: u32, a: u32, b: u32) -> Result<i32> {
        self.events.push(json!(["relation", object, a, b & 0xff]));
        Ok(u(&self.c["relation"]) as i32)
    }
    fn set_target(&mut self, object: u32) -> Result<()> {
        self.events.push(json!(["set_target", object]));
        Ok(())
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        assert_eq!(object, TARGET);
        let p = &self.c["target"]["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn set_point(&mut self, point: [u32; 2]) -> Result<()> {
        self.point = point;
        Ok(())
    }
    fn activate(&mut self, mode: u32) -> Result<bool> {
        self.events.push(json!(["activate", mode]));
        Ok(self.c["activated"].as_bool().unwrap())
    }
}

#[test]
fn bombard_activate_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 400);
    let mut taken = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        let mut host = Host { c, events: Vec::new(), point: [0x1111_1111, 0x2222_2222] };
        let target = (!c["target"].is_null()).then_some(TARGET);
        let got = activate(u(&c["mode"]), target, &mut host).unwrap();
        assert_eq!(u32::from(got), u(&want["result"]), "{name} result");
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        assert_eq!(json!(host.point), want["point"], "{name} point");
        taken += usize::from(got);
    }
    assert!(taken > 100, "{taken}");
}
