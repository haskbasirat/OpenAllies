use aa_actors::components::*;
use serde_json::{Value, json};

struct Hooks<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
fn slot(v: usize) -> ComponentSlot {
    ComponentSlot::from_index(v).unwrap()
}
impl Hooks<'_> {
    fn has(&self, key: &str, index: usize) -> bool {
        self.case[key]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_u64() == Some(index as u64)))
    }
    fn event(&mut self, event: Value, components: &mut Components<usize>) {
        let kind = event["kind"].as_str().unwrap();
        let index = event["slot"].as_u64();
        if let Some(ms) = self.case["mutations"].as_array() {
            for m in ms {
                if m["kind"] == kind && m["slot"].as_u64() == index {
                    if let Some(v) = m["low"].as_u64() {
                        components.state.low = v as u32;
                    }
                    if let Some(v) = m["high"].as_u64() {
                        components.state.high = v as u32;
                    }
                    if let Some(a) = m["add"].as_array() {
                        for s in a {
                            let i = s.as_u64().unwrap() as usize;
                            components.insert(slot(i), i);
                        }
                    }
                    if let Some(a) = m["remove"].as_array() {
                        for s in a {
                            components.remove(slot(s.as_u64().unwrap() as usize));
                        }
                    }
                }
            }
        }
        self.events.push(event);
    }
    fn succeeds(&self, s: ComponentSlot) -> bool {
        self.case["fail_slot"].as_i64() != Some(s.index() as i64)
    }
}
impl Requirements<usize> for Hooks<'_> {
    fn required_component(
        &mut self,
        s: ComponentSlot,
        c: &mut Components<usize>,
    ) -> Option<ComponentSlot> {
        self.event(json!({"kind":"requirement","slot":s.index()}), c);
        self.case["requirements"][s.index().to_string()]
            .as_u64()
            .map(|x| slot(x as usize))
    }
}
impl AssemblyHooks<usize> for Hooks<'_> {
    fn construct(&mut self, s: ComponentSlot, c: &mut Components<usize>) {
        if !self.has("null_factory", s.index()) {
            c.insert(s, s.index());
        }
        self.event(json!({"kind":"create","slot":s.index()}), c);
    }
    fn city_allowed(&mut self, _: &mut Components<usize>) -> bool {
        self.events.push(json!({"kind":"city_predicate"}));
        self.case["city"].as_bool().unwrap_or(false)
    }
    fn construction_ready(&mut self, _: &mut Components<usize>) {
        self.events.push(json!({"kind":"construction_callback"}));
    }
}
impl LifecycleHooks<usize, u32> for Hooks<'_> {
    fn init_create(&mut self, s: ComponentSlot, r: &u32, c: &mut Components<usize>) -> bool {
        self.event(json!({"kind":"init_create","slot":s.index(),"record":r}), c);
        self.succeeds(s)
    }
    fn init_existing(&mut self, s: ComponentSlot, c: &mut Components<usize>) -> bool {
        self.event(json!({"kind":"init_existing","slot":s.index()}), c);
        self.succeeds(s)
    }
    fn post_existing(&mut self, s: ComponentSlot, c: &mut Components<usize>) {
        self.event(json!({"kind":"post_existing","slot":s.index()}), c);
    }
    fn post_create(&mut self, s: ComponentSlot, r: &u32, f: u32, c: &mut Components<usize>) {
        self.event(
            json!({"kind":"post_create","slot":s.index(),"record":r,"flag":f}),
            c,
        );
    }
}

#[test]
fn original_component_selection_dependencies_lifecycle_and_mutations() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../research/phase4-actors/components-native-oracle.json"
    ))
    .unwrap();
    assert_eq!(oracle["cases"].as_array().unwrap().len(), 85);
    for row in oracle["cases"].as_array().unwrap() {
        let case = &row["input"];
        let mut c = Components::new(StateBits {
            low: case["low"].as_u64().unwrap_or(0) as u32,
            high: case["high"].as_u64().unwrap_or(0) as u32,
        });
        if let Some(a) = case["present"].as_array() {
            for s in a {
                let i = s.as_u64().unwrap() as usize;
                c.insert(slot(i), i);
            }
        }
        let mut h = Hooks {
            case,
            events: Vec::new(),
        };
        let result = match case["mode"].as_str().unwrap() {
            "assemble" => {
                match c.assemble_initial(case["mask"].as_u64().unwrap_or(0) as u32, &mut h) {
                    Ok(created) => {
                        assert_eq!(
                            created.iter().map(|s| s.index()).collect::<Vec<_>>(),
                            h.events
                                .iter()
                                .filter(|e| e["kind"] == "create")
                                .map(|e| e["slot"].as_u64().unwrap() as usize)
                                .filter(|s| !h.has("null_factory", *s))
                                .collect::<Vec<_>>()
                        );
                        Some(true)
                    }
                    Err(AssemblyError::Missing(e)) => {
                        h.events.push(json!({"kind":"missing_requirement","slot":e.slot.index(),"required":e.required.index()}));
                        Some(false)
                    }
                    Err(e) => panic!("unexpected {e}"),
                }
            }
            "validate" => {
                let r = c.validate(&mut h);
                if let Err(e) = r {
                    h.events.push(json!({"kind":"missing_requirement","slot":e.slot.index(),"required":e.required.index()}));
                }
                Some(r.is_ok())
            }
            "create" => Some(c.init_create(&0x1234, &mut h).is_ok()),
            "existing" => {
                // The port explicitly begins after these two native phases.
                if case["base_ok"] == false || case["required_object_ok"] == false {
                    Some(false)
                } else {
                    Some(c.init_existing(&mut h).is_ok())
                }
            }
            "post" => {
                c.post_create(&0x1234, 1, &mut h);
                None
            }
            _ => unreachable!(),
        };
        assert_eq!(json!(result), row["result"], "{case}");
        assert_eq!(json!(h.events), row["events"], "{case}");
        assert_eq!(
            json!(c.iter().map(|(s, _)| s.index()).collect::<Vec<_>>()),
            row["present"],
            "{case}"
        );
        assert_eq!(json!(c.state.low), row["low"]);
        assert_eq!(json!(c.state.high), row["high"]);
    }
}

#[test]
fn slot_boundaries_and_failed_initialization_remain_partial() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../research/phase4-actors/components-native-oracle.json"
    ))
    .unwrap();
    for (row, s) in oracle["component_enum"]
        .as_array()
        .unwrap()
        .iter()
        .zip(ALL_SLOTS)
    {
        assert_eq!(row["index"], s.index());
        let expected = if s == ComponentSlot::MobileDeployment {
            "MOBILE_DEPLOYMENT".to_owned()
        } else {
            format!("{s:?}").to_uppercase()
        };
        assert_eq!(row["name"], expected);
    }
    assert_eq!(ComponentSlot::Body.index(), 0);
    assert_eq!(ComponentSlot::Mover.index(), 3);
    assert_eq!(ComponentSlot::Mind.index(), 4);
    assert!(ComponentSlot::from_index(31).is_none());
    let case = json!({"fail_slot":3,"mutations":[{"kind":"init_create","slot":0,"add":[2]}]});
    let mut h = Hooks {
        case: &case,
        events: vec![],
    };
    let mut c = Components::new(StateBits::default());
    c.insert(ComponentSlot::Body, 0);
    c.insert(ComponentSlot::Mover, 3);
    assert_eq!(c.init_create(&7, &mut h), Err(ComponentSlot::Mover));
    assert!(c.present(ComponentSlot::Character)); // No invented rollback.
}
