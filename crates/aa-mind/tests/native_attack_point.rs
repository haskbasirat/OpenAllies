//! 517360 against original x86 executions (research/aircraft-attack).
use aa_mind::Result;
use aa_mind::aai_attack_point::{AttackPointWorld, attack_point};
use aa_numeric::Binary;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/aircraft-attack/native-attack-point.json"))
        .unwrap()
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
    point: [u32; 3],
    previous: [u32; 2],
    flags: [u32; 3],
}
impl AttackPointWorld for State<'_> {
    fn has_performer(&mut self) -> Result<bool> {
        Ok(self.input["performer"].as_bool().unwrap())
    }
    fn map_contains(&mut self, xy: [u32; 2]) -> Result<bool> {
        self.calls.push(json!(["contains", xy[0], xy[1]]));
        Ok(self.input["inside"].as_bool().unwrap())
    }
    fn map_clamp(&mut self, xy: [u32; 2]) -> Result<[u32; 2]> {
        self.calls.push(json!(["clamp", xy]));
        Ok(pair(&self.input["clamped"]))
    }
    fn set_point(&mut self, point: [u32; 3]) -> Result<()> {
        self.point = point;
        Ok(())
    }
    fn ground_height(&mut self, xy: [u32; 2]) -> Result<Binary> {
        self.calls.push(json!(["height", xy, self.point]));
        Binary::from_f64_bits(self.input["height"].as_u64().unwrap())
            .map_err(|e| aa_mind::Error::World(e.to_string()))
    }
    fn set_previous_point(&mut self, xy: [u32; 2]) -> Result<()> {
        self.previous = xy;
        Ok(())
    }
    fn move_state_set_formation(&mut self, formation: u8) -> Result<()> {
        self.calls.push(json!(["set_formation", formation]));
        Ok(())
    }
    fn move_state_retarget(&mut self, xy: [u32; 2]) -> Result<()> {
        self.calls.push(json!(["retarget", xy[0], xy[1], 0]));
        Ok(())
    }
    fn clear_flags(&mut self) -> Result<()> {
        self.flags = [0; 3];
        Ok(())
    }
    fn activate(&mut self, mode: u32) -> Result<u8> {
        self.calls.push(json!(["activate", mode, self.flags[0], self.flags[1], self.flags[2]]));
        Ok(word(&self.input["al"]) as u8)
    }
}

#[test]
fn attack_point_fields_calls_and_al_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1440);
    for row in rows {
        let input = &row["input"];
        let mut s = State {
            input,
            calls: vec![],
            point: [0x1111, 0x2222, 0x3333],
            previous: [0x4444, 0x5555],
            flags: [1, 1, 0x7777],
        };
        let al = attack_point(
            &mut s,
            word(&input["mode"]),
            pair(&input["point"]),
            word(&input["formation"]) as u8,
        )
        .unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(s.point, [word(&row["point"][0]), word(&row["point"][1]), word(&row["point"][2])], "{row}");
        assert_eq!(s.previous, pair(&row["previous"]), "{row}");
    }
}
