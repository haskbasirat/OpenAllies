use aa_body::{
    Result,
    credit::Object,
    property_api::{RemovalWorld, remove_property},
    property_effects::Selection,
};
use serde_json::Value;

struct Adapter {
    component: Option<Object>,
    result: bool,
    calls: Vec<(Object, Selection)>,
}
impl RemovalWorld for Adapter {
    fn property_component(&self, actor: Object) -> Result<Option<Object>> {
        assert_eq!(actor, Object(1));
        Ok(self.component)
    }
    fn remove_from_component(&mut self, component: Object, selection: Selection) -> Result<bool> {
        self.calls.push((component, selection));
        Ok(self.result)
    }
}

#[test]
fn remove_property_wrapper_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-property-wrappers-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let input = &case["input"];
        let component = input["component"].as_u64().unwrap() as u32;
        let property = input["property"].as_u64().unwrap() as u32;
        let source = input["source"].as_u64().unwrap() as u32;
        let mut world = Adapter {
            component: (component != 0).then_some(Object(component)),
            result: input["seam_result"].as_u64().unwrap() != 0,
            calls: vec![],
        };
        let returned = remove_property(
            Object(1),
            Object(property),
            (source != 0).then_some(Object(source)),
            &mut world,
        )
        .unwrap();
        let native_calls = case["calls"].as_array().unwrap();
        assert_eq!(world.calls.len(), native_calls.len());
        for ((c, s), n) in world.calls.iter().zip(native_calls) {
            assert_eq!(c.0 as u64, n["component"].as_u64().unwrap());
            assert_eq!(s.definition.0 as u64, n["property"].as_u64().unwrap());
            assert_eq!(
                s.source.map_or(0, |o| o.0) as u64,
                n["source"].as_u64().unwrap()
            );
        }
        assert_eq!(returned.map(u64::from), case["returned_al"].as_u64());
    }
}
