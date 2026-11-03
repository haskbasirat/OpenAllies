//! The PerformerComponent's ability lifecycle on the port (`aa_combat::performer`):
//! `0x57F290` start, `0x57ED20` begin (the animation draw `0x4C8170`), the use frame
//! `0x57E8C0`, the animation's end, the reload, and `0x57DC90` telling the Mind.
//!
//! An element in the Kill state of its hosted Mind (`crate::movement`'s element fight)
//! fights through this; a lone actor, a Defender and the tasks keep `combat::fire` and
//! the Defender's Tick, whose ability use is immediate.
//!
//! The timed events (`0x50ACC0`, `0x50AD20`, no draw) are kept here and run by
//! [`pump`] in `combat::tick`. Natively they are on the deferred queue, which comes
//! before queue 4 in the update as this does. **Renditions:** the actor is taken to
//! have no animator (the view reads the events, not the cues), `0x57DD90`'s aiming of
//! the other abilities' turrets is not bound, and the set of a model variant is the
//! first one.
use super::{CombatState, alive, x87};
use crate::world::{ActorId, World};
use aa_body::F32;
use aa_combat::performer::{self as port, Actor, Animation, Context, Start};
use aa_numeric::Precision;

const PC: Precision = Precision::Double53;
/// binary32 `+10`: nothing is running.
pub const DONE: i32 = 4;

/// One actor's Performer: the port's fields and its pending timed events.
#[derive(Clone, Debug, Default)]
pub struct Run {
    pub state: Option<port::Performer>,
    pub events: Vec<(f32, i32)>,
}

impl Run {
    /// `+10 != 4`: an ability is running (`0x535240` waits for it).
    pub fn running(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.stage != DONE)
    }
    /// The ability being used, for the view and the state tables.
    pub fn active(&self) -> Option<usize> {
        self.state.as_ref().filter(|s| s.stage != DONE).and_then(|s| s.active_ability).map(|a| a as usize)
    }
}

fn fresh(world: &World, id: ActorId) -> port::Performer {
    let def = world.data.def(world.actors[id as usize].def);
    port::Performer {
        owner: id,
        stage: DONE,
        abilities: def.weapons.iter().enumerate().filter(|(_, w)| !w.defender).map(|(i, _)| i as u32).collect(),
        mana: F32::ZERO,
        target_id: 0,
        target_position: [F32::ZERO; 2],
        active_ability: None,
        active_index: -1,
        started_at: F32::ZERO,
        animation_length: F32::ZERO,
        event_kind: 1,
        animation: None,
        turret_index: -1,
    }
}

fn invalid<T>(what: &'static str) -> aa_combat::Result<T> {
    Err(aa_combat::Error::Invalid(what))
}

struct Host<'a> {
    world: &'a mut World,
    st: &'a mut CombatState,
    id: ActorId,
    events: Vec<(f32, i32)>,
}

impl Host<'_> {
    fn weapon(&self, ability: u32) -> aa_combat::Result<&crate::data::Weapon> {
        match self.world.data.def(self.world.actors[self.id as usize].def).weapons.get(ability as usize) {
            Some(w) => Ok(w),
            None => invalid("ability outside the definition's list"),
        }
    }
    fn now(&self) -> f32 {
        self.world.time
    }
}

/// An animation handle: the ability and its clip.
fn clip_handle(ability: u32, clip: u32) -> u32 {
    (ability << 16) | (clip & 0xffff)
}

impl port::World for Host<'_> {
    fn enabled(&self, _: u32) -> aa_combat::Result<bool> {
        Ok(true)
    }
    fn actor(&self, handle: u32) -> aa_combat::Result<Actor> {
        let Some(a) = self.world.actors.get(handle as usize) else { return invalid("actor outside the world") };
        Ok(Actor { id: handle + 1, position: [F32(a.pos.x.to_bits()), F32(a.pos.y.to_bits()), F32(a.pos.z.to_bits())] })
    }
    fn resolve_id(&self, id: u32) -> aa_combat::Result<Option<u32>> {
        Ok(id.checked_sub(1).filter(|&t| alive(self.world, t)))
    }
    /// `0x4C8F20`; a target that has gone answers not eligible.
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> aa_combat::Result<u32> {
        let Some(target) = context.target else { return Ok(1) };
        Ok(super::admission_with(self.world, self.st, self.id, ability as usize, target, check_range))
    }
    fn activate(&mut self, _: &mut port::Performer) -> aa_combat::Result<bool> {
        Ok(true)
    }
    fn track_target(&mut self, _: &mut port::Performer, _: Start, _: [F32; 2]) -> aa_combat::Result<()> {
        Ok(())
    }
    fn ability_turret(&self, ability: u32) -> aa_combat::Result<i32> {
        Ok(self.weapon(ability)?.turret)
    }
    /// `0x4C81D0`.
    fn ability_uses_turret(&self, ability: u32) -> aa_combat::Result<bool> {
        Ok(self.weapon(ability)?.turret >= 0)
    }
    /// `0x45DC50`: the turret can turn (`0x4C7620`).
    fn turret_turn_required(&self, owner: u32, index: i32) -> aa_combat::Result<bool> {
        let def = self.world.data.def(self.world.actors[owner as usize].def);
        Ok(usize::try_from(index).ok().and_then(|i| def.turrets.get(i)).is_some_and(super::turret_can_turn))
    }
    fn clear_turret_pending(&mut self, _: &mut port::Performer, _: i32) -> aa_combat::Result<()> {
        Ok(())
    }
    fn has_mover(&self, owner: u32) -> aa_combat::Result<bool> {
        Ok(self.world.data.def(self.world.actors[owner as usize].def).speed_max > 0.0)
    }
    /// `0x4EEBE0`.
    fn face_direction(&mut self, owner: u32, direction: [F32; 2]) -> aa_combat::Result<()> {
        let (x, y) = (f32::from_bits(direction[0].0), f32::from_bits(direction[1].0));
        self.world.actors[owner as usize].facing = y.atan2(x);
        Ok(())
    }
    fn now(&self) -> aa_combat::Result<F32> {
        Ok(F32(self.now().to_bits()))
    }
    fn ability_event(&mut self, _: &mut port::Performer, _: u32, _: u32, _: &mut Context) -> aa_combat::Result<()> {
        Ok(())
    }
    fn model_variant(&self, _: u32) -> aa_combat::Result<u32> {
        Ok(0)
    }
    /// `0x4C8170` -> `0x4BE5C0`: one draw whenever the set has an animation.
    fn select_animation(&mut self, ability: u32, _: u32) -> aa_combat::Result<Option<u32>> {
        let count = self.weapon(ability)?.attack_clips.len() as u32;
        if count == 0 {
            return Ok(None);
        }
        let word = self.world.random_u32_at(crate::trace::sites::ABILITY_ANIMATION);
        Ok(Some(clip_handle(ability, word % count)))
    }
    fn keep_previous_animation(&self, _: u32) -> aa_combat::Result<bool> {
        Ok(false)
    }
    fn animation_belongs_to(&self, _: u32, _: u32, _: u32) -> aa_combat::Result<bool> {
        Ok(false)
    }
    fn animation(&self, animation: u32) -> aa_combat::Result<Animation> {
        let w = self.weapon(animation >> 16)?;
        let Some(c) = w.attack_clips.get((animation & 0xffff) as usize) else { return invalid("animation outside its set") };
        Ok(Animation { length: F32(c[0].to_bits()), use_fraction: F32(c[1].to_bits()), cues: Vec::new() })
    }
    fn has_animator(&self, _: u32) -> aa_combat::Result<bool> {
        Ok(false)
    }
    fn play_animation(&mut self, _: &mut port::Performer, _: u32) -> aa_combat::Result<()> {
        Ok(())
    }
    fn animator_begin(&mut self, _: &mut port::Performer) -> aa_combat::Result<()> {
        Ok(())
    }
    fn animator_use(&mut self, _: &mut port::Performer) -> aa_combat::Result<()> {
        Ok(())
    }
    fn animator_cue(&mut self, _: &mut port::Performer, _: i32) -> aa_combat::Result<()> {
        Ok(())
    }
    /// `0x57E030`: the ability's `+10` is the Performer's start (`+3C`), and the use.
    fn use_ability(&mut self, state: &mut port::Performer, ability: u32, context: &mut Context, _: Precision) -> aa_combat::Result<()> {
        let Some(target) = context.target else { return Ok(()) };
        super::use_performed(self.world, self.st, self.id, ability as usize, target, f32::from_bits(state.started_at.0));
        Ok(())
    }
    fn reload_time(&self, ability: u32) -> aa_combat::Result<F32> {
        Ok(F32(self.weapon(ability)?.reload_time.to_bits()))
    }
    fn idle_animation(&self, _: u32, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(None)
    }
    fn play_idle(&mut self, _: &mut port::Performer, _: u32, _: F32) -> aa_combat::Result<()> {
        Ok(())
    }
    fn active_component_is_self(&self, _: &port::Performer) -> aa_combat::Result<bool> {
        Ok(true)
    }
    fn actor_paused(&self, _: u32) -> aa_combat::Result<bool> {
        Ok(false)
    }
    /// `0x57DC90` -> Mind `+0x138` `0x52D9B0`: the Mind's next update is now.
    fn notify_finished(&mut self, state: &mut port::Performer) -> aa_combat::Result<()> {
        crate::movement::think_mind_in(self.world, state.owner, 0.0);
        Ok(())
    }
    /// Stage 0 waits for the turret: [`pump`] steps it each update.
    fn schedule_immediate(&mut self, _: &mut port::Performer, _: i32) -> aa_combat::Result<()> {
        Ok(())
    }
    fn schedule_after(&mut self, _: &mut port::Performer, delay: F32, event: i32) -> aa_combat::Result<()> {
        let at = x87::store(x87::add(x87::load(self.now()), x87::load(f32::from_bits(delay.0))));
        self.events.push((at, event));
        Ok(())
    }
    fn schedule_at(&mut self, _: &mut port::Performer, time: F32, event: i32) -> aa_combat::Result<()> {
        self.events.push((f32::from_bits(time.0), event));
        Ok(())
    }
    fn cancel_event(&mut self, _: &mut port::Performer, event: i32, _: bool) -> aa_combat::Result<()> {
        self.events.retain(|e| e.1 != event);
        Ok(())
    }
}

/// `0x57F290(ability, 1, target, null)`. False when the ability could not start.
pub(crate) fn start(world: &mut World, st: &mut CombatState, id: ActorId, weapon: usize, target: ActorId) -> bool {
    let i = id as usize;
    let mut run = std::mem::take(&mut st.actors[i].performer);
    let mut state = run.state.take().unwrap_or_else(|| fresh(world, id));
    let mut host = Host { world, st, id, events: Vec::new() };
    let result = port::start(&mut state, Start { ability: weapon as u32, event_kind: 1, target: Some(target), point: None }, &mut host, PC);
    let events = std::mem::take(&mut host.events);
    run.state = Some(state);
    if matches!(result, Ok(true)) {
        run.events = events;
    }
    st.actors[i].performer = run;
    match result {
        Ok(started) => started,
        Err(e) => {
            world.books.minds.note(format!("performer {id}: {e}"));
            false
        }
    }
}

/// The Performer leaves: a Mover took the actor (`0x4DB0F0` no longer holds), or it
/// died. Its events go; the next Think finds it finished.
pub(crate) fn cancel(st: &mut CombatState, id: ActorId) {
    if let Some(a) = st.actors.get_mut(id as usize) {
        a.performer.events.clear();
        if let Some(s) = a.performer.state.as_mut() {
            s.stage = DONE;
        }
    }
}

/// One update of the Performer's events: the turret of stage 0 (`0x57EF00`), then
/// every event that is due, in time order (`0x57F040`).
pub(crate) fn pump(world: &mut World, st: &mut CombatState, id: ActorId) {
    let i = id as usize;
    if !st.actors[i].performer.running() {
        return;
    }
    let now = world.time;
    let mut run = std::mem::take(&mut st.actors[i].performer);
    let mut state = run.state.take().unwrap_or_else(|| fresh(world, id));
    let mut events = std::mem::take(&mut run.events);
    if state.stage == 0 {
        let on_target = match (state.target_id.checked_sub(1).filter(|&t| alive(world, t)), state.active_ability) {
            (Some(t), Some(ability)) => super::turret_ready(world, st, i, ability as usize, t),
            _ => true,
        };
        if on_target {
            let mut host = Host { world, st, id, events: std::mem::take(&mut events) };
            let r = port::timed_event(&mut state, 0, &mut host, PC);
            events = std::mem::take(&mut host.events);
            if let Err(e) = r {
                world.books.minds.note(format!("performer {id}: {e}"));
            }
        }
    }
    loop {
        let due = events.iter().enumerate().filter(|(_, e)| e.0 <= now).min_by(|a, b| a.1.0.total_cmp(&b.1.0)).map(|(k, _)| k);
        let Some(k) = due else { break };
        let (_, event) = events.remove(k);
        let mut host = Host { world, st, id, events: std::mem::take(&mut events) };
        let r = port::timed_event(&mut state, event, &mut host, PC);
        events = std::mem::take(&mut host.events);
        if let Err(e) = r {
            world.books.minds.note(format!("performer {id}: {e}"));
            state.stage = DONE;
            events.clear();
            break;
        }
    }
    run.state = Some(state);
    run.events = events;
    st.actors[i].performer = run;
}
