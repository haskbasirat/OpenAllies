use aa_orders::{native::*, *};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase4-orders/orders-native-oracle.json"
    ))
    .unwrap()
}
fn position(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::from_value(v.clone()).unwrap()
}
fn words(v: &Value) -> [u32; 3] {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn complete_native_crt_registry_keeps_duplicates_and_original_ordinals() {
    let f = fixture();
    let rows = f["registry"].as_array().unwrap();
    assert_eq!(rows.len(), DESCRIPTORS.len());
    for (native, rust) in rows.iter().zip(DESCRIPTORS) {
        assert_eq!(native["id"], rust.id);
        assert_eq!(native["name"], rust.name);
        assert_eq!(
            u32::from_str_radix(
                native["handler"].as_str().unwrap().trim_start_matches("0x"),
                16
            )
            .unwrap(),
            rust.handler_va
        );
    }
    let r = registry().unwrap();
    assert_eq!(registry_bits(r.names().len()).unwrap(), 9);
    assert_eq!(DESCRIPTORS[MOVE_DECREE_ID].handler_va, 0x60f830);
    assert_eq!(DESCRIPTORS[MOVE_REQUEST_ID].handler_va, 0x616d80);
    assert!(
        DESCRIPTORS
            .iter()
            .filter(|d| d.name == "SelectionClearOrderDecree")
            .count()
            > 1
    );
}

#[test]
fn complete_movement_payload_matches_native_writer_and_decoder_at_every_offset() {
    for row in fixture()["payloads"].as_array().unwrap() {
        let raw = bytes(&row["raw"]);
        let end = position(&row["end"]);
        let mut cursor =
            Cursor::with_bounds(&raw, position(&row["start"]), end.absolute().unwrap()).unwrap();
        let movement = MoveOrder::decode(&mut cursor).unwrap();
        assert_eq!(
            movement.selector_utf16,
            serde_json::from_value::<Vec<u16>>(row["selector"].clone()).unwrap()
        );
        assert_eq!(movement.words, words(&row["words"]));
        assert_eq!(cursor.position(), end);
    }
}

#[test]
fn variable_buffer_primitive_matches_all_original_scalar_cases() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase3-orders/orders-native-oracle.json"
    ))
    .unwrap();
    let f = &f["cursor"];
    let raw = bytes(&f["data"]);
    for r in f["cases"].as_array().unwrap() {
        let width = r["width"].as_u64().unwrap() as usize;
        let mut c = Cursor::with_bounds(&raw, position(&r["start"]), raw.len() * 8).unwrap();
        let b = c.read_raw_bits(width).unwrap();
        let mut scalar = [0; 4];
        scalar[..b.len()].copy_from_slice(&b);
        assert_eq!(
            u32::from_le_bytes(scalar),
            r["value"].as_u64().unwrap() as u32
        );
        assert_eq!(c.position(), position(&r["end"]));
    }
}

struct Context {
    can: bool,
    exists: bool,
    allows: bool,
    events: Vec<Value>,
}
impl Context {
    fn from(row: &Value) -> Self {
        Self {
            can: row["can_process"].as_bool().unwrap(),
            exists: row["actor_exists"].as_bool().unwrap(),
            allows: row["actor_allows"].as_bool().unwrap(),
            events: vec![],
        }
    }
}
impl MoveContext for Context {
    type Actor = ();
    fn can_process(&mut self) -> bool {
        self.events.push(json!({"kind":"can_process"}));
        self.can
    }
    fn resolve_actor(&mut self, s: &[u16]) -> Option<()> {
        self.events
            .push(json!({"kind":"resolve_actor","selector":s}));
        self.exists.then_some(())
    }
    fn allows_command(&mut self, _: &(), t: u32) -> bool {
        self.events.push(json!({"kind":"allows_command","type":t}));
        self.allows
    }
    fn deliver(&mut self, _: &(), t: u32, c: MoveCommand) {
        assert_eq!(t, 5);
        self.events.push(json!({"kind":"deliver","destination_bits":c.destination_bits,"auxiliary_bits":c.auxiliary_bits,"word_28":c.word_28}));
    }
}
fn normalize(events: &Value) -> Vec<Value> {
    events.as_array().unwrap().iter().map(|e| {
        if e["kind"]!="deliver" {return e.clone()}
        assert_eq!(e["fields"]["0xc"],0x5c);assert_eq!(e["byte_fields"]["0x30"],0);assert_eq!(e["byte_fields"]["0x3c"],0);
        json!({"kind":"deliver","destination_bits":[e["fields"]["0x18"],e["fields"]["0x1c"]],"auxiliary_bits":[e["fields"]["0x20"],e["fields"]["0x24"]],"word_28":e["fields"]["0x28"]})
    }).collect()
}

#[test]
fn native_movement_guard_order_and_constructor_fields_match() {
    for row in fixture()["process"].as_array().unwrap() {
        let order = MoveOrder {
            selector_utf16: "movement_fixture".encode_utf16().collect(),
            words: words(&row["words"]),
        };
        let mut context = Context::from(row);
        let outcome = process_move(&order, &mut context);
        assert_eq!(context.events, normalize(&row["events"]));
        assert_eq!(
            outcome,
            if !context.can {
                MoveOutcome::ProcessingDisabled
            } else if !context.exists {
                MoveOutcome::ActorNotFound
            } else if !context.allows {
                MoveOutcome::CommandUnavailable
            } else {
                MoveOutcome::Delivered
            }
        );
    }
}

#[test]
fn actual_decree_dispatch_runs_real_payload_in_native_order() {
    for row in fixture()["packets"].as_array().unwrap() {
        let packet = Packet::new(bytes(&row["packet"])).unwrap();
        let mut ctx = Context::from(row);
        let mut decoded = vec![];
        let result = dispatch(
            Some(&packet),
            None,
            &registry().unwrap(),
            false,
            &mut Gate {
                session: false,
                admin: false,
            },
            |id, c, _| {
                match decode_payload(id, c).map_err(|e| e.to_string())? {
                    KnownPayload::MoveDecree(order) => {
                        decoded.push(json!({"selector":order.selector_utf16,"words":order.words}));
                        process_move(&order, &mut ctx);
                    }
                    KnownPayload::MoveRequest(_) => panic!("request must not be applied as decree"),
                }
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(ctx.events, normalize(&row["events"]));
        assert_eq!(decoded, row["orders"].as_array().unwrap().clone());
        assert_eq!(result.executed, decoded.len());
        assert_eq!(row["packet_references"], 1); // Original by-value parameter consumes one of two references.
    }
}

#[test]
fn unsupported_and_truncated_payloads_never_invent_or_deliver_commands() {
    let raw = [0u8; 4];
    let mut c = Cursor::with_bounds(&raw, Position { byte: 0, bit: 0 }, 32).unwrap();
    let before = c.position();
    assert!(matches!(
        decode_payload(0, &mut c),
        Err(Error::UnsupportedNativeDescriptor(0))
    ));
    assert_eq!(c.position(), before);
    assert!(matches!(
        MoveOrder::decode(&mut c),
        Err(Error::Truncated { .. })
    ));
    let f = fixture();
    let row = &f["payloads"][12];
    let raw = bytes(&row["raw"]);
    let end = position(&row["end"]).absolute().unwrap();
    for cut in [1, 8, 31, 32, 64, 95] {
        let mut c = Cursor::with_bounds(&raw, position(&row["start"]), end - cut).unwrap();
        assert!(matches!(
            MoveOrder::decode(&mut c),
            Err(Error::Truncated { .. })
        ));
    }
}
