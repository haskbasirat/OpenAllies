use aa_actors::{
    ActorId,
    components::{ComponentSlot, StateBits},
    init_create::{
        self, Definition, FactoryKind, LinkedDefinition, LinkedRequest, Options, Result, World,
    },
};
use aa_body::F32;
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn vecf<const N: usize>(v: &Value) -> [F32; N] {
    std::array::from_fn(|i| F32(n(&v[i])))
}
struct Fixture {
    input: Value,
    present: BTreeSet<u8>,
    low: u32,
    high: u32,
    linked: ActorId,
    parent: ActorId,
    events: Vec<Value>,
    owner_mutated: bool,
}
impl World for Fixture {
    type Actor = u32;
    type Definition = u32;
    type Record = ();
    fn definition(&self, _: u32) -> Result<Definition<u32>> {
        let i = &self.input;
        Ok(Definition {
            identity: 1,
            component_mask: n(&i["mask"]),
            number_registration: (n(&i["number_definition"]) != 0)
                .then(|| n(&i["number_definition"])),
            automatic_number: b(&i["auto_number"]),
            anchor: vecf(&i["anchor"]),
            linked: b(&i["linked"]).then(|| LinkedDefinition {
                definition: 2,
                anchor: vecf(&i["child_anchor"]),
            }),
        })
    }
    fn initialize_object(&mut self, _: u32, _: &(), _: Precision) -> Result<bool> {
        self.events.push(json!({"kind":"object"}));
        Ok(b(&self.input["base_ok"]))
    }
    fn state_group(&self, _: u32, _: u32) -> Result<Option<u32>> {
        Ok((self.low & 0xfe000000 != 0).then_some(27))
    }
    fn set_state(&mut self, _: u32, index: u32, enable: bool) -> Result<()> {
        self.events
            .push(json!({"kind":"set_state","index":index,"enable":enable}));
        self.low |= 1 << index;
        Ok(())
    }
    fn state_bits(&self, _: u32) -> Result<StateBits> {
        Ok(StateBits {
            low: self.low,
            high: self.high,
        })
    }
    fn has_component(&self, actor: u32, slot: ComponentSlot) -> Result<bool> {
        Ok(if actor == 2 {
            slot == ComponentSlot::Body && b(&self.input["child_body"])
        } else {
            self.present.contains(&(slot as u8))
        })
    }
    fn construct_component(
        &mut self,
        _: u32,
        slot: ComponentSlot,
        _: FactoryKind,
        _: Precision,
    ) -> Result<()> {
        self.events.push(json!({"kind":"create","slot":slot as u8}));
        self.present.insert(slot as u8);
        Ok(())
    }
    fn component_requirement(
        &mut self,
        _: u32,
        slot: ComponentSlot,
    ) -> Result<Option<ComponentSlot>> {
        self.events
            .push(json!({"kind":"requirement","slot":slot as u8}));
        Ok(None)
    }
    fn city_allowed(&mut self, _: u32) -> Result<bool> {
        self.events.push(json!({"kind":"city_predicate"}));
        Ok(b(&self.input["city"]))
    }
    fn construction_ready(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind":"construction_callback"}));
        Ok(())
    }
    fn register_number(&mut self, _: u32, definition: &u32) -> Result<()> {
        self.events
            .push(json!({"kind":"number_definition","definition":definition}));
        Ok(())
    }
    fn stored_owner(&self, _: u32) -> Result<Option<u32>> {
        Ok(b(&self.input["owner"]).then_some(7))
    }
    fn owner(&mut self, _: u32) -> Result<Option<u32>> {
        self.owner_mutated = b(&self.input["mutate_owner"]);
        self.stored_owner(1)
    }
    fn assembly_completed(&mut self, _: u32, _: &[ComponentSlot]) -> Result<()> {
        self.events.push(json!({"kind":"assembly_completed"}));
        Ok(())
    }
    fn assign_owner_number(&mut self, _: u32, owner: u32) -> Result<()> {
        self.events
            .push(json!({"kind":"owner_number","owner":owner}));
        Ok(())
    }
    fn character_has_start(&self, _: u32) -> Result<bool> {
        Ok(b(&self.input["view_count"]) && b(&self.input["view_first"]))
    }
    fn start_character(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind":"character_start"}));
        Ok(())
    }
    fn default_stance(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"kind":"default_stance"}));
        Ok(())
    }
    fn position(&self, _: u32) -> Result<[F32; 3]> {
        Ok(vecf(
            &self.input[if self.owner_mutated {
                "mutated_position"
            } else {
                "position"
            }],
        ))
    }
    fn facing_xy(&self, _: u32) -> Result<[F32; 2]> {
        Ok(vecf(
            &self.input[if self.owner_mutated {
                "mutated_facing"
            } else {
                "facing"
            }],
        ))
    }
    fn actor_id(&self, actor: u32) -> Result<ActorId> {
        Ok(ActorId::from_raw(if actor == 2 {
            0x90002
        } else {
            0x90001
        }))
    }
    fn create_linked(
        &mut self,
        request: LinkedRequest<u32, u32>,
        _: Precision,
    ) -> Result<Option<u32>> {
        self.events.push(json!({"kind":"linked_create","position":request.position.map(|f|f.0),"facing":request.facing_xy.map(|f|f.0),"owner":request.owner.unwrap_or(0),"parent":request.parent_id.raw(),"nonblocking":request.nonblocking}));
        Ok(b(&self.input["child_created"]).then_some(2))
    }
    fn set_linked_id(&mut self, _: u32, id: ActorId) -> Result<()> {
        self.linked = id;
        Ok(())
    }
    fn resolve_id(&self, id: ActorId) -> Result<Option<u32>> {
        Ok((id.raw() == 0x90002 && b(&self.input["child_resolves"])).then_some(2))
    }
    fn set_parent_id(&mut self, _: u32, id: ActorId) -> Result<()> {
        self.parent = id;
        Ok(())
    }
    fn copy_linked_health(&mut self, _: u32, _: u32, _: Precision) -> Result<()> {
        self.events.push(json!({"kind":"copy_health"}));
        Ok(())
    }
    fn initialize_component(
        &mut self,
        _: u32,
        slot: ComponentSlot,
        _: &(),
        _: Precision,
    ) -> Result<bool> {
        self.events
            .push(json!({"kind":"init_create","slot":slot as u8}));
        Ok(i64::from(slot as u8) != self.input["fail_slot"].as_i64().unwrap())
    }
    fn register_construction(&mut self, _: u32, owner: Option<u32>, _: &u32) -> Result<()> {
        self.events
            .push(json!({"kind":"register_construction","owner":owner.unwrap_or(0)}));
        Ok(())
    }
}
#[test]
fn full_original_init_create_order_and_linked_anchor_stores_match() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-init-create.json"
    ))
    .unwrap();
    for (index, row) in corpus["cases"].as_array().unwrap().iter().enumerate() {
        let i = &row["input"];
        let out = &row["output"];
        let mut world = Fixture {
            input: i.clone(),
            present: i["present"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| n(x) as u8)
                .collect(),
            low: n(&i["low"]),
            high: n(&i["high"]),
            linked: ActorId::NULL,
            parent: ActorId::NULL,
            events: vec![],
            owner_mutated: false,
        };
        let pc = match n(&i["precision"]) {
            24 => Precision::Single24,
            53 => Precision::Double53,
            _ => Precision::Extended64,
        };
        let result = init_create::initialize(
            1,
            &(),
            Options {
                reason: n(&i["reason"]),
                start_character: b(&i["start"]),
                nonblocking: b(&i["nonblocking"]),
            },
            &mut world,
            pc,
        )
        .unwrap();
        assert_eq!(result, b(&out["result"]), "row{index}");
        assert_eq!(json!(world.events), out["events"], "row{index}");
        assert_eq!(
            json!(world.present.into_iter().collect::<Vec<_>>()),
            out["present"],
            "row{index}"
        );
        assert_eq!(world.low, n(&out["low"]));
        assert_eq!(world.high, n(&out["high"]));
        assert_eq!(world.linked.raw(), n(&out["linked"]));
        assert_eq!(world.parent.raw(), n(&out["parent"]));
    }
}

#[test]
fn native_factory_mind_precedence_and_slot_dispatch_match() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-factory-dispatch.json"
    ))
    .unwrap();
    for row in corpus["cases"].as_array().unwrap() {
        let slot = aa_actors::components::ALL_SLOTS[n(&row["slot"]) as usize];
        let selected = init_create::factory_kind(slot, n(&row["mask"]));
        assert_eq!(selected.is_some(), b(&row["output"]["installed"]));
        let expected = row["output"]["constructors"].as_array().unwrap();
        match selected {
            Some(FactoryKind::OrganizationMind) => assert_eq!(expected, &[json!("0x526260")]),
            Some(FactoryKind::AircraftMind) => assert_eq!(expected, &[json!("0x51bff0")]),
            Some(FactoryKind::ElementMind) => assert_eq!(expected, &[json!("0x535ba0")]),
            Some(FactoryKind::Standard(s)) => {
                assert_eq!(s, slot);
                assert_eq!(expected.len(), 1);
            }
            None => assert!(expected.is_empty()),
        }
    }
}
