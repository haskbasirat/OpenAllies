use aa_actors::{
    ActorId,
    components::ComponentSlot,
    ground_states::{LayoutDefinition, OrganizationState},
    state_effects::{self as effects, Applicability, Descriptor, FxList, StateFx, ViewRule, World},
};
use aa_body::{F32, credit::Object};
use aa_numeric::Precision;
use serde_json::{Value, json};
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
struct Fixture {
    input: Value,
    health: u32,
    view: i32,
    flags: u32,
    org: OrganizationState<()>,
    events: Vec<Value>,
    fail_add: bool,
}
impl Fixture {
    fn new(i: &Value) -> Self {
        let mut org = OrganizationState::new(
            ActorId::from_raw(1),
            LayoutDefinition {
                definition: (),
                parameters: vec![],
                slots: 0,
            },
            F32(2f32.to_bits()),
            F32::ZERO,
            [F32::ZERO; 2],
        )
        .unwrap();
        org.words.insert(0x70, n(&i["pending"]));
        Self {
            input: i.clone(),
            health: n(&i["health"]),
            view: i["old_view"].as_i64().unwrap() as i32,
            flags: n(&i["flags"]),
            org,
            events: vec![],
            fail_add: false,
        }
    }
}
impl Applicability for Fixture {
    fn has_properties(&self, _: ActorId) -> effects::Result<bool> {
        Ok(b(&self.input["properties"]))
    }
    fn property_requirements(&self, p: Object) -> effects::Result<Vec<Object>> {
        Ok(self.input["requirements"][p.0.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| Object(n(v)))
            .collect())
    }
    fn actor_has_property(&self, _: ActorId, p: Object) -> effects::Result<bool> {
        Ok(self.input["available_properties"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| n(v) == p.0))
    }
    fn organization_count(&self, _: ActorId) -> effects::Result<Option<usize>> {
        Ok(b(&self.input["organization"]).then_some(0))
    }
    fn organization_member(&self, _: ActorId, _: usize) -> effects::Result<Option<ActorId>> {
        panic!("no members")
    }
    fn vacant_member_definition(&self, _: ActorId, _: usize) -> effects::Result<Option<Object>> {
        panic!("no members")
    }
    fn definition_component_mask(&self, _: Object) -> effects::Result<u32> {
        panic!("no vacant definitions")
    }
    fn definition_properties(&self, _: Object) -> effects::Result<Vec<Object>> {
        panic!("no vacant definitions")
    }
}
impl World for Fixture {
    fn descriptor(&self, index: u8) -> effects::Result<Descriptor> {
        let current = index == n(&self.input["index"]) as u8;
        Ok(Descriptor {
            index,
            properties: if current {
                self.input["state_properties"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_u64().map(|p| Object(p as u32)))
                    .collect()
            } else {
                vec![]
            },
            entrenchable: b(&self.input[if current {
                "entrenchable"
            } else {
                "old_entrenchable"
            }]),
            remove_from_group: current && b(&self.input["remove_group"]),
        })
    }
    fn has_component(&self, _: ActorId, slot: ComponentSlot) -> effects::Result<bool> {
        Ok(match slot {
            ComponentSlot::Organization => b(&self.input["organization"]),
            ComponentSlot::Economy => b(&self.input["economy"]),
            ComponentSlot::Character => b(&self.input["character"]),
            _ => false,
        })
    }
    fn state_bits(&self, _: ActorId) -> effects::Result<u64> {
        Ok(self.input["state"].as_u64().unwrap())
    }
    fn body_health(&self, _: ActorId) -> effects::Result<Option<(F32, F32)>> {
        Ok(
            b(&self.input["has_body"])
                .then_some((F32(self.health), F32(n(&self.input["maximum"])))),
        )
    }
    fn view_rules(&self, _: ActorId) -> effects::Result<Vec<ViewRule>> {
        Ok(self.input["views"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| ViewRule {
                health_limit: F32(n(&r["limit"])),
                required_low_states: n(&r["mask"]),
                property: r["property"].as_u64().map(|p| Object(p as u32)),
            })
            .collect())
    }
    fn current_view(&self, _: ActorId) -> effects::Result<i32> {
        Ok(self.view)
    }
    fn set_current_view(&mut self, _: ActorId, index: i32) -> effects::Result<()> {
        self.view = index;
        Ok(())
    }
    fn character_view(&mut self, _: ActorId, index: i32) -> effects::Result<()> {
        self.events.push(json!(["character", index]));
        self.view = index;
        Ok(())
    }
    fn add_property(
        &mut self,
        actor: ActorId,
        p: Object,
        kind: u32,
        source: ActorId,
    ) -> effects::Result<()> {
        assert_eq!(actor, source);
        self.events.push(json!(["add", p.0, kind]));
        if self.fail_add {
            return Err(effects::Error::World("insertion failure".into()));
        }
        Ok(())
    }
    fn remove_property(
        &mut self,
        actor: ActorId,
        p: Option<Object>,
        source: ActorId,
    ) -> effects::Result<()> {
        assert_eq!(actor, source);
        self.events.push(json!(["remove", p.map(|p| p.0)]));
        Ok(())
    }
    fn organization_transition(
        &mut self,
        _: ActorId,
        new: &Descriptor,
        old: Option<&Descriptor>,
    ) -> effects::Result<()> {
        let events = &mut self.events;
        effects::organization_transition(&mut self.org, new, old, F32(10f32.to_bits()), |value| {
            events.push(json!(["pending", value]));
            Ok(())
        })
    }
    fn refresh_economy(&mut self, _: ActorId) -> effects::Result<()> {
        self.events.push(json!(["economy"]));
        if let Some(value) = self.input["economy_health"].as_u64() {
            self.health = value as u32;
        }
        Ok(())
    }
    fn state_fx(&self, _: ActorId, _: FxList) -> effects::Result<Vec<StateFx>> {
        Ok(self.input["fx"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, f)| StateFx {
                handle: Object(i as u32),
                state_index: n(&f["state"]),
            })
            .collect())
    }
    fn active_fx_only(&self) -> effects::Result<bool> {
        Ok(b(&self.input["active_only"]))
    }
    fn fx_active(&mut self, fx: Object) -> effects::Result<bool> {
        self.events.push(json!(["fx_active", fx.0]));
        Ok(b(&self.input["fx"][fx.0 as usize]["active"]))
    }
    fn emit_state_fx(
        &mut self,
        _: ActorId,
        list: FxList,
        index: u8,
        active: bool,
    ) -> effects::Result<()> {
        self.events.push(json!([
            "fx_emit",
            if list == FxList::Enter {
                "enter"
            } else {
                "leave"
            },
            index,
            active
        ]));
        Ok(())
    }
    fn group_reference(&self, _: ActorId) -> effects::Result<ActorId> {
        Ok(ActorId::from_raw(if b(&self.input["group"]) {
            0x90002
        } else {
            0
        }))
    }
    fn resolve_reference(&self, id: ActorId) -> effects::Result<Option<ActorId>> {
        Ok((id.raw() == 0x90002 && b(&self.input["group_valid"])).then_some(id))
    }
    fn remove_from_group(&mut self, _: ActorId, _: ActorId) -> effects::Result<()> {
        self.events.push(json!(["remove_group"]));
        Ok(())
    }
    fn thing_flags(&self, _: ActorId) -> effects::Result<u32> {
        Ok(self.flags)
    }
    fn definition_flags(&self, _: ActorId) -> effects::Result<u32> {
        Ok(n(&self.input["def_flags"]))
    }
    fn set_thing_flags(&mut self, _: ActorId, flags: u32) -> effects::Result<()> {
        self.flags = flags;
        Ok(())
    }
}
#[test]
fn original_state_effects_property_view_and_fx_order_matches() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-state-effects.json"
    ))
    .unwrap();
    for (index, row) in corpus["cases"].as_array().unwrap().iter().enumerate() {
        let i = &row["input"];
        let out = &row["output"];
        let mut world = Fixture::new(i);
        let owner = ActorId::from_raw(1);
        let pc = match n(&i["precision"]) {
            24 => Precision::Single24,
            53 => Precision::Double53,
            _ => Precision::Extended64,
        };
        if i["mode"] == "enter" {
            effects::enter(
                &mut world,
                owner,
                n(&i["index"]) as u8,
                i["old"].as_u64().map(|x| x as u8),
                pc,
            )
        } else {
            effects::leave(&mut world, owner, n(&i["index"]) as u8, pc)
        }
        .unwrap();
        assert_eq!(json!(world.events), out["events"], "row{index}");
        assert_eq!(world.view, out["view"].as_i64().unwrap() as i32);
        assert_eq!(world.flags, n(&out["flags"]));
        assert_eq!(world.org.created_at.0, n(&out["organization_time"]));
        assert_eq!(world.health, n(&out["health"]));
    }
}
#[test]
fn property_callback_error_keeps_prior_economy_effect_and_stops_later_work() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-state-effects.json"
    ))
    .unwrap();
    let input = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["input"])
        .find(|i| i["mode"] == "enter" && b(&i["economy"]) && b(&i["properties"]))
        .unwrap();
    let mut world = Fixture::new(input);
    world.fail_add = true;
    assert!(
        effects::enter(
            &mut world,
            ActorId::from_raw(1),
            n(&input["index"]) as u8,
            input["old"].as_u64().map(|n| n as u8),
            Precision::Double53
        )
        .is_err()
    );
    assert!(world.events.iter().any(|e| e[0] == "economy"));
    assert_eq!(world.events.last().unwrap()[0], "add");
}
