//! Replays research/phase8-restoration/native-create-gthing.json: the
//! original 4F9C10 with every callee seamed. The Rust control flow must make
//! the same calls, in the same order, with the same arguments and outcome.
use aa_actors::{
    components::ComponentSlot,
    world_creation::{CreateRequest, CreateWorld, Error, Result, create},
};
use serde_json::{Value, json};

struct Recorder<'a> {
    case: &'a Value,
    calls: Vec<Value>,
    created: u32,
    latch: bool,
}
impl Recorder<'_> {
    fn flag(&self, name: &str) -> bool {
        self.case[name].as_bool().unwrap()
    }
}
impl CreateWorld<u32, ()> for Recorder<'_> {
    type Object = u32;
    fn instance_name_gtid(&mut self, _: &[u16]) -> Result<Option<u32>> {
        self.calls.push(json!(["instance_name_lookup"]));
        Ok(self.case["name_gtid"].as_u64().map(|v| v as u32))
    }
    fn gtid_live(&self, _: u32) -> Result<bool> {
        Ok(self.flag("name_generation_matches") && self.flag("name_object_live"))
    }
    fn log(&mut self, line: u32, format: &'static str, _: &str) -> Result<()> {
        self.calls.push(json!(["log", line, format]));
        Ok(())
    }
    fn construct(&mut self, class: u32, size: u32, _: &CreateRequest<u32, ()>) -> Result<Option<u32>> {
        self.calls.push(json!(["allocate", size]));
        if self.flag("allocation_fails") {
            return Ok(None);
        }
        self.calls.push(json!(["construct", class]));
        let object = self.created;
        self.created += 1;
        Ok(Some(object))
    }
    fn set_state(&mut self, object: u32, state: u8, enabled: bool) -> Result<()> {
        self.calls.push(json!(["set_state", object, state, u32::from(enabled)]));
        Ok(())
    }
    fn create_component(&mut self, object: u32, slot: ComponentSlot) -> Result<()> {
        self.calls.push(json!(["create_component", object, slot.index()]));
        Ok(())
    }
    fn definition_flags(&self, _: u32) -> Result<u32> {
        Ok(self.case["definition_flags"].as_u64().unwrap() as u32)
    }
    fn construction_assert_latch(&self) -> Result<bool> {
        Ok(self.latch)
    }
    fn set_construction_assert_latch(&mut self, value: bool) -> Result<()> {
        self.latch = value;
        Ok(())
    }
    fn state_active(&mut self, object: u32, state: u8) -> Result<bool> {
        self.calls.push(json!(["state_active", object, state]));
        Ok(self.flag("construction_state"))
    }
    fn assertion_dialog(&mut self, _: &'static str, _: u32) -> Result<bool> {
        self.calls.push(json!(["assert_dialog"]));
        Ok(self.flag("dialog_result"))
    }
    fn set_organization_layout(&mut self, _: u32, layout: &u32) -> Result<()> {
        // Native: ecx is the Organization stored by 4CF480(7) (seam 0x7007).
        self.calls.push(json!(["set_organization_layout", 0x7007, layout]));
        Ok(())
    }
    fn init_create(&mut self, object: u32, _: &CreateRequest<u32, ()>) -> Result<bool> {
        self.calls.push(json!(["init_create", object]));
        Ok(self.flag("init_create"))
    }
    fn init_existing(&mut self, object: u32, _: &CreateRequest<u32, ()>) -> Result<bool> {
        self.calls.push(json!(["init_existing", object]));
        Ok(self.flag("init_existing"))
    }
    fn post_create(&mut self, object: u32, _: &CreateRequest<u32, ()>, flag: u32) -> Result<()> {
        self.calls.push(json!(["post_create", object, flag]));
        Ok(())
    }
    fn register(&mut self, object: u32) -> Result<()> {
        self.calls.push(json!(["register", object]));
        Ok(())
    }
    fn register_updatable(&mut self, object: u32) -> Result<()> {
        // Native: ecx is GWorld+130 (seam 0x7130).
        self.calls.push(json!(["register_updatable", 0x7130, object]));
        Ok(())
    }
    fn fires_creation_fx(&mut self, object: u32) -> Result<bool> {
        self.calls.push(json!(["fires_creation_fx", object]));
        Ok(self.flag("fires_creation_fx"))
    }
    fn creation_fx(&mut self, object: u32) -> Result<()> {
        self.calls.push(json!(["creation_fx", object, 0]));
        Ok(())
    }
    fn destroy(&mut self, object: u32) -> Result<()> {
        self.calls.push(json!(["destroy", object]));
        Ok(())
    }
}

#[test]
fn create_gthing_matches_native_call_sequence() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-restoration/native-create-gthing.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 100);
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let name: Vec<u16> = input["name"].as_str().unwrap().encode_utf16().collect();
        let layout = input["layout"].as_u64().unwrap() as u32;
        let request = CreateRequest {
            class: input["class"].as_u64().unwrap() as u32,
            definition_ids: "def_ids".into(),
            instance_name: name,
            actor_data: input["actor_params"].as_bool().unwrap(),
            kingdom_data: input["kingdom_params"].as_bool().unwrap(),
            organization_layout: (layout != 0).then_some(layout),
            mode: input["mode"].as_u64().unwrap() as u32,
            byte_50: input["byte_50"].as_u64().unwrap() != 0,
            payload: (),
        };
        let mut world = Recorder {
            case: input,
            calls: Vec::new(),
            created: 0,
            latch: input["latch"].as_bool().unwrap(),
        };
        let result = create(&request, &mut world);
        let label = format!("{input}");
        match native["outcome"].as_str().unwrap() {
            "object" => assert_eq!(result.clone().unwrap(), Some(0), "{label}"),
            "null" => assert_eq!(result.clone().unwrap(), None, "{label}"),
            "null_dereference" => {
                assert!(matches!(result, Err(Error::NullDereference(_))), "{label}: {result:?}");
                // The native records calls up to (not including) the faulting
                // use of the null object; Rust stops at the same point.
                let mut expected = native["calls"].as_array().unwrap().clone();
                expected.retain(|c| c[0] != "params_vf0" && c[0] != "params_vf4");
                assert_eq!(world.calls, expected, "{label}");
                continue;
            }
            other => panic!("{other}"),
        }
        let mut expected = native["calls"].as_array().unwrap().clone();
        // Params virtuals 0/4 are plain fields of the request in Rust.
        expected.retain(|c| c[0] != "params_vf0" && c[0] != "params_vf4");
        assert_eq!(world.calls, expected, "{label}");
        assert_eq!(u8::from(world.latch), native["latch"].as_u64().unwrap() as u8, "{label}");
    }
}
