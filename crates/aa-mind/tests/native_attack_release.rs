//! 51B290 against original x86 executions (research/aircraft-attack), with
//! the normalizers bound to aa-movement's ports of 4453A0 / 409230.
use aa_mind::Result;
use aa_mind::aai_attack_release::{AttackReleaseWorld, release};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/aircraft-attack/native-attack-release.json"))
        .unwrap()
}

fn precision(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn triple(v: &Value) -> [u32; 3] {
    [word(&v[0]), word(&v[1]), word(&v[2])]
}

fn fail(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(e.to_string())
}

struct State<'a> {
    doc: &'a Value,
    input: &'a Value,
    p: Precision,
    calls: Vec<Value>,
    flight: [u32; 5],
}
impl AttackReleaseWorld for State<'_> {
    type Object = ();
    type Ability = u32;
    fn dogfighter(&mut self) -> Result<u8> {
        Ok(word(&self.input["dogfighter"]) as u8)
    }
    fn target_slot(&mut self) -> Result<Option<()>> {
        Ok(self.input["live"].as_bool().unwrap().then_some(()))
    }
    fn object_position(&mut self, object: Option<()>) -> Result<[u32; 3]> {
        object.ok_or_else(|| fail("null slot"))?;
        Ok(triple(&self.input["target"]))
    }
    fn owner_position(&mut self) -> Result<[u32; 3]> {
        Ok(triple(&self.input["owner"]))
    }
    fn owner_heading(&mut self) -> Result<[u32; 3]> {
        Ok(triple(&self.input["heading"]))
    }
    fn aim_point(&mut self) -> Result<[u32; 3]> {
        Ok(triple(&self.input["aim"]))
    }
    fn normalize3(&mut self, v: [u32; 3]) -> Result<[u32; 3]> {
        let mut f = v.map(f32::from_bits);
        aa_movement::falling::normalize(&mut f, self.p).map_err(fail)?;
        Ok(f.map(f32::to_bits))
    }
    fn normalize2(&mut self, v: [u32; 2]) -> Result<[u32; 2]> {
        let mut f = v.map(f32::from_bits);
        aa_movement::transport_drop::normalize2(&mut f, self.p).map_err(fail)?;
        Ok(f.map(f32::to_bits))
    }
    fn pick_ability(&mut self, slot: Option<()>, aim: [u32; 3]) -> Result<(u32, Option<u32>)> {
        self.calls.push(json!(["pick", slot.is_some(), aim, 0]));
        let ability = self.input["ability"].as_bool().unwrap().then(|| word(&self.doc["ability"]));
        Ok((word(&self.input["klass"]), ability))
    }
    fn attack_object_byte(&mut self) -> Result<u8> {
        Ok(word(&self.input["object_byte"]) as u8)
    }
    fn target_gtid(&mut self) -> Result<u32> {
        let gtid = word(&self.doc["gtid"]);
        Ok(if self.input["live"].as_bool().unwrap() { gtid } else { gtid ^ 0x1_0000 })
    }
    fn arm_object(&mut self, gtid: u32) -> Result<()> {
        self.flight[0] = gtid;
        Ok(())
    }
    fn arm_point(&mut self, point: [u32; 3]) -> Result<()> {
        self.flight[1..4].copy_from_slice(&point);
        Ok(())
    }
    fn arm_ability(&mut self, ability: Option<u32>) -> Result<()> {
        self.flight[4] = ability.unwrap_or(0);
        Ok(())
    }
}

#[test]
fn facing_term_ability_and_arming_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 5034);
    for row in rows {
        let mut s = State {
            doc: &doc,
            input: &row["input"],
            p: precision(row),
            calls: vec![],
            flight: [0x9494, 0x9898, 0x9c9c, 0xa0a0, 0xa4a4],
        };
        let (al, out) = release(&mut s, precision(row)).unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(out, word(&row["out"]), "{row}");
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        let flight: Vec<u32> = row["flight"].as_array().unwrap().iter().map(word).collect();
        assert_eq!(s.flight.to_vec(), flight, "{row}");
    }
}
