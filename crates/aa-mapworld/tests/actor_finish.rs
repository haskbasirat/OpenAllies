use aa_actors::{
    ActorId,
    components::ComponentSlot,
    loading::{LoadKind, construct_shell_with_components},
    owned::{self, Context, Definitions},
};
use aa_mapworld::{
    actor_finish::finish_unlinked_actor,
    actor_payload::ActorTail,
    restore::{RestoreDefinitions, RestoredComponent, RuntimeDefinition},
};
use aa_numeric::Precision;
struct Bindings(bool);
#[test]
fn ground_requirement_dependencies_match_all_original_subsets() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-ground-requirements.json"
    ))
    .unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2048);
    for c in cases {
        let mask = c["mask"].as_u64().unwrap() as u32;
        let mut valid = true;
        for slot in 0..31 {
            if mask & (1 << slot) != 0 {
                let slot = ComponentSlot::from_index(slot).unwrap();
                if let Some(required) =
                    aa_mapworld::actor_finish::required_by_ground_slot(slot).unwrap()
                {
                    valid &= mask & (1 << required.index()) != 0;
                }
            }
        }
        assert_eq!(valid, c["valid"].as_bool().unwrap());
    }
}
impl RestoreDefinitions for Bindings {
    fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
        Err("unused".into())
    }
    fn localize(&self, key: &[u16]) -> Result<Vec<u16>, String> {
        assert!(key.is_empty());
        Ok(vec![])
    }
    fn actor_numbered(&self, _: &[u16], _: Precision) -> Result<bool, String> {
        Ok(self.0)
    }
}
#[test]
fn actual_start_location_owned_tail_matches_native_return() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-start-location.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        let mut base = construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, ());
        base.thing.flags = case["runtime_flags"].as_u64().unwrap() as u32;
        let actor = base.actor.as_mut().unwrap();
        let owner = if case["native"]["owner_present"].as_bool().unwrap() {
            case["saved_owner"].as_u64().unwrap() as u32
        } else {
            0
        };
        actor.words.insert(0xe8, owner);
        let input: ActorTail = serde_json::from_value(case["tail"].clone()).unwrap();
        let component = owned::construct(
            ComponentSlot::Economy,
            &Definitions::<String> {
                resource_domain_count: Some(7),
                ..Default::default()
            },
            Context {
                owner: ActorId::from_raw(case["source_id"].as_u64().unwrap() as u32),
                world_time: aa_body::F32::ZERO,
                position: [aa_body::F32::ZERO; 3],
                precision: Precision::Double53,
            },
        )
        .unwrap();
        actor.components.insert(ComponentSlot::Economy, component);
        let mut registry = aa_actors::IdentityRegistry::new();
        let mut messages = Vec::new();
        finish_unlinked_actor(
            &mut base,
            &input,
            &"start_loc".encode_utf16().collect::<Vec<_>>(),
            "start_loc",
            &Bindings(false),
            Precision::Double53,
            &mut aa_mapworld::actor_finish::NameTail {
                id: ActorId::from_raw(0x10005),
                actor_version: 14,
                registry: &mut registry,
                messages: &mut messages,
                source_ordinal: 0,
                world_time_bits: 0,
            },
        )
        .unwrap();
        let actor = base.actor.as_ref().unwrap();
        assert_eq!(actor.words[&0xe8], owner);
        assert!(actor.components.present(ComponentSlot::Economy));
        for offset in [0x158u32, 0x15c, 0x160, 0x164, 0x168, 0x16c, 0x170, 0x174] {
            assert_eq!(
                actor.words[&offset],
                case["native"]["fields"][format!("word_{offset:x}")]
                    .as_u64()
                    .unwrap() as u32
            );
        }
    }
}
#[test]
fn live_spatial_and_numbering_branches_remain_explicit_after_saved_fields() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-start-location.json"
    ))
    .unwrap();
    let mut tail: ActorTail = serde_json::from_value(fixture["cases"][0]["tail"].clone()).unwrap();
    tail.word_168 = Some(123);
    let mut base = construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, ());
    let mut registry = aa_actors::IdentityRegistry::new();
    let mut messages = Vec::new();
    macro_rules! names {
        () => {
            &mut aa_mapworld::actor_finish::NameTail {
                id: ActorId::from_raw(0x10005),
                actor_version: 14,
                registry: &mut registry,
                messages: &mut messages,
                source_ordinal: 0,
                world_time_bits: 0xbf80_0000,
            }
        };
    }
    // 4D67F0 queries the spatial index only for bit 10 set, bit 29 clear and
    // an eligible state (4D10A0).
    base.thing.flags = 1 << 10;
    assert!(
        finish_unlinked_actor(&mut base, &tail, &[], "x", &Bindings(false), Precision::Double53, names!())
            .unwrap_err()
            .contains("4D67F0")
    );
    assert_eq!(base.actor.as_ref().unwrap().words[&0x168], 123);
    base.thing.flags |= 1 << 29;
    base.actor.as_mut().unwrap().words.insert(0xe8, 1);
    // Numbered ActorData with an owner and no saved number: 4E4980 on the
    // owner Kingdom's restored +35C table; without the ActorDefinition
    // identity bank (or a restored Kingdom) this is an explicit error.
    let error = finish_unlinked_actor(&mut base, &tail, &[], "x", &Bindings(true), Precision::Double53, names!())
        .unwrap_err();
    assert!(error.contains("identity bank"), "{error}");
    assert!(messages.is_empty());
}
