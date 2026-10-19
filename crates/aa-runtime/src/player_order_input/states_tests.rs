use super::*;
use crate::player_order_input::pick::Model;
use crate::player_order_input::requests::Requests;
use crate::player_order_input::ui_world::StaticRef;
use aa_numeric::Precision;
use serde_json::{json, Value};

struct ActorSpec {
    b4: bool,
    can: Vec<u32>,
    handle: u32,
    /// vt+0x150's scripted answer (0x436950's loop).
    can150: bool,
}
/// The oracle's views, actors, definitions and scripted answers; records
/// every observable call the machine makes.
struct Mock {
    views: Vec<Option<ActorSpec>>,
    definitions: Vec<Option<CommandDefinition>>,
    pointer: [u32; 3],
    map_xy: [u32; 2],
    aux: [u32; 2],
    trace: Vec<Value>,
}
struct NoCamera;
impl crate::view_pointer::CameraWords for NoCamera {
    fn word(&self, _: u32) -> Result<u32> {
        unreachable!("no camera in this oracle")
    }
}
fn wire_json(w: &CommandWire) -> Value {
    json!({"flags": w.flags, "word": w.word, "position": w.position, "auxiliary": w.auxiliary, "scalar": w.scalar,
           "trailing": w.trailing, "command": w.command, "object": w.object})
}
/// The native command words in the same shape (+0x08 / +0x14 NULL = None).
fn native_command(c: &Value) -> Value {
    let flags = c["flags"].as_u64().unwrap();
    assert_eq!(flags >> 16, 0, "{c}");
    let none_08_14 = |v: &Value| -> Value {
        assert_eq!(v.as_u64().unwrap(), 0, "only NULL +0x08/+0x14 are expected here: {c}");
        Value::Null
    };
    json!({"flags": flags, "word": c["word"], "position": c["position"], "auxiliary": c["auxiliary"], "scalar": c["scalar"],
           "trailing": c["trailing"].as_u64().unwrap() != 0, "command": none_08_14(&c["command_08"]), "object": none_08_14(&c["object_14"])})
}

macro_rules! unreachable_leaves {
    ($($name:ident($($t:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $(_: $t),*) -> $ret { unreachable!(stringify!($name)) })*
    };
}
impl Leaves for Mock {
    type View = usize;
    fn precision(&self) -> Precision {
        Precision::Double53
    }
    fn actor_present_14(&mut self, v: usize) -> Result<bool> {
        Ok(self.views[v].is_some())
    }
    fn actor_45b7a0(&mut self, v: usize) -> Result<bool> {
        Ok(self.views[v].as_ref().is_some_and(|a| a.b4))
    }
    // 0x4363B0 is the oracle's seam: the hovered view maps to itself.
    fn link_16c_view(&mut self, _: usize) -> Result<Option<usize>> {
        Ok(None)
    }
    fn root_view_4ce6c0(&mut self, v: usize) -> Result<Option<usize>> {
        Ok(Some(v))
    }
    fn flags_54(&mut self, _: usize) -> Result<u32> {
        Ok(0)
    }
    fn actor_virtual_150(&mut self, v: usize, wire: &mut CommandWire) -> Result<bool> {
        self.trace.push(json!(["actor_150", v, wire_json(wire)]));
        Ok(self.views[v].as_ref().expect("vt+0x150 of a 0x45B7A0 actor").can150)
    }
    unreachable_leaves! {
        set_flags_54(usize, u32) -> Result<()>;
        position_38(usize) -> Result<[u32; 3]>;
        field_10_58cb30(usize) -> Result<u64>;
        pickable_9c(usize) -> Result<bool>;
        class_78(usize) -> Result<bool>;
        class_84(usize) -> Result<bool>;
        class_7c(usize) -> Result<bool>;
        class_80(usize) -> Result<bool>;
        virtual_50(usize) -> Result<bool>;
        virtual_54(usize) -> Result<bool>;
        actor_flag8_bit0(usize) -> Result<bool>;
        actor_virtual_58(usize) -> Result<bool>;
        actor_virtual_84(usize) -> Result<bool>;
        actor_4d12c0(usize) -> Result<bool>;
        actor_virtual_20(usize) -> Result<bool>;
        actor_70(usize) -> Result<bool>;
        actor_90(usize) -> Result<bool>;
        link_94_view(usize) -> Result<Option<usize>>;
        link_94_live(usize) -> Result<bool>;
        hidden_4549c0(usize) -> Result<bool>;
        actor_view_0c(usize) -> Result<Option<usize>>;
        voiced_436600(usize) -> Result<Option<Option<usize>>>;
        selection_sound_436850(usize) -> Result<()>;
        command_masks_437eb0(&[usize], Option<u64>, Option<bool>, &mut [u32; 4]) -> Result<()>;
        local_kingdom_765e00() -> Result<Option<u64>>;
        kingdom_groups_34c(u64) -> Result<Vec<crate::player_order_input::ui_world::KingdomGroup>>;
        view_of_handle_b364c0(u32) -> Result<Option<Option<usize>>>;
        player_wildcard_byte_50() -> Result<bool>;
        brush_changed_42d4d0() -> Result<()>;
        vworld_list_changed_4325c0() -> Result<()>;
        vworld_ad() -> Result<bool>;
        floater_remove_42eb10(usize) -> Result<()>;
        floater_update_453470(usize) -> Result<()>;
    }
    fn requests(&mut self) -> &mut dyn Requests<usize> {
        unreachable!("no selection requests on the right-click path")
    }
}
impl Scene for Mock {
    type Camera = NoCamera;
    unreachable_leaves! {
        on_list_100() -> Result<Vec<usize>>;
        proximity_18(usize) -> Result<i32>;
        model_78(usize) -> Result<Option<Model>>;
        selection_radius_469ac0(usize) -> Result<u32>;
        update_model_86e8d0(usize) -> Result<Model>;
        viewport_b396d0() -> Result<(i32, i32)>;
    }
    fn camera(&mut self) -> Result<&NoCamera> {
        unreachable!("camera")
    }
}
impl Orders for Mock {
    fn acknowledge_61eef0(&mut self, wire: &CommandWire) -> Result<()> {
        self.trace.push(json!(["acknowledge", wire_json(wire)]));
        Ok(())
    }
    fn emit_61e960(&mut self, wire: &CommandWire) -> Result<()> {
        self.trace.push(json!(["emit", wire_json(wire)]));
        Ok(())
    }
    fn diagnostic_61eef0(&mut self, _: &CommandWire, sent: bool) -> Result<()> {
        self.trace.push(json!(["diagnostic", sent]));
        Ok(())
    }
    fn marker_43db10(&mut self, xy: [u32; 2]) -> Result<()> {
        self.trace.push(json!(["marker", xy]));
        Ok(())
    }
}
impl Host for Mock {
    fn map_present_b36518(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn project_43c4b0(&mut self, _: [i32; 2], _: &mut [u32; 3]) -> Result<[u32; 3]> {
        Ok(self.pointer)
    }
    fn widget_route_5e0690(&mut self, _: &Event) -> Result<bool> {
        self.trace.push(json!(["base"]));
        Ok(true)
    }
    fn brush_installed_bc(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn camera_slot_c0(&mut self) -> Result<CameraSlot> {
        Ok(CameraSlot::Empty)
    }
    fn map_world_5d74b0(&mut self, cursor: [i32; 2]) -> Result<[u32; 2]> {
        self.trace.push(json!(["map_world", cursor]));
        Ok(self.map_xy)
    }
    fn map_auxiliary_b279c0(&mut self) -> Result<[u32; 2]> {
        Ok(self.aux)
    }
    fn actor_can_34(&mut self, v: usize, index: u32) -> Result<bool> {
        Ok(self.views[v].as_ref().expect("vt+0x34 of a 0x45B7A0 actor").can.contains(&index))
    }
    fn command_definition_298(&mut self, index: u32) -> Result<Option<CommandDefinition>> {
        Ok(self.definitions[index as usize])
    }
    fn actor_handle_14(&mut self, v: usize) -> Result<u32> {
        Ok(self.views[v].as_ref().expect("actor+0x14 of a target actor").handle)
    }
    fn flash_459600(&mut self, v: usize) -> Result<()> {
        self.trace.push(json!(["flash", v]));
        Ok(())
    }
    fn facing_begin_689380(&mut self, _: &[usize]) -> Result<()> {
        self.trace.push(json!(["facing_begin"]));
        Ok(())
    }
    unreachable_leaves! {
        widget_pick_64(i32, i32) -> Result<Option<usize>>;
        pressed_widget_chain_5efff0(&Event) -> Result<bool>;
        drag_threshold_b39990() -> Result<i32>;
        camera_frame_10() -> Result<bool>;
        camera_frame_8() -> Result<()>;
        cursor_feedback_620640(CursorSelection<usize>) -> Result<()>;
        box_rectangle_begin(&Event) -> Result<()>;
        box_rectangle_frame() -> Result<()>;
        box_rectangle_end() -> Result<()>;
        facing_drag_start() -> Result<()>;
        facing_frame([u32; 3], bool) -> Result<()>;
        facing_end_6885f0() -> Result<()>;
    }
}

fn u32s<const N: usize>(v: &Value) -> [u32; N] {
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), N);
    std::array::from_fn(|i| a[i].as_u64().unwrap() as u32)
}

/// Every case of the original-code oracle for 0x620760 (right-click order
/// of the default state): the same calls with the same command words.
#[test]
fn right_click_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/states/native-right-click.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert!(rows.len() >= 400);
    for row in rows {
        let a = &row["args"];
        let n = &row["native"];
        let views = a["views"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (!v["actor"].is_null()).then(|| ActorSpec {
                    b4: v["actor"]["b4"].as_bool().unwrap(),
                    can: v["actor"]["can"].as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as u32).collect(),
                    handle: v["actor"]["handle"].as_u64().unwrap() as u32,
                    can150: v["actor"]["can150"].as_bool().unwrap(),
                })
            })
            .collect();
        let definitions = a["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| {
                (!d.is_null()).then(|| CommandDefinition { byte_40: d["byte_40"].as_bool().unwrap(), flags_2c: d["flags_2c"].as_u64().unwrap() as u32 })
            })
            .collect();
        let mut mock = Mock {
            views,
            definitions,
            pointer: u32s(&a["pointer_184"]),
            map_xy: u32s(&a["map_xy"]),
            aux: u32s(&a["aux"]),
            trace: Vec::new(),
        };
        let selected: Vec<usize> = a["selected"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        let target = a["target"].as_u64().map(|t| t as usize);
        let mut m = Machine::<usize>::for_game_interface_60bae0();
        m.selection = Selection::for_state_tests(selected, target);
        m.pressed_60 = match a["widget"].as_str().unwrap() {
            "world" => Widget::World,
            "map" => Widget::Map,
            "other" => Widget::Other(1),
            "none" => Widget::None,
            w => panic!("{w}"),
        };
        let modifiers = a["modifiers"].as_u64().unwrap() as u32;
        for (bit, vk) in [(1, 0x12), (2, 0x10), (4, 0x11)] {
            if modifiers & bit != 0 {
                m.input_mut().modifier_key_59a5c0(vk, flag::PRESSED);
            }
        }
        assert_eq!(m.input().modifiers_28(), modifiers);
        let cursor = u32s::<2>(&a["cursor"]);
        m.set_cursor([cursor[0] as i32, cursor[1] as i32]);
        let e = Event {
            x: 0,
            y: 0,
            flags: a["event"]["flags"].as_u64().unwrap() as u32,
            code: a["event"]["code"].as_u64().unwrap() as u16,
            delta: 0,
            world: [0; 3],
            origin: [0; 3],
            direction: [0; 3],
        };
        let handled = m.event(&mut mock, &e).unwrap();
        // 0x5CD190 is the oracle's seam: its world left press pushes the
        // click state without a host call.
        if m.modal_names() == ["click"] {
            mock.trace.push(json!(["base"]));
        }
        let mut expected = Vec::new();
        let mut facing = None;
        for t in n["trace"].as_array().unwrap() {
            match t[0].as_str().unwrap() {
                "hovered" | "free" => {}
                // 0x5CD190 (the oracle's seam): a VWorld left press pushes the
                // click state; otherwise 0x5E0690 swallows a left release whose
                // +4 is NULL (the constructor value here) and routes the rest
                // to the host's chain leaf only for VWorld / NULL (a HUD
                // widget's or the minimap's chain is the UI thread's).
                "base" => {
                    let left_release = e.code == code::LEFT && e.released();
                    let host_side = matches!(m.pressed_widget(), Widget::World | Widget::None);
                    if host_side && !left_release {
                        expected.push(json!(["base"]));
                    }
                }
                "actor_150" => expected.push(json!(["actor_150", t[1], native_command(&t[2])])),
                "send" => {
                    // 0x61EEF0 is the oracle's seam; its own CanSend is the
                    // 0x436950 the direct rows below prove.
                    let c = native_command(&t[1]);
                    let mut can = false;
                    for &v in m.selection().selected() {
                        let Some(a) = mock.views[v].as_ref().filter(|a| a.b4) else { continue };
                        expected.push(json!(["actor_150", v, c]));
                        if a.can150 {
                            can = true;
                            break;
                        }
                    }
                    if can {
                        expected.push(json!(["acknowledge", c]));
                        expected.push(json!(["emit", c]));
                        expected.push(json!(["diagnostic", true]));
                    } else {
                        expected.push(json!(["diagnostic", false]));
                    }
                }
                "flash" => expected.push(json!(["flash", t[1]])),
                "map_world" => expected.push(json!(["map_world", [t[1][0].as_u64().unwrap() as i32, t[1][1].as_u64().unwrap() as i32]])),
                "marker" => expected.push(json!(["marker", t[1]])),
                "facing" => {
                    expected.push(json!(["facing_begin"]));
                    facing = Some((native_command(&t[1]), t[2].as_u64().unwrap() != 0));
                }
                other => panic!("{other}"),
            }
        }
        assert_eq!(mock.trace, expected, "{row}");
        assert_eq!(handled, n["al"].as_u64().unwrap() == 1, "{row}");
        match facing {
            Some((c, left)) => {
                assert_eq!(wire_json(m.facing().command().unwrap()), c, "{row}");
                assert_eq!(m.facing().left_button(), left, "{row}");
                assert_eq!(m.modal_names(), ["facing"], "{row}");
            }
            None => assert!(m.modal_names().is_empty() || m.modal_names() == ["click"], "{row}"),
        }
    }
}

/// Every direct 0x436950 row: static group +0x40 (byte +0x18), explicit
/// commands (flag bit 6, definition +0x0C 0x25/0x26/other) and the selected
/// actors' vt+0x150, in native order.
#[test]
fn can_send_436950_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/states/native-right-click.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = doc["can_send_rows"].as_array().unwrap();
    assert!(rows.len() >= 200);
    for row in rows {
        let a = &row["args"];
        let views = a["views"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (!v.is_null()).then(|| ActorSpec {
                    b4: v["b4"].as_bool().unwrap(),
                    can: Vec::new(),
                    handle: 0,
                    can150: v["can150"].as_bool().unwrap(),
                })
            })
            .collect();
        let mut mock = Mock { views, definitions: Vec::new(), pointer: [0; 3], map_xy: [0; 2], aux: [0; 2], trace: Vec::new() };
        let selected: Vec<usize> = a["selected"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        let selection = Selection::for_state_tests(selected, None).with_static_for_state_tests(a["static_group"].as_bool());
        let mut wire = CommandWire { flags: a["flags"].as_u64().unwrap() as u16, command: a["definition"].as_u64().map(|d| d as u32), ..Default::default() };
        let got = selection.can_send_436950(&mut mock, &mut wire).unwrap();
        let expected: Vec<Value> = row["native"]["trace"].as_array().unwrap().iter().map(|t| t[1].clone()).collect();
        let calls: Vec<Value> = mock.trace.iter().map(|t| t[1].clone()).collect();
        assert_eq!(calls, expected, "{row}");
        assert_eq!(got, row["native"]["al"].as_u64().unwrap() == 1, "{row}");
    }
}

fn widget_of(name: &Value) -> Widget {
    match name.as_str() {
        None => Widget::None,
        Some("vworld") => Widget::World,
        Some("hud_a") => Widget::Other(1),
        Some("hud_b") => Widget::Other(2),
        Some("minimap") => Widget::Map,
        Some(other) => panic!("{other}"),
    }
}

/// Every 0x5E0690 row: the +4 pressed widget afterwards, the early swallow
/// of a left release, and the chain step reached for VWorld/NULL (the
/// host's leaf) or a HUD widget / the minimap (the UI thread's handler).
#[test]
fn route_5e0690_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/states/native-route-5e0690.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert!(rows.len() >= 300);
    for row in rows {
        let a = &row["args"];
        let n = &row["native"];
        let mut mock = Mock { views: Vec::new(), definitions: Vec::new(), pointer: [0; 3], map_xy: [0; 2], aux: [0; 2], trace: Vec::new() };
        let mut m = Machine::<usize>::for_game_interface_60bae0();
        m.pressed_60 = widget_of(&a["ui_60"]);
        m.default_4 = widget_of(&a["state_4"]);
        let e = Event {
            x: 0,
            y: 0,
            flags: a["flags"].as_u64().unwrap() as u32,
            code: a["code"].as_u64().unwrap() as u16,
            delta: 0,
            world: [0; 3],
            origin: [0; 3],
            direction: [0; 3],
        };
        let got = m.route_5e0690(&mut mock, &e).unwrap();
        assert_eq!(m.default_pressed_4(), widget_of(&n["state_4"]), "{row}");
        let chain_reached = !n["chain"].as_array().unwrap().is_empty();
        let host_side = matches!(m.pressed_60, Widget::World | Widget::None);
        let host_called = mock.trace == vec![json!(["base"])];
        assert_eq!(host_called, chain_reached && host_side, "{row}");
        if !chain_reached {
            assert_eq!(got, n["al"].as_u64().unwrap() == 1, "{row}");
        }
    }
}
