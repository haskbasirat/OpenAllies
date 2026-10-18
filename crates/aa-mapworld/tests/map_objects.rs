use aa_mapworld::{
    map_objects::{Area, Camera},
    restore::{RestoreDefinitions, RuntimeDefinition},
    restore_components::UpdatableTarget,
};
use aa_numeric::Precision;
use std::cell::RefCell;
#[derive(Default)]
struct Bindings(RefCell<Vec<(i32, UpdatableTarget)>>);
impl RestoreDefinitions for Bindings {
    fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
        Err("unused".into())
    }
    fn localize(&self, _: &[u16]) -> Result<Vec<u16>, String> {
        Ok(vec![])
    }
    fn register_updatable(&self, id: i32, target: &UpdatableTarget) -> Result<(), String> {
        self.0.borrow_mut().push((id, target.clone()));
        Ok(())
    }
}
#[test]
fn original_area_camera_constructors_and_loads_match() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-map-objects.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 90);
    assert_eq!(fixture["constructors"].as_array().unwrap().len(), 6);
    for c in fixture["constructors"].as_array().unwrap() {
        assert_eq!(c["label_length"], 0);
        let values = c["words"].as_object().unwrap();
        for (o, v) in values {
            if o == "0x60" && c["kind"] == "Area" {
                assert_eq!(*v, 0xa289d8);
            } else {
                assert_eq!(*v, 0);
            }
        }
    }
    for c in cases {
        let bytes: Vec<_> = c["payload_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let definitions = Bindings::default();
        let mut trace = vec![];
        if c["kind"] == "Camera" {
            let mut state = Camera::default();
            let n = state.load(&bytes, &definitions).unwrap();
            assert_eq!(n, bytes.len());
            assert_eq!(
                state.word_60,
                c["native"]["word60"].as_u64().unwrap() as u32
            );
            assert_eq!(
                serde_json::to_value(state.label_utf16).unwrap(),
                c["native"]["label"]
            );
        } else {
            let mut state = Area::default();
            let n = state
                .load(
                    &bytes,
                    c["world_version"].as_u64().unwrap() as u32,
                    42,
                    &definitions,
                    &mut trace,
                )
                .unwrap();
            assert_eq!(n, bytes.len());
            assert_eq!(
                serde_json::to_value(state.label_utf16).unwrap(),
                c["native"]["label"]
            );
            assert_eq!(
                serde_json::to_value(state.shape_words).unwrap(),
                c["native"]["shape"]
            );
            let entries: Vec<_> = state
                .entries
                .iter()
                .map(|v| [v.saved_handle, v.byte as u32])
                .collect();
            assert_eq!(
                serde_json::to_value(entries).unwrap(),
                c["native"]["entries"]
            );
            let registered = c["native"]["registered_id0"].as_bool().unwrap();
            assert_eq!(!definitions.0.borrow().is_empty(), registered);
            assert_eq!(trace.len(), usize::from(registered));
            if registered {
                assert_eq!(
                    definitions.0.borrow()[0],
                    (0, UpdatableTarget::Area { owner: 42 })
                );
            }
        }
    }
}
#[test]
fn area_registration_failure_precedes_version_shape_and_labels() {
    struct Reject;
    impl RestoreDefinitions for Reject {
        fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
            Err("unused".into())
        }
        fn register_updatable(&self, _: i32, _: &UpdatableTarget) -> Result<(), String> {
            Err("duplicate".into())
        }
    }
    let mut area = Area::default();
    let mut trace = vec![];
    let bytes: Vec<_> = [1u32, 6].into_iter().flat_map(u32::to_le_bytes).collect();
    let error = area.load(&bytes, 28, 10, &Reject, &mut trace).unwrap_err();
    assert!(error.to_string().contains("duplicate"));
    assert_eq!(area.source_version, None);
    assert_eq!(area.shape_words, [0; 6]);
    assert!(area.label_utf16.is_empty());
    assert!(trace.is_empty());
}
