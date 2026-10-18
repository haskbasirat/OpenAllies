use aa_actors::{
    ActorId,
    ability_flags::{self as flags, ActorFields, AttackGroup, ElementView, World},
    components::ComponentSlot,
};
use serde_json::{Value, json};
fn n(v: &Value) -> u64 {
    v.as_u64().unwrap()
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
struct Fixture {
    input: Value,
    actions: u64,
    parent_actions: u64,
    element_owner: u32,
    events: Vec<Value>,
}
impl World for Fixture {
    fn actor_fields(&self, actor: ActorId) -> flags::Result<ActorFields> {
        let id = actor.raw();
        Ok(match id {
            1 => ActorFields {
                thing_flags: n(&self.input["flags"]) as u32,
                state_bits: n(&self.input["state"]),
                action_bits: self.actions,
            },
            2 => ActorFields {
                thing_flags: self.input["parent_flags"].as_u64().unwrap_or(0) as u32,
                state_bits: n(&self.input["parent_state"]),
                action_bits: self.parent_actions,
            },
            _ if id >= 10 => ActorFields {
                thing_flags: 0,
                state_bits: 0,
                action_bits: self.input["members"][(id - 10) as usize]
                    .as_u64()
                    .unwrap_or(0),
            },
            _ => return Err(flags::Error::World("fixture actor missing".into())),
        })
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        match actor.raw() {
            1 => self.actions = bits,
            2 => self.parent_actions = bits,
            _ => panic!("unexpected action recipient"),
        };
        Ok(())
    }
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> flags::Result<bool> {
        if actor.raw() == 2 {
            return Ok(slot == ComponentSlot::Organization);
        }
        Ok(match slot {
            ComponentSlot::Element => b(&self.input["has_element"]),
            ComponentSlot::Organization => b(&self.input["has_org"]),
            ComponentSlot::Wall => b(&self.input["wall"]),
            ComponentSlot::Mind => b(&self.input["mind"]),
            _ => self.input["other"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| n(x) == slot as u64),
        })
    }
    fn element(&self, _: ActorId) -> flags::Result<ElementView> {
        Ok(ElementView {
            owner: ActorId::from_raw(self.element_owner),
            definition_present: b(&self.input["element_definition"]),
            organization: b(&self.input["element_parent"]).then_some(ActorId::from_raw(2)),
            ready: b(&self.input["element_ready"]),
        })
    }
    fn organization_member_count(&self, _: ActorId) -> flags::Result<usize> {
        Ok(self.input["members"].as_array().unwrap().len())
    }
    fn organization_member(&self, _: ActorId, index: usize) -> flags::Result<Option<ActorId>> {
        Ok(self.input["members"][index]
            .as_u64()
            .map(|_| ActorId::from_raw(10 + index as u32)))
    }
    fn organization_member_44(&self, _: ActorId) -> flags::Result<Option<ActorId>> {
        Ok(self.input["selected"]
            .as_u64()
            .map(|index| ActorId::from_raw(10 + index as u32)))
    }
    fn organization_reference_d0(&self, _: ActorId) -> flags::Result<ActorId> {
        Ok(ActorId::from_raw(0x90002))
    }
    fn valid_reference(&self, id: ActorId) -> flags::Result<bool> {
        assert_eq!(id.raw(), 0x90002);
        Ok(b(&self.input["valid_ref"]))
    }
    fn organization_group_count(&self, _: ActorId) -> flags::Result<usize> {
        Ok(self.input["groups"].as_array().unwrap().len())
    }
    fn organization_attack_group(&self, _: ActorId, index: usize) -> flags::Result<AttackGroup> {
        let g = &self.input["groups"][index];
        Ok(AttackGroup {
            locked: b(&g["locked"]),
            class_flags: g["classes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| n(v) as u32)
                .collect(),
        })
    }
    fn organization_group_locked(&self, _: ActorId, index: usize) -> flags::Result<bool> {
        Ok(b(&self.input["groups"][index]["locked"]))
    }
    fn organization_group_ability_count(&self, _: ActorId, index: usize) -> flags::Result<usize> {
        self.input["groups"][index]["classes"]
            .as_array()
            .map(Vec::len)
            .ok_or(flags::Error::Missing("native null parameter"))
    }
    fn organization_group_ability_flags(
        &self,
        _: ActorId,
        index: usize,
        ability: usize,
    ) -> flags::Result<u32> {
        self.input["groups"][index]["classes"][ability]
            .as_u64()
            .map(|n| n as u32)
            .ok_or(flags::Error::Missing("native null local AbilityData"))
    }
    fn owner(&mut self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        self.events.push(json!(["owner", actor.raw()]));
        if self.input["rebind_element_owner"]
            .as_bool()
            .unwrap_or(false)
        {
            self.element_owner = 2;
        }
        Ok(b(&self.input["owner"]).then_some(ActorId::from_raw(7)))
    }
    fn kingdom_word_218(&self, owner: ActorId) -> flags::Result<u32> {
        assert_eq!(owner.raw(), 7);
        Ok(n(&self.input["owner218"]) as u32)
    }
    fn state_resupply(&self, index: u8) -> flags::Result<bool> {
        Ok(b(&self.input["resupply"][index as usize]))
    }
    fn owner_can_recruit(&mut self, owner: ActorId, _: ActorId) -> flags::Result<bool> {
        assert_eq!(owner.raw(), 7);
        self.events.push(json!(["can_recruit"]));
        Ok(b(&self.input["eligible"]))
    }
    fn owner_recruit_count(&mut self, owner: ActorId, _: ActorId) -> flags::Result<i32> {
        assert_eq!(owner.raw(), 7);
        self.events.push(json!(["recruit_count"]));
        Ok(self.input["recruit_count"].as_i64().unwrap() as i32)
    }
    fn action_available(&mut self, actor: ActorId, index: u32) -> flags::Result<bool> {
        flags::available(self.actor_fields(actor)?.action_bits, index)
    }
    fn other_contributor(&mut self, _: ActorId, slot: ComponentSlot) -> flags::Result<()> {
        self.events.push(json!(["other", slot as u32]));
        self.actions |= 1 << (slot as u8);
        Ok(())
    }
    fn mind_contributor(&mut self, _: ActorId) -> flags::Result<()> {
        self.events.push(json!(["mind"]));
        self.actions |= n(&self.input["mind_mask"]);
        Ok(())
    }
    fn changed(&mut self, _: ActorId) -> flags::Result<()> {
        self.events.push(json!(["changed", self.actions]));
        Ok(())
    }
}
#[test]
fn original_full_rebuild_element_organization_member_scans_and_callback_order_match() {
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-ability-flags.json"
    ))
    .unwrap();
    for (index, row) in native["cases"].as_array().unwrap().iter().enumerate() {
        let i = &row["input"];
        let mut world = Fixture {
            input: i.clone(),
            actions: n(&i["actions"]),
            parent_actions: 0,
            element_owner: 1,
            events: vec![],
        };
        let owner = ActorId::from_raw(1);
        match i["mode"].as_str().unwrap() {
            "rebuild" => flags::rebuild(&mut world, owner),
            "element" => flags::element(&mut world, owner),
            "organization" => flags::organization(&mut world, owner),
            _ => panic!("mode"),
        }
        .unwrap();
        assert_eq!(
            world.actions,
            n(&row["output"]["actions"]),
            "row{index}: {i}"
        );
        assert_eq!(world.parent_actions, n(&row["output"]["parent_actions"]));
        assert_eq!(
            json!(world.events),
            row["output"]["events"],
            "row{index}: {i}"
        );
    }
}
#[test]
fn invalid_command_index_preserves_previous_mask() {
    let mut bits = 17;
    assert!(flags::set_action(&mut bits, 64, true).is_err());
    assert_eq!(bits, 17);
    assert!(flags::available(bits, 64).is_err());
}

#[test]
fn native_group_scan_skips_unlocked_null_parameters_and_unread_late_abilities() {
    let cases: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-group-classes/native-group-scan.json"
    ))
    .unwrap();
    assert_eq!(cases["cases"].as_array().unwrap().len(), 96);
    for row in cases["cases"].as_array().unwrap() {
        let groups: Vec<_> = row["input"]["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| json!({"locked":g["locked"],"classes":g["abilities"]}))
            .collect();
        let world = Fixture {
            input: json!({"groups":groups}),
            actions: 0,
            parent_actions: 0,
            element_owner: 1,
            events: vec![],
        };
        let actual = flags::has_attack_class(
            &world,
            ActorId::from_raw(1),
            b(&row["input"]["locked_only"]),
        );
        if let Some(expected) = row["output"]["result"].as_bool() {
            assert_eq!(actual.unwrap(), expected, "{}", row["input"]["name"]);
        } else {
            assert!(
                actual.is_err(),
                "native invalid read must not become success: {}",
                row["input"]["name"]
            );
        }
    }
}

#[test]
fn native_organization_status_preserves_conditional_reads_and_stack_query() {
    use aa_actors::ground_states::{LayoutDefinition, OrganizationState};
    use aa_body::F32;
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-organization-status.json"
    ))
    .unwrap();
    for row in native["cases"].as_array().unwrap() {
        let i = &row["input"];
        let mut org = OrganizationState::new(
            ActorId::from_raw(1),
            LayoutDefinition {
                definition: (),
                parameters: vec![],
                slots: 0,
            },
            F32::ZERO,
            F32(n(&i["threshold"]) as u32),
            [F32::ZERO; 2],
        )
        .unwrap();
        org.words.insert(0x60, n(&i["word60"]) as u32);
        org.words.insert(0x5c, n(&i["word5c"]) as u32);
        let mut calls = 0;
        let status = flags::organization_status(
            &org,
            b(&i["mind"]).then_some([n(&i["words"][0]) as u32, n(&i["words"][1]) as u32]),
            || {
                calls += 1;
                Ok(b(&i["exhausted"]))
            },
        )
        .unwrap();
        assert_eq!(u64::from(status), n(&row["output"]["status"]));
        assert_eq!(calls, n(&row["output"]["calls"]));
    }
    let mut org = OrganizationState::new(
        ActorId::from_raw(1),
        LayoutDefinition {
            definition: (),
            parameters: vec![],
            slots: 0,
        },
        F32::ZERO,
        F32(20f32.to_bits()),
        [F32::ZERO; 2],
    )
    .unwrap();
    assert!(flags::organization_status(&org, None, || panic!("must reject null Mind")).is_err());
    org.words.insert(0x60, 0x7fc00000);
    assert_eq!(
        flags::organization_status(&org, Some([1, 0]), || panic!("unread")).unwrap(),
        2
    );
    assert!(flags::organization_status(&org, Some([0, 0]), || panic!("nonfinite")).is_err());
}
