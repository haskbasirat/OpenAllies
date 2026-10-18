use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    actor_init_null::{self, World},
    components::ComponentSlot,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeSet};

struct State {
    fields: Value,
    slots: BTreeSet<usize>,
    events: Vec<Value>,
    counts: [i32; 2],
    count_reads: usize,
}
struct Services<'a> {
    input: &'a Value,
    state: RefCell<State>,
}
impl<'a> Services<'a> {
    fn new(input: &'a Value) -> Self {
        let mut fields = input.clone();
        fields["definition"] = json!(1);
        Self {
            input,
            state: RefCell::new(State {
                fields,
                slots: input["slots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as usize)
                    .collect(),
                events: vec![],
                counts: [input["tracks"].as_array().unwrap().len() as i32; 2],
                count_reads: 0,
            }),
        }
    }
    fn word(&self, key: &str) -> u32 {
        self.state.borrow().fields[key].as_u64().unwrap() as u32
    }
    fn event(&self, event: Value) -> Result<()> {
        let name = event[0].as_str().unwrap().to_owned();
        let mut s = self.state.borrow_mut();
        s.events.push(event);
        if let Some(changes) = self.input["mutations"].as_array() {
            for change in changes.iter().filter(|c| c["at"] == name) {
                for n in change["add_slots"].as_array().into_iter().flatten() {
                    s.slots.insert(n.as_u64().unwrap() as usize);
                }
                for n in change["remove_slots"].as_array().into_iter().flatten() {
                    s.slots.remove(&(n.as_u64().unwrap() as usize));
                }
                for key in [
                    "flags",
                    "state",
                    "required",
                    "associated",
                    "word138",
                    "word144",
                    "definition",
                ] {
                    if let Some(value) = change.get(key) {
                        s.fields[key] = value.clone();
                    }
                }
            }
        }
        if self.input["failure"] == name {
            Err(Error::World(name))
        } else {
            Ok(())
        }
    }
    fn al(&self, key: &str) -> bool {
        self.input[key].as_u64().unwrap() as u8 != 0
    }
    fn output(&self, result: Result<bool>) -> Value {
        let (outcome, result) = match result {
            Ok(value) => ("ok".to_owned(), json!(value)),
            Err(Error::World(name)) => (name, Value::Null),
            Err(Error::NullDereference(name)) if name.contains("zone2") => {
                ("zone_null".into(), Value::Null)
            }
            Err(Error::NullDereference(_)) => ("native_null".into(), Value::Null),
            Err(Error::DivideByZero(_)) => ("native_idiv_fault".into(), Value::Null),
            Err(Error::Domain(name)) if name.contains("idiv overflow") => {
                ("native_idiv_fault".into(), Value::Null)
            }
            Err(error) => panic!("unexpected boundary: {error}"),
        };
        let s = self.state.borrow();
        json!({"outcome":outcome,"result":result,"events":s.events,
            "word138":s.fields["word138"],"word144":s.fields["word144"],
            "flags":s.fields["flags"],"state":s.fields["state"],
            "required":s.fields["required"],"definition":s.fields["definition"],"slots":s.slots})
    }
}
macro_rules! simple {
    ($method:ident, $event:literal) => {
        fn $method(&mut self, _: ActorId) -> Result<()> {
            self.event(json!([$event]))
        }
    };
}
impl World for Services<'_> {
    type Definition = u32;
    type Track = (u32, usize);
    type Zone = u32;
    type Updatable = u32;
    fn object_init_null(&mut self, _: ActorId, _: Precision) -> Result<bool> {
        self.event(json!(["object_null"]))?;
        Ok(self.al("object_return"))
    }
    fn required_object_init(&mut self, _: ActorId) -> Result<bool> {
        self.event(json!(["required_init"]))?;
        Ok(self.al("required_return"))
    }
    fn has_component(&self, _: ActorId, slot: ComponentSlot) -> Result<bool> {
        Ok(self.state.borrow().slots.contains(&slot.index()))
    }
    fn component_init_existing(
        &mut self,
        _: ActorId,
        slot: ComponentSlot,
        _: Precision,
    ) -> Result<bool> {
        self.event(json!([format!("init_{}", slot.index())]))?;
        Ok(self.input["component_returns"][slot.index().to_string()]
            .as_u64()
            .unwrap_or(1) as u8
            != 0)
    }
    fn component_after_init(&mut self, _: ActorId, slot: ComponentSlot) -> Result<()> {
        self.event(json!([format!("after_{}", slot.index())]))
    }
    fn required_object(&self, _: ActorId) -> Result<ActorId> {
        Ok(ActorId::from_raw(self.word("required")))
    }
    fn gtid_live(&self, actor: ActorId) -> Result<bool> {
        Ok(if actor == ActorId::from_raw(2) {
            self.input["required_live"].as_bool().unwrap()
        } else if actor == ActorId::from_raw(3) {
            self.input["associated_live"].as_bool().unwrap()
        } else {
            false
        })
    }
    fn required_object_attach(&mut self, _: ActorId, _: ActorId) -> Result<()> {
        self.event(json!(["attach", self.word("required")]))
    }
    simple!(refresh_4d4cf0, "refresh_fields");
    simple!(refresh_4d1350, "refresh_actions");
    simple!(refresh_4d67f0, "refresh_overlaps");
    fn thing_flags(&self, _: ActorId) -> Result<u32> {
        Ok(self.word("flags"))
    }
    fn global_property(&self) -> Result<Option<u32>> {
        let property = self.word("property");
        self.event(json!(["global_property", property]))?;
        Ok((property != 0).then_some(property))
    }
    fn property_applies(&mut self, _: ActorId, property: u32) -> Result<bool> {
        self.event(json!(["property_applies", property]))?;
        Ok(self.al("property_applies"))
    }
    fn properties_add(&mut self, _: ActorId, property: u32) -> Result<()> {
        self.event(json!(["property_add", property]))
    }
    simple!(refresh_4d4130, "refresh_zones");
    simple!(organization_refresh, "organization");
    simple!(settlement_refresh, "settlement");
    simple!(performer_refresh, "performer");
    simple!(denizen_refresh, "denizen");
    fn needs_settlement_zone(&mut self, _: ActorId) -> Result<bool> {
        self.event(json!(["needs_zone", self.word("definition")]))?;
        Ok(self.input["needs_zone"].as_bool().unwrap())
    }
    fn associated_object(&self, _: ActorId) -> Result<ActorId> {
        Ok(ActorId::from_raw(self.word("associated")))
    }
    fn create_zone2(&mut self, _: ActorId) -> Result<bool> {
        self.event(json!(["create_zone2"]))?;
        Ok(self.al("create_zone"))
    }
    fn discovery_radius(&mut self, _: Precision) -> Result<u32> {
        self.event(json!(["radius"]))?;
        Ok(self.word("radius"))
    }
    fn position_xy(&self, _: ActorId) -> Result<[u32; 2]> {
        Ok(serde_json::from_value(self.input["xy"].clone()).unwrap())
    }
    fn zone2(&self, _: ActorId) -> Result<Option<u32>> {
        Ok((self.input["zone_count"].as_i64().unwrap() > 2
            && self.input["zone_present"].as_bool().unwrap())
        .then_some(1))
    }
    fn move_zone(&mut self, _: u32, xy: [u32; 2], radius: u32) -> Result<()> {
        self.event(json!(["move_zone", xy, radius]))
    }
    fn sai_present(&self) -> Result<bool> {
        Ok(self.input["sai"].as_bool().unwrap())
    }
    simple!(sai_notify, "sai");
    fn track_definition(&self, _: ActorId) -> Result<u32> {
        self.event(json!(["track_definition", self.word("definition")]))?;
        Ok(self.word("definition"))
    }
    fn track_count(&self, definition: u32) -> Result<i32> {
        self.event(json!(["track_count", definition]))?;
        let mut s = self.state.borrow_mut();
        if let Some(value) = self.input["count_reads"][s.count_reads].as_i64() {
            s.counts[definition as usize - 1] = value as i32;
        }
        s.count_reads += 1;
        Ok(s.counts[definition as usize - 1])
    }
    fn track_at(&self, definition: u32, index: usize) -> Result<(u32, usize)> {
        self.event(json!(["track_at", definition, index]))?;
        Ok((definition, index))
    }
    fn track_kind(&self, (definition, index): (u32, usize)) -> Result<u32> {
        self.event(json!(["track_kind", definition, index]))?;
        Ok(self.input["tracks"][index][0].as_u64().unwrap() as u32)
    }
    fn track_length_bits(&self, (definition, index): (u32, usize)) -> Result<u32> {
        self.event(json!(["track_length", definition, index]))?;
        Ok(self.input["tracks"][index][1].as_u64().unwrap() as u32)
    }
    fn track_scale_bits(&self) -> Result<u32> {
        self.event(json!(["scale"]))?;
        Ok(self.word("scale"))
    }
    fn track_divisor(&self) -> Result<i32> {
        self.event(json!(["divisor"]))?;
        Ok(self.input["divisor"].as_i64().unwrap() as i32)
    }
    fn allocate_track(&mut self) -> Result<Option<u32>> {
        self.event(json!(["allocate"]))?;
        Ok(self.input["allocate"].as_bool().unwrap().then_some(99))
    }
    fn construct_track(&mut self, allocation: u32, _: ActorId, period: i32) -> Result<u32> {
        self.event(json!(["construct", period]))?;
        Ok(allocation)
    }
    fn set_track_updatable(&mut self, _: ActorId, value: Option<u32>) -> Result<()> {
        self.event(json!(["track_store", value]))?;
        self.state.borrow_mut().fields["word144"] = json!(value.unwrap_or(0));
        Ok(())
    }
    fn state_low(&self, _: ActorId) -> Result<u32> {
        Ok(self.word("state"))
    }
    fn existing_fx(&mut self, _: ActorId) -> Result<()> {
        self.event(json!(["fx", self.word("definition")]))
    }
}

#[test]
fn original_and_v14_saved_actor_init_null_match_every_prefix_and_final_field() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-actor-init-null/native-actor-init-null.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 972);
    for row in rows {
        let mut world = Services::new(&row["input"]);
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let result = actor_init_null::initialize(&mut world, ActorId::from_raw(1), pc);
        assert_eq!(
            world.output(result),
            row["output"],
            "{} {} {}",
            row["image"],
            row["precision"],
            row["input"]["label"]
        );
    }
}

/// Diagnostic boundary only: native indefinite/wider ftol results are outside
/// this coordinator's proved numeric domain, so no native parity is asserted.
#[test]
fn unsupported_track_numerator_stops_before_divisor_and_retains_prior_fields() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-actor-init-null/native-actor-init-null.json"
    ))
    .unwrap();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for scale in [0x7fc00000u32, 0x7f800000, 0x4f000000] {
            let mut input = fixture["cases"][0]["input"].clone();
            input["tracks"] = json!([[1, 0x3f800000u32]]);
            input["scale"] = json!(scale);
            let mut world = Services::new(&input);
            let result = actor_init_null::initialize(&mut world, ActorId::from_raw(1), pc);
            assert!(
                matches!(result, Err(Error::Numeric(_)) | Err(Error::Domain(_))),
                "{pc:?} {scale:x}: {result:?}"
            );
            let state = world.state.borrow();
            assert_eq!(state.events.last(), Some(&json!(["scale"])));
            assert!(!state.events.iter().any(|e| {
                ["divisor", "allocate", "track_store", "fx"]
                    .iter()
                    .any(|name| e[0] == *name)
            }));
            assert_eq!(state.fields["word138"], input["word138"]);
            assert_eq!(state.fields["word144"], input["word144"]);
        }
    }
}
