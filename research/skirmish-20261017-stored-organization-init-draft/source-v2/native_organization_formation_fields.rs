//!Exact577F30/57BFF0 fields and required-child prefixes against both PE images.
use aa_actors::{
    ActorId,
    organization_state::{
        self as native, FormationFields, LayoutInfo, LoadError, Result, StateWords,
    },
};
use serde_json::{Value, json};
struct Probe<'a> {
    input: &'a Value,
    desired: u32,
    current: u32,
    bytes: [u8; 3],
    trace: Vec<Value>,
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl Probe<'_> {
    fn event(&mut self, v: Value) -> Result<()> {
        self.trace.push(v);
        if self.input.get("fail_at").and_then(Value::as_u64) == Some((self.trace.len() - 1) as u64)
        {
            Err(LoadError::Unsupported("required".into()))
        } else {
            Ok(())
        }
    }
}
impl FormationFields for Probe<'_> {
    fn word(&mut self, o: u32) -> Result<u32> {
        Ok(match o {
            0x70 => self.desired,
            0x74 => self.current,
            _ => panic!("word{o:x}"),
        })
    }
    fn put_word(&mut self, o: u32, v: u32) -> Result<()> {
        match o {
            0x70 => self.desired = v,
            0x74 => self.current = v,
            _ => panic!("word{o:x}"),
        }
        Ok(())
    }
    fn byte(&mut self, o: u32) -> Result<u8> {
        Ok(self.bytes[(o - 0x8c) as usize])
    }
    fn put_byte(&mut self, o: u32, v: u8) -> Result<()> {
        self.bytes[(o - 0x8c) as usize] = v;
        Ok(())
    }
    fn state_group4(&mut self) -> Result<Option<StateWords>> {
        self.event(json!(["group", 4]))?;
        Ok(self.input["state"].as_array().map(|s| StateWords {
            word_20: u(&s[0]),
            word_24: u(&s[1]),
        }))
    }
    fn layout_info(&mut self) -> Result<LayoutInfo> {
        Ok(LayoutInfo {
            name: "actual lab layout".into(),
            primary_slot: -1,
            entries: vec![],
            slot_groups: vec![],
            formations: vec![1, 2],
            formation_44: 0,
            formation_48: 1,
            property_50: 100,
        })
    }
    fn formation_facts(&mut self, f: u32) -> Result<(u8, u32)> {
        assert!(matches!(f, 1 | 2));
        Ok(((f - 1) as u8, 100 + f))
    }
    fn has_applied_property(&mut self, p: u32, s: ActorId) -> Result<bool> {
        assert_eq!(s.raw(), 1);
        self.event(json!(["has", p, 1]))?;
        Ok(self.input["has"].as_bool().unwrap())
    }
    fn remove_property(&mut self, p: u32, s: ActorId) -> Result<()> {
        assert_eq!(s.raw(), 1);
        self.event(json!(["remove", p, 1]))
    }
    fn apply_property(&mut self, p: u32, k: u32, s: ActorId) -> Result<()> {
        assert_eq!(s.raw(), 1);
        self.event(json!(["apply", p, k, 1]))
    }
    fn formation_refresh(&mut self) -> Result<()> {
        self.event(json!(["refresh"]))?;
        if let Some(v) = self.input.get("mutate_desired") {
            self.desired = u(v)
        }
        Ok(())
    }
    fn owner_object_164(&mut self) -> Result<Option<u32>> {
        self.event(json!(["view"]))?;
        Ok(self.input["view"].as_bool().unwrap().then_some(1))
    }
    fn object_refresh(&mut self, o: u32) -> Result<()> {
        assert_eq!(o, 1);
        Ok(())
    }
}
#[test]
fn native_formation_fields_and_required_prefixes() {
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-organization-init-draft/native-formation-fields.json")).unwrap();
    let mut n = 0;
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut p = Probe {
            input: c,
            desired: u(&c["desired"]),
            current: u(&c["current"]),
            bytes: std::array::from_fn(|i| u(&c["bytes"][i]) as u8),
            trace: vec![],
        };
        let result = if c["operation"] == "byte" {
            native::formation_byte_fields(&mut p, u(&c["value"]) as u8)
        } else {
            native::set_formation_fields(&mut p, u(&c["desired"]), ActorId::from_raw(1))
        };
        let stop = result.err().map(|_| "required");
        assert_eq!(
            json!({"trace":p.trace,"stop":stop,"desired":p.desired,"current":p.current,"bytes":p.bytes}),
            row["output"],
            "{row}"
        );
        n += 1;
    }
    assert_eq!(n, 756);
}

#[test]
fn native_default_layout_vector_and_morale_prefixes() {
    use aa_actors::ground_states::{LayoutDefinition, OrganizationState};
    use aa_body::F32;
    use aa_numeric::{Binary, Precision};
    struct Morale<'a>(&'a Value);
    impl native::MaximumMoraleWorld<String> for Morale<'_> {
        fn modifier(&mut self, _: ActorId) -> Result<Option<u32>> {
            Ok(None)
        }
        fn apply(&mut self, _: u32, _: u32) -> Result<Binary> {
            panic!("no supplied modifiers")
        }
        fn contribution(&self, id: &String) -> Result<u32> {
            Ok(u(&self.0[id][1]))
        }
    }
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-organization-init-draft/native-default-layout.json")).unwrap();
    let mut cases = 0;
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        let layout = c["layout"].as_str().unwrap();
        let data = &doc["supplied_layouts"][layout];
        let groups: Vec<u32> = data[1].as_array().unwrap().iter().map(u).collect();
        let source = LayoutDefinition {
            definition: layout.to_owned(),
            parameters: c["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Some(p.as_str().unwrap().to_owned()))
                .collect(),
            slots: groups.len(),
        };
        let mut org = OrganizationState::new(
            ActorId::from_raw(1),
            source.clone(),
            F32::ZERO,
            F32::ZERO,
            [F32::ZERO; 2],
        )
        .unwrap();
        if c["dirty"] == true {
            org.members.push(ActorId::from_raw(0x1234));
            org.words_34.push(0x1234);
            org.words_50.push(0x1234);
            org.offsets_78.push([F32(0x3f800000), F32(0x40000000)]);
        }
        native::initialize_default_fields(&mut org, &source, groups.len());
        let prefix = &row["economy_prefix"];
        assert_eq!(org.members.len(), u(&prefix["member_count"]) as usize);
        assert!(org.members.iter().all(|p| p.is_null()));
        assert_eq!(org.words_34.len(), u(&prefix["recruit_count"]) as usize);
        assert!(org.words_34.iter().all(|v| *v == 0));
        assert!(
            prefix["recruit_items"]
                .as_array()
                .unwrap()
                .iter()
                .all(Value::is_null)
        );
        assert_eq!(json!(org.words_50), prefix["words_50"]);
        if c["cut"] == true {
            cases += 1;
            continue;
        }
        let info = LayoutInfo {
            name: layout.into(),
            primary_slot: data[2].as_i64().unwrap() as i32,
            entries: vec![],
            slot_groups: groups,
            formations: vec![],
            formation_44: 0,
            formation_48: 0,
            property_50: 0,
        };
        let pc = match c["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        native::recompute_maximum_morale_fields(
            &mut org,
            &info,
            ActorId::from_raw(1),
            &mut Morale(&doc["supplied_actor_data"]),
            pc,
        )
        .unwrap();
        assert_eq!(org.words[&0x60], u(&row["final"]["words"]["0x60"]), "{c}");
        assert_eq!(org.words[&0x5c], u(&row["final"]["words"]["0x5c"]), "{c}");
        org.offsets_78.resize(info.slot_count(), [F32::ZERO; 2]);
        let bits: Vec<[u32; 2]> = org.offsets_78.iter().map(|xy| xy.map(|v| v.0)).collect();
        assert_eq!(json!(bits), row["final"]["offsets_78"], "{c}");
        cases += 1;
    }
    assert_eq!(cases, 72);
}
