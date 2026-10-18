use aa_actors::{ActorId, ground_states::CharacterState};
use aa_animation::runtime::{self, Animation, Character, Effect, Result, borrowed};
use aa_body::F32;
use aa_numeric::Precision;

struct Owner {
    state: Box<Character>,
    queue_head: Option<u64>,
    calls: Vec<String>,
    addresses: Vec<usize>,
    stop: Option<usize>,
}
impl Owner {
    fn new() -> Self {
        Self {
            state: Box::new(CharacterState::new(ActorId::from_raw(7), &[Some(1)])),
            queue_head: None,
            calls: vec![],
            addresses: vec![],
            stop: None,
        }
    }
    fn call(&mut self, name: String) -> Result<()> {
        self.addresses
            .push(self.state.as_ref() as *const _ as usize);
        self.calls.push(name);
        if self.stop == Some(self.calls.len()) {
            Err(runtime::Error::Service("fixture cutoff".into()))
        } else {
            Ok(())
        }
    }
}
impl borrowed::World for Owner {
    fn character(&self) -> Result<&Character> {
        Ok(&self.state)
    }
    fn character_mut(&mut self) -> Result<&mut Character> {
        Ok(&mut self.state)
    }
    fn scheduled_head(&self) -> Result<Option<u64>> {
        Ok(self.queue_head)
    }
    fn animation(&self, _: u32) -> Result<Animation> {
        Ok(Animation {
            kind: 0,
            length: F32(0x40000000),
            use_fraction: F32::ONE,
            walk_distance: F32::ONE,
            cues: vec![],
        })
    }
    fn looping_type(&self, k: u32) -> Result<bool> {
        runtime::canonical_looping_type(k)
    }
    fn now(&self) -> Result<F32> {
        Ok(F32(0x40a00000))
    }
    fn remap_variant(&mut self, v: i32, _: u32) -> Result<u32> {
        self.call(format!("remap:{v}"))?;
        self.state.variant = 4;
        Ok(2)
    }
    fn apply_variant(&mut self, v: i32) -> Result<()> {
        self.call(format!("variant:{v}"))
    }
    fn movement_speed(&self) -> Result<Option<F32>> {
        unreachable!()
    }
    fn visual_scale(&mut self) -> Result<F32> {
        unreachable!()
    }
    fn effect(&mut self, a: u32, e: Effect) -> Result<()> {
        self.call(format!("effect:{a}:{e:?}"))?;
        if e == Effect::End {
            self.state.animation = Some(9);
        }
        Ok(())
    }
    fn cancel_events(&mut self) -> Result<()> {
        self.call("cancel".into())?;
        self.queue_head = None;
        self.state.word_28 = 11;
        Ok(())
    }
    fn schedule_after(&mut self, d: F32, e: u32) -> Result<()> {
        self.call(format!("schedule:{:x}:{e}", d.0))?;
        self.queue_head = Some(17);
        Ok(())
    }
}
impl borrowed::StanceWorld for Owner {
    fn active_component_is_character(&self) -> Result<bool> {
        Ok(true)
    }
    fn activate(&mut self) -> Result<bool> {
        self.call("activate".into())?;
        Ok(true)
    }
    fn performer(&self) -> Result<Option<u32>> {
        Ok(None)
    }
    fn component_is_active(&self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn actor_flag(&self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn performer_ability(&self, _: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn model_variant(&self) -> Result<u32> {
        unreachable!()
    }
    fn ability_idle(&self, _: u32, _: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn first_idle(&self) -> Result<Option<u32>> {
        Ok(Some(2))
    }
    fn idle_repetitions(&mut self) -> Result<i32> {
        self.call("repeat".into())?;
        Ok(3)
    }
}
#[test]
fn borrowed_fields_use_same_allocation_reread_callbacks_and_preserve_every_error_prefix() {
    for p in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut full = Owner::new();
        full.state.variant = 3;
        let address = full.state.as_ref() as *const _ as usize;
        borrowed::play(&mut full, 2, F32::ONE, p).unwrap();
        assert_eq!(
            full.calls,
            [
                "remap:3",
                "variant:4",
                "effect:1:End",
                "effect:9:Start",
                "cancel",
                "schedule:3f800000:11"
            ]
        );
        assert!(full.addresses.iter().all(|a| *a == address));
        assert_eq!(full.state.animation, Some(9));
        assert_eq!(full.state.variant, -2);
        for cutoff in 1..=full.calls.len() {
            let mut partial = Owner::new();
            partial.state.variant = 3;
            partial.stop = Some(cutoff);
            assert!(borrowed::play(&mut partial, 2, F32::ONE, p).is_err());
            assert_eq!(partial.calls, full.calls[..cutoff]);
            assert_eq!(
                partial.state.variant,
                if cutoff == 1 {
                    3
                } else if cutoff == 2 {
                    4
                } else {
                    -2
                }
            );
            assert_eq!(
                partial.state.animation,
                if cutoff <= 2 {
                    Some(1)
                } else if cutoff == 3 {
                    Some(2)
                } else {
                    Some(9)
                }
            );
            assert_eq!(partial.state.word_28, if cutoff <= 5 { 0 } else { 11 });
            assert!(partial.queue_head.is_none());
        }
    }
}
#[test]
fn borrowed_stance_reads_authoritative_queue_head_and_commits_word_before_schedule() {
    let mut owner = Owner::new();
    owner.state.header.scheduled_head = Some(99);
    owner.stop = Some(5);
    //A stale constructor head must not admit563BA0's early return.
    assert!(borrowed::default_stance(&mut owner, Precision::Double53).is_err());
    assert_eq!(
        owner.calls,
        [
            "activate",
            "repeat",
            "effect:1:End",
            "effect:9:Start",
            "schedule:40c00000:0"
        ]
    );
    assert_eq!(owner.state.word_28, 0);
    assert!(owner.queue_head.is_none());
    owner.calls.clear();
    owner.stop = None;
    owner.queue_head = Some(18);
    borrowed::default_stance(&mut owner, Precision::Double53).unwrap();
    assert!(owner.calls.is_empty());
}
