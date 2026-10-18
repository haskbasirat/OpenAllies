use aa_actors::{
    components::ComponentSlot,
    leaf_init::{self, BodySupply, CreationRecord},
};
use aa_body::{Body, Environment, Event, F32, Host};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use aa_world::queues::{Clock, Kind, Queues};
use serde_json::{Value, json};
struct Recorder(Vec<Event>);
impl Host for Recorder {
    fn event(&mut self, event: Event, _: &mut Body, _: &mut Environment) -> Result<(), String> {
        self.0.push(event);
        Ok(())
    }
}
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn slot(s: &str) -> ComponentSlot {
    match s {
        "Body" => ComponentSlot::Body,
        "Character" => ComponentSlot::Character,
        "Mover" => ComponentSlot::Mover,
        "Experience" => ComponentSlot::Experience,
        "Properties" => ComponentSlot::Properties,
        "Organization" => ComponentSlot::Organization,
        "Element" => ComponentSlot::Element,
        "Detection" => ComponentSlot::Detection,
        "Economy" => ComponentSlot::Economy,
        _ => panic!("kind"),
    }
}
#[test]
fn native_initialization_queue_rng_and_body_request_gates_match() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-leaf-initializers.json"
    ))
    .unwrap();
    for row in corpus["cases"].as_array().unwrap() {
        let i = &row["input"];
        let out = &row["output"];
        let pc = match n(&i["precision"]) {
            24 => Precision::Single24,
            53 => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut queues = Queues::new(
            Clock {
                begin_bits: 0,
                end_bits: 0x3d800000,
                step_bits: 0x3d800000,
            },
            pc,
        )
        .unwrap();
        queues
            .initialize(1, Kind::Periodic, n(&i["buckets"]) as i32)
            .unwrap();
        let mut rng = NativeRng::new(n(&i["seed"]));
        if b(&i["existing"]) {
            queues.add_periodic(1, 99, 0, &mut rng).unwrap();
        }
        let kind = slot(i["kind"].as_str().unwrap());
        let result = if kind == ComponentSlot::Body {
            let mut body = Body::from_resolved(F32(100f32.to_bits()), F32::ZERO).unwrap();
            let mut env = Environment {
                actor_id: 1,
                has_linked_body: false,
                destructed: false,
                destructible: true,
                restore_disabled_ratio: F32(0.25f32.to_bits()),
                actor_flags: n(&i["state_bits"]),
                derived_normalization_maximum: F32(100f32.to_bits()),
                health_rules: vec![],
                owner_position: [F32::ZERO; 3],
                has_recovery_component: false,
                world_time: F32::ZERO,
            };
            let mut host = Recorder(vec![]);
            let result = leaf_init::body(
                &mut body,
                &mut env,
                &mut host,
                b(&i["record"]).then_some(CreationRecord {
                    word40: n(&i["word40"]),
                    word48: n(&i["word48"]),
                }),
                BodySupply {
                    unit_supply_enable: b(&i["unit_supply"]),
                    has_organization: b(&i["organization"]),
                    unit_shortfall_damage_rate: F32(n(&i["damage_rate"])),
                },
                99,
                &mut queues,
                &mut rng,
                pc,
            )
            .unwrap();
            let changed = !out["health_changes"].as_array().unwrap().is_empty();
            assert_eq!(
                body.health.0,
                if changed {
                    1f32.to_bits()
                } else {
                    100f32.to_bits()
                }
            );
            if changed {
                assert_eq!(out["health_changes"], json!([[1f32.to_bits(), 0, 1, 0]]));
                assert!(!host.0.is_empty());
            } else {
                assert!(host.0.is_empty());
            }
            result
        } else {
            leaf_init::ground_support(kind, b(&i["supply_required"]), 99, &mut queues, &mut rng)
                .unwrap()
        };
        assert_eq!(result, n(&out["result"]) != 0);
        assert_eq!(rng.state, n(&out["rng_state"]), "{i}");
        assert_eq!(
            serde_json::to_value(&queues.snapshot()[0]).unwrap(),
            out["queue"],
            "{i}"
        );
    }
}
#[test]
fn unsupported_leaves_and_unread_nonfinite_supply_are_explicit() {
    let input = BodySupply {
        unit_supply_enable: false,
        has_organization: false,
        unit_shortfall_damage_rate: F32(0x7fc00000),
    };
    assert!(!leaf_init::body_supply_enabled(input).unwrap());
    assert!(
        leaf_init::body_supply_enabled(BodySupply {
            unit_supply_enable: true,
            ..input
        })
        .is_err()
    );
    let mut queues = Queues::new(
        Clock {
            begin_bits: 0,
            end_bits: 0x3d800000,
            step_bits: 0x3d800000,
        },
        Precision::Extended64,
    )
    .unwrap();
    queues.initialize(1, Kind::Periodic, 1).unwrap();
    let mut rng = NativeRng::new(10);
    assert!(
        leaf_init::ground_support(ComponentSlot::Performer, true, 99, &mut queues, &mut rng)
            .is_err()
    );
    assert_eq!(rng.state, 10);
}

#[test]
fn original_ground_requirement_virtuals_match() {
    use aa_actors::init_create::FactoryKind;
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/native-component-requirements.json"
    ))
    .unwrap();
    for row in doc["cases"].as_array().unwrap() {
        let index = n(&row["slot"]) as usize;
        let kind = if index == 4 {
            match row["condition"].as_str().unwrap() {
                "Organization bit7" => FactoryKind::OrganizationMind,
                "Aircraft bit23" => FactoryKind::AircraftMind,
                "Element bit8" => FactoryKind::ElementMind,
                _ => panic!("kind"),
            }
        } else {
            FactoryKind::Standard(aa_actors::components::ALL_SLOTS[index])
        };
        assert_eq!(
            leaf_init::ground_requirement(kind)
                .unwrap()
                .map(|s| s as u32)
                .unwrap_or(u32::MAX),
            n(&row["result"])
        );
    }
}
