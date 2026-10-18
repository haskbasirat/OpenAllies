use super::*;
use serde_json::{Value, json};
fn u(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
struct Probe {
    input: Value,
    trace: Vec<Value>,
    bound: bool,
    label: Vec<u16>,
    creator: Vec<u16>,
    glyph: Vec<u16>,
    fail: Option<&'static str>,
    next_reads: u32,
}
impl Probe {
    fn new(input: Value) -> Self {
        Self {
            input,
            trace: Vec::new(),
            bound: false,
            label: u("old-label"),
            creator: u("old-setup"),
            glyph: u("old-glyph"),
            fail: None,
            next_reads: 0,
        }
    }
    fn step(&mut self, key: &'static str) -> Result<()> {
        if self.fail == Some(key) {
            Err(missing(key))
        } else {
            Ok(())
        }
    }
}
impl World for Probe {
    type Player = ();
    type Kingdom = u32;
    type Creator = ();
    type Glyph = u16;
    fn start_diagnostic(&mut self) -> Result<()> {
        self.trace.push(json!(["diagnostic"]));
        self.step("start")
    }
    fn first_player(&mut self) -> Result<Option<()>> {
        self.step("first")?;
        Ok(Some(()))
    }
    fn next_player(&mut self, _: &()) -> Result<Option<()>> {
        self.next_reads += 1;
        self.step("next")?;
        Ok(None)
    }
    fn kingdom_name(&mut self, _: &()) -> Result<Vec<u16>> {
        Ok(if self.input["assigned"] == true {
            u("K")
        } else {
            vec![]
        })
    }
    fn find_kingdom(&mut self, name: &[u16]) -> Result<Option<u32>> {
        self.trace.push(json!(["find_kingdom", name]));
        self.step("find")?;
        Ok((self.input["assigned"] == true).then_some(1))
    }
    fn replace_kingdom(&mut self, _: &(), k: Option<u32>) -> Result<()> {
        self.bound = k.is_some();
        self.trace.push(json!(["bind_765E10", self.bound]));
        self.step("bind")
    }
    fn ai(&mut self, _: &()) -> Result<bool> {
        self.step("ai")?;
        Ok(self.input["ai"] == true)
    }
    fn local_link(&mut self, _: &()) -> Result<bool> {
        self.step("link")?;
        Ok(self.input["local_link"] == true)
    }
    fn create_sai(&mut self, _: &()) -> Result<()> {
        self.trace.push(json!(["create_sai", 0]));
        self.step("sai")
    }
    fn diagnostic(&mut self, _: &()) -> Result<()> {
        self.trace.push(json!(["diagnostic"]));
        self.step("diagnostic")
    }
    fn application_mode(&mut self) -> Result<u32> {
        let mode = self.input["mode"].as_u64().unwrap() as u32;
        self.trace.push(json!(["application_mode", mode]));
        self.step("application")?;
        Ok(mode)
    }
    fn editor_nation_text(&mut self, _: u32) -> Result<Vec<u16>> {
        self.step("nation")?;
        Ok(u("Nation"))
    }
    fn set_kingdom_label(&mut self, _: u32, text: &[u16]) -> Result<()> {
        self.label = text.to_vec();
        self.trace.push(json!(["kingdom_label", text]));
        self.step("label")
    }
    fn normal_name_mode(&mut self) -> Result<u32> {
        self.trace.push(json!(["mode_4F1310"]));
        self.step("mode")?;
        Ok(0)
    }
    fn display_name(&mut self, _: &()) -> Result<Vec<u16>> {
        Ok(u("Host"))
    }
    fn kingdom_thing_name(&mut self, _: u32) -> Result<Vec<u16>> {
        Ok(u("K"))
    }
    fn creator_set_name(&mut self, k: &[u16], name: &[u16]) -> Result<()> {
        self.trace.push(json!(["creator_find", k]));
        self.step("creator_find")?;
        self.creator = name.to_vec();
        self.trace.push(json!(["creator_name", name]));
        self.step("creator_name")
    }
    fn player_glyph(&mut self, _: &()) -> Result<Option<u16>> {
        let value = self.input["glyph"].as_u64().map(|v| v as u16);
        self.trace.push(json!(["player_glyph", value.is_some()]));
        self.step("glyph_lookup")?;
        Ok(value)
    }
    fn glyph_index(&mut self, g: &u16) -> Result<u16> {
        self.step("glyph_index")?;
        Ok(*g)
    }
    fn creator_set_glyph(&mut self, _: &(), text: &[u16]) -> Result<()> {
        self.glyph = text.to_vec();
        self.trace.push(if let Some(unit) = text.first() {
            json!(["glyph_format", unit])
        } else {
            json!(["glyph_clear"])
        });
        self.step("glyph_write")
    }
}
#[test]
fn native_773e30_caller_order_and_partial_source_branches_replay() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/begin-playing/native-prepare-players.json")).unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 42);
    for row in fixture["cases"].as_array().unwrap() {
        let mut w = Probe::new(row["input"].clone());
        prepare(&mut w).unwrap();
        assert_eq!(json!(w.trace), row["trace"], "{row}");
        assert_eq!(json!(w.bound), row["bound"]);
        assert_eq!(json!(w.label), row["kingdom_label"]);
        assert_eq!(json!(w.creator), row["creator_label"]);
        assert_eq!(json!(w.glyph), row["creator_glyph"]);
        assert_eq!(w.next_reads, 1);
    }
}
#[test]
fn reached_failures_keep_prior_bound_player_and_real_naming_prefix() {
    let input = json!({"assigned":true,"mode":1,"ai":false,"local_link":true,"glyph":132});
    for stage in [
        "start",
        "first",
        "find",
        "bind",
        "ai",
        "diagnostic",
        "application",
        "mode",
        "label",
        "creator_find",
        "creator_name",
        "glyph_lookup",
        "glyph_index",
        "glyph_write",
        "next",
    ] {
        let mut w = Probe::new(input.clone());
        w.fail = Some(stage);
        assert!(prepare(&mut w).is_err(), "{stage}");
        if ["start", "first", "find"].contains(&stage) {
            assert!(!w.bound);
        } else {
            assert!(w.bound);
        }
        assert_eq!(w.next_reads, u32::from(stage == "next"));
    }
    let mut human = Probe::new(input);
    human.fail = Some("link");
    prepare(&mut human).unwrap();
    let mut ai =
        Probe::new(json!({"assigned":true,"mode":1,"ai":true,"local_link":false,"glyph":null}));
    ai.fail = Some("sai");
    prepare(&mut ai).unwrap();
    let mut observer =
        Probe::new(json!({"assigned":false,"mode":4,"ai":false,"local_link":true,"glyph":null}));
    observer.fail = Some("nation");
    prepare(&mut observer).unwrap();
    assert!(observer.trace.iter().any(|v| v == &json!(["mode_4F1310"])));
    assert!(eq_wcs(&[65, 0, 66], &[65]));
    assert!(!eq_wcs(&[65], &[97]));
}
