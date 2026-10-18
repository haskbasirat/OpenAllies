use aa_orders::*;
use serde_json::{Value, json};
const ORIGINAL: &str = include_str!("../../../research/phase3-orders/orders-native-oracle.json");
fn oracle() -> Value {
    serde_json::from_str(ORIGINAL).unwrap()
}
fn position(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}
fn data(v: &Value) -> Option<Packet> {
    (!v.is_null()).then(|| Packet::new(serde_json::from_value(v.clone()).unwrap()).unwrap())
}
fn registry(count: usize, join: Option<usize>) -> Registry {
    Registry::new(
        (0..count)
            .map(|i| {
                if Some(i) == join {
                    "TGC_PlayerJoinOrderDecree".into()
                } else {
                    format!("FixtureOpaque{i}")
                }
            })
            .collect(),
    )
    .unwrap()
}
fn expected_events(case: &Value) -> Vec<Value> {
    case["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| match e["kind"].as_str().unwrap() {
            "decoded" => {
                Some(json!({"kind":"decoded","index":e["index"],"position":e["position"]}))
            }
            "execute" => Some(json!({"kind":"execute","index":e["index"],"start":e["start"]})),
            "payload_read" => Some(e.clone()),
            "diagnostic" if e["name"] == "i.GetBitSize()" => {
                Some(json!({"kind":"diagnostic","value":e["value"]}))
            }
            _ => None,
        })
        .collect()
}
fn observed_events(result: &Dispatch, payloads: &[u32]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut payloads = payloads.iter();
    for event in &result.events {
        match event {
            Event::DiagnosticBits { bits } => out.push(json!({"kind":"diagnostic","value":bits})),
            Event::Decoded { index, position } => {
                out.push(json!({"kind":"decoded","index":index,"position":position}))
            }
            Event::Executed { index, start, end } => {
                out.push(json!({"kind":"execute","index":index,"start":start}));
                out.push(json!({"kind":"payload_read","index":index,"value":payloads.next().unwrap(),"end":end}));
            }
        }
    }
    assert!(payloads.next().is_none());
    out
}
fn fields(state: &ClientState) -> Value {
    json!({"0x20c":state.next_index as u32,"0x248":state.resume_byte as u32,"0x24c":state.resume_bit as u32,"0x250":state.field_250,"0x284":state.field_284})
}
fn history(case: &Value) -> History {
    let begin = case["begin"].as_i64().unwrap() as i32;
    let mut history = History::new(begin, case["capacity"].as_u64().unwrap() as usize).unwrap();
    history
        .set_end(case["end"].as_i64().unwrap() as i32)
        .unwrap();
    for (i, p) in case["packets"].as_array().unwrap().iter().enumerate() {
        history.put(begin + i as i32, data(p)).unwrap();
    }
    history
}

#[test]
fn original_bit_copy_all_offsets_widths_and_registry_domains() {
    let o = oracle();
    let source: Vec<u8> = serde_json::from_value(o["cursor"]["data"].clone()).unwrap();
    let rows = o["cursor"]["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 891);
    for row in rows {
        let mut reader =
            Cursor::with_bounds(&source, position(&row["start"]), source.len() * 8).unwrap();
        assert_eq!(
            reader
                .read_uint(row["width"].as_u64().unwrap() as u8)
                .unwrap(),
            row["value"].as_u64().unwrap() as u32,
            "{row}"
        );
        assert_eq!(reader.position(), position(&row["end"]), "{row}");
    }
    for row in o["cursor"]["registry_widths"].as_array().unwrap() {
        assert_eq!(
            registry_bits(row["count"].as_u64().unwrap() as usize).unwrap(),
            row["bits"].as_u64().unwrap() as u8
        );
    }
}

#[test]
fn original_decree_dispatch_order_resume_gate_and_join_diagnostics() {
    let o = oracle();
    let rows = o["dispatch"].as_array().unwrap();
    assert_eq!(rows.len(), 14);
    for case in rows {
        let packet = data(&case["packet"]);
        let start = (!case["start"].is_null()).then(|| position(&case["start"]));
        let registry = registry(
            case["registry_count"].as_u64().unwrap() as usize,
            case["join_index"].as_u64().map(|v| v as usize),
        );
        let mut gate: Gate = serde_json::from_value(case["initial_gate"].clone()).unwrap();
        let mut payloads = Vec::new();
        let result = dispatch(
            packet.as_ref(),
            start,
            &registry,
            case["diagnostics"].as_bool().unwrap(),
            &mut gate,
            |_, reader, gate| {
                payloads.push(
                    reader
                        .read_uint(case["payload_width"].as_u64().unwrap() as u8)
                        .map_err(|e| e.to_string())?,
                );
                if case["gate_after"]
                    .as_u64()
                    .is_some_and(|n| payloads.len() >= n as usize)
                {
                    *gate = Gate {
                        session: true,
                        admin: true,
                    };
                }
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            observed_events(&result, &payloads),
            expected_events(case),
            "{}",
            case["name"]
        );
        assert_eq!(result.executed, payloads.len());
        let decoded = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "decoded")
            .count();
        assert_eq!(
            result.stopped_before_execute.is_some(),
            decoded > payloads.len()
        );
    }
}

#[test]
fn original_history_growth_wrap_retirement_and_stale_slots() {
    let o = oracle();
    let rows = o["history"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    for case in rows {
        let mut history = history(case);
        for step in case["steps"].as_array().unwrap() {
            let index = step["index"].as_i64().unwrap() as i32;
            if step["action"] == "end" {
                history.set_end(index).unwrap();
            } else {
                history.retire_before(index).unwrap();
            }
            assert_eq!(
                serde_json::to_value(history.snapshot()).unwrap(),
                step["history"],
                "{} {step}",
                case["name"]
            );
        }
    }
}

#[test]
fn original_client_boundary_retains_packets_resets_fields_and_rereads_index() {
    let o = oracle();
    let rows = o["client"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for case in rows {
        let mut history = history(case);
        let mut state = ClientState::new(case["index"].as_i64().unwrap() as i32);
        if !case["resume"].is_null() {
            let p = position(&case["resume"]);
            state.resume_byte = p.byte as i32;
            state.resume_bit = i32::from(p.bit);
        }
        state.field_250 = 77;
        state.field_284 = 99;
        let mut gate = Gate {
            session: case["abort_gate"].as_bool().unwrap(),
            admin: case["abort_gate"].as_bool().unwrap(),
        };
        let registry = registry(4, Some(1));
        let mut payloads = Vec::new();
        let mut handler_fields = Vec::new();
        let mut decoder_fields = None;
        let mut delta = case["handler_index_delta"].as_i64().unwrap() as i32;
        let result = consume_next(&mut history, &mut state, |packet, start, state, _| {
            decoder_fields = Some(fields(state));
            dispatch(packet, start, &registry, true, &mut gate, |_, reader, _| {
                handler_fields.push(fields(state));
                payloads.push(reader.read_uint(3).map_err(|e| e.to_string())?);
                state.next_index += delta;
                delta = 0;
                Ok(())
            })
        })
        .unwrap();
        assert_eq!(fields(&state), case["fields"], "{}", case["name"]);
        assert_eq!(
            serde_json::to_value(history.snapshot()).unwrap(),
            case["history"],
            "{}",
            case["name"]
        );
        assert_eq!(
            observed_events(&result.dispatch, &payloads),
            expected_events(case),
            "{}",
            case["name"]
        );
        let events = case["events"].as_array().unwrap();
        assert_eq!(
            decoder_fields.unwrap(),
            events
                .iter()
                .find(|e| e["kind"] == "decoder_entry")
                .unwrap()["fields"]
        );
        assert_eq!(
            handler_fields,
            events
                .iter()
                .filter(|e| e["kind"] == "execute")
                .map(|e| e["fields"].clone())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn malformed_input_fails_before_opaque_execution_and_never_guesses_order() {
    assert_eq!(Packet::new(vec![]), Err(Error::InvalidPacket));
    assert_eq!(Packet::new(vec![7]), Err(Error::InvalidPacket));
    assert!(Registry::new(vec![]).is_err());
    assert_eq!(registry_bits(65537), Err(Error::RegistrySize));
    let packet = Packet::new(vec![0x18]).unwrap();
    let mut count = 0;
    let bad = dispatch(
        Some(&packet),
        None,
        &registry(3, None),
        false,
        &mut Gate::default(),
        |_, _, _| {
            count += 1;
            Ok(())
        },
    );
    assert_eq!(bad, Err(Error::RegistryIndex { index: 3, count: 3 }));
    assert_eq!(count, 0);
    let short = dispatch(
        Some(&packet),
        None,
        &registry(256, None),
        false,
        &mut Gate::default(),
        |_, _, _| {
            count += 1;
            Ok(())
        },
    );
    assert!(matches!(short, Err(Error::Truncated { .. })));
    assert_eq!(count, 0);
    let mut cursor = Cursor::for_packet(packet.bytes(), None).unwrap();
    let before = cursor.position();
    assert!(cursor.read_uint(6).is_err());
    assert_eq!(cursor.position(), before);
    assert!(cursor.read_uint(33).is_err());
    assert_eq!(cursor.position(), before);
    assert!(Cursor::for_packet(packet.bytes(), Some(Position { byte: 0, bit: 0 })).is_err());
    assert!(
        Cursor::with_bounds(
            &[],
            Position {
                byte: usize::MAX,
                bit: 8
            },
            0
        )
        .is_err()
    );
    assert!(Cursor::with_bounds(&[0], Position { byte: 0, bit: 9 }, 8).is_err());
    assert!(
        Cursor::with_bounds(&[0], Position { byte: 0, bit: 0 }, 8)
            .unwrap()
            .relative_bits()
            .is_err()
    );
}

#[test]
fn safe_failure_contract_does_not_replay_partial_handlers_or_wrap_indices() {
    let packet = Packet::new(vec![0x18]).unwrap();
    let mut calls = 0;
    let result = dispatch(
        Some(&packet),
        None,
        &registry(1, None),
        false,
        &mut Gate::default(),
        |_, _, _| {
            calls += 1;
            Ok(())
        },
    );
    assert_eq!(result, Err(Error::NoProgress { index: 0 }));
    assert_eq!(calls, 1);
    let mut history = History::new(0, 1).unwrap();
    history.put(0, Some(packet)).unwrap();
    let mut state = ClientState::new(0);
    state.field_250 = 77;
    let result = consume_next(&mut history, &mut state, |packet, start, _, _| {
        dispatch(
            packet,
            start,
            &registry(1, None),
            false,
            &mut Gate::default(),
            |_, reader, _| {
                reader.read_uint(2).unwrap();
                Err("explicit leaf failure".into())
            },
        )
    });
    assert!(matches!(result, Err(Error::Handler { .. })));
    assert_eq!(state.next_index, 0);
    assert_eq!(state.field_284, 77);
    // Callers must handle partial leaf failure; the library never auto-retries.
    assert!(history.get(0).unwrap().is_some());
    assert!(History::new(0, 0).is_err());
    assert!(history.set_end(-1).is_err());
    history.retire_before(1).unwrap();
    assert_eq!(history.retire_before(0), Err(Error::BackwardRetirement));
    assert!(matches!(
        history.ensure_index(0),
        Err(Error::BeforeWindow { .. })
    ));
    let mut exhausted = History::new(i32::MAX, 1).unwrap();
    assert_eq!(exhausted.ensure_index(i32::MAX), Err(Error::IndexExhausted));
}
