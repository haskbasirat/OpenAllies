use aa_actors::{ActorId, components::ComponentSlot, owned};
use aa_body::F32;
use aa_mapworld::saved_components::SavedComponent;
use aa_numeric::Precision;
use serde_json::{Value, json};
#[test]
fn native_hq_factory_and_wire_framing_match_all_images_and_precisions() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/saved-hq/native-saved-hq.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 162);
    for row in cases {
        let input = &row["input"];
        let native = &row["native"];
        let name = format!("{} {} {}", row["image"], row["precision"], input["name"]);
        let definition = owned::Definitions::<String> {
            hq_slots_530: Some(input["slots_530"].as_i64().unwrap() as i32),
            ..Default::default()
        };
        let made: owned::Component<String> = owned::construct(
            ComponentSlot::Hq,
            &definition,
            owned::Context {
                owner: ActorId::from_raw(0x10001),
                world_time: F32(0),
                position: [F32(0); 3],
                precision: Precision::Double53,
            },
        )
        .unwrap();
        assert_eq!(made.slot(), ComponentSlot::Hq);
        let owned::Component::Hq(hq) = made else {
            panic!("actual owned HQ constructor")
        };
        assert_eq!(
            hq.badge_index as i64,
            native["after_ctor"]["index"].as_i64().unwrap(),
            "{name}"
        );
        assert_eq!(
            json!(
                hq.slots
                    .iter()
                    .map(|v| v.map(|i| [i.kind, i.word_04, i.word_08]))
                    .collect::<Vec<_>>()
            ),
            native["after_ctor"]["slots"]["items"],
            "{name}"
        );
        assert!(hq.pending.is_empty());
        let text = input["payload"].as_str().unwrap();
        let bytes = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        if native["outcome"].is_null() {
            let (payload, used) = SavedComponent::decode_prefix(30, &bytes).unwrap();
            assert_eq!(used as u64, native["consumed"].as_u64().unwrap(), "{name}");
            let SavedComponent::Hq(payload) = payload else {
                panic!("HQ codec variant")
            };
            assert_eq!(payload.bytes, bytes[..used], "{name}");
        }
    }
    let missing = owned::construct::<String, std::convert::Infallible, std::convert::Infallible>(
        ComponentSlot::Hq,
        &owned::Definitions::default(),
        owned::Context {
            owner: ActorId::from_raw(1),
            world_time: F32(0),
            position: [F32(0); 3],
            precision: Precision::Double53,
        },
    );
    assert!(matches!(missing, Err(owned::Error::Missing { .. }))); // no invented +530 default
}

struct Definitions<'a> {
    input: &'a Value,
    registered: std::cell::RefCell<Vec<i32>>,
}
impl aa_mapworld::restore::RestoreDefinitions for Definitions<'_> {
    fn resolve(
        &self,
        _: &[u16],
        _: Precision,
    ) -> Result<aa_mapworld::restore::RuntimeDefinition, String> {
        Err("unreached definition lookup".into())
    }
    fn updatable_table_bound(&self) -> Result<i32, String> {
        Ok(self.input["table_max"].as_i64().unwrap() as i32)
    }
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool, String> {
        Ok(self.input["occupied"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_i64() == Some(id as i64)))
    }
    fn register_updatable(
        &self,
        id: i32,
        target: &aa_mapworld::restore_components::UpdatableTarget,
    ) -> Result<(), String> {
        assert_eq!(
            target,
            &aa_mapworld::restore_components::UpdatableTarget::Component {
                owner: 0x10001,
                slot: 30
            }
        );
        self.registered.borrow_mut().push(id);
        Ok(())
    }
}
#[test]
fn live_hq_load_retains_native_error_prefix_and_real_registration_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/saved-hq/native-saved-hq.json"
    ))
    .unwrap();
    let mut tested = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let native = &row["native"];
        let text = input["payload"].as_str().unwrap();
        let bytes = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        // The historical harness deliberately maps an extra table cell for
        // id==count. Production correctly refuses that out-of-allocation read.
        if input["updatable_id"] == input["table_max"] {
            continue;
        }
        // These cases never query a Nation/name; actual owner branches are tested
        // by the original tutorial02 record. Other missing services stay explicit.
        if bytes.get(12..16) != Some(&u32::MAX.to_le_bytes()[..]) {
            continue;
        }
        let definitions = Definitions {
            input,
            registered: Default::default(),
        };
        let mut hq = aa_actors::hq_state::HqState::new(
            ActorId::from_raw(0x10001),
            input["slots_530"].as_i64().unwrap() as i32,
        );
        let registry = aa_actors::IdentityRegistry::new();
        let mut registrations = Vec::new();
        let result = aa_mapworld::saved_hq::load(
            &mut hq,
            &bytes,
            aa_mapworld::saved_hq::Owner {
                actor: ActorId::from_raw(0x10001),
                kingdom: None,
                registry: &registry,
                registrations: &mut registrations,
                precision: Precision::Double53,
            },
            &definitions,
        );
        if native["outcome"].is_null() {
            assert_eq!(result.unwrap() as u64, native["consumed"].as_u64().unwrap());
        } else {
            let error = result.unwrap_err().to_string();
            if ["ThrowException", "Fatal"].contains(&native["outcome"]["kind"].as_str().unwrap()) {
                assert!(
                    error.contains(aa_mapworld::restore::LOAD_WARNING_EXCEPTION),
                    "{error}"
                );
            }
        }
        assert_eq!(
            hq.badge_index as i64,
            native["state"]["index"].as_i64().unwrap()
        );
        assert_eq!(
            json!(
                hq.slots
                    .iter()
                    .map(|v| v.map(|i| [i.kind, i.word_04, i.word_08]))
                    .collect::<Vec<_>>()
            ),
            native["state"]["slots"]["items"]
        );
        assert_eq!(
            json!(
                hq.pending
                    .iter()
                    .map(|i| [i.kind, i.word_04, i.word_08])
                    .collect::<Vec<_>>()
            ),
            native["state"]["pending"]["items"]
        );
        assert_eq!(
            registrations.len(),
            usize::from(native["table_slot"] == "hq+8")
        );
        assert_eq!(registrations.len(), definitions.registered.borrow().len());
        tested += 1;
    }
    assert!(
        tested > 70,
        "all relevant original/v14/PC prefix cases visited"
    );
}
