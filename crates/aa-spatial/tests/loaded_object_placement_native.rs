use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    loaded_object_placement::{
        self as placement, CoordinatorArguments, Descriptor, Request, World,
    },
};
use serde_json::{Value, json};

fn number(v: &Value, key: &str) -> u32 {
    v[key].as_u64().unwrap() as u32
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
fn descriptor(d: &Descriptor<u64, u64>) -> Value {
    let mut w = vec![placement::NATIVE_DESCRIPTOR_VTABLE as u64, d.definition];
    w.extend(d.position_bits.map(u64::from));
    w.extend(d.facing_bits.map(u64::from));
    w.push(d.temporary_name);
    w.extend(d.words_20_to_4c.map(u64::from));
    json!({"words":w,"bytes":d.bytes_50_to_52})
}
struct Probe {
    input: Value,
    events: Vec<Value>,
    definition: u64,
    flags: Option<u32>,
    position: Vec<u32>,
    facing: Vec<u32>,
    refs: [u32; 2],
    descriptor_after: Option<Value>,
}
impl Probe {
    fn new(c: &Value) -> Self {
        Self {
            input: c.clone(),
            events: vec![],
            definition: 2,
            flags: Some(number(c, "flags")),
            position: words(&c["position"]),
            facing: words(&c["facing"]),
            refs: [
                number(c, "refs"),
                c["replacement_refs"].as_u64().unwrap_or(1) as u32,
            ],
            descriptor_after: None,
        }
    }
    fn event(&mut self, name: &str, event: Value) -> Result<()> {
        self.events.push(event);
        if self.input["failure"].as_str() == Some(name) {
            return Err(Error::World(name.into()));
        }
        Ok(())
    }
    fn mutate(&mut self, prefix: &str) {
        if let Some(flags) = self.input[format!("{prefix}_flags")].as_u64() {
            self.flags = Some(flags as u32);
        }
        if self.input[format!("{prefix}_definition")].as_bool() == Some(true) {
            self.definition = 3;
        }
        let position = &self.input[format!("{prefix}_position")];
        if !position.is_null() {
            self.position = words(position);
        }
        let facing = &self.input[format!("{prefix}_facing")];
        if !facing.is_null() {
            self.facing = words(facing);
        }
    }
    fn output(self, result: Result<u32>) -> Value {
        let (outcome, result) = match result {
            Ok(r) => ("ok".to_owned(), Some(r)),
            Err(Error::World(e)) => (e, None),
            Err(e) => panic!("unexpected error {e}"),
        };
        json!({"outcome":outcome,"result":result,"events":self.events,
               "descriptor_after":self.descriptor_after,"refs":self.refs,
               "object":{"definition":self.definition,"flags":self.flags.unwrap(),
                         "position":self.position,"facing":self.facing}})
    }
}
impl World for Probe {
    type Object = u64;
    type Vector = u64;
    type Definition = u64;
    type Name = u64;
    fn empty_temporary_name(&mut self) -> Result<u64> {
        self.event("empty", json!(["empty"]))?;
        self.mutate("empty");
        Ok(4)
    }
    fn definition(&mut self, object: u64) -> Result<u64> {
        assert_eq!(object, 1);
        self.event("definition", json!(["definition", self.definition]))?;
        Ok(self.definition)
    }
    fn object_word(&mut self, object: u64, offset: u32) -> Result<u32> {
        assert_eq!(object, 1);
        let value = match offset {
            8 => self
                .flags
                .ok_or_else(|| Error::World("missing current flags".into()))?,
            0x20 | 0x24 | 0x28 => self.position[((offset - 0x20) / 4) as usize],
            0x2c | 0x30 => self.facing[((offset - 0x2c) / 4) as usize],
            _ => return Err(Error::World("unsupported word".into())),
        };
        self.event("object_word", json!(["object_word", offset, value]))?;
        Ok(value)
    }
    fn vector_word(&mut self, vector: u64, offset: u32) -> Result<u32> {
        let source = match vector {
            2 => "override_position",
            3 => "override_facing",
            _ => return Err(Error::World("unknown vector".into())),
        };
        let value = self.input[source][(offset / 4) as usize].as_u64().unwrap() as u32;
        self.event("vector_word", json!(["vector_word", vector, offset, value]))?;
        Ok(value)
    }
    fn virtual_b4(&mut self, object: u64) -> Result<bool> {
        assert_eq!(object, 1);
        self.event("b4", json!(["b4"]))?;
        self.mutate("b4");
        Ok(number(&self.input, "b4") as u8 != 0)
    }
    fn virtual_108(&mut self, object: u64) -> Result<u32> {
        assert_eq!(object, 1);
        self.event("v108", json!(["v108"]))?;
        self.mutate("v108");
        Ok(number(&self.input, "v108"))
    }
    fn coordinator_4f3640(
        &mut self,
        d: &mut Descriptor<u64, u64>,
        a: CoordinatorArguments<u64>,
        _pc: Precision,
    ) -> Result<u32> {
        self.event(
            "coordinator",
            json!([
                "coordinator",
                descriptor(d),
                [
                    a.source,
                    a.other,
                    a.zero_4 as u64,
                    a.zero_5 as u64,
                    a.argument_6 as u64,
                    a.argument_7 as u64
                ]
            ]),
        )?;
        if self.input["replace_name"].as_bool() == Some(true) {
            d.temporary_name = 5;
        }
        if self.input["mutate_descriptor"].as_bool() == Some(true) {
            d.definition = 3;
            d.words_20_to_4c[0] = 0x13579bdf;
            d.bytes_50_to_52[0] = 0x87;
        }
        self.mutate("coordinator");
        self.descriptor_after = Some(descriptor(d));
        Ok(number(&self.input, "result"))
    }
    fn release_temporary_name(&mut self, name: u64) -> Result<()> {
        self.event("release", json!(["release", name]))?;
        let index = (name - 4) as usize;
        self.refs[index] = self.refs[index].wrapping_sub(1);
        if self.refs[index] as i32 <= 0 {
            self.event("free", json!(["free", name]))?;
        }
        Ok(())
    }
}
fn pc(value: &Value) -> Precision {
    match value.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!(),
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-loaded-placement/native-loaded-placement.json"
    ))
    .unwrap()
}
#[test]
fn original_v14_wrapper_descriptor_and_callback_order() {
    let fixture = fixture();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 159);
    for row in rows {
        let c = &row["input"];
        let mut host = Probe::new(c);
        let request = Request {
            object: 1,
            position: c["use_position"].as_bool().unwrap().then_some(2),
            facing: c["use_facing"].as_bool().unwrap().then_some(3),
            argument_4: number(c, "arg4"),
            argument_5: number(c, "arg5"),
        };
        let result = placement::check(request, pc(&row["precision"]), &mut host);
        assert_eq!(
            host.output(result),
            row["output"],
            "{} / {}",
            c["label"],
            row["precision"]
        );
    }
}
#[test]
fn saved_null_input_matches_wrapper_and_missing_flags_are_not_zeroed() {
    let fixture = fixture();
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["input"]["label"] == "saved_null")
    {
        let mut host = Probe::new(&row["input"]);
        let precision = pc(&row["precision"]);
        let result = placement::check_saved(1, precision, &mut host);
        assert_eq!(host.output(result), row["output"]);
        let mut host = Probe::new(&row["input"]);
        host.flags = None;
        let result = placement::check_saved(1, precision, &mut host);
        assert!(matches!(result,Err(Error::World(ref s)) if s=="missing current flags"));
        assert_eq!(host.events.last(), Some(&json!(["v108"])));
        assert!(!host.events.iter().any(|v| v[0] == "coordinator"));
        assert_eq!(host.refs, [2, 1]);
    }
}
