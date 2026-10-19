//! Replays every native 6E8730/6E83A0/6E8300 row
//! (research/development10/game-state-entry/native-loading-refresh.json).
use aa_ui::loading_refresh::{on_connected_6e8730, Art, Result, Services, Widget};
use serde_json::{json, Value};

struct Replay<'a> {
    c: &'a Value,
    flags: std::collections::BTreeMap<&'static str, u32>,
    trace: Vec<Value>,
}
impl Replay<'_> {
    fn b(&self, k: &str) -> bool {
        self.c[k].as_u64().unwrap() != 0
    }
    fn u(&self, k: &str) -> u32 {
        self.c[k].as_u64().unwrap() as u32
    }
    fn art(&self, k: &str) -> bool {
        self.c["art"][k].as_u64().unwrap() != 0
    }
    fn creator_off(&self) -> u32 {
        match self.u("launch_mode") {
            0 => 0x74,
            2 => 0x70,
            5 => 0x7c,
            _ => 0x6c,
        }
    }
}
impl Services for Replay<'_> {
    type Texture = &'static str;
    fn base_on_connected_5c08e0(&mut self) -> Result<()> {
        self.trace.push(json!(["base_on_connected_5c08e0"]));
        Ok(())
    }
    fn game_app_kind_c(&mut self) -> Result<u32> {
        Ok(self.u("kind"))
    }
    fn preload_campaign_mission_art(&mut self) -> Result<()> {
        self.trace.push(json!(["campaign_entry_411790", 1]));
        self.trace.push(json!(["preload_mission_art", "mission_10", 1, true]));
        if self.b("mission_left") && self.b("mission_right") {
            self.trace.push(json!(["preload_mission_art", "mission_left", 1, true]));
            self.trace.push(json!(["preload_mission_art", "mission_right", 1, true]));
        }
        Ok(())
    }
    fn campaign_mission_art(&mut self) -> Result<Art<&'static str>> {
        self.trace.push(json!(["campaign_entry_411790", 1]));
        Ok((self.b("mission_left").then_some("mission_left"), self.b("mission_right").then_some("mission_right")))
    }
    fn session_phase_f8(&mut self) -> Result<u32> {
        Ok(self.u("phase"))
    }
    fn session_launch_mode_58(&mut self) -> Result<u32> {
        Ok(self.u("launch_mode"))
    }
    fn set_visible(&mut self, widget: Widget, visible: bool) -> Result<()> {
        let f = self.flags[widget.name()];
        let f = if visible { f | 1 } else { f & !1 };
        self.flags.insert(widget.name(), f);
        self.trace.push(json!(["visible", widget.name(), f & 1]));
        Ok(())
    }
    fn set_map_name_text(&mut self) -> Result<()> {
        self.trace.push(json!(["set_text_c8", "MapNameLabel", "map_name"]));
        Ok(())
    }
    fn set_map_description_text(&mut self) -> Result<()> {
        self.trace.push(json!(["set_text_c8", "MapDescriptionLabel", "map_description"]));
        Ok(())
    }
    fn objectives_populate_673d40(&mut self) -> Result<()> {
        self.trace.push(json!(["objectives_populate_673d40"]));
        Ok(())
    }
    fn objectives_clear_673720(&mut self) -> Result<()> {
        self.trace.push(json!(["objectives_clear_items"]));
        Ok(())
    }
    fn ui_present(&mut self) -> Result<bool> {
        Ok(self.b("ui"))
    }
    fn local_player_present_5c0e10(&mut self) -> Result<bool> {
        Ok(self.b("local_player"))
    }
    fn local_player_nation_art(&mut self) -> Result<Option<Option<Art<&'static str>>>> {
        let matches = self.c["player_side"] == self.c["creator_side"] && self.u("creator_off") == self.creator_off();
        if !matches {
            return Ok(None);
        }
        if !self.b("nation") {
            return Ok(Some(None));
        }
        Ok(Some(Some((self.b("nation_left").then_some("nation_left"), self.b("nation_right").then_some("nation_right")))))
    }
    fn default_art(&mut self) -> Result<Art<&'static str>> {
        Ok((self.art("default_left").then_some("default_left"), self.art("default_right").then_some("default_right")))
    }
    fn dynamic_art(&mut self) -> Result<Art<&'static str>> {
        Ok((self.art("dynamic_left").then_some("dynamic_left"), self.art("dynamic_right").then_some("dynamic_right")))
    }
    fn set_texture_5ee580(&mut self, widget: Widget, texture: Option<&'static str>) -> Result<()> {
        self.trace.push(json!(["set_texture_5ee580", widget.name(), texture, 0]));
        Ok(())
    }
    fn session_status_6e8300(&mut self) -> Result<()> {
        self.trace.push(json!(["session_status_76e4c0"]));
        self.trace.push(json!(["set_text_c8", "StatusLabel", "session_status"]));
        Ok(())
    }
    fn audio_hold_745580(&mut self) -> Result<()> {
        self.trace.push(json!(["audio_hold_745580", true]));
        Ok(())
    }
}

#[test]
fn native_loading_screen_on_connected_rows() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/game-state-entry/native-loading-refresh.json");
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(fixture["precision_independent"], true);
    let mut checked = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut replay = Replay { c, flags: Default::default(), trace: vec![] };
        for w in ["MapNameLabel", "MapDescriptionLabel", "ObjectivesList", "LoadingTextureLeft", "LoadingTextureRight", "ReadyButton"] {
            replay.flags.insert(w, c["flags"].as_u64().unwrap() as u32);
        }
        on_connected_6e8730(&mut replay).unwrap();
        assert_eq!(json!(replay.trace), row["output"]["trace"], "{c}");
        for (k, v) in row["output"]["flags"].as_object().unwrap() {
            assert_eq!(json!(replay.flags[k.as_str()]), *v, "{k} {c}");
        }
        checked += 1;
    }
    assert_eq!(checked, 212);
}
