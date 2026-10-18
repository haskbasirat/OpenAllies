use aa_body::F32;
use aa_combat::{Result, performer::*};
use aa_numeric::Precision;
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
    calls: usize,
    turret_pending: bool,
}
impl Host {
    fn flag(&self, k: &str) -> bool {
        self.input[k].as_bool().unwrap()
    }
    fn bits(&self, k: &str) -> F32 {
        F32(self.input[k].as_u64().unwrap() as u32)
    }
    fn event(&mut self, name: &str, fields: Value) {
        let mut v = fields;
        v.as_object_mut()
            .unwrap()
            .insert("name".into(), json!(name));
        self.events.push(v);
    }
}
impl World for Host {
    fn enabled(&self, _: u32) -> Result<bool> {
        Ok(self.flag("enabled"))
    }
    fn actor(&self, h: u32) -> Result<Actor> {
        Ok(Actor {
            id: if h == 1 { 0x10001 } else { 0x10002 },
            position: serde_json::from_value(
                self.input[if h == 1 {
                    "owner_position"
                } else {
                    "target_position"
                }]
                .clone(),
            )
            .unwrap(),
        })
    }
    fn resolve_id(&self, id: u32) -> Result<Option<u32>> {
        Ok((id == 0x10002 && self.flag("target_live")).then_some(2))
    }
    fn can_use(&mut self, a: u32, c: &mut Context, check: bool) -> Result<u32> {
        let r = self.input["rejections"].as_array().unwrap();
        let result = r[self.calls.min(r.len() - 1)].as_u64().unwrap() as u32;
        self.calls += 1;
        self.event(
            "can_use",
            json!({"ability":a,"context":c,"check_range":check,"result":result}),
        );
        Ok(result)
    }
    fn activate(&mut self, _: &mut Performer) -> Result<bool> {
        let result = self.flag("activate");
        self.event("activate", json!({"result":result}));
        Ok(result)
    }
    fn track_target(&mut self, _: &mut Performer, r: Start, p: [F32; 2]) -> Result<()> {
        self.event(
            "track",
            json!({"event_kind":r.event_kind,"target":r.target,"point":p}),
        );
        Ok(())
    }
    fn ability_turret(&self, _: u32) -> Result<i32> {
        Ok(if self.flag("uses_turret") {
            self.input["turret_index"].as_i64().unwrap() as i32
        } else {
            -1
        })
    }
    fn ability_uses_turret(&self, _: u32) -> Result<bool> {
        Ok(self.flag("uses_turret"))
    }
    fn turret_turn_required(&self, _: u32, _: i32) -> Result<bool> {
        Ok(self.flag("turret_wait"))
    }
    fn clear_turret_pending(&mut self, _: &mut Performer, _: i32) -> Result<()> {
        self.turret_pending = false;
        Ok(())
    }
    fn has_mover(&self, _: u32) -> Result<bool> {
        Ok(self.flag("mover"))
    }
    fn face_direction(&mut self, _: u32, d: [F32; 2]) -> Result<()> {
        self.event("face", json!({"direction":d}));
        Ok(())
    }
    fn now(&self) -> Result<F32> {
        Ok(self.bits("now"))
    }
    fn ability_event(&mut self, _: &mut Performer, a: u32, e: u32, c: &mut Context) -> Result<()> {
        self.event("fx", json!({"ability":a,"event":e,"context":c}));
        Ok(())
    }
    fn model_variant(&self, _: u32) -> Result<u32> {
        Ok(0)
    }
    fn select_animation(&mut self, a: u32, v: u32) -> Result<Option<u32>> {
        self.event("select", json!({"ability":a,"variant":v}));
        Ok(self.input["selected"].as_u64().map(|v| v as u32))
    }
    fn keep_previous_animation(&self, _: u32) -> Result<bool> {
        Ok(self.flag("keep_previous"))
    }
    fn animation_belongs_to(&self, _: u32, _: u32, _: u32) -> Result<bool> {
        Ok(self.flag("belongs"))
    }
    fn animation(&self, a: u32) -> Result<Animation> {
        Ok(serde_json::from_value(
            self.input[if a == 5 { "animation" } else { "old_animation" }].clone(),
        )
        .unwrap())
    }
    fn has_animator(&self, _: u32) -> Result<bool> {
        Ok(self.flag("animator"))
    }
    fn play_animation(&mut self, _: &mut Performer, a: u32) -> Result<()> {
        self.event("play", json!({"animation":a}));
        Ok(())
    }
    fn animator_begin(&mut self, _: &mut Performer) -> Result<()> {
        self.event("animator_begin", json!({}));
        Ok(())
    }
    fn animator_use(&mut self, _: &mut Performer) -> Result<()> {
        self.event("animator_use", json!({}));
        Ok(())
    }
    fn animator_cue(&mut self, _: &mut Performer, c: i32) -> Result<()> {
        self.event("cue", json!({"cue":c}));
        Ok(())
    }
    fn use_ability(
        &mut self,
        _: &mut Performer,
        a: u32,
        c: &mut Context,
        _: Precision,
    ) -> Result<()> {
        self.event("use", json!({"ability":a,"context":c}));
        Ok(())
    }
    fn reload_time(&self, _: u32) -> Result<F32> {
        Ok(self.bits("reload"))
    }
    fn idle_animation(&self, _: u32, _: u32) -> Result<Option<u32>> {
        Ok(self.input["idle"].as_u64().map(|v| v as u32))
    }
    fn play_idle(&mut self, _: &mut Performer, a: u32, d: F32) -> Result<()> {
        self.event("idle", json!({"animation":a,"duration":d}));
        Ok(())
    }
    fn active_component_is_self(&self, _: &Performer) -> Result<bool> {
        Ok(self.flag("active"))
    }
    fn actor_paused(&self, _: u32) -> Result<bool> {
        Ok(self.flag("paused"))
    }
    fn notify_finished(&mut self, s: &mut Performer) -> Result<()> {
        self.event("finish", json!({"stage":s.stage}));
        Ok(())
    }
    fn schedule_immediate(&mut self, _: &mut Performer, e: i32) -> Result<()> {
        self.event("immediate", json!({"event":e}));
        Ok(())
    }
    fn schedule_after(&mut self, _: &mut Performer, t: F32, e: i32) -> Result<()> {
        self.event("after", json!({"time":t,"event":e}));
        Ok(())
    }
    fn schedule_at(&mut self, _: &mut Performer, t: F32, e: i32) -> Result<()> {
        self.event("at", json!({"time":t,"event":e}));
        Ok(())
    }
    fn cancel_event(&mut self, _: &mut Performer, e: i32, p: bool) -> Result<()> {
        self.event("cancel", json!({"event":e,"periodic":p}));
        Ok(())
    }
}
#[test]
fn native_performer_start_use_reload_sequences_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-performer/native-performer-fixtures.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 102);
    for c in cases {
        let input = &c["input"];
        let mut state: Performer = serde_json::from_value(input["state"].clone()).unwrap();
        let mut world = Host {
            input: input.clone(),
            events: vec![],
            calls: 0,
            turret_pending: true,
        };
        let precision: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        for (index, op) in input["operations"].as_array().unwrap().iter().enumerate() {
            let result = match op["kind"].as_str().unwrap() {
                "start" => Some(
                    start(
                        &mut state,
                        serde_json::from_value(input["request"].clone()).unwrap(),
                        &mut world,
                        precision,
                    )
                    .unwrap(),
                ),
                "begin" => {
                    begin(&mut state, &mut world, precision).unwrap();
                    None
                }
                "event" => {
                    timed_event(
                        &mut state,
                        op["event"].as_i64().unwrap() as i32,
                        &mut world,
                        precision,
                    )
                    .unwrap();
                    None
                }
                _ => panic!("unexpected operation"),
            };
            assert_eq!(
                json!({"return":result,"state":state,"events":world.events,"turret_pending":world.turret_pending}),
                c["steps"][index],
                "{} step{}",
                input["name"],
                index
            );
        }
    }
}
