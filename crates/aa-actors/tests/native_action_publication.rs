use aa_actors::{
    ActorId,
    ability_flags::{self, Error},
    action_publication::{self, World},
};
use serde_json::{Value, json};
struct Fixture {
    input: Value,
    events: Vec<Value>,
    byte: u8,
    renderer: Option<u32>,
    calls: usize,
    fail: Option<&'static str>,
}
fn number(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl Fixture {
    fn new(input: Value) -> Self {
        Self {
            byte: number(&input["byte"]) as u8,
            renderer: input["renderer"].as_u64().map(|v| v as u32),
            input,
            events: vec![],
            calls: 0,
            fail: None,
        }
    }
    fn event(&mut self, name: &'static str, row: Value) -> ability_flags::Result<()> {
        self.events.push(row);
        if self.fail == Some(name) {
            return Err(Error::World(name.into()));
        }
        Ok(())
    }
}
impl World for Fixture {
    fn element_organization(&self, a: ActorId) -> ability_flags::Result<Option<ActorId>> {
        assert_eq!(a.raw(), 1);
        Ok(
            (self.input["element"].as_bool().unwrap() && self.input["parent"].as_bool().unwrap())
                .then_some(ActorId::from_raw(2)),
        )
    }
    fn organization_owner(&self, a: ActorId) -> ability_flags::Result<ActorId> {
        assert_eq!(a.raw(), 2);
        Ok(ActorId::from_raw(3))
    }
    fn rebuild_actions(&mut self, a: ActorId) -> ability_flags::Result<()> {
        self.event("rebuild", json!(["rebuild", a.raw()]))?;
        if self.input["parent_mutates"].as_bool().unwrap() {
            self.byte = number(&self.input["after_parent_byte"]) as u8;
        }
        Ok(())
    }
    fn owner(&mut self, a: ActorId) -> ability_flags::Result<Option<ActorId>> {
        self.event("owner", json!(["owner", a.raw()]))?;
        let out = self.input["owners"][self.calls]
            .as_u64()
            .map(|v| ActorId::from_raw(v as u32));
        self.calls += 1;
        if self.calls == 1 && self.input["owner_mutates"].as_bool().unwrap() {
            self.byte = number(&self.input["after_owner_byte"]) as u8;
        }
        Ok(out)
    }
    fn object_byte_4c(&self, _: ActorId) -> ability_flags::Result<u8> {
        Ok(self.byte)
    }
    fn refresh_owner_views(&mut self, owner: Option<ActorId>) -> ability_flags::Result<()> {
        self.event(
            "owner_views",
            json!(["owner_views", owner.map(ActorId::raw)]),
        )?;
        if self.input["ui_mutates"].as_bool().unwrap() {
            self.renderer = self.input["after_ui_renderer"].as_u64().map(|v| v as u32);
        }
        Ok(())
    }
    fn renderer_flags(&self, _: ActorId) -> ability_flags::Result<Option<u32>> {
        Ok(self.renderer)
    }
    fn refresh_selected_view(&mut self, a: ActorId) -> ability_flags::Result<()> {
        self.event("selected", json!(["selected", a.raw()]))
    }
}
fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-action-publication.json"
    ))
    .unwrap()
}
#[test]
fn matches_full_original_changed_callback_order_and_live_rereads() {
    for row in fixtures()["cases"].as_array().unwrap() {
        let mut world = Fixture::new(row["input"].clone());
        action_publication::changed(ActorId::from_raw(1), &mut world).unwrap();
        assert_eq!(
            json!({"events":world.events,"byte":world.byte,"renderer":world.renderer}),
            row["output"],
            "{}",
            row["input"]
        );
    }
}
#[test]
fn failure_keeps_prior_callbacks_and_stops_later_publication() {
    let cases = fixtures();
    let row = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["output"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v[0] == "owner_views")
                && r["input"]["element"] == true
                && r["input"]["parent"] == true
        })
        .unwrap();
    let mut world = Fixture::new(row["input"].clone());
    world.fail = Some("owner_views");
    assert!(action_publication::changed(ActorId::from_raw(1), &mut world).is_err());
    assert_eq!(world.events[0], json!(["rebuild", 3]));
    assert_eq!(world.events.last().unwrap()[0], "owner_views");
}
