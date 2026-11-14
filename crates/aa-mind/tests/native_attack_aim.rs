//! 51B0C0 against original x86 executions (research/aircraft-attack), bit
//! for bit at every x87 precision.
use aa_mind::Result;
use aa_mind::aai_attack_aim::{AttackAimWorld, aim};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/aircraft-attack/native-attack-aim.json"))
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

fn pair(v: &Value) -> [u32; 2] {
    [word(&v[0]), word(&v[1])]
}

struct State<'a> {
    input: &'a Value,
    calls: Vec<Value>,
    aim: [u32; 3],
}
impl State<'_> {
    fn value(&mut self, name: &str, key: &str) -> Result<Binary> {
        self.calls.push(json!([name]));
        Binary::from_f64_bits(self.input[key].as_u64().unwrap())
            .map_err(|e| aa_mind::Error::World(e.to_string()))
    }
}
impl AttackAimWorld for State<'_> {
    type Object = ();
    fn target(&mut self) -> Result<()> {
        Ok(())
    }
    fn object_position(&mut self, _: ()) -> Result<[u32; 2]> {
        Ok(pair(&self.input["target"]))
    }
    fn owner_position(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.input["owner"]))
    }
    fn owner_speed(&mut self) -> Result<Binary> {
        self.value("owner_speed", "owner_speed")
    }
    fn object_speed(&mut self, _: ()) -> Result<Binary> {
        self.value("target_speed", "target_speed")
    }
    fn object_heading(&mut self, _: ()) -> Result<[u32; 2]> {
        Ok(pair(&self.input["heading"]))
    }
    fn set_aim_point(&mut self, point: [u32; 3]) -> Result<()> {
        self.aim = point;
        Ok(())
    }
    fn previous_point(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.input["previous"]))
    }
    fn owner_reach(&mut self) -> Result<Binary> {
        self.value("reach", "reach")
    }
    fn flight_byte_90(&mut self) -> Result<u8> {
        Ok(word(&self.input["flight_90"]) as u8)
    }
    fn flight_word_34(&mut self) -> Result<u32> {
        Ok(word(&self.input["flight_34"]))
    }
    fn retarget_pending(&mut self) -> Result<u8> {
        Ok(word(&self.input["pending"]) as u8)
    }
    fn plan(&mut self) -> Result<()> {
        self.calls.push(json!(["plan"]));
        Ok(())
    }
}

#[test]
fn aim_point_bits_and_replan_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4800);
    for row in rows {
        let mut s = State { input: &row["input"], calls: vec![], aim: [0x1111, 0x2222, 0x3333] };
        aim(&mut s, precision(row)).unwrap();
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(s.aim, [word(&row["aim"][0]), word(&row["aim"][1]), word(&row["aim"][2])], "{row}");
    }
}
