use aa_actors::{ActorId, ground_states::CharacterState};
use aa_animation::runtime::{self, Animation, Character, Effect, Result, StanceWorld, World};
use aa_body::F32;
use aa_numeric::Precision;
use std::collections::BTreeMap;
fn f(v: f32) -> F32 {
    F32(v.to_bits())
}
struct LiveServices {
    definitions: BTreeMap<String, Animation>,
    now: F32,
    effects: Vec<(String, Effect)>,
    addresses: Vec<usize>,
    scheduled: Vec<(F32, u32)>,
    idle: String,
}
impl LiveServices {
    fn witness(&mut self, state: &Character<String>) {
        self.addresses.push(state as *const _ as usize);
    }
}
impl World<String> for LiveServices {
    fn animation(&self, key: String) -> Result<Animation> {
        self.definitions
            .get(&key)
            .cloned()
            .ok_or(runtime::Error::Invalid("unknown scoped key"))
    }
    fn now(&self) -> Result<F32> {
        Ok(self.now)
    }
    fn remap_variant(
        &mut self,
        _state: &mut Character<String>,
        _variant: i32,
        _requested: String,
    ) -> Result<String> {
        Err(runtime::Error::Invalid(
            "test does not substitute unproved remap",
        ))
    }
    fn apply_variant(&mut self, _state: &mut Character<String>, _variant: i32) -> Result<()> {
        Err(runtime::Error::Invalid(
            "test does not substitute unproved variant",
        ))
    }
    fn movement_speed(&self, _state: &Character<String>) -> Result<Option<F32>> {
        Ok(Some(f(2.)))
    }
    fn visual_scale(&mut self, state: &mut Character<String>) -> Result<F32> {
        self.witness(state);
        Ok(F32::ONE)
    }
    fn effect(&mut self, state: &mut Character<String>, key: String, effect: Effect) -> Result<()> {
        self.witness(state);
        self.effects.push((key, effect));
        Ok(())
    }
    fn cancel_events(&mut self, state: &mut Character<String>) -> Result<()> {
        self.witness(state);
        state.header.scheduled_head = None;
        Ok(())
    }
    fn schedule_after(
        &mut self,
        state: &mut Character<String>,
        delay: F32,
        event: u32,
    ) -> Result<()> {
        self.witness(state);
        state.header.scheduled_head = Some(7);
        self.scheduled.push((delay, event));
        Ok(())
    }
}
impl StanceWorld<String> for LiveServices {
    fn active_component_is_character(&self, _state: &Character<String>) -> Result<bool> {
        Ok(false)
    }
    fn activate(&mut self, state: &mut Character<String>) -> Result<bool> {
        self.witness(state);
        Ok(true)
    }
    fn performer(&self, _state: &Character<String>) -> Result<Option<u32>> {
        Ok(None)
    }
    fn component_is_active(&self, _component: u32) -> Result<bool> {
        unreachable!()
    }
    fn actor_flag(&self, _state: &Character<String>, _flag: u32) -> Result<bool> {
        unreachable!()
    }
    fn performer_ability(&self, _performer: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn model_variant(&self, _state: &Character<String>) -> Result<u32> {
        unreachable!()
    }
    fn ability_idle(&self, _ability: u32, _variant: u32) -> Result<Option<String>> {
        unreachable!()
    }
    fn first_idle(&self, _state: &Character<String>) -> Result<Option<String>> {
        Ok(Some(self.idle.clone()))
    }
    fn idle_repetitions(&mut self, state: &mut Character<String>) -> Result<i32> {
        self.witness(state);
        Ok(2)
    }
}
#[test]
fn scoped_string_keys_preserve_same_authoritative_character_box_and_native_identity() {
    //Same authored name can exist in two distinct native model-set definitions.
    //Scoped identity must trigger restart; equality of display names must not.
    let base = "usa_infantry/set3/row0".to_owned();
    let stance = "usa_infantry/set1/row0".to_owned();
    let definition = Animation {
        kind: 0,
        length: f(2.),
        use_fraction: f(0.5),
        walk_distance: f(1.),
        cues: vec![f(0.75)],
    };
    let mut services = LiveServices {
        definitions: BTreeMap::from([
            (base.clone(), definition.clone()),
            (stance.clone(), definition),
        ]),
        now: f(5.),
        effects: vec![],
        addresses: vec![],
        scheduled: vec![],
        idle: base.clone(),
    };
    let mut state = Box::new(CharacterState::new(
        ActorId::from_raw(0x1234),
        &[Some(base.clone())],
    ));
    let address = state.as_ref() as *const _ as usize;
    runtime::play(
        &mut state,
        stance.clone(),
        f(3.),
        &mut services,
        Precision::Double53,
    )
    .unwrap();
    assert_eq!(state.animation, Some(stance.clone()));
    assert_eq!(state.elapsed, f(5.));
    assert_eq!(
        services.effects,
        [(base.clone(), Effect::End), (stance.clone(), Effect::Start)]
    );
    assert_eq!(state.header.scheduled_head, Some(7));
    assert_eq!(services.scheduled, [(f(3.), 0)]);
    services.now = f(6.);
    runtime::play(
        &mut state,
        stance.clone(),
        F32::ZERO,
        &mut services,
        Precision::Double53,
    )
    .unwrap();
    assert_eq!(services.effects.len(), 2);
    assert_eq!(state.elapsed, f(5.));
    runtime::use_animation(&mut state, &mut services).unwrap();
    runtime::cue(&mut state, 0, &mut services).unwrap();
    assert_eq!(
        &services.effects[2..],
        [
            (stance.clone(), Effect::Use),
            (stance.clone(), Effect::Cue(0))
        ]
    );
    runtime::pause(&mut state, f(7.)).unwrap();
    runtime::resume(&mut state, f(9.), Precision::Double53).unwrap();
    assert_eq!(state.elapsed, f(7.));
    runtime::begin(&mut state, &mut services).unwrap();
    assert_eq!(services.scheduled.last(), Some(&(f(2.), 0)));
    runtime::default_stance(&mut state, &mut services, Precision::Double53).unwrap();
    assert_eq!(state.animation, Some(base));
    assert_eq!(services.scheduled.last(), Some(&(f(4.), 0)));
    runtime::stop(&mut state, 0, &mut services).unwrap();
    assert_eq!(state.header.scheduled_head, None);
    assert_eq!(state.as_ref() as *const _ as usize, address);
    assert!(services.addresses.iter().all(|&p| p == address));
    assert_eq!(state.header.owner, ActorId::from_raw(0x1234));
    let mut rng = aa_simulation::NativeRng::new(3);
    let bank = runtime::AbilityAnimations {
        variants: vec![vec![stance.clone()]],
        idle: vec![Some(stance.clone())],
        flags: 0,
    };
    assert_eq!(bank.select(0, &mut rng).unwrap(), Some(stance.clone()));
    assert!(bank.contains(0, stance.clone()).unwrap());
    assert_eq!(bank.idle(0).unwrap(), Some(stance));
}
