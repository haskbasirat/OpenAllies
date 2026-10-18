use aa_actors::{
    ActorId,
    actor_creation::{Error, Result, Shape},
    object_init_null::{self, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const ACTOR: ActorId = ActorId::from_raw(0x10023);
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32 fixture") as u32
}
fn shape(v: &Value) -> Shape {
    Shape {
        kind: u(&v[0]),
        radius_bits: u(&v[1]),
        extent_10_bits: u(&v[2]),
        extent_14_bits: u(&v[3]),
    }
}
struct Host {
    input: Value,
    words: BTreeMap<u32, u32>,
    events: Vec<Value>,
    stores: Vec<Value>,
    tile_at: usize,
    shape_at: usize,
    current_shape: Shape,
}
impl Host {
    fn new(input: Value) -> Self {
        let mut words = BTreeMap::from([
            (8, u(&input["flags"])),
            (0x38, 0x1234_5678),
            (0x3c, u(&input["scale"])),
            (0x44, 0x9988_7766),
            (0x48, 0x5566_7788),
            (0x4c, 0xa1b2_c3d4),
            (0x58, u(&input["instances"])),
            (0x5c, 0x5566_7700),
        ]);
        for (i, b) in input["position"].as_array().unwrap().iter().enumerate() {
            words.insert(0x20 + i as u32 * 4, u(b));
        }
        let current_shape = shape(&input["shapes"][0]);
        Self {
            input,
            words,
            events: vec![],
            stores: vec![],
            tile_at: 0,
            shape_at: 0,
            current_shape,
        }
    }
    fn event(&mut self, value: Value) -> Result<()> {
        let name = value[0].as_str().unwrap().to_owned();
        self.events.push(value);
        if self.input.get("failure").and_then(Value::as_str) == Some(name.as_str()) {
            Err(Error::World(name))
        } else {
            Ok(())
        }
    }
    fn position(&self) -> Vec<u32> {
        [0x20, 0x24, 0x28].map(|o| self.words[&o]).to_vec()
    }
    fn position_from(&mut self, key: &str) {
        if let Some(values) = self.input.get(key).and_then(Value::as_array) {
            for (i, b) in values.iter().enumerate() {
                self.words.insert(0x20 + i as u32 * 4, u(b));
            }
        }
    }
    fn output(&self, result: Result<bool>) -> Value {
        let (outcome, result) = match result {
            Ok(v) => ("ok".to_owned(), Some(v)),
            Err(Error::World(s)) => (s, None),
            Err(e) => (e.to_string(), None),
        };
        let words: Vec<_> = self.words.iter().map(|(&o, &v)| json!([o, v])).collect();
        json!({"outcome":outcome,"result":result,"words":words,"events":self.events,"stores":self.stores})
    }
}
impl World for Host {
    fn word(&mut self, actor: ActorId, offset: u32) -> Result<u32> {
        assert_eq!(actor, ACTOR);
        self.words
            .get(&offset)
            .copied()
            .ok_or_else(|| Error::World(format!("missing {offset:x}")))
    }
    fn set_word(&mut self, actor: ActorId, offset: u32, value: u32) -> Result<()> {
        assert_eq!(actor, ACTOR);
        assert!(self.words.contains_key(&offset));
        self.stores.push(json!([offset, 4, value]));
        self.words.insert(offset, value);
        Ok(())
    }
    fn clear_byte_4c(&mut self, actor: ActorId) -> Result<()> {
        let value = self.word(actor, 0x4c)? & 0xffff_ff00;
        self.stores.push(json!([0x4c, 1, 0]));
        self.words.insert(0x4c, value);
        Ok(())
    }
    fn definition_height_bits(&mut self, _: ActorId) -> Result<u32> {
        Ok(u(&self.input["height"]))
    }
    fn tile_size_bits(&mut self) -> Result<u32> {
        let values = self.input["tiles"].as_array().unwrap();
        let b = u(&values[self.tile_at.min(values.len() - 1)]);
        self.tile_at += 1;
        self.event(json!(["tile", b]))?;
        Ok(b)
    }
    fn tile_shift(&mut self) -> Result<u32> {
        let b = u(&self.input["shift"]);
        self.event(json!(["shift", b]))?;
        self.position_from("shift_position");
        Ok(b)
    }
    fn radius(&mut self, _: ActorId) -> Result<Binary> {
        self.event(json!(["radius"]))?;
        Ok(Binary::from_f32_bits(self.current_shape.radius_bits)?)
    }
    fn shape(&mut self, _: ActorId) -> Result<Shape> {
        let values = self.input["shapes"].as_array().unwrap();
        let value = values[self.shape_at.min(values.len() - 1)].clone();
        self.shape_at += 1;
        self.current_shape = shape(&value);
        self.event(json!(["shape", value]))?;
        Ok(self.current_shape)
    }
    fn thickness_warning(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["thickness_warning"]))
    }
    fn snap(&mut self, _: ActorId, shape: Shape, tile: u32, _: Precision) -> Result<()> {
        assert_eq!(shape.kind, self.current_shape.kind);
        self.event(json!(["snap", tile, self.position()]))?;
        self.position_from("snapped");
        Ok(())
    }
    fn placement_null(&mut self, _: ActorId) -> Result<u32> {
        self.event(json!(["placement", self.position()]))?;
        Ok(u(&self.input["placement"]))
    }
    fn placement_failure(&mut self, _: ActorId, code: u32) -> Result<()> {
        self.event(json!(["placement_failure", code, self.position()]))
    }
    fn map_mode(&mut self) -> Result<Option<u32>> {
        let mode = self.input["mode"].clone();
        self.event(json!(["mode", mode]))?;
        Ok(mode.as_u64().map(|n| n as u32))
    }
    fn placement_map_warning(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["map_warning", self.position()]))
    }
    fn spatial_membership(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["index"]))?;
        if let Some(value) = self.input.get("index_flags") {
            self.words.insert(8, u(value));
        }
        Ok(())
    }
    fn terrain_height(&mut self, _: ActorId, flags: u32) -> Result<Binary> {
        self.event(json!(["height", flags, self.position()]))?;
        Ok(Binary::from_f64_bits(
            self.input["terrain_f64"].as_u64().unwrap(),
        )?)
    }
    fn blocking_notify(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["blocking"]))
    }
    fn tile_footprint(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["footprint"]))?;
        self.position_from("footprint_position");
        Ok(())
    }
    fn world_list_insert(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["list"]))?;
        if let Some(value) = self.input.get("list_instances") {
            self.words.insert(0x58, u(value));
        }
        Ok(())
    }
    fn has_instances(&mut self, actor: ActorId) -> Result<bool> {
        let n = self.word(actor, 0x58)?;
        self.event(json!(["has_instances", n]))?;
        Ok((n as i32) > 0)
    }
    fn retain_instances(&mut self, _: ActorId) -> Result<bool> {
        self.event(json!(["retain"]))?;
        Ok(self.input["retain"].as_bool().unwrap())
    }
    fn clear_instances(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["clear"]))?;
        self.words.insert(0x58, 0);
        self.words.insert(0x5c, 0);
        Ok(())
    }
    fn visibility_refresh(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["visibility"]))
    }
}

#[test]
fn original_and_v14_object_null_native_rows() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-object-init-null/native-object-init-null.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 306);
    assert_eq!(fixture["native_executions"], 612);
    for row in rows {
        let mut host = Host::new(row["input"].clone());
        let precision: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let result = object_init_null::initialize(ACTOR, precision, &mut host);
        assert_eq!(
            host.output(result),
            row["output"],
            "{} {}",
            row["precision"],
            row["input"]["label"]
        );
    }
}

#[test]
fn absent_saved_instances_is_an_error_after_native_prefix_not_zero() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-object-init-null/native-object-init-null.json"
    ))
    .unwrap();
    let mut host = Host::new(fixture["cases"][0]["input"].clone());
    host.words.remove(&0x58);
    let result = object_init_null::initialize(ACTOR, Precision::Double53, &mut host);
    assert!(matches!(result,Err(Error::World(ref s)) if s=="missing 58"));
    assert_eq!(host.words[&0x4c], 0xa1b2_c300);
    assert_ne!(host.words[&0x44], 0x9988_7766);
    assert_eq!(host.events.last().unwrap(), &json!(["list"]));
    assert!(!host.events.iter().any(|v| v[0] == "visibility"));
}
