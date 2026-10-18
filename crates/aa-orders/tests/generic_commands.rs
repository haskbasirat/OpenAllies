use aa_orders::{commands::*, native, *};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase5-orders/commands-native-oracle.json"
    ))
    .unwrap()
}
fn field<T: serde::de::DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn cursor_end(p: Position) -> usize {
    p.absolute().unwrap()
}
fn sizes() -> RegistrySizes {
    RegistrySizes {
        command: 7,
        object: 5,
        parameter: 3,
        parameter_set: 2,
        extra: 8,
    }
}
fn budget() -> DecodeBudget {
    DecodeBudget {
        max_parameter_entries: 16,
    }
}

#[test]
fn complete_native_writer_and_decoder_cover_each_presence_flag_and_offset() {
    for row in fixture()["payloads"].as_array().unwrap() {
        let raw: Vec<u8> = field(&row["raw"]);
        let start: Position = field(&row["start"]);
        let end: Position = field(&row["end"]);
        let expected: CommandWire = field(&row["command"]);
        let counts: RegistrySizes = field(&row["counts"]);
        let mut reader = Cursor::with_bounds(&raw, start, cursor_end(end)).unwrap();
        assert_eq!(
            CommandWire::decode(&mut reader, counts, budget()).unwrap(),
            expected
        );
        assert_eq!(reader.position(), end);
        let mut writer = WireWriter::new(start).unwrap();
        expected.encode(&mut writer, counts).unwrap();
        assert_eq!(writer.bytes(), raw);
        assert_eq!(writer.position(), end);
    }
}

#[test]
fn actor_suffix_is_native_slot_not_generation_id_and_payload_roles_stay_distinct() {
    for row in fixture()["slots"].as_array().unwrap() {
        let raw: Vec<u8> = field(&row["raw"]);
        let start: Position = field(&row["start"]);
        let end: Position = field(&row["end"]);
        let mut reader = Cursor::with_bounds(&raw, start, cursor_end(end)).unwrap();
        assert_eq!(
            reader.read_uint(16).unwrap(),
            row["slot"].as_u64().unwrap() as u32
        );
        let mut writer = WireWriter::new(start).unwrap();
        writer
            .write_uint(row["slot"].as_u64().unwrap() as u32, 16)
            .unwrap();
        assert_eq!(writer.bytes(), raw);
        assert_eq!(writer.position(), end);
        // Original resolution requires a present slot and virtual+B4, not high16.
        assert_eq!(row["resolved"], row["slot"] != 0 && row["valid"] == true);
    }
    let command = CommandWire {
        flags: 0x5c,
        position: [1, 2],
        trailing: true,
        ..Default::default()
    };
    for id in [
        SELECTED_REQUEST_ID,
        SELECTED_DECREE_ID,
        ACTOR_REQUEST_ID,
        ACTOR_DECREE_ID,
    ] {
        let mut writer = WireWriter::new(Position::PACKET_START).unwrap();
        command.encode(&mut writer, sizes()).unwrap();
        if matches!(id, ACTOR_REQUEST_ID | ACTOR_DECREE_ID) {
            writer.write_uint(65535, 16).unwrap();
        }
        let mut reader = Cursor::with_bounds(
            writer.bytes(),
            Position::PACKET_START,
            cursor_end(writer.position()),
        )
        .unwrap();
        let actual = decode_payload(id, &mut reader, sizes(), budget()).unwrap();
        let actor = ActorCommandWire {
            command: command.clone(),
            actor: ActorSlotReference(65535),
        };
        let expected = match id {
            SELECTED_REQUEST_ID => Payload::SelectedRequest(command.clone()),
            SELECTED_DECREE_ID => Payload::SelectedDecree(command.clone()),
            ACTOR_REQUEST_ID => Payload::ActorRequest(actor),
            _ => Payload::ActorDecree(actor),
        };
        assert_eq!(actual, expected);
        assert_eq!(reader.position(), writer.position());
    }
    assert_eq!(native::DESCRIPTORS[SELECTED_DECREE_ID].handler_va, 0x61f840);
    assert_eq!(native::DESCRIPTORS[ACTOR_DECREE_ID].handler_va, 0x61f970);
    assert_eq!(
        native::DESCRIPTORS[SET_PLAYER_DECREE_ID].handler_va,
        0x750dd0
    );
}

struct ActorHost {
    row: Value,
    events: Vec<Value>,
}
impl ActorCommandContext for ActorHost {
    type Actor = ();
    type Command = CommandWire;
    type Group = ();
    fn resolve_slot(&mut self, _: ActorSlotReference) -> Option<()> {
        (self.row["exists"] == true).then_some(())
    }
    fn permit(&mut self, _: &(), command: &mut CommandWire) -> bool {
        self.events
            .push(json!({"kind":"permit","player":0,"selection":0}));
        if self.row["replace"] == true {
            *command = field(&self.row["replacement"]);
        }
        self.row["allowed"] == true
    }
    fn has_valid_link(&mut self, _: &()) -> bool {
        self.row["linked"] == true
    }
    fn linked_group(&mut self, _: &()) {
        self.events.push(json!({"kind":"linked_group"}));
    }
    fn prepare_group(&mut self, _: ()) {
        self.events.push(json!({"kind":"prepare_group"}));
    }
    fn deliver(&mut self, _: &(), command: &mut CommandWire) {
        self.events
            .push(json!({"kind":"deliver","command":command}));
    }
}
#[test]
fn actual_actor_guard_and_command_replacement_traces_match() {
    for row in fixture()["actor_process"].as_array().unwrap() {
        let mut host = ActorHost {
            row: row.clone(),
            events: Vec::new(),
        };
        let mut command = field(&row["initial"]);
        let result = process_actor(ActorSlotReference(65535), &mut command, &mut host);
        assert_eq!(host.events, row["events"].as_array().unwrap().to_vec());
        let expected = if row["exists"] == false {
            ActorOutcome::ActorNotFound
        } else if row["allowed"] == false {
            ActorOutcome::PermissionDenied
        } else {
            ActorOutcome::Delivered
        };
        assert_eq!(result, expected);
    }
}
struct SelectedHost {
    current: bool,
    empty: bool,
    events: Vec<Value>,
}
impl SelectedCommandContext for SelectedHost {
    type Selection = ();
    type Command = CommandWire;
    fn current_selection(&mut self) -> Option<()> {
        self.current.then_some(())
    }
    fn is_empty(&mut self, _: &()) -> bool {
        self.empty
    }
    fn inspect_selected(&mut self, _: &(), _: &mut CommandWire) {}
    fn deliver_selection(&mut self, _: &(), command: &mut CommandWire) {
        self.events
            .push(json!({"kind":"selection_delivery","command":command}));
    }
}
#[test]
fn selected_gate_matches_actual_outer_handler_without_inventing_actor_loop() {
    for row in fixture()["selected_process"].as_array().unwrap() {
        let mut host = SelectedHost {
            current: true,
            empty: row["name"] == "empty",
            events: Vec::new(),
        };
        let mut command = field(&row["command"]);
        process_selected(&mut command, &mut host).unwrap();
        assert_eq!(host.events, row["events"].as_array().unwrap().to_vec());
    }
    let mut host = SelectedHost {
        current: false,
        empty: false,
        events: Vec::new(),
    };
    assert_eq!(
        process_selected(&mut CommandWire::default(), &mut host),
        Err(Error::MissingCurrentPlayer)
    );
    assert!(host.events.is_empty());
}
struct PlayerHost {
    row: Value,
    current: Option<usize>,
    clients: [Option<usize>; 2],
    abort: bool,
    events: Vec<Value>,
}
impl PlayerContext for PlayerHost {
    type Player = usize;
    type Client = usize;
    fn film_mode(&mut self) -> bool {
        self.row["film"] == true
    }
    fn find_player(&mut self, id: u32, film: bool) -> Option<usize> {
        [(17, false), (17, true), (18, false)]
            .iter()
            .position(|v| *v == (id, film))
    }
    fn player_is_film(&mut self, player: &usize) -> bool {
        *player == 1
    }
    fn is_authorizing_admin(&mut self) -> bool {
        self.row["authority"] == true
    }
    fn player_client(&mut self, player: &usize) -> usize {
        usize::from(*player != 0)
    }
    fn ordering_client(&mut self) -> usize {
        0
    }
    fn bind_client_player(&mut self, client: usize, player: &usize) {
        self.clients[client] = Some(*player);
    }
    fn set_current_player(&mut self, player: Option<usize>) {
        self.current = player;
    }
    fn stop_film(&mut self) {
        self.events.push(json!({"kind":"stop_film"}));
        if self.row["stop_mutates"] == true {
            self.current = Some(0);
            self.abort = false;
        }
    }
    fn set_abort(&mut self) {
        self.abort = true;
    }
}
#[test]
fn actual_player_ownership_film_abort_and_callback_mutation_match() {
    for row in fixture()["player_process"].as_array().unwrap() {
        let mut host = PlayerHost {
            row: row.clone(),
            current: Some(2),
            clients: [None; 2],
            abort: false,
            events: Vec::new(),
        };
        process_set_player(row["id"].as_u64().unwrap() as u32, &mut host);
        assert_eq!(json!(host.current), row["current_player"]);
        assert_eq!(json!(host.clients), row["client_players"]);
        assert_eq!(json!(host.abort), row["abort"]);
        assert_eq!(json!(host.events), row["events"]);
    }
}

#[test]
fn original_mixed_decree_packets_establish_player_before_generic_command() {
    for row in fixture()["packets"].as_array().unwrap() {
        let packet = Packet::new(field(&row["packet"])).unwrap();
        let mut player = PlayerHost {
            row: json!({"film":false,"authority":true,"stop_mutates":false}),
            current: None,
            clients: [None; 2],
            abort: false,
            events: Vec::new(),
        };
        let mut events = Vec::new();
        let mut decoded = Vec::new();
        let result = dispatch(Some(&packet), None, &native::registry().unwrap(), false,
            &mut Gate::default(), |id, cursor, _| {
                decoded.push(id);
                match decode_payload(id, cursor, sizes(), budget()).map_err(|e|e.to_string())? {
                    Payload::SetPlayerDecree(id) => { process_set_player(id, &mut player); }
                    Payload::ActorDecree(mut wire) => {
                        assert_eq!(player.current, Some(0));
                        assert_eq!(wire.actor, ActorSlotReference(65535));
                        let mut host = ActorHost { row: json!({"exists":true,"allowed":true,"linked":false,"replace":false}), events: Vec::new() };
                        process_actor(wire.actor, &mut wire.command, &mut host);
                        events.extend(host.events);
                    }
                    Payload::SelectedDecree(mut command) => {
                        assert_eq!(player.current, Some(0));
                        let mut host = SelectedHost { current: player.current.is_some(), empty: row["name"]=="player_empty_selected", events: Vec::new() };
                        process_selected(&mut command, &mut host).map_err(|e|e.to_string())?;
                        events.extend(host.events);
                    }
                    other => panic!("unexpected request or payload in native decree packet: {other:?}"),
                }
                Ok(())
            }).unwrap();
        assert_eq!(json!(decoded), row["decoded"]);
        assert_eq!(json!(events), row["events"]);
        assert_eq!(json!(player.current), row["current_player"]);
        assert_eq!(result.executed, 2);
        assert_eq!(row["packet_refs"], 1);
    }
}

#[test]
fn malformed_lengths_indices_and_unbounded_zero_width_lists_fail_safely() {
    let command = CommandWire {
        flags: 0x20,
        parameters: Some(ParameterSet {
            definition: Some(0),
            objects: vec![None, None],
            negative_count: None,
        }),
        ..Default::default()
    };
    let mut writer = WireWriter::new(Position::PACKET_START).unwrap();
    let mut counts = sizes();
    counts.object = 0;
    command.encode(&mut writer, counts).unwrap();
    let mut reader = Cursor::with_bounds(
        writer.bytes(),
        Position::PACKET_START,
        cursor_end(writer.position()),
    )
    .unwrap();
    assert!(matches!(
        CommandWire::decode(
            &mut reader,
            counts,
            DecodeBudget {
                max_parameter_entries: 1
            }
        ),
        Err(Error::ParameterBudget {
            requested: 2,
            budget: 1
        })
    ));
    let mut writer = WireWriter::new(Position::PACKET_START).unwrap();
    writer.write_uint(0x20, 11).unwrap();
    writer.write_uint(0, 1).unwrap();
    writer.write_uint(0, 2).unwrap();
    writer.write_uint(u32::MAX, 32).unwrap();
    writer.write_uint(1, 1).unwrap();
    let mut reader = Cursor::with_bounds(
        writer.bytes(),
        Position::PACKET_START,
        cursor_end(writer.position()),
    )
    .unwrap();
    // Phase8 native evidence (order-traces.json, 0x4C2C60/0x50B530/0x4C2050):
    // a negative count reads no entries and is re-serialized unchanged.
    let decoded = CommandWire::decode(&mut reader, sizes(), budget()).unwrap();
    let set = decoded.parameters.clone().unwrap();
    assert_eq!((set.objects.len(), set.negative_count), (0, Some(-1)));
    let mut again = WireWriter::new(Position::PACKET_START).unwrap();
    decoded.encode(&mut again, sizes()).unwrap();
    assert_eq!(again.bytes(), writer.bytes());
    let mut invalid = CommandWire {
        flags: 0x40,
        command: Some(99),
        ..Default::default()
    };
    assert!(matches!(
        invalid.encode(
            &mut WireWriter::new(Position::PACKET_START).unwrap(),
            sizes()
        ),
        Err(Error::RegistryIndex { .. })
    ));
    invalid.flags = 0x800;
    assert_eq!(
        invalid.encode(
            &mut WireWriter::new(Position::PACKET_START).unwrap(),
            sizes()
        ),
        Err(Error::CommandFlags)
    );
    for row in fixture()["payloads"].as_array().unwrap() {
        let raw: Vec<u8> = field(&row["raw"]);
        let end: Position = field(&row["end"]);
        let start: Position = field(&row["start"]);
        let mut reader = Cursor::with_bounds(&raw, start, cursor_end(end) - 1).unwrap();
        assert!(matches!(
            CommandWire::decode(&mut reader, field(&row["counts"]), budget()),
            Err(Error::Truncated { .. })
        ));
    }
    let mut reader = Cursor::with_bounds(&[0xff], Position { byte: 0, bit: 0 }, 8).unwrap();
    assert!(matches!(
        decode_payload(9, &mut reader, sizes(), budget()),
        Err(Error::UnsupportedNativeDescriptor(9))
    ));
    assert_eq!(reader.position(), Position { byte: 0, bit: 0 });
}
