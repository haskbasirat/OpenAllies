use aa_mapworld::{
    dynamic_doodad::{Definition, DynamicDoodad},
    restore_components::UpdatableTarget,
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Fixture {
    constructors: Vec<Ctor>,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Ctor {
    id: u32,
    vtable: u32,
    slots: [u32; 2],
}
#[derive(Deserialize)]
struct Case {
    definition_mask: u32,
    world_mode: u32,
    payload_hex: String,
    native: Native,
}
#[derive(Deserialize)]
struct Native {
    components: [Option<NativeComponent>; 2],
    registrations: Vec<[u32; 2]>,
    failed: Option<String>,
    consumed: usize,
}
#[derive(Deserialize)]
struct NativeComponent {
    owner_matches: bool,
    definition_matches: bool,
    head: u32,
    words: Vec<u32>,
}
#[test]
fn whole_native_doodad_load_and_two_slot_factory_match() {
    let f: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-dynamic-doodad.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 512);
    assert_eq!(f.constructors.len(), 3);
    for c in f.constructors {
        assert_ne!(c.id, 0);
        assert_eq!(c.vtable, 0xa28e90);
        assert_eq!(c.slots, [0, 0]);
        assert!(
            DynamicDoodad::<u32>::default()
                .components
                .iter()
                .all(Option::is_none)
        );
    }
    for case in f.cases {
        let bytes: Vec<_> = case
            .payload_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let mut s = DynamicDoodad::default();
        let mut events = vec![];
        let mut callbacks = vec![];
        let result = s.load(
            &bytes,
            &Definition {
                owner_id: "authored-doodad".into(),
                component_mask: case.definition_mask,
            },
            42,
            Some(case.world_mode),
            |id, target| {
                callbacks.push((id, target.clone()));
                Ok(())
            },
            &mut events,
        );
        assert_eq!(
            result.is_err(),
            case.native.failed.is_some(),
            "{}",
            case.payload_hex
        );
        if let Ok(n) = result {
            assert_eq!(n, case.native.consumed);
        }
        for (slot, expected) in case.native.components.iter().enumerate() {
            assert_eq!(s.components[slot].is_some(), expected.is_some());
            if let Some(native) = expected {
                let ours = s.components[slot].as_ref().unwrap();
                assert!(native.owner_matches && native.definition_matches);
                assert_eq!(native.head, 0);
                assert_eq!(ours.owner, 42);
                assert_eq!(ours.definition_owner, "authored-doodad");
                assert_eq!(ours.slot, slot as u8);
                assert_eq!(ours.scheduled_head, None);
                assert_eq!(ours.words, native.words);
            }
        }
        let expected: Vec<_> = case
            .native
            .registrations
            .into_iter()
            .map(|[id, slot]| {
                (
                    id as i32,
                    UpdatableTarget::DynamicDoodad {
                        owner: 42,
                        slot: slot as u8,
                    },
                )
            })
            .collect();
        assert_eq!(callbacks, expected);
        assert_eq!(events.len(), callbacks.len());
    }
}
#[test]
fn updatable_failure_stops_before_later_wire_fields_and_second_slot() {
    // First Disturbable layer carries Updatable1/id7 but the following version
    // and second component payload are intentionally absent: registration fails
    // before the original reader could observe either missing portion.
    let bytes: Vec<_> = [0u32, 3, 3, 0, 1, 7]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    let mut state = DynamicDoodad::default();
    let mut trace = vec![];
    let mut calls = 0;
    let error = state
        .load(
            &bytes,
            &Definition {
                owner_id: "source".into(),
                component_mask: 3,
            },
            10,
            Some(0),
            |id, target| {
                calls += 1;
                assert_eq!(id, 7);
                assert_eq!(
                    *target,
                    UpdatableTarget::DynamicDoodad { owner: 10, slot: 0 }
                );
                Err("duplicate ID".into())
            },
            &mut trace,
        )
        .unwrap_err();
    assert!(error.to_string().contains("duplicate ID"));
    assert_eq!(calls, 1);
    assert!(trace.is_empty());
    assert_eq!(state.components[0].as_ref().unwrap().words, [0]);
    assert_eq!(
        state.components[0]
            .as_ref()
            .unwrap()
            .registered_updatable_id,
        None
    );
    assert!(state.components[1].is_none());
}
