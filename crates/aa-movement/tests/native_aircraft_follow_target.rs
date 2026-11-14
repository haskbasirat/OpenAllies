//! 518A30 against original x86 executions (research/aircraft-move-state),
//! bit for bit at every x87 precision.
use aa_movement::Result;
use aa_movement::aircraft_follow_target::{FollowTargetWorld, follow_target};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-move-state/native-aircraft-follow-target.json"
    ))
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
    followed: [u32; 2],
}
impl State<'_> {
    fn value(&mut self, name: &str) -> Result<Binary> {
        self.calls.push(json!([name]));
        Ok(Binary::from_f64_bits(self.input[name].as_u64().unwrap())?)
    }
}
impl FollowTargetWorld for State<'_> {
    type Object = ();
    fn followed(&mut self) -> Result<()> {
        Ok(())
    }
    fn object_position(&mut self, _: ()) -> Result<[u32; 2]> {
        Ok(pair(&self.input["object"]))
    }
    fn set_followed_position(&mut self, xy: [u32; 2]) -> Result<()> {
        self.followed = xy;
        Ok(())
    }
    fn destination(&mut self) -> Result<[u32; 3]> {
        let d = &self.input["destination"];
        Ok([word(&d[0]), word(&d[1]), word(&d[2])])
    }
    fn owner_position(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.input["owner"]))
    }
    fn owner_radius(&mut self) -> Result<Binary> {
        self.value("owner_radius")
    }
    fn object_radius(&mut self, _: ()) -> Result<Binary> {
        self.value("object_radius")
    }
    fn object_speed(&mut self, _: ()) -> Result<Binary> {
        self.value("object_speed")
    }
    fn owner_speed(&mut self) -> Result<Binary> {
        self.value("owner_speed")
    }
    fn object_heading(&mut self, _: ()) -> Result<[u32; 2]> {
        Ok(pair(&self.input["heading"]))
    }
    fn commanded(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.input["commanded"]))
    }
    fn retarget(&mut self, xy: [u32; 2]) -> Result<()> {
        self.calls.push(json!(["retarget", xy[0], xy[1], true]));
        Ok(())
    }
}

#[test]
fn follow_point_and_retarget_bits_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 5400);
    for row in rows {
        let input = &row["input"];
        let mut s = State { input, calls: vec![], followed: [0x3333, 0x4444] };
        follow_target(&mut s, input["force"].as_u64().unwrap() != 0, precision(row)).unwrap();
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(s.followed, pair(&row["followed"]), "{row}");
    }
}
