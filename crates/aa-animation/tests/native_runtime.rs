use aa_actors::{ActorId, ground_states::ComponentHeader};
use aa_animation::runtime::{self, Animation, Character, Effect, Result, StanceWorld, World};
use aa_body::F32;
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
struct State {
    scheduled: bool,
    elapsed: F32,
    resumed_at: F32,
    rate: F32,
    animation: u32,
    active: bool,
    word_28: u32,
    variant: i32,
}
impl State {
    fn character(&self) -> Character {
        Character {
            header: ComponentHeader {
                owner: ActorId::from_raw(1),
                scheduled_head: self.scheduled.then_some(1),
            },
            elapsed: self.elapsed,
            resumed_at: self.resumed_at,
            rate: self.rate,
            animation: Some(self.animation),
            active: self.active,
            word_28: self.word_28,
            variant: self.variant,
        }
    }
    fn from_character(s: &Character) -> Self {
        Self {
            scheduled: s.header.scheduled_head.is_some(),
            elapsed: s.elapsed,
            resumed_at: s.resumed_at,
            rate: s.rate,
            animation: s.animation.unwrap(),
            active: s.active,
            word_28: s.word_28,
            variant: s.variant,
        }
    }
}
#[derive(Clone, Deserialize)]
struct Input {
    precision: Precision,
    operation: String,
    now: F32,
    state: State,
    animations: BTreeMap<u32, Animation>,
    selected: u32,
    looping: Vec<bool>,
    speed: Option<F32>,
    scale: F32,
    duration: F32,
    cue: i32,
    replace_on_end: Option<u32>,
    replace_on_cancel: Option<u32>,
    remap: Option<u32>,
    fallback: u32,
    active_component: bool,
    activate: bool,
    performer: bool,
    performer_active: bool,
    ability: bool,
    flags: Vec<u32>,
    idle: u32,
    model_variant: u32,
    rng: u32,
    repeat_min: i32,
    repeat_max: i32,
}
#[derive(Deserialize)]
struct Output {
    state: State,
    events: Vec<Value>,
    rng: u32,
    restarted: Option<bool>,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Output,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
struct Services {
    input: Input,
    events: Vec<Value>,
    rng: NativeRng,
}
impl World for Services {
    fn animation(&self, id: u32) -> Result<Animation> {
        Ok(self.input.animations[&id].clone())
    }
    fn looping_type(&self, kind: u32) -> Result<bool> {
        Ok(self.input.looping[kind as usize])
    }
    fn now(&self) -> Result<F32> {
        Ok(self.input.now)
    }
    fn remap_variant(&mut self, _: &mut Character, variant: i32, requested: u32) -> Result<u32> {
        self.events
            .push(json!({"name":"remap","variant":variant,"requested":requested}));
        Ok(self.input.remap.unwrap_or(self.input.fallback))
    }
    fn apply_variant(&mut self, _: &mut Character, variant: i32) -> Result<()> {
        self.events
            .push(json!({"name":"variant","variant":variant}));
        Ok(())
    }
    fn movement_speed(&self, _: &Character) -> Result<Option<F32>> {
        Ok(self.input.speed)
    }
    fn visual_scale(&mut self, _: &mut Character) -> Result<F32> {
        Ok(self.input.scale)
    }
    fn effect(&mut self, s: &mut Character, animation: u32, effect: Effect) -> Result<()> {
        self.events
            .push(json!({"name":"effect","animation":animation,"effect":effect}));
        if effect == Effect::End && self.input.replace_on_end.is_some() {
            s.animation = self.input.replace_on_end;
        }
        Ok(())
    }
    fn cancel_events(&mut self, s: &mut Character) -> Result<()> {
        self.events.push(json!({"name":"cancel"}));
        s.header.scheduled_head = None;
        if self.input.replace_on_cancel.is_some() {
            s.animation = self.input.replace_on_cancel;
            s.word_28 = 9;
        }
        Ok(())
    }
    fn schedule_after(&mut self, s: &mut Character, delay: F32, event: u32) -> Result<()> {
        self.events
            .push(json!({"name":"schedule","delay":delay,"event":event}));
        s.header.scheduled_head = Some(1);
        Ok(())
    }
}
impl StanceWorld for Services {
    fn active_component_is_character(&self, _: &Character) -> Result<bool> {
        Ok(self.input.active_component)
    }
    fn activate(&mut self, _: &mut Character) -> Result<bool> {
        self.events.push(json!({"name":"activate"}));
        Ok(self.input.activate)
    }
    fn performer(&self, _: &Character) -> Result<Option<u32>> {
        Ok(self.input.performer.then_some(7))
    }
    fn component_is_active(&self, _: u32) -> Result<bool> {
        Ok(!self.input.active_component && self.input.performer_active)
    }
    fn actor_flag(&self, _: &Character, flag: u32) -> Result<bool> {
        Ok(self.input.flags.contains(&flag))
    }
    fn performer_ability(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.input.ability.then_some(1))
    }
    fn model_variant(&self, _: &Character) -> Result<u32> {
        Ok(self.input.model_variant)
    }
    fn ability_idle(&self, _: u32, _: u32) -> Result<Option<u32>> {
        Ok(Some(self.input.idle))
    }
    fn first_idle(&self, _: &Character) -> Result<Option<u32>> {
        Ok(Some(self.input.idle))
    }
    fn idle_repetitions(&mut self, _: &mut Character) -> Result<i32> {
        self.events.push(json!({"name":"repetitions"}));
        runtime::idle_repetitions(self.input.repeat_min, self.input.repeat_max, &mut self.rng)
    }
}

#[test]
fn native_selection_and_playback_store_boundaries_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-animation-runtime/native-runtime.json"
    ))
    .unwrap();
    for c in f["selection"]["selection"].as_array().unwrap() {
        let a = runtime::AbilityAnimations {
            variants: serde_json::from_value(c["variants"].clone()).unwrap(),
            ..Default::default()
        };
        let mut rng = NativeRng::new(c["seed"].as_u64().unwrap() as u32);
        let variant = c["variant"].as_u64().unwrap() as u32;
        assert_eq!(
            serde_json::to_value(a.select(variant, &mut rng).unwrap()).unwrap(),
            c["chosen"]
        );
        assert_eq!(rng.state, c["after"].as_u64().unwrap() as u32);
        for entry in c["contains"].as_array().unwrap() {
            assert_eq!(
                a.contains(variant, entry["item"].as_u64().unwrap() as u32)
                    .unwrap(),
                entry["result"].as_bool().unwrap()
            );
        }
    }
    for c in f["selection"]["modes"].as_array().unwrap() {
        let a: runtime::AbilityAnimations = runtime::AbilityAnimations {
            flags: c["flags"].as_u64().unwrap() as u32,
            ..Default::default()
        };
        assert_eq!(
            a.has_mode(c["mode"].as_u64().unwrap() as u8),
            c["result"].as_bool().unwrap()
        );
    }
    for c in f["selection"]["idle"].as_array().unwrap() {
        let a = runtime::AbilityAnimations {
            idle: c["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    let n = v.as_u64().unwrap() as u32;
                    (n != 0).then_some(n)
                })
                .collect(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(a.idle(c["variant"].as_u64().unwrap() as u32).unwrap()).unwrap(),
            c["result"]
        );
    }
    for c in f["playback"].as_array().unwrap() {
        let n = |s: &str| F32(c[s].as_u64().unwrap() as u32);
        let s = Character {
            elapsed: n("start"),
            resumed_at: n("pause"),
            rate: n("rate"),
            active: c["active"].as_bool().unwrap(),
            ..Character::new(ActorId::from_raw(1), &[Some(1)])
        };
        let result = runtime::playback_parameters(
            &s,
            n("now"),
            n("kf"),
            n("length"),
            serde_json::from_value(c["precision"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.start, n("result_start"));
        assert_eq!(result.rate, n("result_rate"));
    }
}

#[test]
fn native_actor_default_stance_rereads_callback_changed_component() {
    struct Dispatch {
        character: bool,
        active: Option<u32>,
        replacement: Option<u32>,
        events: Vec<Value>,
    }
    impl runtime::ActorStanceWorld for Dispatch {
        fn character(&self, _: u32) -> Result<Option<u32>> {
            Ok(self.character.then_some(7))
        }
        fn character_default_stance(&mut self, _: u32) -> Result<()> {
            self.events.push(json!({"name":"character"}));
            Ok(())
        }
        fn active_component(&self, _: u32) -> Result<Option<u32>> {
            Ok(self.active)
        }
        fn clear_active_component(&mut self, _: u32) -> Result<()> {
            self.active = None;
            Ok(())
        }
        fn deactivate(&mut self, component: u32, reason: u32) -> Result<()> {
            assert_eq!(reason, 0);
            self.events
                .push(json!({"name":"deactivate","component":component,"active":self.active}));
            self.active = self.replacement;
            Ok(())
        }
        fn resume_component(&mut self, component: u32, reason: u32) -> Result<()> {
            assert_eq!(reason, 0);
            self.events
                .push(json!({"name":"resume","component":component,"active":self.active}));
            Ok(())
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-animation-runtime/native-runtime.json"
    ))
    .unwrap();
    for c in fixture["actor_dispatch"].as_array().unwrap() {
        let mut w = Dispatch {
            character: c["character"].as_bool().unwrap(),
            active: serde_json::from_value(c["old"].clone()).unwrap(),
            replacement: serde_json::from_value(c["replacement"].clone()).unwrap(),
            events: vec![],
        };
        runtime::actor_default_stance(1, &mut w).unwrap();
        assert_eq!(serde_json::to_value(w.active).unwrap(), c["active"]);
        assert_eq!(serde_json::to_value(w.events).unwrap(), c["events"]);
    }
}
#[test]
fn native_character_timing_selection_and_callbacks_match_all_precisions() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-animation-runtime/native-runtime.json"
    ))
    .unwrap();
    for (index, case) in fixture.cases.into_iter().enumerate() {
        let c = &case.input;
        let mut s = c.state.character();
        let mut w = Services {
            input: c.clone(),
            events: vec![],
            rng: NativeRng::new(c.rng),
        };
        let restarted = match c.operation.as_str() {
            "pause" => {
                runtime::pause(&mut s, c.now).unwrap();
                None
            }
            "resume" => {
                runtime::resume(&mut s, c.now, c.precision).unwrap();
                None
            }
            "direct" => Some(
                runtime::start_direct(
                    &mut s,
                    c.selected,
                    c.looping[c.animations[&c.selected].kind as usize],
                    c.now,
                )
                .unwrap(),
            ),
            "move" => {
                let old = w.animation(s.animation.unwrap()).unwrap().kind;
                Some(
                    runtime::start_move(
                        &mut s,
                        c.selected,
                        old,
                        c.speed.unwrap(),
                        c.scale,
                        c.animations[&c.selected].walk_distance,
                        c.now,
                        c.precision,
                    )
                    .unwrap(),
                )
            }
            "play" => {
                runtime::play(&mut s, c.selected, c.duration, &mut w, c.precision).unwrap();
                None
            }
            "begin" => {
                runtime::begin(&mut s, &mut w).unwrap();
                None
            }
            "stop" => {
                runtime::stop(&mut s, 17, &mut w).unwrap();
                None
            }
            "use" => {
                runtime::use_animation(&mut s, &mut w).unwrap();
                None
            }
            "cue" => {
                runtime::cue(&mut s, c.cue, &mut w).unwrap();
                None
            }
            "stance" => {
                runtime::default_stance(&mut s, &mut w, c.precision).unwrap();
                None
            }
            _ => panic!("unknown operation"),
        };
        assert_eq!(
            State::from_character(&s),
            case.output.state,
            "case{index} {}",
            c.operation
        );
        assert_eq!(
            w.events, case.output.events,
            "events case{index} {}",
            c.operation
        );
        assert_eq!(w.rng.state, case.output.rng, "RNG case{index}");
        assert_eq!(restarted, case.output.restarted, "restart case{index}");
    }
}
