//! Replays every native 612010 row (research/development10/game-state-entry/
//! native-game-update.json, original = v14, PC24/PC53/PC64).
use aa_numeric::Precision;
use aa_ui::game_interface_update::{update_612010, BannerTiming, Field, Result, Services};
use serde_json::{json, Value};
use std::collections::BTreeMap;

struct Replay<'a> {
    input: &'a Value,
    flags: BTreeMap<&'static str, u32>,
    trace: Vec<Value>,
}
const ALL: [Field; 12] = [
    Field::TimeMessage,
    Field::GroupFloater,
    Field::FactionAbilitiesFloater,
    Field::DeveloperModeLabel,
    Field::CameraInformationLabel,
    Field::ButtonSupplyBorder,
    Field::ButtonChat,
    Field::DisplayBannerPanel,
    Field::GameMenuDialog,
    Field::DisplayPause,
    Field::GameChatPanel,
    Field::UpkeepShortageIcon,
];
fn key(field: Field) -> &'static str {
    match field {
        Field::TimeMessage => "time_flags",
        Field::GroupFloater => "group_flags",
        Field::FactionAbilitiesFloater => "ability_flags",
        Field::DeveloperModeLabel => "dev_flags",
        Field::CameraInformationLabel => "camera_flags",
        Field::ButtonSupplyBorder => "supply_button_flags",
        Field::ButtonChat => "chat_button_flags",
        Field::DisplayBannerPanel => "banner_flags",
        Field::GameMenuDialog => "menu_flags",
        Field::DisplayPause => "pause_flags",
        Field::GameChatPanel => "chat_flags",
        Field::UpkeepShortageIcon => "upkeep_flags",
    }
}
impl Replay<'_> {
    fn u(&self, k: &str) -> u32 {
        self.input[k].as_u64().unwrap() as u32
    }
    fn b(&self, k: &str) -> bool {
        self.u(k) != 0
    }
}
impl Services for Replay<'_> {
    fn uvar_view_show_elapsed_game_time(&mut self) -> Result<bool> {
        Ok(self.b("uvar_time"))
    }
    fn uvar_camera_information(&mut self) -> Result<bool> {
        Ok(self.b("uvar_camera"))
    }
    fn uvar_view_show_zone_friendly_supply(&mut self) -> Result<bool> {
        Ok(self.b("uvar_supply"))
    }
    fn time_message_parent_is_interface(&mut self) -> Result<bool> {
        Ok(self.b("time_parent_is_this"))
    }
    fn flags(&mut self, field: Field) -> Result<u32> {
        Ok(self.flags[field.name()])
    }
    fn store_flags(&mut self, field: Field, flags: u32) -> Result<()> {
        self.flags.insert(field.name(), flags);
        Ok(())
    }
    fn add_child_d0(&mut self, field: Field) -> Result<()> {
        self.trace.push(json!(["add_child_d0", field.name()]));
        Ok(())
    }
    fn remove_child_5efb60(&mut self, field: Field) -> Result<()> {
        self.trace.push(json!(["remove_child_5efb60", field.name(), 0]));
        Ok(())
    }
    fn update_60ba50(&mut self) -> Result<()> {
        self.trace.push(json!(["update_60ba50"]));
        Ok(())
    }
    fn local_kingdom_present_5c0e30(&mut self) -> Result<bool> {
        self.trace.push(json!(["local_kingdom_5c0e30"]));
        Ok(self.b("local_kingdom"))
    }
    fn base_update_5c06b0(&mut self) -> Result<()> {
        self.trace.push(json!(["base_update_5c06b0"]));
        Ok(())
    }
    fn developer_label_present(&mut self) -> Result<bool> {
        Ok(self.b("dev_label"))
    }
    fn dev_mode_40eaa0(&mut self) -> Result<bool> {
        self.trace.push(json!(["dev_mode_40eaa0"]));
        Ok(self.b("dev_mode"))
    }
    fn set_visible_5efb00(&mut self, field: Field, visible: bool) -> Result<()> {
        let f = self.flags[field.name()];
        let f = if visible { f | 1 } else { f & !1 };
        self.flags.insert(field.name(), f);
        self.trace.push(json!(["visible", field.name(), f & 1]));
        Ok(())
    }
    fn node7_current(&mut self) -> Result<bool> {
        Ok(self.b("node7_current"))
    }
    fn camera_information_text(&mut self) -> Result<()> {
        self.trace.push(json!(["camera_information_text"]));
        Ok(())
    }
    fn selection_group_e4(&mut self, _: Field) -> Result<bool> {
        Ok(false)
    }
    fn functor_e0(&mut self, field: Field) -> Result<bool> {
        Ok(match field {
            Field::ButtonSupplyBorder => self.b("supply_functor"),
            Field::ButtonChat => self.b("chat_functor"),
            _ => unreachable!(),
        })
    }
    fn post_selection_event(&mut self, field: Field, selected: bool) -> Result<()> {
        self.trace.push(json!(["selection_event_5dc780", format!("{}+E0", field.name()), u8::from(selected), 0]));
        self.trace.push(json!(["post_event_5c3eb0"]));
        Ok(())
    }
    fn update_427e60(&mut self) -> Result<()> {
        self.trace.push(json!(["update_427e60"]));
        Ok(())
    }
    fn timer_ms_b396e0(&mut self) -> Result<u32> {
        self.trace.push(json!(["timer_b396e0"]));
        Ok(self.u("now"))
    }
    fn sim_paused_28(&mut self) -> Result<u8> {
        Ok(self.u("paused") as u8)
    }
    fn banner_fade_duration_ms(&mut self) -> Result<u32> {
        Ok(self.u("fade_ms"))
    }
    fn banner_background_color_5ee520(&mut self) -> Result<[u32; 4]> {
        self.trace.push(json!(["get_color", "DisplayBannerBackground"]));
        Ok([0.25f32, 0.5, 0.75, 0.125].map(f32::to_bits))
    }
    fn set_banner_background_color_5ee600(&mut self, color: [u32; 4]) -> Result<()> {
        self.trace.push(json!(["set_color", "DisplayBannerBackground", color]));
        Ok(())
    }
    fn banner_color_5edff0(&mut self) -> Result<[u32; 4]> {
        self.trace.push(json!(["get_color", "DisplayBanner"]));
        Ok([0.25f32, 0.5, 0.75, 0.125].map(f32::to_bits))
    }
    fn set_banner_color_5edfe0(&mut self, color: [u32; 4]) -> Result<()> {
        self.trace.push(json!(["set_color", "DisplayBanner", color]));
        Ok(())
    }
    fn game_menu_present(&mut self) -> Result<bool> {
        Ok(self.b("game_menu"))
    }
    fn pause_timer_elapsed(&mut self) -> Result<u32> {
        self.trace.push(json!(["pause_timer_elapsed"]));
        Ok(self.u("pause_elapsed"))
    }
    fn pause_timer_restart(&mut self) -> Result<()> {
        self.trace.push(json!(["pause_timer_restart", true]));
        Ok(())
    }
    fn local_kingdom_shortage_ratio_5c1760(&mut self) -> Result<Option<u32>> {
        self.trace.push(json!(["local_kingdom_5c1760"]));
        Ok(self.b("upkeep_kingdom").then(|| self.u("shortage_ratio")))
    }
    fn update_60b9d0(&mut self) -> Result<()> {
        self.trace.push(json!(["update_60b9d0"]));
        Ok(())
    }
}

/// Collapse the native camera-text children into the one service marker.
fn native_trace(trace: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut in_camera = false;
    for t in trace {
        let name = t[0].as_str().unwrap();
        if name == "text_manager_856370" {
            in_camera = true;
            out.push(json!(["camera_information_text"]));
            continue;
        }
        if in_camera {
            if name == "camera_label_set_text_c8" {
                in_camera = false;
            }
            continue;
        }
        out.push(t.clone());
    }
    out
}

#[test]
fn native_game_interface_update_rows() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/game-state-entry/native-game-update.json");
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut checked = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        let out = &row["output"];
        let precision = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut replay = Replay { input, flags: BTreeMap::new(), trace: Vec::new() };
        for field in ALL {
            let present = match field {
                Field::GameMenuDialog => replay.b("game_menu"),
                Field::DeveloperModeLabel => replay.b("dev_label"),
                _ => true,
            };
            if present {
                let v = replay.u(key(field));
                replay.flags.insert(field.name(), v);
            }
        }
        let b = input["banner"].as_array().unwrap();
        let mut banner = BannerTiming {
            start_19c: b[0].as_u64().unwrap() as u32,
            last_1a0: b[1].as_u64().unwrap() as u32,
            accumulated_1a4: b[2].as_u64().unwrap() as u32,
            prev_paused_1a8: input["banner_prev_paused"].as_u64().unwrap() as u8,
        };
        update_612010(&mut banner, &mut replay, precision).unwrap();
        assert_eq!(json!(replay.trace), json!(native_trace(out["trace"].as_array().unwrap())), "{input}");
        assert_eq!(
            json!([banner.start_19c, banner.last_1a0, banner.accumulated_1a4, banner.prev_paused_1a8]),
            out["banner"],
            "{input}"
        );
        for (name, flags) in out["flags"].as_object().unwrap() {
            //Background/Banner flags are not read or written by 612010.
            let ours = replay.flags.get(name.as_str()).copied().unwrap_or(0);
            assert_eq!(json!(ours), *flags, "{name} {input}");
        }
        checked += 1;
    }
    assert_eq!(checked, 308 * 3);
}
