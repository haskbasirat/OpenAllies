//! The element Mind (UAI.cpp) on the exact ports: a `HostedKind::Element` Mind
//! per element on update queue 4 (`crate::minds`), running
//! `aa_mind::element_update`, `element_move`, `element_move_continuation`,
//! `node_reached`, `aa_navigation::try_action_go`, `formation_follow` and
//! `result_plan` over [`MovementState`]. Census cause M4 (and M2, M3).
//!
//! Hosted states: Move (Mind+7C), Sleep (+68, `uai_sleep`), Deploy (+48), Unpack
//! (+188), Reattach (+178, `uai_reattach`), and the commands Move and Stop through
//! `0x52C320` (`aa_mind::command`, `element_command`), which a state of a higher
//! priority defers. The element states of other systems (Kill, Engage, Bombard,
//! Repair, Join, Unload) still live in `crate::combat` and reach the Mind as plain
//! moves.
//!
//! **Renditions, each marked at its place.** `0x536840` (a thing met is taken as a
//! target) stands aside for the combat states; `0x5709D0`'s Element +1C byte is read
//! from the zones engine (census F2); the idle state's overlap escape is not hosted.
use super::*;
use aa_mind::uai_thinks::element as offsets;
use aa_mind::{
    Arena, MindId, StateId,
    command::{self, Context, Definition},
    element_command,
    element_move::{self, Payload},
    element_move_continuation as cont, node_reached,
    uai_thinks::{ThinkWorld, uai_reattach, uai_sleep},
};
use aa_navigation::{ActorState, MoveState, MoverRequest, OwnedPath, PathNode, formation_follow as follow, result_plan};
use aa_numeric::Binary;

const PC: aa_numeric::Precision = PC53;
/// binary32 1e-4 ("immediately").
const IMMEDIATE_BITS: u32 = 0x38d1_b717;
const ONE_BITS: u32 = 0x3f80_0000;
/// The organization things are named `ORG | index` in the ports' actor words.
const ORG: u32 = 0x8000_0000;
/// aa_navigation's "not a region" collection +18 (`0x52CD90`).
const NO_REGION: u32 = u32::MAX;
/// Site names of the census (`aa_parity::native_trace::getter_group`).
const GO_SITE: &str = "ActionGo from continuation 5379C0";
const GO_SITE_FORWARD: &str = "ActionGo from 537150";
const GO_SITE_BACK: &str = "ActionGo from get-back-to-path 532B50";
const GO_SITE_PLAN: &str = "ActionGo from low path 5336B0";
const GO_SITE_RESULT: &str = "ActionGo from 536F40";
const GO_SITE_AVOID: &str = "ActionGo from 533140";
const GO_SITE_WAIT: &str = "ActionGo from queue wait 536AD0";
const GO_SITE_LOW: &str = "ActionGo from 531520";
const ORDER_SITE: &str = "mover order 5773C0 -> queue 0";

/// The switch: the ports unless `AA_ELEMENT_MIND=old` asks for the earlier f32
/// reimplementation of `movement.rs`.
pub(super) fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("AA_ELEMENT_MIND").map_or(true, |v| v != "old"))
}

fn err(what: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(format!("element mind: {what}"))
}
fn bits(v: Vec2) -> [u32; 2] {
    [v.x.to_bits(), v.y.to_bits()]
}
fn vec2(b: [u32; 2]) -> Vec2 {
    Vec2::new(f32::from_bits(b[0]), f32::from_bits(b[1]))
}
pub(super) fn node_words(pos: Vec2, dir: Vec2, kind: u32) -> PathNode {
    let (p, d) = (bits(pos), bits(dir));
    PathNode { words: [p[0], p[1], d[0], d[1], kind, 0, p[0], p[1]] }
}
pub(super) fn new_payload() -> Box<Payload> {
    Box::new(Payload {
        plan: result_plan::State {
            movement: MoveState {
                target_id: 0,
                destination_bits: [0, 0],
                auxiliary_bits: [0, 0],
                requested_speed_bits: 0,
                phase: 0,
                mode: 0,
                waypoint_offset_bits: 0,
                path: OwnedPath { nodes: Vec::new(), kind1_count: 0, kind2_count: 0, dirty: false, word18: NO_REGION },
            },
            attempts: 0,
            previous_destination_bits: [0, 0],
        },
        collision: [0; 4],
        collision_vector: [0, 0],
        detached_time_bits: 0,
        word78: 0,
    })
}

struct NoDraws;
impl aa_mind::construct::World for NoDraws {
    /// `crate::creation` makes the creation draw (0x530E50); the jitter word
    /// only serves abilities, which are not hosted.
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
}

/// `0x529C00`'s state virtual +4 for the element states: `0x529FC0` for those of
/// metadata priority 2, always for Deploy and Unload (`0x6A71D0`), `0x529F80(2)` for
/// Sleep. Priorities from their static initializers (sleep 0, move 2, deploy 3).
fn priority(offset: u32) -> i32 {
    match offset {
        offsets::SLEEP => 0,
        offsets::DEPLOY | offsets::UNLOAD => 3,
        _ => 2,
    }
}

/// The Load of a saved element Mind (`0x536340`) over `0x4CF480` case 4: the stores,
/// the stack and the commands, the Move state's fields into the payload, the Mover
/// and the Character as saved (`0x4D8FA0`). Nothing is scheduled: the queue Load names
/// the Updatables.
pub(super) fn restore(ms: &mut MovementState, world: &mut World, id: ActorId, saved: &aa_mind::saved_load::Saved, placed: &crate::map::PlacedThing) -> Result<(), String> {
    let built = aa_mind::construct::construct(aa_mind::construct::Kind::Element, id, &mut NoDraws, PC).map_err(|e| e.to_string())?;
    let applied = world.books.minds.install_saved(id, HostedKind::Element, built, saved)?;
    if world.books.minds.sleep_unless_hosted(id, &[offsets::MOVE, offsets::SLEEP, offsets::DEPLOY, offsets::UNPACK, offsets::REATTACH], offsets::SLEEP) {
        ms.note(format!("element {id}: the saved Mind state is not hosted, it sleeps"));
    }
    let minds = &world.books.minds;
    let word = |offset: u32| minds.word(id, offset);
    let nodes = applied.element_path.unwrap_or_default();
    let count = |kind: u32| nodes.iter().filter(|n| n.words[4] == kind).count() as i32;
    let (kind1_count, kind2_count) = (count(1), count(2));
    let target = match world.ids.thing(word(0x8c)) {
        Some(crate::ids::Thing::Actor(a)) => a + 1,
        _ => 0,
    };
    let payload = Box::new(Payload {
        plan: result_plan::State {
            movement: MoveState {
                target_id: target,
                destination_bits: [word(0x90), word(0x94)],
                auxiliary_bits: [word(0x98), word(0x9c)],
                requested_speed_bits: word(0xa0),
                phase: word(0xa8),
                mode: word(0xac),
                waypoint_offset_bits: word(0xd0),
                path: OwnedPath { nodes, kind1_count, kind2_count, dirty: minds.byte(id, 0xe8) != 0, word18: word(0xec) },
            },
            attempts: word(0xa4) as i32,
            previous_destination_bits: [word(0xb0), word(0xb4)],
        },
        collision: [word(0xb8), word(0xbc), word(0xc0), word(0xc4)],
        collision_vector: [word(0xc8), word(0xcc)],
        detached_time_bits: word(0xf0),
        word78: word(0xf4),
    });
    let moving = minds.state(id, offsets::MOVE).and_then(|s| minds.arena.states.get(&s)).is_some_and(|s| s.status == 2);
    let am = &mut ms.actors[id as usize];
    am.active = moving;
    mirror(am, &payload);
    am.payload = Some(payload);
    let facing = Vec2::new(placed.facing[0], placed.facing[1]);
    if facing != Vec2::ZERO {
        am.mover.facing = facing;
    }
    let current = placed.current_component;
    for (slot, component) in &placed.components {
        use aa_mapworld::saved_components::SavedComponent as C;
        match component {
            C::Mover(m) => {
                let am = &mut ms.actors[id as usize];
                am.mover.dest = vec2([m.vector_10[0], m.vector_10[1]]);
                am.mover.speed = f32::from_bits(m.word_20);
                am.mover.requested = f32::from_bits(m.word_24);
                if let Some(d) = m.vector_38 {
                    am.mover.desired = vec2([d[0], d[1]]);
                }
                if let Some(factor) = m.word_50 {
                    am.mover.factor = f32::from_bits(factor);
                }
                am.mover.instant = m.byte_54.is_some_and(|b| b != 0);
                am.mover.active = current == Some(i32::from(*slot));
            }
            C::Character(c) => {
                let b = world.books.actor_mut(id);
                b.character.active = current == Some(i32::from(*slot));
                b.character.mode = c.word_28;
            }
            _ => {}
        }
    }
    Ok(())
}

/// `0x4CF480` case 4 for an element, on first use: its Mind, idle in Sleep.
pub(super) fn ensure_mind(ms: &mut MovementState, world: &mut World, id: ActorId) {
    if ms.actors[id as usize].payload.is_none() {
        ms.actors[id as usize].payload = Some(new_payload());
    }
    if world.books.minds.is_hosted(id, HostedKind::Element) {
        return;
    }
    match aa_mind::construct::construct(aa_mind::construct::Kind::Element, id, &mut NoDraws, PC) {
        Ok(built) => {
            if let Err(e) = world.books.minds.install(id, HostedKind::Element, built) {
                ms.note(format!("element Mind {id}: {e}"));
                return;
            }
            let minds = &mut world.books.minds;
            if let Some(sleep) = minds.state(id, offsets::SLEEP) {
                if let Some(m) = minds.arena.minds.get_mut(&MindId(id)) {
                    m.stack.push_front(Some(sleep));
                }
                if let Some(s) = minds.arena.states.get_mut(&sleep) {
                    s.status = 2;
                }
            }
            // Mind +8 `0x529EC0` -> +0x1A0 at creation: the Sleep thinks in this pass.
            if !world.is_scheduled(Sched::Mind(id)) {
                think_mind_in(world, id, 0.0);
            }
        }
        Err(e) => ms.note(format!("element Mind {id}: {e}")),
    }
}

/// What a port asked for that needs the Mind's whole payload or the world
/// without the Mind's stores taken out: done after the port returned, which is
/// where the port's last call stood.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum After {
    None,
    NodeReached,
    Clear,
    Failed,
    /// Mind +0x15C `0x52F260(1)`.
    PathFailed,
    ContinueTurn(bool),
    ContinueCurrent,
}

/// One Mind's world while its payload and the arena are out of their stores.
pub struct Host<'w> {
    ms: &'w mut MovementState,
    world: &'w mut World,
    rules: MoveRules,
    id: ActorId,
    /// `0x575C80`: the unpack the Think asked for, run after the Mind is back.
    unpack: Option<[u32; 2]>,
    died: bool,
    /// [B3D2D4]: the Mind default `0x52D7C0` is running.
    resetting: bool,
    /// The census name of the ActionGo a port is about to run.
    go_site: &'static str,
}

/// Run `f` with `id`'s Mind out of the stores.
fn with_mind<T>(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, f: impl FnOnce(&mut Host<'_>, &mut Payload, &mut Arena) -> T) -> (T, Option<[u32; 2]>, bool) {
    let mut arena = std::mem::take(&mut world.books.minds.arena);
    let result = with_mind_in(ms, world, rules, id, &mut arena, f);
    world.books.minds.arena = arena;
    result
}

/// [`with_mind`] when the arena is already in the caller's hands (a regiment's Think
/// delivering to its members).
pub(super) fn with_mind_in<T>(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, arena: &mut Arena, f: impl FnOnce(&mut Host<'_>, &mut Payload, &mut Arena) -> T) -> (T, Option<[u32; 2]>, bool) {
    // `ensure_mind` works on the store's arena.
    std::mem::swap(&mut world.books.minds.arena, arena);
    ensure_mind(ms, world, id);
    std::mem::swap(&mut world.books.minds.arena, arena);
    let mut p = ms.actors[id as usize].payload.take().unwrap_or_else(new_payload);
    let (r, unpack, died) = {
        let mut host = Host { ms, world, rules, id, unpack: None, died: false, resetting: false, go_site: GO_SITE };
        let r = f(&mut host, &mut p, arena);
        (r, host.unpack, host.died)
    };
    mirror(&mut ms.actors[id as usize], &p);
    ms.actors[id as usize].payload = Some(p);
    (r, unpack, died)
}

/// The fields the regiment layer and the view read from `ActorMove`.
fn mirror(am: &mut ActorMove, p: &Payload) {
    let m = &p.plan.movement;
    am.nodes.clear();
    am.nodes.extend(m.path.nodes.iter().map(|n| Node { pos: vec2([n.words[0], n.words[1]]), dir: vec2([n.words[2], n.words[3]]), kind: n.words[4] as u8 }));
    am.requested = f32::from_bits(m.requested_speed_bits);
    am.mode = m.mode as u8;
    am.waypoint_offset = f32::from_bits(m.waypoint_offset_bits);
}

/// The Mind primitives (`aa_mind::World`) with the payload in hand.
struct Stack<'h, 'w> {
    host: &'h mut Host<'w>,
    p: &'h mut Payload,
}
impl aa_mind::World for Stack<'_, '_> {
    /// State virtual +4 (see [`priority`]).
    fn can_activate(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<bool> {
        let offset = self.host.offset_of(state);
        if matches!(offset, offsets::DEPLOY | offsets::UNLOAD) {
            return Ok(true);
        }
        let mine = if offset == offsets::SLEEP { 2 } else { priority(offset) };
        let top = a.mind(MindId(self.host.id))?.stack.front().copied().flatten();
        Ok(match top {
            Some(top) => priority(self.host.offset_of(top)) <= mine,
            None => -1 < mine,
        })
    }
    fn enter(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        match self.host.offset_of(state) {
            offsets::MOVE => {
                self.host.ms.actors[self.host.id as usize].active = true;
                element_move::enter(state, self.p, a, PC, self.host)
            }
            offsets::DEPLOY => {
                // 0x52F560: status 2, ethereal and not blocking (`MemberWalk`),
                // then 0x52DF60 mode 0 to +10/+14 with movement mode 1.
                a.state_mut(state)?.status = 2;
                self.host.deploy_flags();
                let to = self.host.ms.actors[self.host.id as usize].walk_to;
                self.host.activate_embedded_move(to, 1, self.p, a)
            }
            offsets::UNPACK => {
                // 0x52EB90: status 2; away from the point (0x52A240), 0x52DF60
                // mode 0 to it with movement mode 1.
                a.state_mut(state)?.status = 2;
                let at = self.host.ms.actors[self.host.id as usize].unpack_at;
                let pos = bits(pos2(self.host.world, self.host.id));
                if !element_move::near_point(pos, bits(at), 0x3c23_d70a, PC)? {
                    self.host.activate_embedded_move(at, 1, self.p, a)?;
                }
                Ok(())
            }
            offsets::SLEEP => {
                // 0x52FC20: status 2, then 0x4CFC80 unless actor +0x104 bit 1 (the
                // world's first update; the Mind is built without an Enter here).
                a.state_mut(state)?.status = 2;
                self.host.deactivate();
                Ok(())
            }
            other => {
                a.state_mut(state)?.status = 2;
                if other == offsets::REATTACH {
                    self.host.ms.actors[self.host.id as usize].reattaching = true;
                }
                Ok(())
            }
        }
    }
    fn leave(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        match self.host.offset_of(state) {
            offsets::MOVE => {
                self.host.ms.actors[self.host.id as usize].active = false;
                element_move::leave(state, self.p, a, self.host)
            }
            offsets::DEPLOY | offsets::UNPACK | offsets::REATTACH => {
                // 0x52F660 / 0x52EB50: the embedded Move leaves (0x529C20).
                let mv = self.host.state(offsets::MOVE)?;
                if a.state(mv)?.status != 0 {
                    self.leave(mv, a)?;
                }
                a.state_mut(state)?.status = 0;
                if self.host.offset_of(state) == offsets::REATTACH {
                    self.host.ms.actors[self.host.id as usize].reattaching = false;
                }
                if self.host.offset_of(state) == offsets::DEPLOY {
                    self.host.ms.actors[self.host.id as usize].walk = None;
                    self.host.deploy_flags();
                }
                Ok(())
            }
            _ => {
                a.state_mut(state)?.status = 0;
                Ok(())
            }
        }
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.host.schedule_mind(delay);
        Ok(())
    }
    /// Mind virtual +1A0 `0x52D7C0`: guarded by [B3D2D4], the pending commands
    /// (`0x52BF30`); none taken, Sleep (`0x52A810(2)`) and the current command goes
    /// (`0x52A660`).
    fn reset_default(&mut self, mind: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        if self.host.resetting {
            return Ok(());
        }
        self.host.resetting = true;
        let resumed = command::resume_pending(mind, a, self, &mut Context::default());
        let resumed = match resumed {
            Ok(r) => r,
            Err(e) => {
                self.host.resetting = false;
                return Err(e);
            }
        };
        if resumed {
            self.host.resetting = false;
            return Ok(());
        }
        let sleep = self.host.state(offsets::SLEEP)?;
        let activated = aa_mind::activate(sleep, 2, a, self);
        self.host.resetting = false;
        activated?;
        command::set_current(mind, None, a, self)
    }
    /// Mind virtual +1A4 `0x52D710`: the actor state the view reads.
    fn state_changed(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}

/// `aa_mind::command::World`: `0x52C320` and `0x52BF30` with the element handlers.
impl command::World for Stack<'_, '_> {
    fn definition(&self, c: u32) -> aa_mind::Result<Definition> {
        let cmd = self.host.world.books.minds.commands.get(&c).ok_or_else(|| err(format!("unknown command {c}")))?;
        let d = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.get(cmd.command as usize).ok_or_else(|| err(format!("command index {}", cmd.command)))?;
        Ok(Definition { component_slot: d.component_slot, mind_type: d.mind_type, behavior_flags: d.behavior_flags })
    }
    fn trailing(&self, c: u32) -> aa_mind::Result<bool> {
        self.host.world.books.minds.commands.get(&c).map(|x| x.trailing).ok_or_else(|| err(format!("unknown command {c}")))
    }
    fn clone_owned(&mut self, c: u32) -> aa_mind::Result<u32> {
        let minds = &mut self.host.world.books.minds;
        let mut copy = minds.commands.get(&c).cloned().ok_or_else(|| err(format!("unknown command {c}")))?;
        copy.refs = 1;
        minds.next_command += 1;
        let id = minds.next_command;
        minds.commands.insert(id, copy);
        Ok(id)
    }
    fn retain(&mut self, c: u32) -> aa_mind::Result<()> {
        self.host.world.books.minds.commands.get_mut(&c).ok_or_else(|| err(format!("unknown command {c}")))?.refs += 1;
        Ok(())
    }
    fn release(&mut self, c: u32) -> aa_mind::Result<()> {
        let commands = &mut self.host.world.books.minds.commands;
        let x = commands.get_mut(&c).ok_or_else(|| err(format!("unknown command {c}")))?;
        x.refs = x.refs.saturating_sub(1);
        if x.refs == 0 {
            commands.remove(&c);
        }
        Ok(())
    }
    /// The Mind's handler virtuals. Move +0xBC `0x538340`, Stop +0xD4 `0x52D910`;
    /// the other kinds are refused here (the combat system gives its orders apart).
    fn invoke(&mut self, _: MindId, slot: u32, c: u32, a: &mut Arena, _: &mut Context) -> aa_mind::Result<bool> {
        match slot {
            0xbc => {
                let mind = MindId(self.host.id);
                let mut world = MoveCommand { stack: self, a };
                element_command::move_command(mind, c, &mut world, PC)
            }
            0xd4 => {
                let sleep = self.host.state(offsets::SLEEP)?;
                if !aa_mind::activate(sleep, 2, a, self)? {
                    return Ok(false);
                }
                // 0x4EEBE0: the organization's facing.
                if let Some(o) = self.host.org() {
                    let f = self.host.ms.orgs[o as usize].facing;
                    if f != Vec2::ZERO {
                        self.host.am().mover.facing = f;
                        let id = self.host.id;
                        write_facing(self.host.ms, self.host.world, id);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.host.schedule_mind(delay);
        Ok(())
    }
    /// State virtual +8: `0x6A71D0` for Deploy and Unload.
    fn defers_rejected_command(&mut self, state: StateId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(matches!(self.host.offset_of(state), offsets::DEPLOY | offsets::UNLOAD))
    }
    /// A native type 6 Stop through `0x4D9B60` (the failed command's fallback).
    fn deliver_stop(&mut self, actor: u32, a: &mut Arena) -> aa_mind::Result<bool> {
        let c = self.host.world.books.minds.new_command(STOP_COMMAND, None, None);
        let result = command::process(MindId(actor), c, a, self, &mut Context::default());
        command::World::release(self, c)?;
        result
    }
    fn actor_flags(&self, actor: u32) -> aa_mind::Result<u32> {
        Ok(u32::from(!alive(self.host.world, actor)))
    }
    /// `0x44DEA0`: aa-play keeps no command groups (census X2).
    fn has_live_group(&self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn finish_group_commands(&mut self, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}

impl Host<'_> {
    pub(super) fn parts(&self) -> (&MovementState, &World) {
        (&*self.ms, &*self.world)
    }
    pub(super) fn parts_ref(&self) -> (&MovementState, &World) {
        (&*self.ms, &*self.world)
    }
    pub(super) fn parts_mut(&mut self) -> (&mut MovementState, &mut World) {
        (&mut *self.ms, &mut *self.world)
    }
    pub(super) fn actor_id(&self) -> ActorId {
        self.id
    }
    pub(super) fn position_bits(&self) -> [u32; 2] {
        bits(self.position())
    }
    pub(super) fn deploy_status(&self) -> u32 {
        u32::from(self.ms.actors[self.id as usize].walk.is_some()) * 2
    }
    pub(super) fn set_go_site_low(&mut self) {
        self.go_site = GO_SITE_LOW;
    }
    pub(super) fn stop_move(&mut self) {
        self.stop();
    }
    pub(super) fn deactivate_now(&mut self) {
        self.deactivate();
    }
    pub(super) fn schedule_now(&mut self, delay: u32) {
        self.schedule_mind(delay);
    }
    pub(super) fn ignored_thing(&self, target_id: u32) -> Option<u32> {
        self.ignored(target_id)
    }
    /// `0x5300A0` on fields a port holds; what it asks of the whole Mind comes back.
    pub(super) fn action_go_on(&mut self, state: &mut MoveState, speed: u32) -> aa_navigation::Result<After> {
        let mut actor = self.action_actor();
        let context = aa_navigation::ActionContext {
            precision: PC,
            group_speed_minimum_bits: self.rules.group_speed_minimum.to_bits(),
            catchup_speed_factor_bits: self.rules.catchup.to_bits(),
        };
        let mut go = GoHost { host: self, after: After::None };
        let result = aa_navigation::try_action_go(state, &mut actor, speed, context, &mut go);
        let after = go.after;
        self.am().mover.instant = actor.mover_instant_turn;
        result?;
        Ok(after)
    }
    /// One of the `0x534A40` / `0x536AD0` results over the tile lists and the
    /// low-path solver, then what it left for the whole Mind.
    fn uai(&mut self, p: &mut Payload, a: &mut Arena, f: impl FnOnce(&mut cont::CollisionEnvironment<'_, '_>, &mut Payload, &mut uai::Uai<'_, '_>) -> aa_mind::Result<()>) -> aa_mind::Result<()> {
        let Some(mut boxed) = self.ms.space.take() else { return Err(err("no tile lists")) };
        let after = {
            let space::Space { index, geometry, grid, solver, busy, .. } = &mut *boxed;
            let tile_shape = aa_navigation::uai_collision::Shape { kind: geometry.cell_shape.kind, radius_bits: geometry.cell_shape.radius_bits, bounds_bits: geometry.cell_shape.bounds };
            let (tile_shift, cell_size_bits) = (index.grid().coordinate_shift, index.grid().cell_size_bits);
            let mut environment = aa_navigation::low_path::Environment { grid, search_busy: busy, stats: None, settings: uai::settings(&self.world.data), options: uai::options() };
            let mut env = cont::CollisionEnvironment {
                world_step_bits: self.world.clock.step.0,
                world_time_bits: self.world.clock.begin.0,
                wait_minimum_bits: self.rules.low_path_wait.to_bits(),
                solver,
                environment: &mut environment,
                tile_shift,
                cell_size_bits,
                tile_shape,
            };
            let mut nav = uai::Uai { host: self, index, geometry: &*geometry, after: After::None };
            let result = f(&mut env, p, &mut nav);
            let after = nav.after;
            if let Err(e) = result {
                self.ms.space = Some(boxed);
                self.go_site = GO_SITE;
                return Err(e);
            }
            after
        };
        self.ms.space = Some(boxed);
        self.go_site = GO_SITE;
        let state = self.state(offsets::MOVE)?;
        match after {
            After::NodeReached => {
                node_reached::apply(state, p, a, PC, self)?;
            }
            After::Clear => self.clear_for_regiment(p, a)?,
            After::PathFailed => self.path_failed_52f260(true, p, a)?,
            _ => {}
        }
        Ok(())
    }
    /// `0x52F260`: the Mover stops, the leading kind-0 nodes go; no path left fails the
    /// Move; a head within reach goes on at once (`0x52E130`, as `0x5313F0`'s straight
    /// line), `fail` fails it, else the head counts a try and the Mind waits 1 s.
    fn path_failed_52f260(&mut self, fail: bool, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        self.stop();
        aa_navigation::low_path::pop_front_kind0(&mut p.plan.movement.path);
        let Some(head) = p.plan.movement.path.nodes.first().copied() else {
            self.failed(p, a)?;
            return Ok(());
        };
        let pos = self.position();
        let d = vec2([head.words[0], head.words[1]]) - pos;
        if f64::from(d.x) * f64::from(d.x) + f64::from(d.y) * f64::from(d.y) <= 1.0 {
            let state = self.state(offsets::MOVE)?;
            element_move::clear_path(p);
            self.continue_turn(state, false, p, a)?;
            return Ok(());
        }
        if fail {
            self.failed(p, a)?;
            return Ok(());
        }
        let node = &mut p.plan.movement.path.nodes[0];
        node.words[5] = node.words[5].wrapping_add(1);
        self.schedule_mind(ONE_BITS);
        Ok(())
    }
    /// Run `f` with the tile lists out of `MovementState` and the descriptor policy.
    fn with_space<T>(&mut self, f: impl FnOnce(&mut space::Space, &mut space::Policy<'_>) -> T) -> Option<T> {
        with_space_of(self.ms, self.world, f)
    }
    /// The thing a move to `target` ignores: `0x432E40` then `0x4CE6E0`.
    fn ignored(&self, target_id: u32) -> Option<u32> {
        let thing = target_id.checked_sub(1)?;
        if thing & ORG != 0 {
            let o = thing & !ORG;
            return self.ms.orgs.get(o as usize)?.leader.map(space::actor_key);
        }
        alive(self.world, thing).then(|| space::actor_key(thing))
    }
    /// `0x531170` (from the actor) and `0x5312C0` (from `origin`).
    fn probe(&mut self, origin: Option<[u32; 2]>, direction: [u32; 2], distance: u32, target_id: u32) -> [u32; 4] {
        let id = self.id;
        let target = self.ignored(target_id);
        let position = bits(self.position());
        self.with_space(|s, policy| {
            let mut q = space::query_for(s, policy, id, false)?;
            let (from, goal) = match origin {
                Some(from) => (from, aa_spatial::actor_context::explicit_origin_goal(from, direction, distance, PC).ok()?),
                None => (position, aa_spatial::actor_context::current_actor_goal(position, direction, distance, PC).ok()?),
            };
            q.set_position(&s.index, from, policy).ok()?;
            q.placement.facing = direction;
            q.descriptor.ignore_actor = target;
            Some(space::words(&space::sweep_to(s, policy, &mut q, goal)))
        })
        .flatten()
        .unwrap_or([0; 4])
    }
    /// The flags of the Deploy state's Enter and Exit, at once in the tile lists.
    fn deploy_flags(&mut self) {
        let id = self.id;
        let flags = space::thing_flags(self.ms, self.world, id);
        if let Some(s) = self.ms.space.as_mut() {
            s.set_flags(id, flags);
        }
    }
    /// `0x530F40(true)`, `ResultPath`: the head node's tries; a path node whose point
    /// no longer takes the actor moves to one that does (`0x53D340`). 1 waits, 2 goes
    /// on along the moved node, 0 falls through to `0x534A40`.
    fn result_path_530f40(&mut self, p: &mut Payload) -> aa_mind::Result<u32> {
        let Some(head) = p.plan.movement.path.nodes.first().copied() else { return Err(err("0x530F40 reads a null head")) };
        if (head.words[5] as i32) > 0 {
            let tries = head.words[5] as i32 + 1;
            p.plan.movement.path.nodes[0].words[5] = tries as u32;
            if tries > 2 {
                return Ok(1);
            }
        }
        if head.words[4] != 1 {
            return Ok(0);
        }
        let id = self.id;
        let target = self.ignored(p.plan.movement.target_id);
        let position = bits(self.position());
        let mut fallback = position;
        let mut to = [head.words[0], head.words[1]];
        if p.plan.movement.mode == 3
            && let Some(o) = self.org()
        {
            // `0x52CED0`: the regiment path's kind-1 node as far from its tail as this
            // node is from the tail of its own path, the node the slot's was made from.
            let regiment = regiment::owned_path(&self.ms.orgs[o as usize].order);
            let mut left = p.plan.movement.path.kind1_count - 1;
            if regiment.kind1_count > left {
                for n in regiment.nodes.iter().rev() {
                    if n.words[4] == 1 {
                        left -= 1;
                        if left < 0 {
                            fallback = [n.words[0], n.words[1]];
                            break;
                        }
                    }
                }
            }
        }
        let m = &self.world.map;
        let bounds = aa_navigation::endpoint::Bounds { cell_size_bits: m.cell_size.to_bits(), extent_bits: bits(Vec2::new(m.width as f32, m.height as f32) * m.cell_size) };
        let adjusted = self.with_space(|s, policy| {
            let mut q = space::query_for(s, policy, id, false)?;
            q.required_actor_flags |= 0x2000_0000;
            q.descriptor.ignore_actor = target;
            let mut w = uai::Endpoint { space: s, policy, query: q };
            aa_navigation::endpoint::adjust(position, &mut to, fallback, bounds, PC, &mut w).ok()
        });
        let Some(Some(ok)) = adjusted else { return Ok(0) };
        if !ok {
            let node = &mut p.plan.movement.path.nodes[0];
            node.words[5] = node.words[5].wrapping_add(1);
            return Ok(1);
        }
        let same = f32::from_bits(to[0]) == f32::from_bits(head.words[0]) && f32::from_bits(to[1]) == f32::from_bits(head.words[1]);
        if same {
            return Ok(0);
        }
        let (dx, dy) = (f64::from(f32::from_bits(to[0])) - f64::from(f32::from_bits(position[0])), f64::from(f32::from_bits(to[1])) - f64::from(f32::from_bits(position[1])));
        let length = (dx * dx + dy * dy).sqrt();
        let dir = if length > f64::from(EPSILON) { [(dx * (1.0 / length)) as f32, (dy * (1.0 / length)) as f32] } else { [0.0, 0.0] };
        let node = &mut p.plan.movement.path.nodes[0];
        node.words[0] = to[0];
        node.words[1] = to[1];
        node.words[2] = dir[0].to_bits();
        node.words[3] = dir[1].to_bits();
        Ok(2)
    }
}

/// [`Host::with_space`] for callers without a Mind in hand.
fn with_space_of<T>(ms: &mut MovementState, world: &World, f: impl FnOnce(&mut space::Space, &mut space::Policy<'_>) -> T) -> Option<T> {
    let mut s = ms.space.take()?;
    let r = {
        let mut policy = space::Policy { ms: &*ms, world };
        f(&mut s, &mut policy)
    };
    ms.space = Some(s);
    Some(r)
}

/// `0x5774A0`'s query: `0x554350(actor, 0)`, `0x552250(xy)`, `0x553BA0`.
pub(super) fn ground_hit(ms: &mut MovementState, world: &World, id: ActorId, xy: [u32; 2]) -> Option<[u32; 4]> {
    with_space_of(ms, world, |s, policy| {
        let mut q = space::query_for(s, policy, id, false)?;
        q.set_position(&s.index, xy, policy).ok()?;
        let hit = space::first_hit(s, policy, &q);
        hit.blocked().then(|| space::words(&hit))
    })
    .flatten()
}

/// `aa_mind::element_command::World` of one `0x538340`.
struct MoveCommand<'c, 'h, 'w> {
    stack: &'c mut Stack<'h, 'w>,
    a: &'c mut Arena,
}
impl element_command::World for MoveCommand<'_, '_, '_> {
    fn command(&self, id: u32) -> aa_mind::Result<element_command::Command> {
        let c = self.stack.host.world.books.minds.commands.get(&id).ok_or_else(|| err(format!("unknown command {id}")))?;
        Ok(element_command::Command { flags: c.flags, target: c.target, position_bits: c.point, auxiliary_bits: c.auxiliary, speed_bits: c.speed })
    }
    fn element(&self, _: MindId) -> aa_mind::Result<element_command::Element> {
        let h = &self.stack.host;
        let m = &self.stack.p.plan.movement;
        Ok(element_command::Element {
            actor: h.id,
            move_state: h.state(offsets::MOVE)?,
            sleep_state: h.state(offsets::SLEEP)?,
            kill_state: h.state(offsets::KILL)?,
            bombard_state: h.state(offsets::BOMBARD)?,
            move_destination_bits: m.destination_bits,
            move_target: m.target_id,
            update_pending: h.world.is_scheduled(Sched::Mind(h.id)),
        })
    }
    fn actor(&self, id: u32) -> aa_mind::Result<element_command::Actor> {
        let a = self.stack.host.world.actors.get(id as usize).ok_or_else(|| err("actor absent"))?;
        Ok(element_command::Actor { flags: u32::from(!a.alive || a.removed), position_bits: bits(a.pos.truncate()), performer: None })
    }
    fn resolve(&self, id: u32) -> aa_mind::Result<Option<u32>> {
        element_move::World::resolve(&*self.stack.host, id)
    }
    fn active(&self, state: StateId) -> aa_mind::Result<bool> {
        aa_mind::is_current(state, self.a)
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: u32) -> aa_mind::Result<()> {
        self.stack.host.schedule_mind(delay);
        Ok(())
    }
    fn indirect_fire(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn target_accepts(&mut self, _: u32, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn activate_sleep(&mut self, state: StateId, mode: u32) -> aa_mind::Result<bool> {
        aa_mind::activate(state, mode, self.a, self.stack)
    }
    fn activate_move(&mut self, state: StateId, request: aa_navigation::Activation) -> aa_mind::Result<bool> {
        let Stack { host, p } = &mut *self.stack;
        host.activate_move(state, request, p, self.a)
    }
    fn activate_kill(&mut self, _: StateId, _: u32, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn activate_bombard(&mut self, _: StateId, _: u32, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    /// `0x532930`: a move to a thing, mode 2 follows it.
    fn activate_follow(&mut self, state: StateId, mode: u32, target: u32, movement_mode: u32, speed: u32, auxiliary: [u32; 2]) -> aa_mind::Result<bool> {
        let Stack { host, p } = &mut *self.stack;
        host.activate_to_object(state, mode, target, movement_mode, speed, auxiliary, p, self.a)
    }
}

impl Host<'_> {
    /// `0x52DF60`: past the eligibility `0x529C00`, the state's words, then `0x52A810`.
    fn activate_move(&mut self, state: StateId, request: aa_navigation::Activation, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        if request.activation_mode > 1 && !aa_mind::World::can_activate(&mut Stack { host: self, p }, state, a)? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.target_id = 0;
        m.destination_bits = request.destination_bits;
        m.mode = request.movement_mode;
        m.requested_speed_bits = request.requested_speed_bits;
        m.auxiliary_bits = request.auxiliary_bits;
        m.phase = 0;
        aa_mind::activate(state, request.activation_mode, a, &mut Stack { host: self, p })
    }
    /// `0x532930`: as [`Host::activate_move`], to a thing (its GTID at +0x10).
    #[allow(clippy::too_many_arguments)]
    fn activate_to_object(&mut self, state: StateId, mode: u32, target: u32, movement_mode: u32, speed: u32, auxiliary: [u32; 2], p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        if mode > 1 && !aa_mind::World::can_activate(&mut Stack { host: self, p }, state, a)? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.target_id = target;
        m.destination_bits = bits(self.thing_position(target.wrapping_sub(1)));
        m.mode = movement_mode;
        m.requested_speed_bits = speed;
        m.auxiliary_bits = auxiliary;
        m.phase = 0;
        aa_mind::activate(state, mode, a, &mut Stack { host: self, p })
    }
    /// An actor's position, or an organization's (`ORG | o`).
    fn thing_position(&self, thing: u32) -> Vec2 {
        if thing & ORG != 0 {
            return self.world.orgs.get((thing & !ORG) as usize).map_or(Vec2::ZERO, |o| o.anchor);
        }
        self.world.actors.get(thing as usize).map_or(Vec2::ZERO, |a| a.pos.truncate())
    }
    fn am(&mut self) -> &mut ActorMove {
        &mut self.ms.actors[self.id as usize]
    }
    fn state(&self, offset: u32) -> aa_mind::Result<StateId> {
        self.world.books.minds.state(self.id, offset).ok_or_else(|| err(format!("Mind+{offset:X} state absent")))
    }
    fn offset_of(&self, state: StateId) -> u32 {
        self.world.books.minds.owner(state).map_or(u32::MAX, |(_, o)| o)
    }
    fn org(&self) -> Option<OrgId> {
        self.world.actors[self.id as usize].org.filter(|&o| self.world.orgs.get(o as usize).is_some_and(|x| x.alive))
    }
    /// Element +1C (`0x5705A0` sets it at the join, `0x5709D0` decides it after).
    fn attached(&self) -> bool {
        element_attached(self.world, self.id)
    }
    fn max(&mut self) -> f32 {
        max_speed(&mut self.ms.speeds, self.world, self.id)
    }
    fn position(&self) -> Vec2 {
        pos2(self.world, self.id)
    }
    /// `0x52DC10`: an active Mover whose queue entry is gone stops being active
    /// when the Mind waits (delay above 0, `0x4CFC80`), then `0x50AD80`.
    fn schedule_mind(&mut self, delay: u32) {
        let id = self.id;
        if f32::from_bits(delay) > 0.0 && self.am().mover.active && !self.world.is_scheduled(Sched::Mover(id)) {
            self.deactivate();
        }
        think_mind_in(self.world, id, f32::from_bits(delay));
    }
    /// `0x4CFC80`: the active Mover's leave `0x576B60(0)`, then with a Character
    /// its default stance `0x563BA0` (`character::mover_stopped_at`; `site` names
    /// the chain for the census).
    fn deactivate(&mut self) {
        self.deactivate_at(crate::trace::sites::CHARACTER_OTHER);
    }
    fn deactivate_at(&mut self, site: &'static str) {
        let id = self.id;
        if self.world.is_scheduled(Sched::Mover(id)) {
            self.world.unschedule(Sched::Mover(id));
        }
        let m = &mut self.am().mover;
        if m.active {
            m.active = false;
            m.turning = false;
            m.speed = 0.0;
            m.last_step = 0.0;
        }
        crate::character::mover_stopped_at(self.world, id, site);
    }
    /// `0x5767C0(0, 0)`, then `0x4CFC80` when the Mover is not active.
    fn stop(&mut self) {
        let id = self.id;
        let m = &mut self.am().mover;
        m.turning = false;
        m.last_step = 0.0;
        if self.world.is_scheduled(Sched::Mover(id)) {
            self.world.unschedule(Sched::Mover(id));
        }
        if !self.am().mover.active {
            self.deactivate();
        }
    }
    /// `0x52DF60(0, point, movement mode, 0, null)` on the embedded Move.
    fn activate_embedded_move(&mut self, to: Vec2, movement_mode: u32, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let m = &mut p.plan.movement;
        m.target_id = 0;
        m.destination_bits = bits(to);
        m.mode = movement_mode;
        m.requested_speed_bits = 0;
        m.auxiliary_bits = [0, 0];
        m.phase = 0;
        let mv = self.state(offsets::MOVE)?;
        aa_mind::activate(mv, 0, a, &mut Stack { host: self, p })?;
        Ok(())
    }
    fn is_current(&self, a: &Arena, offset: u32) -> aa_mind::Result<bool> {
        Ok(a.mind(MindId(self.id))?.stack.front().copied().flatten() == Some(self.state(offset)?))
    }
    /// `aa_navigation::ActorState` of ActionGo.
    fn action_actor(&mut self) -> ActorState {
        let max = self.max();
        let org = self.org();
        let pos = self.position();
        let m = &self.am().mover;
        ActorState {
            flags: 0,
            position_bits: bits(pos),
            mover_instant_turn: m.instant,
            mover_requested_speed_bits: m.requested.to_bits(),
            mover_speed_factor_bits: m.factor.to_bits(),
            mover_maximum_speed_bits: max.to_bits(),
            has_mover: true,
            // Element +1C (`0x5709D0`).
            organization_special: self.attached(),
            has_leader: org.is_some(),
        }
    }
    /// `0x5300A0` with the callbacks of [`GoHost`].
    fn action_go(&mut self, state: StateId, speed: u32, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        let mut actor = self.action_actor();
        let context = aa_navigation::ActionContext {
            precision: PC,
            group_speed_minimum_bits: self.rules.group_speed_minimum.to_bits(),
            catchup_speed_factor_bits: self.rules.catchup.to_bits(),
        };
        let mut go = GoHost { host: self, after: After::None };
        let go_result = aa_navigation::try_action_go(&mut p.plan.movement, &mut actor, speed, context, &mut go).map_err(err);
        let after = go.after;
        let m = &mut self.am().mover;
        m.instant = actor.mover_instant_turn;
        let result = go_result?;
        match after {
            After::NodeReached => {
                node_reached::apply(state, p, a, PC, self)?;
            }
            After::Clear => self.clear_for_regiment(p, a)?,
            _ => {}
        }
        Ok(result)
    }
    /// `0x52A3C0` from the formation gate: back to Sleep, until the regiment's next
    /// dispatch.
    fn clear_for_regiment(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let id = self.id;
        aa_mind::clear(MindId(id), a, &mut Stack { host: self, p })
    }
    /// `0x5709D0`; a member that joined (`0x578ED0` -> `0x522840`) is listed.
    fn refresh_membership(&mut self) {
        let id = self.id;
        if let Some((before, after)) = crate::fog::element_refresh(self.world, id)
            && after
            && !before
            && let Some(o) = self.org()
        {
            regiment::list_member(self.ms, o, id);
        }
    }
    /// The order of an actor outside any organization ends with its state.
    fn lone_order_over(&mut self) {
        let id = self.id;
        if self.world.actors[id as usize].org.is_none() && self.am().order.active {
            self.am().order.active = false;
            if self.world.actors[id as usize].order != Order::Idle {
                set_order(self.world, self.ms, Owner::Actor(id), Order::Idle);
            }
        }
    }
    /// Mind +0x124 `0x537750` for the hosted states.
    fn failed(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        if self.is_current(a, offsets::DEPLOY)? && !self.deploy_arrived(p, a)? {
            return Ok(true);
        }
        let mind = MindId(self.id);
        let id = self.id;
        // The Unpack state cannot reach its point: tai_error_unpack, the order ends.
        if self.is_current(a, offsets::UNPACK)? {
            let actor = &mut self.world.actors[id as usize];
            actor.order = Order::Idle;
            let (org, kingdom) = (actor.org, actor.kingdom);
            if let Some(o) = org {
                self.world.orgs[o as usize].order = Order::Idle;
            }
            if let Some(kingdom) = kingdom {
                self.world.events.push(crate::world::Event::Refused { kingdom, reason: "the deploy spot cannot be reached".into() });
            }
        }
        aa_mind::pop(mind, a, &mut Stack { host: self, p })?;
        self.lone_order_over();
        Ok(true)
    }
    /// `0x52F7A0` EventReachedDestination: the actor's shape must be clear where
    /// it stands (`0x553BA0`, static blockers only), else it redeploys to a spot
    /// `0x554B30` finds within 8 m; true when it may go on. More than 5 tries, or
    /// no spot, and it dies (`0x4D7960`).
    fn deploy_arrived(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        let id = self.id;
        let at = self.position();
        // 0x554350(actor, 0), 0x552210(1, water), 0x553BA0.
        let clear = with_space_of(self.ms, self.world, |s, policy| {
            let mut q = space::query_for(s, policy, id, false)?;
            q.required_actor_flags |= 0x400 | 0x2000_0000;
            q.descriptor.mask |= 0x500;
            Some(!space::first_hit(s, policy, &q).blocked())
        })
        .flatten();
        if clear.unwrap_or_else(|| spot_free(self.ms, self.world, id, at, false)) {
            return Ok(true);
        }
        let tries = self.am().walk.map_or(0, |w| w.tries) + 1;
        self.am().walk = Some(MemberWalk { tries });
        let next = if tries > REDEPLOY_TRIES { None } else { open_position_near(self.ms, self.world, id, at, REDEPLOY_SEARCH_RADIUS, false) };
        let Some(q) = next else {
            self.died = true;
            return Ok(false);
        };
        let mv = self.state(offsets::MOVE)?;
        if a.state(mv)?.status != 0 {
            aa_mind::World::leave(&mut Stack { host: self, p }, mv, a)?;
        }
        self.activate_embedded_move(q, 1, p, a)?;
        self.schedule_mind(IMMEDIATE_BITS);
        Ok(false)
    }
    /// Mind +0x144 `0x5324A0` for the hosted states.
    fn completed(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        self.deactivate();
        let mind = MindId(self.id);
        if self.is_current(a, offsets::DEPLOY)? {
            if !self.deploy_arrived(p, a)? {
                return Ok(true);
            }
            if self.org().is_some() {
                let reattach = self.state(offsets::REATTACH)?;
                aa_mind::activate(reattach, 1, a, &mut Stack { host: self, p })?;
                return Ok(true);
            }
            aa_mind::pop(mind, a, &mut Stack { host: self, p })?;
            return Ok(true);
        }
        if self.is_current(a, offsets::UNPACK)? {
            // The embedded Move leaves, +0xB8 (0x52F350: the view's state),
            // and the Unpack Think runs in the next update.
            let mv = self.state(offsets::MOVE)?;
            if a.state(mv)?.status != 0 {
                aa_mind::World::leave(&mut Stack { host: self, p }, mv, a)?;
            }
            self.schedule_mind(IMMEDIATE_BITS);
            return Ok(false);
        }
        if self.is_current(a, offsets::REATTACH)? {
            // 0x532716: the state ends, 0x5709D0, the next update.
            aa_mind::pop(mind, a, &mut Stack { host: self, p })?;
            self.refresh_membership();
            self.schedule_mind(IMMEDIATE_BITS);
            return Ok(true);
        }
        if self.is_current(a, offsets::MOVE)? {
            // 0x52DA60 (+0x148, the target is not a live other actor): a member
            // takes its organization's facing (0x4EEBE0 at +0x2C), then the state
            // ends and Sleep thinks in 1 s.
            if let Some(o) = self.org() {
                let f = self.ms.orgs[o as usize].facing;
                if f != Vec2::ZERO {
                    self.am().mover.facing = f;
                    self.am().mover.desired = f;
                    let id = self.id;
                    write_facing(self.ms, self.world, id);
                }
            }
            aa_mind::pop(mind, a, &mut Stack { host: self, p })?;
            self.schedule_mind(ONE_BITS);
            self.lone_order_over();
            return Ok(true);
        }
        self.schedule_mind(IMMEDIATE_BITS);
        Ok(false)
    }
    /// `0x52F160` EventReattach (Mind +0xFC): with its Reattach current, a wake in
    /// 0.6 s when none is due; else that state in mode 2.
    fn event_reattach(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        if !alive(self.world, self.id) {
            return Ok(false);
        }
        if self.is_current(a, offsets::REATTACH)? {
            if !self.world.is_scheduled(Sched::Mind(self.id)) {
                self.schedule_mind(0x3f19_999a);
            }
            return Ok(true);
        }
        if self.org().is_none() {
            return Ok(false);
        }
        let reattach = self.state(offsets::REATTACH)?;
        aa_mind::activate(reattach, 2, a, &mut Stack { host: self, p })
    }
    #[allow(dead_code)]
    fn collision_rendition_unused(&mut self, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let id = self.id;
        let now = self.world.time;
        let pos = self.position();
        let path = &mut p.plan.movement.path;
        let Some(head) = path.nodes.first().copied() else {
            self.stop();
            self.schedule_mind(ONE_BITS);
            return Ok(());
        };
        let head_pos = vec2([head.words[0], head.words[1]]);
        let radius = self.ms.blocking_radius.get(self.world.actors[id as usize].def as usize).copied().unwrap_or(0.0);
        if p.collision[0] == 2 {
            let other = p.collision[1].wrapping_sub(1);
            if !alive(self.world, other) {
                self.action_go(self.state(offsets::MOVE)?, 0, p, a)?;
                return Ok(());
            }
            let q = pos2(self.world, other);
            let r = self.ms.blocking_radius.get(self.world.actors[other as usize].def as usize).copied().unwrap_or(0.0);
            if is_moving(self.ms, other) {
                let own = &self.ms.actors[id as usize].mover;
                let them = &self.ms.actors[other as usize].mover;
                let heading = own.facing.dot(them.facing);
                let (own_speed, other_speed) = (own.speed, them.speed);
                if heading > 0.0 {
                    if own_speed <= other_speed * heading {
                        self.action_go(self.state(offsets::MOVE)?, 0, p, a)?;
                        return Ok(());
                    }
                    if own_speed - QUEUE_SPEED_MARGIN <= other_speed {
                        self.stop();
                        self.schedule_mind(QUEUE_WAIT.to_bits());
                        return Ok(());
                    }
                }
            }
            self.am().ignore.push((other, now + IGNORE_SECONDS));
            let dir = self.am().mover.facing;
            let left = Vec2::new(-dir.y, dir.x);
            let side = if (head_pos - q).dot(left) >= 0.0 { 1.0 } else { -1.0 };
            let reach = radius + r;
            let lateral = left * side * reach;
            let aside = pos + lateral;
            let past = q + dir * reach + lateral;
            let (d1, d2, d3) = (normalize(aside - pos), normalize(past - aside), normalize(head_pos - past));
            if d1 != Vec2::ZERO && d2 != Vec2::ZERO {
                let path = &mut p.plan.movement.path;
                if d3 != Vec2::ZERO {
                    path.nodes.insert(0, node_words(head_pos, d3, 0));
                }
                path.nodes.insert(0, node_words(past, d2, 0));
                path.nodes.insert(0, node_words(aside, d1, 0));
                path.dirty = true;
            }
        } else {
            let def = def_of(self.world, id);
            let class = mover_class(def);
            let grid = self.ms.occupancy.grid(&self.world.map, class);
            if grid.blocked_at(head_pos) {
                match nearest_open(&grid, head_pos, NODE_SHIFT_CELLS) {
                    Some(q) => {
                        let bits = bits(q);
                        let node = &mut p.plan.movement.path.nodes[0];
                        node.words[0] = bits[0];
                        node.words[1] = bits[1];
                    }
                    None if p.plan.movement.path.nodes.len() > 1 => {
                        node_reached::remove_head(p)?;
                    }
                    None => {}
                }
            } else {
                let begin = f32::from_bits(self.world.clock.begin.0);
                let last = self.am().last_reroute;
                if last > 0.0 && now - last < self.rules.low_path_wait {
                    self.stop();
                    self.schedule_mind((last + self.rules.low_path_wait - begin).to_bits());
                    return Ok(());
                }
                // Three re-routes that end in another block fail the move, as the
                // native's ResultPlan attempts do (`0x5336B0`, +0x28).
                p.plan.attempts += 1;
                if p.plan.attempts > 3 {
                    return self.failed(p, a).map(|_| ());
                }
                let request = Request { from: pos, to: head_pos, class, max_speed: self.max() };
                let rules = self.rules.path;
                let map = self.world.map.clone();
                let route = self.ms.solve(&map, &rules, &request);
                self.am().last_reroute = now.max(EPSILON);
                if route.status == Status::Failed || route.waypoints.is_empty() {
                    node_reached::remove_head(p)?;
                } else {
                    if (route.goal - head_pos).length_squared() > EPSILON && p.plan.movement.path.nodes.len() > 1 {
                        node_reached::remove_head(p)?;
                    }
                    let path = &mut p.plan.movement.path;
                    for w in route.waypoints.iter().rev() {
                        path.nodes.insert(0, node_words(w.pos, w.dir, 0));
                    }
                    path.dirty = true;
                }
            }
        }
        if p.plan.movement.path.nodes.is_empty() {
            return self.failed(p, a).map(|_| ());
        }
        let radius = turn_radius(def_of(self.world, id)).to_bits();
        cont::waypoint_offset(p, radius, PC, true)?;
        p.plan.movement.phase = p.plan.movement.phase.min(1);
        let speed = p.plan.movement.requested_speed_bits;
        self.action_go(self.state(offsets::MOVE)?, speed, p, a)?;
        Ok(())
    }
}

/// The callbacks of `0x5300A0`.
struct GoHost<'g, 'w> {
    host: &'g mut Host<'w>,
    after: After,
}
impl aa_navigation::TryActionWorld for GoHost<'_, '_> {
    fn node_reached(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        // The port returns right after this call.
        self.after = After::NodeReached;
        Ok(())
    }
    /// `0x52E6A0`: the point lies inside one of the two turning circles.
    fn single_node_placement(&mut self, waypoint: [u32; 2], _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        let radius = f64::from(turn_radius(def_of(self.host.world, self.host.id)));
        let pos = self.host.position();
        let f = self.host.am().mover.facing;
        let (fx, fy) = (radius * f64::from(f.x), radius * f64::from(f.y));
        let (wx, wy) = (f64::from(f32::from_bits(waypoint[0])), f64::from(f32::from_bits(waypoint[1])));
        let (px, py) = (f64::from(pos.x), f64::from(pos.y));
        let reach = radius * radius + f64::from(EPSILON);
        let (a, b) = (wy - (py - fx), wx - (px + fy));
        if a * a + b * b < reach {
            return Ok(true);
        }
        let (a, b) = (wy - (fx + py), wx - (px - fy));
        Ok(a * a + b * b < reach)
    }
    fn actor_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        Ok(self.host.am().mover.active)
    }
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> aa_navigation::Result<bool> {
        let element = follow_element(self.host, state);
        let catchup = self.host.rules.catchup.to_bits();
        let ready = follow::formation_speed(state, actor, element.as_ref(), catchup, PC, self)?;
        let m = &mut self.host.am().mover;
        m.requested = f32::from_bits(actor.mover_requested_speed_bits);
        m.factor = f32::from_bits(actor.mover_speed_factor_bits);
        Ok(ready)
    }
    fn actor_or_leader_handle_valid(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        // Actor +0x164 (the group, census X2).
        Ok(false)
    }
    /// `0x578E90`: neither the regiment's Mind +64 nor +7C.
    /// Mind +0xA4 `0x52D780`: its Reattach (+0x178) is current (+0x184 is 2).
    fn organization_ready(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        Ok(self.host.am().reattaching)
    }
    fn leader_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        Ok(leader_moving(self.host))
    }
    fn stop_action(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        self.host.stop();
        Ok(())
    }
    /// `0x5773C0`: the Mover becomes the active component (`0x4CFC10`: its
    /// leave and enter each cancel its queue entry; the same component keeps
    /// its speed), then it registers on queue 0, one getter and one draw.
    fn begin_mover(&mut self, request: MoverRequest, _: &mut MoveState, actor: &mut ActorState) -> aa_navigation::Result<bool> {
        let id = self.host.id;
        let threshold = f64::from(self.host.rules.reached_threshold);
        let pos = self.host.position();
        let m = &mut self.host.ms.actors[id as usize].mover;
        m.instant = actor.mover_instant_turn;
        m.turning = false;
        m.last_step = 0.0;
        m.active = true;
        m.dest = vec2(request.destination_bits);
        m.desired = vec2(request.direction_bits);
        let (dx, dy) = (f64::from(m.dest.x) - f64::from(pos.x), f64::from(m.dest.y) - f64::from(pos.y));
        if dx * dx + dy * dy < threshold * threshold {
            m.speed = 0.0;
        } else {
            m.factor = f32::from_bits(request.speed_factor_bits);
            m.requested = f32::from_bits(request.requested_speed_bits);
        }
        actor.mover_requested_speed_bits = m.requested.to_bits();
        actor.mover_speed_factor_bits = m.factor.to_bits();
        if self.host.world.is_scheduled(Sched::Mover(id)) {
            self.host.world.unschedule(Sched::Mover(id));
        }
        crate::character::mover_started(self.host.world, id);
        self.host.world.schedule_periodic_at(ORDER_SITE, crate::scheduler::EVERY_TICK, Sched::Mover(id), 0);
        Ok(true)
    }
    fn next_world_random_u32(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<u32> {
        let site = self.host.go_site;
        Ok(self.host.world.random_u32_at(site))
    }
    fn schedule(&mut self, delay: u32, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        self.host.schedule_mind(delay);
        Ok(())
    }
}
impl follow::FollowWorld for GoHost<'_, '_> {
    fn leader_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<bool> {
        Ok(leader_moving(self.host))
    }
    fn stop_action(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        self.host.stop();
        Ok(())
    }
    fn schedule(&mut self, delay: u32, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        self.host.schedule_mind(delay);
        Ok(())
    }
    fn finish_state(&mut self, _: &mut MoveState, _: &mut ActorState) -> aa_navigation::Result<()> {
        // `0x52A3C0`: the port returns false and ActionGo returns at once.
        self.after = After::Clear;
        Ok(())
    }
}

/// Element +1C of `id`: as the zones engine holds it; set while there is none.
pub(super) fn element_attached(world: &World, id: ActorId) -> bool {
    world.fog.engine.as_ref().and_then(|e| e.attached(id)).unwrap_or(true)
}

fn leader_moving(host: &Host<'_>) -> bool {
    host.org().and_then(|o| host.ms.orgs[o as usize].leader).is_some_and(|l| host.ms.actors.get(l as usize).is_some_and(|a| a.mover.active))
}

/// The regiment path as the port's collection (Regiment Mind+9C).
fn regiment_path(om: &OrgMove) -> OwnedPath {
    regiment::owned_path(&om.order)
}

/// actor+0x80 as `aa_navigation::formation_follow` reads it.
fn follow_element(host: &Host<'_>, state: &MoveState) -> Option<follow::FollowElement> {
    let o = host.org()?;
    let world = &*host.world;
    let om = &host.ms.orgs[o as usize];
    let leader = om.leader.filter(|&l| alive(world, l)).map(|l| follow::FollowLeader {
        position_bits: bits(pos2(world, l)),
        path: if l == host.id { state.path.clone() } else { host.ms.actors[l as usize].payload.as_ref().map_or_else(|| state.path.clone(), |p| p.plan.movement.path.clone()) },
    });
    let mut slots: Vec<[u32; 2]> = om.slots.iter().map(|&s| bits(s)).collect();
    let slot = member_slot(world, o, host.id);
    if slots.len() <= slot {
        slots.resize(slot + 1, [0, 0]);
    }
    Some(follow::FollowElement {
        slot: slot as i32,
        organization: Some(follow::FollowOrganization {
            position_bits: bits(world.orgs[o as usize].anchor),
            facing_bits: bits(om.facing),
            path: regiment_path(om),
            slot_positions_bits: slots,
            leader,
        }),
    })
}

impl element_move::World for Host<'_> {
    fn actor(&self, actor: u32) -> aa_mind::Result<element_move::Actor> {
        if actor & ORG != 0 {
            let o = actor & !ORG;
            let org = self.world.orgs.get(o as usize).ok_or_else(|| err("organization absent"))?;
            return Ok(element_move::Actor { flags: 0, position: bits(org.anchor), mover: None, mind: None, component74: None, organization: Some(actor), element: None, component88: None, component_d0: false, kingdom_slots: Vec::new() });
        }
        let a = self.world.actors.get(actor as usize).ok_or_else(|| err("actor absent"))?;
        let flags = u32::from(!a.alive || a.removed);
        Ok(element_move::Actor {
            flags,
            position: bits(a.pos.truncate()),
            mover: Some(actor),
            mind: Some(MindId(actor)),
            component74: None,
            organization: None,
            element: a.org.map(|_| actor),
            component88: None,
            component_d0: false,
            kingdom_slots: Vec::new(),
        })
    }
    fn element(&self, element: u32) -> aa_mind::Result<element_move::Element> {
        let org = self.world.actors.get(element as usize).and_then(|a| a.org).filter(|&o| self.world.orgs.get(o as usize).is_some_and(|x| x.alive));
        Ok(element_move::Element { organization_actor: org.map(|o| ORG | o), special: element_attached(self.world, element) })
    }
    fn mover(&self, mover: u32) -> aa_mind::Result<element_move::Mover> {
        let m = &self.ms.actors[mover as usize].mover;
        let mut speeds = SpeedCache::default();
        Ok(element_move::Mover {
            last_step_speed: m.last_step.to_bits(),
            speed: m.speed.to_bits(),
            maximum_speed: max_speed(&mut speeds, self.world, mover).to_bits(),
            locked: false,
            turning: m.turning,
            falling: false,
        })
    }
    fn resolve(&self, id: u32) -> aa_mind::Result<Option<u32>> {
        if id == 0 {
            return Ok(None);
        }
        let thing = id - 1;
        if thing & ORG != 0 {
            return Ok(self.world.orgs.get((thing & !ORG) as usize).filter(|o| o.alive).map(|_| thing));
        }
        Ok(Some(thing).filter(|&a| alive(self.world, a)))
    }
    fn world_time_bits(&self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    fn world_extents_bits(&self) -> aa_mind::Result<[u32; 2]> {
        let m = &self.world.map;
        Ok(bits(Vec2::new(m.width as f32, m.height as f32) * m.cell_size))
    }
    fn ensure_detached_setting(&mut self) -> aa_mind::Result<()> {
        Ok(())
    }
    fn detached_seconds_bits(&self) -> aa_mind::Result<u32> {
        Ok(self.world.data.rule("MoverDetachedSeconds").unwrap_or(5.0).to_bits())
    }
    fn diagnostic_already_emitted(&self) -> bool {
        true
    }
    fn emit_status_diagnostic(&mut self, _: StateId, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn set_diagnostic_emitted(&mut self) {}
    fn organization_ready(&mut self, organization: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(!org_active(self.ms, self.world, organization & !ORG))
    }
    fn destination_has_kingdom_object(&mut self, _: [u32; 2], _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn component88_blocks(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn actor_can_move(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn mind_event(&mut self, _: MindId, event: element_move::MindEvent, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        use element_move::MindEvent as E;
        match event {
            // 0x52F160: a detached element reattaches.
            E::DetachedFc => self.event_reattach(p, a),
            E::Failed124 => self.failed(p, a),
            // 0x52D9C0, and +0xF4: the state ends.
            E::TargetLost140(_) | E::TargetReachedF4(_) | E::Completed144 => self.completed(p, a),
        }
    }
    fn update_target(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        element_move::World::resolve(self, p.plan.movement.target_id)
    }
    fn radius(&mut self, actor: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<Binary> {
        let def = match actor & ORG {
            0 => self.world.actors.get(actor as usize).map(|a| a.def),
            _ => self.world.orgs.get((actor & !ORG) as usize).map(|o| o.def),
        };
        let r = def.and_then(|d| self.ms.blocking_radius.get(d as usize)).copied().unwrap_or(0.0);
        Binary::from_f32_bits(r.to_bits()).map_err(err)
    }
    fn property(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn accepts_target_property(&mut self, _: u32, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn soft_shapes_in_range(&mut self, _: u32, _: u32, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn mind_target(&mut self, _: MindId, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn mind_has_location(&mut self, _: MindId, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn mind_location(&mut self, _: MindId, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn target_point(&mut self, _: u32, _: u32, _: &mut [u32; 2], _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn component74_target(&mut self, _: u32, _: u32, _: Option<u32>, _: [u32; 2], _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn clear_component74(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn region_at(&self, point: [u32; 2]) -> aa_mind::Result<u32> {
        Ok(self.ms.regions.as_ref().map_or(u32::MAX, |r| r.region_at(point) as u32))
    }
    fn region_neighbors(&self, region: u32) -> aa_mind::Result<Vec<u32>> {
        Ok(self.ms.regions.as_ref().map_or_else(Vec::new, |r| r.neighbors(region).to_vec()))
    }
    /// `0x5336B0` with the callbacks of [`PlanHost`], and what they left for
    /// the payload.
    fn result_plan(&mut self, state: StateId, trigger: Option<u32>, argument: bool, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let f = self.am().mover.facing;
        let radius = turn_radius(def_of(self.world, self.id));
        let mut actor = result_plan::Actor { object: result_plan::Object(self.id), position_bits: bits(self.position()), facing_bits: bits(f), turn_radius_bits: radius.to_bits() };
        let mut plan = PlanHost { host: self, a, after: After::None };
        let result = result_plan::result_plan(&mut p.plan, &mut actor, trigger.map(result_plan::Object), u32::from(argument), PC, &mut plan);
        let after = plan.after;
        result.map_err(err)?;
        match after {
            After::Failed => {
                self.failed(p, a)?;
            }
            After::ContinueTurn(turn) => self.continue_turn(state, turn, p, a)?,
            After::ContinueCurrent => {
                // 0x531520.
                let world_time_bits = self.world.clock.begin.0;
                let wait_minimum_bits = self.rules.low_path_wait.to_bits();
                self.uai(p, a, |env, p, nav| {
                    let input = aa_navigation::low_path::ContinuationInput { argument: u32::from(argument), world_time_bits, wait_minimum_bits };
                    aa_navigation::low_path::continue_current(input, &mut p.plan.movement, &mut p.word78, env.solver, env.environment, nav).map(|_| ()).map_err(err)
                })?;
            }
            After::NodeReached => {
                node_reached::apply(state, p, a, PC, self)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn continue_path(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        cont::continue_path(state, p, a, PC, self)
    }
    fn stop_mover(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        let id = self.id;
        let m = &mut self.am().mover;
        m.turning = false;
        m.last_step = 0.0;
        if self.world.is_scheduled(Sched::Mover(id)) {
            self.world.unschedule(Sched::Mover(id));
        }
        Ok(())
    }
    fn mover_active(&self, mover: u32) -> aa_mind::Result<bool> {
        Ok(self.ms.actors[mover as usize].mover.active)
    }
    fn stop_character(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.deactivate();
        Ok(())
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.schedule_mind(delay);
        Ok(())
    }
    fn write_requested_speed(&mut self, mover: u32, value: u32) -> aa_mind::Result<()> {
        self.ms.actors[mover as usize].mover.requested = f32::from_bits(value);
        Ok(())
    }
    fn write_speed_factor(&mut self, mover: u32, value: u32) -> aa_mind::Result<()> {
        self.ms.actors[mover as usize].mover.factor = f32::from_bits(value);
        Ok(())
    }
}

impl Host<'_> {
    /// `0x5313F0`: a single node at the destination, along the line to it (the
    /// actor's facing when it stands there), kind 1 near the goal, kind 0 inside
    /// the turning circle; ActionGo(0).
    fn continue_turn(&mut self, state: StateId, turn: bool, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        element_move::clear_path(p);
        let to = vec2(p.plan.movement.destination_bits);
        let delta = to - self.position();
        let length = (f64::from(delta.x) * f64::from(delta.x) + f64::from(delta.y) * f64::from(delta.y)).sqrt();
        let dir = if length <= f64::from(EPSILON) {
            self.am().mover.facing
        } else {
            Vec2::new((f64::from(delta.x) * (1.0 / length)) as f32, (f64::from(delta.y) * (1.0 / length)) as f32)
        };
        let kind = u32::from(!turn);
        let path = &mut p.plan.movement.path;
        path.nodes.push(PathNode { words: [to.x.to_bits(), to.y.to_bits(), dir.x.to_bits(), dir.y.to_bits(), kind, 0, 0, 0] });
        if kind == 1 {
            path.kind1_count = path.kind1_count.wrapping_add(1);
        }
        path.dirty = true;
        self.action_go(state, 0, p, a)?;
        Ok(())
    }
}

/// The callbacks of `0x5336B0`.
struct PlanHost<'g, 'w> {
    host: &'g mut Host<'w>,
    a: &'g mut Arena,
    after: After,
}
fn plan_err(what: impl std::fmt::Display) -> result_plan::Error {
    result_plan::Error::World(what.to_string())
}
impl result_plan::World for PlanHost<'_, '_> {
    fn special_type(&mut self, _: result_plan::Object, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<bool> {
        Ok(false)
    }
    fn has_special_component(&self, _: result_plan::Object) -> bool {
        false
    }
    fn region_at(&mut self, point: [u32; 2], _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<i32> {
        if self.host.ms.regions.is_none() {
            self.host.ms.regions = route::Regions::of(self.host.world);
        }
        Ok(self.host.ms.regions.as_ref().map_or(-1, |r| r.region_at(point)))
    }
    /// `0x554350(actor, false)`.
    fn collision_context(&mut self, object: result_plan::Object, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<result_plan::CollisionContext> {
        let query = self.host.with_space(|s, policy| space::query_for(s, policy, object.0, false)).flatten();
        Ok(query.as_ref().map_or(result_plan::CollisionContext { words: [0; 22] }, uai::context_from_query))
    }
    fn resolve_actor(&self, id: u32) -> Option<result_plan::Object> {
        (id != 0 && (id - 1) & ORG == 0 && alive(self.host.world, id - 1)).then_some(result_plan::Object(id - 1))
    }
    fn replacement_actor(&self, object: result_plan::Object) -> result_plan::Object {
        object
    }
    fn object_definition(&self, _: result_plan::Object) -> u32 {
        0
    }
    /// `0x553CE0`.
    fn sweep(&mut self, context: &mut result_plan::CollisionContext, goal: [u32; 2], _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<result_plan::CollisionHit> {
        let mut query = uai::query_from_context(context);
        let hit = self.host.with_space(|s, policy| space::sweep_to(s, policy, &mut query, goal)).unwrap_or_default();
        Ok(result_plan::CollisionHit { kind: hit.kind, actor_id: hit.reference })
    }
    fn continue_turn(&mut self, turn: bool, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<()> {
        self.after = After::ContinueTurn(turn);
        Ok(())
    }
    fn continue_current(&mut self, _: u32, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<()> {
        self.after = After::ContinueCurrent;
        Ok(())
    }
    fn property(&mut self, _: result_plan::Object, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<u32> {
        Ok(0)
    }
    fn property_active(&mut self, _: u32, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<bool> {
        Ok(false)
    }
    /// `0x53E1F0` on the tile solver (`crate::path`, the region pre-pass
    /// `0x53DC00` not hosted: collection +18 stays -1).
    fn solve(&mut self, context: &mut result_plan::CollisionContext, request: aa_navigation::solver::Request, state: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<i32> {
        let host = &mut *self.host;
        let id = host.id;
        let def = def_of(host.world, id);
        let class = mover_class(def);
        let max = host.max();
        // `0x53E1F0`, on the map's regions and tile lists ([`route`]).
        let penalty = host.rules.path.terrain_change_penalty;
        let query = uai::query_from_context(context);
        if let Some(status) = route::solve(host.ms, host.world, query, request.start_bits, request.goal_bits, request.option_word, max, penalty, &mut state.movement.path) {
            return Ok(status);
        }
        let walking_out = host.am().walk.is_some();
        let route_request = Request { from: vec2(request.start_bits), to: vec2(request.goal_bits), class, max_speed: max };
        let rules = host.rules.path;
        let map = host.world.map.clone();
        let route = if walking_out { host.ms.solve_walking(&map, &rules, &route_request) } else { host.ms.solve(&map, &rules, &route_request) };
        if route.status == Status::Failed || route.waypoints.is_empty() {
            return Ok(0);
        }
        let path = &mut state.movement.path;
        path.clear_nodes();
        path.kind1_count = 0;
        path.kind2_count = 0;
        for w in &route.waypoints {
            path.nodes.push(node_words(w.pos, w.dir, 1));
            path.kind1_count += 1;
        }
        path.word18 = NO_REGION;
        path.dirty = true;
        Ok(1)
    }
    fn update_waypoint_offset(&mut self, state: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<()> {
        let radius = turn_radius(def_of(self.host.world, self.host.id)).to_bits();
        let mut whole = Payload { plan: state.clone(), collision: [0; 4], collision_vector: [0, 0], detached_time_bits: 0, word78: 0 };
        cont::waypoint_offset(&mut whole, radius, PC, true).map_err(plan_err)?;
        state.movement.waypoint_offset_bits = whole.plan.movement.waypoint_offset_bits;
        Ok(())
    }
    fn action_go(&mut self, speed: u32, state: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<()> {
        let mut actor = self.host.action_actor();
        let context = aa_navigation::ActionContext {
            precision: PC,
            group_speed_minimum_bits: self.host.rules.group_speed_minimum.to_bits(),
            catchup_speed_factor_bits: self.host.rules.catchup.to_bits(),
        };
        self.host.go_site = GO_SITE_PLAN;
        let mut go = GoHost { host: &mut *self.host, after: After::None };
        let result = aa_navigation::try_action_go(&mut state.movement, &mut actor, speed, context, &mut go);
        let after = go.after;
        self.host.am().mover.instant = actor.mover_instant_turn;
        result.map_err(plan_err)?;
        let _ = &self.a;
        if after != After::None {
            self.after = after;
        }
        Ok(())
    }
    fn failed(&mut self, _: &mut result_plan::State, _: &mut result_plan::Actor) -> result_plan::Result<()> {
        self.after = After::Failed;
        Ok(())
    }
}

impl cont::World for Host<'_> {
    fn action_go_site(&mut self, site: u32) {
        self.go_site = match site {
            0x537251 | 0x5372a3 => GO_SITE_FORWARD,
            0x5330da => GO_SITE_BACK,
            0x537005 => GO_SITE_RESULT,
            _ => GO_SITE,
        };
    }
    fn actor(&self, actor: u32) -> aa_mind::Result<cont::Actor> {
        let a = self.world.actors.get(actor as usize).ok_or_else(|| err("actor absent"))?;
        let am = &self.ms.actors[actor as usize];
        Ok(cont::Actor {
            // Thing +8: 0x537150 reads its bit 18, ethereal while the Deploy is current.
            flags: space::thing_flags(self.ms, self.world, actor),
            position: bits(a.pos.truncate()),
            facing: bits(am.mover.facing),
            mover: Some(cont::Mover { turn_radius: turn_radius(self.world.data.def(a.def)).to_bits(), requested_direction: bits(am.mover.desired) }),
        })
    }
    fn resolve(&self, id: u32) -> aa_mind::Result<Option<u32>> {
        element_move::World::resolve(self, id)
    }
    fn world_step_bits(&self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.step.0)
    }
    fn precision_exception_pending(&self) -> bool {
        true
    }
    fn actor_is_moving(&mut self, actor: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(self.ms.actors[actor as usize].mover.active)
    }
    fn action_go(&mut self, state: StateId, speed: u32, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        let r = Host::action_go(self, state, speed, p, a);
        self.go_site = GO_SITE;
        r
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.schedule_mind(delay);
        Ok(())
    }
    fn get_back_to_path(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        cont::get_back_to_path(state, p, a, PC, self)
    }
    /// Actor virtual +0x18: the Mover's current speed.
    fn movement_radius(&mut self, actor: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<Binary> {
        Binary::from_f32_bits(self.ms.actors[actor as usize].mover.speed.to_bits()).map_err(err)
    }
    /// `0x531170`.
    fn forward_probe(&mut self, _: StateId, direction: [u32; 2], distance: u32, result: &mut [u32; 4], p: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        *result = self.probe(None, direction, distance, p.plan.movement.target_id);
        Ok(())
    }
    fn node_reached(&mut self, _: MindId, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let state = self.state(offsets::MOVE)?;
        node_reached::apply(state, p, a, PC, self)?;
        Ok(())
    }
    /// `0x536840`: the engage and kill decision through `0x5358A0` belongs to the
    /// combat states, not hosted here: no thing is taken as a target.
    fn avoid_actor(&mut self, _: StateId, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn refresh_visibility(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn overlaps_actor(&mut self, actor: u32, target: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        let r = |x: u32| self.ms.blocking_radius.get(self.world.actors[x as usize].def as usize).copied().unwrap_or(0.0);
        let reach = r(actor) + r(target);
        Ok((pos2(self.world, actor) - pos2(self.world, target)).length_squared() < reach * reach)
    }
    fn overlap_diagnostic(&mut self, _: u32, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn bounds_radius(&mut self, actor: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<Binary> {
        let r = self.ms.blocking_radius.get(self.world.actors[actor as usize].def as usize).copied().unwrap_or(0.0);
        Binary::from_f32_bits(r.to_bits()).map_err(err)
    }
    /// `0x536AD0`.
    fn stop_distance(&mut self, _: StateId, target: u32, distance: u32, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        self.go_site = GO_SITE_WAIT;
        self.uai(p, a, |env, p, nav| cont::stop_distance(space::actor_key(target), distance, p, env, nav).map(|_| ()))
    }
    /// `0x530F40(true)`.
    fn result_path(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> aa_mind::Result<u32> {
        self.result_path_530f40(p)
    }
    fn wait(&mut self, _: StateId, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.schedule_mind(ONE_BITS);
        Ok(())
    }
    /// `0x534A40`.
    fn collision_plan(&mut self, _: StateId, obstacle: cont::Obstacle, direction: [u32; 2], p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        let obstacle = match obstacle {
            cont::Obstacle::Actor(id) => cont::Obstacle::Actor(space::actor_key(id)),
            tile => tile,
        };
        self.go_site = GO_SITE_AVOID;
        self.uai(p, a, |env, p, nav| cont::collision_plan(obstacle, direction, p, env, nav).map(|_| ()))
    }
}
impl cont::TurnRecoveryWorld for Host<'_> {
    fn turn_recovery(&mut self, state: StateId, input: cont::TurnRecovery, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<bool> {
        cont::recover_turn(state, input, p, a, PC, self)
    }
}
impl cont::RecoveryGeometryWorld for Host<'_> {
    /// `0x554350`, bit 29 required, `0x552250`, `0x5522B0`, the target ignored, `0x553BA0`.
    fn recovery_placement_blocked(&mut self, _: StateId, endpoint: [u32; 2], direction: [u32; 2], p: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        let id = self.id;
        let target = self.ignored(p.plan.movement.target_id);
        let hit = self.with_space(|s, policy| {
            let mut q = space::query_for(s, policy, id, false)?;
            q.required_actor_flags |= 0x2000_0000;
            q.set_position(&s.index, endpoint, policy).ok()?;
            q.placement.facing = direction;
            q.descriptor.ignore_actor = target;
            Some(space::first_hit(s, policy, &q))
        });
        Ok(hit.flatten().is_some_and(|h| h.blocked()))
    }
    /// `0x5312C0`.
    fn recovery_probe(&mut self, _: StateId, origin: [u32; 2], direction: [u32; 2], distance: u32, result: &mut [u32; 4], p: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        *result = self.probe(Some(origin), direction, distance, p.plan.movement.target_id);
        Ok(result[0] != 0)
    }
    fn recovery_diagnostic(&mut self, _: u32, _: u32, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}

/// A collision as the native 16-byte result: kind 2 with the thing, kind 1 with
/// the tile.
fn collision_words(blocker: Option<Blocker>) -> [u32; 4] {
    match blocker {
        None => [0; 4],
        Some(Blocker::Actor(other, _, _)) => [2, other + 1, 0, 0],
        Some(Blocker::Tile) => [1, 0, 0, 0],
        Some(Blocker::Hit(words)) => words,
    }
}

impl node_reached::World for Host<'_> {
    type Query = Option<aa_spatial::query::PreparedQuery>;
    fn actor(&self, actor: u32) -> aa_mind::Result<node_reached::Actor> {
        let a = self.world.actors.get(actor as usize).ok_or_else(|| err("actor absent"))?;
        Ok(node_reached::Actor { position: bits(a.pos.truncate()), leader_mind: a.org.map(|o| MindId(ORG | o)) })
    }
    fn world_step_bits(&self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.step.0)
    }
    /// Mind+54: the Deploy state's status.
    fn kind54(&self, _: MindId) -> aa_mind::Result<u32> {
        Ok(u32::from(self.ms.actors[self.id as usize].walk.is_some()) * 2)
    }
    /// The regiment's virtual +0x14C `0x51F7A0`.
    fn leader_node_reached(&mut self, _: MindId, actor: u32, p: &mut Payload, a: &mut Arena) -> aa_mind::Result<()> {
        mirror(&mut self.ms.actors[actor as usize], p);
        if let Some(o) = self.org() {
            // `0x51E8D0`: every node of the leader's path.
            let count = p.plan.movement.path.nodes.len();
            let rules = self.rules;
            regiment::leader_node_reached(self.ms, self.world, rules, o, actor, count, a);
        }
        Ok(())
    }
    fn waypoint_offset(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        cont::waypoint_offset(p, turn_radius(def_of(self.world, self.id)).to_bits(), PC, true)
    }
    /// `0x554350(actor, false)`.
    fn begin_query(&mut self, actor: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<Self::Query> {
        Ok(self.with_space(|s, policy| space::query_for(s, policy, actor, false)).flatten())
    }
    /// `0x552250`.
    fn query_position(&mut self, q: &mut Self::Query, position: [u32; 2], _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        if let Some(query) = q.as_mut() {
            self.with_space(|s, policy| query.set_position(&s.index, position, policy).ok());
        }
        Ok(())
    }
    /// `0x5522B0`.
    fn query_facing(&mut self, q: &mut Self::Query, facing: [u32; 2], _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        if let Some(query) = q.as_mut() {
            query.placement.facing = facing;
        }
        Ok(())
    }
    /// The GTID resolved directly, unlike `0x531170`'s `0x4CE6E0`.
    fn query_ignore(&mut self, q: &mut Self::Query, target_id: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        if let Some(query) = q.as_mut() {
            query.descriptor.ignore_actor = element_move::World::resolve(&*self, target_id)?.filter(|t| t & ORG == 0).map(space::actor_key);
        }
        Ok(())
    }
    fn query_kind2_flags(&mut self, q: &mut Self::Query) -> aa_mind::Result<()> {
        if let Some(query) = q.as_mut() {
            query.required_actor_flags |= 0x400;
            query.descriptor.mask |= 0x400;
        }
        Ok(())
    }
    /// `0x553BA0`.
    fn first_hit(&mut self, q: &mut Self::Query, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<[u32; 4]> {
        let Some(query) = q.as_ref() else { return Ok([0; 4]) };
        Ok(self.with_space(|s, policy| space::words(&space::first_hit(s, policy, query))).unwrap_or([0; 4]))
    }
    /// Mind +0x158 `0x52F1F0`.
    fn collision_response(&mut self, _: MindId, p: &mut Payload, _: &mut Arena) -> aa_mind::Result<bool> {
        self.blocked_response(p);
        Ok(false)
    }
    fn default_stance(&mut self, _: u32, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.deactivate_at(crate::trace::sites::CHARACTER_NODE);
        Ok(())
    }
    fn schedule_zero(&mut self, _: MindId, _: &mut Payload, _: &mut Arena) -> aa_mind::Result<()> {
        self.schedule_mind(0);
        Ok(())
    }
}

impl Host<'_> {
    /// `0x52F1F0`: a first collision of a move without a path plans (phase 1);
    /// else the Think handles it (phase 2), or with no path left the Mover stops
    /// (`0x4DB090`).
    fn blocked_response(&mut self, p: &mut Payload) {
        if p.plan.movement.phase == 0 {
            p.plan.movement.phase = 1;
            element_move::clear_path(p);
            self.schedule_mind(0);
        } else if p.plan.movement.path.nodes.is_empty() {
            self.deactivate();
        } else {
            p.plan.movement.phase = 2;
            self.schedule_mind(0);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry points for movement.rs.

/// `0x531D30`, the element Mind's queue-4 Update (`aa_mind::element_update`).
pub(super) fn update(ms: &mut MovementState, world: &mut World, id: ActorId, rules: MoveRules) {
    let (result, unpack, died) = with_mind(ms, world, rules, id, |host, p, a| {
        let mut update = Update { host, p, a };
        aa_mind::element_update::update(&mut update, MindId(id))
    });
    if let Err(e) = result {
        ms.note(format!("element {id} update: {e}"));
    }
    after_mind(ms, world, id, unpack, died);
}

/// What the Mind asked for that needs the world's stores whole.
fn after_mind(ms: &mut MovementState, world: &mut World, id: ActorId, unpack: Option<[u32; 2]>, died: bool) {
    if died {
        world.actors[id as usize].health = 0.0;
    }
    if unpack.is_some() {
        ms.unpacks.push(id);
    }
}

/// Give the element `nodes` as a path: `0x532170` (mode 0) or `0x532310`
/// (formation, mode 3), which replace the whole state stack with Move.
pub(super) fn give_path(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, nodes: Vec<Node>, mode: u8, speed: f32) {
    let (result, ..) = with_mind(ms, world, rules, id, |host, p, a| -> aa_mind::Result<bool> {
        let Some(last) = nodes.last().copied() else { return Ok(false) };
        let formation = mode == 3;
        if !formation && host.is_current(a, offsets::DEPLOY)? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.target_id = 0;
        m.destination_bits = bits(last.pos);
        m.mode = u32::from(mode);
        m.requested_speed_bits = speed.to_bits();
        m.auxiliary_bits = [0, 0];
        m.phase = 0;
        let mv = host.state(offsets::MOVE)?;
        if !aa_mind::activate(mv, 2, a, &mut Stack { host, p })? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.phase = 1;
        m.path.nodes.clear();
        m.path.nodes.extend(nodes.iter().map(|n| node_words(n.pos, n.dir, u32::from(n.kind))));
        m.path.kind1_count = nodes.iter().filter(|n| n.kind == 1).count() as i32;
        m.path.kind2_count = 0;
        m.path.word18 = NO_REGION;
        m.path.dirty = true;
        Ok(true)
    });
    if let Err(e) = result {
        ms.note(format!("element {id} path: {e}"));
    }
}

/// Member Mind +0x100 `0x532170` and +0x10C `0x532310`: activate the Move for the
/// path's tail (mode 3 for the formation event), copy the path `0x52D0F0`, take its
/// kind-2 tail off `0x52CE00`, and path +0x18 is -1.
pub(super) fn give_owned_path(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, path: &OwnedPath, mode: u8, speed: u32) {
    ms.actors[id as usize].pursue = None;
    ms.actors[id as usize].task = None;
    let (result, ..) = with_mind(ms, world, rules, id, |host, p, a| -> aa_mind::Result<bool> {
        let Some(last) = path.nodes.last().copied() else { return Ok(false) };
        let formation = mode == 3;
        if !formation && host.is_current(a, offsets::DEPLOY)? {
            return Ok(false);
        }
        // `0x52DF60`: `0x529C00(2)` first; a refused Move keeps its fields.
        let mv = host.state(offsets::MOVE)?;
        if !aa_mind::World::can_activate(&mut Stack { host, p }, mv, a)? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.target_id = 0;
        m.destination_bits = [last.words[0], last.words[1]];
        m.mode = u32::from(mode);
        m.requested_speed_bits = speed;
        m.auxiliary_bits = [0, 0];
        m.phase = 0;
        let mv = host.state(offsets::MOVE)?;
        if !aa_mind::activate(mv, 2, a, &mut Stack { host, p })? {
            return Ok(false);
        }
        let m = &mut p.plan.movement;
        m.phase = 1;
        m.path.assign_from(path).map_err(err)?;
        m.path.prune_kind2_tail().map_err(err)?;
        m.path.word18 = NO_REGION;
        Ok(true)
    });
    if let Err(e) = result {
        ms.note(format!("element {id} path: {e}"));
    }
}

/// [`clear`] with the arena in the caller's hands.
pub(super) fn clear_in(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, arena: &mut Arena) {
    if !world.books.minds.is_hosted(id, HostedKind::Element) {
        return;
    }
    let (result, ..) = with_mind_in(ms, world, rules, id, arena, |host, p, a| aa_mind::clear(MindId(id), a, &mut Stack { host, p }));
    if let Err(e) = result {
        ms.note(format!("element {id} clear: {e}"));
    }
}

/// Mind +0xFC `0x52F160`, from the rampage Think `0x5267E0`.
pub(super) fn reattach_in(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, arena: &mut Arena) {
    let (result, ..) = with_mind_in(ms, world, rules, id, arena, |host, p, a| host.event_reattach(p, a));
    if let Err(e) = result {
        ms.note(format!("element {id} reattach: {e}"));
    }
}

/// Mind +0x64 `0x535E40`: the Move's status is 2.
pub(super) fn move_active(ms: &MovementState, world: &World, id: ActorId) -> bool {
    let _ = world;
    ms.actors.get(id as usize).is_some_and(|a| a.active)
}

/// Mind +0x88 `0x52E610`: the state's speed, and the Mover's at factor 1 (`0x576D20`).
pub(super) fn set_speed(ms: &mut MovementState, id: ActorId, speed: u32) {
    let am = &mut ms.actors[id as usize];
    if let Some(p) = am.payload.as_mut() {
        p.plan.movement.requested_speed_bits = speed;
    }
    am.requested = f32::from_bits(speed);
    am.mover.requested = f32::from_bits(speed);
    am.mover.factor = 1.0;
}

/// Mind +0x80 `0x535EE0`: where the Move goes while it is current.
pub(super) fn move_destination(ms: &MovementState, id: ActorId) -> Option<[u32; 2]> {
    let am = ms.actors.get(id as usize)?;
    if !am.active {
        return None;
    }
    am.payload.as_ref().map(|p| p.plan.movement.destination_bits)
}

/// `0x530440(2, point, facing)` delivered to the unit (Mind +0xE8 `0x530960`), the
/// arena in the regiment's hands. False when its Mind refuses.
pub(super) fn begin_unpack_in(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, at: Vec2, arena: &mut Arena) -> bool {
    if !alive(world, id) {
        return false;
    }
    ms.actors[id as usize].unpack_at = at;
    let (result, unpack, died) = with_mind_in(ms, world, rules, id, arena, |host, p, a| {
        let state = host.state(offsets::UNPACK)?;
        aa_mind::activate(state, 2, a, &mut Stack { host, p })
    });
    after_mind(ms, world, id, unpack, died);
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("element {id} unpack: {e}"));
            false
        }
    }
}

/// `0x52A3C0`: the state stack clears and the Mind goes back to Sleep (the
/// regiment's and the combat system's stop).
pub(super) fn clear(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId) {
    if !world.books.minds.is_hosted(id, HostedKind::Element) {
        return;
    }
    let (result, ..) = with_mind(ms, world, rules, id, |host, p, a| aa_mind::clear(MindId(id), a, &mut Stack { host, p }));
    if let Err(e) = result {
        ms.note(format!("element {id} clear: {e}"));
    }
}

/// The Stop a regiment delivers (`0x4D9E20(6)`, `0x4D9B60`).
const STOP_COMMAND: u32 = 6;
/// The Move a regiment delivers (`0x4CA760`).
const MOVE_COMMAND: u32 = 5;

/// `0x4D9B60` -> `0x52C320` with `c` from the commands store, then `0x4CA2D0`.
fn process_command(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, c: u32, arena: &mut Arena) {
    let (result, unpack, died) = with_mind_in(ms, world, rules, id, arena, |host, p, a| {
        let mut stack = Stack { host, p };
        let r = command::process(MindId(id), c, a, &mut stack, &mut Context::default());
        command::World::release(&mut stack, c)?;
        r
    });
    if let Err(e) = result {
        ms.note(format!("element {id} command: {e}"));
    }
    after_mind(ms, world, id, unpack, died);
}

/// `0x4CA760(destination, speed, auxiliary)` delivered to an element.
pub(super) fn deliver_move(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, destination: [u32; 2], speed: u32, auxiliary: [u32; 2], arena: &mut Arena) {
    let minds = &mut world.books.minds;
    let c = minds.new_command(MOVE_COMMAND, None, Some(destination));
    if let Some(x) = minds.commands.get_mut(&c) {
        x.flags |= 8 | 0x10;
        x.auxiliary = auxiliary;
        x.speed = speed;
    }
    process_command(ms, world, rules, id, c, arena);
}

/// `0x4D9E20(6)` delivered to an element.
pub(super) fn deliver_stop(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, arena: &mut Arena) {
    let c = world.books.minds.new_command(STOP_COMMAND, None, None);
    process_command(ms, world, rules, id, c, arena);
}

/// `0x52DE40` KKC_UAIStateDeploy::Activate with the free spot already found:
/// mode 2, the embedded Move in movement mode 1 (a recruit walking out).
pub(super) fn begin_deploy(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, at: Vec2, tries: u8) {
    ms.actors[id as usize].walk_to = at;
    ms.actors[id as usize].walk = Some(MemberWalk { tries });
    let (result, ..) = with_mind(ms, world, rules, id, |host, p, a| {
        let deploy = host.state(offsets::DEPLOY)?;
        aa_mind::activate(deploy, 2, a, &mut Stack { host, p })
    });
    if let Err(e) = result {
        ms.note(format!("element {id} deploy: {e}"));
    }
}

/// The unpack action delivered to the truck (`0x51E800`, ordinal 0xB ->
/// KKC_UAIStateUnpack activation): state +10/+14 the order's point, then Enter
/// `0x52EB90` drives there. The building snaps only when it is made (`0x4F3640`).
pub(super) fn begin_unpack(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, at: Vec2) {
    ms.actors[id as usize].unpack_at = at;
    let (result, ..) = with_mind(ms, world, rules, id, |host, p, a| {
        let unpack = host.state(offsets::UNPACK)?;
        aa_mind::activate(unpack, 2, a, &mut Stack { host, p })
    });
    if let Err(e) = result {
        ms.note(format!("element {id} unpack: {e}"));
    }
}

/// [`begin_unpack`] unless the Unpack state is current already.
pub(super) fn begin_unpack_once(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, at: Vec2) {
    ensure_mind(ms, world, id);
    if world.books.minds.top_offset(id) != Some(offsets::UNPACK) {
        begin_unpack(ms, world, rules, id, at);
    }
}

/// What the Mover tells its Mind inside its queue-0 delivery.
pub(super) enum MoverCall<'c> {
    /// Mind +0x12C -> +0x160 `0x530AE0`.
    Reached,
    /// Mind +0x134 `0x530A70`: `0x52FD90`.
    TurnCompleted,
    /// Mind +0x154 `0x530A80`.
    Blocked(&'c Blocker, [f32; 2]),
    /// Mind +0x128 -> `0x52F260(0)`.
    Failed,
}

/// `Mover` fields must already be published; the caller refreshes after.
pub(super) fn mover_call(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, call: MoverCall<'_>) {
    let (result, unpack, died) = with_mind(ms, world, rules, id, |host, p, a| -> aa_mind::Result<()> {
        let state = host.state(offsets::MOVE)?;
        match call {
            MoverCall::Reached => {
                node_reached::apply(state, p, a, PC, host)?;
            }
            MoverCall::TurnCompleted => {
                cont::waypoint_offset(p, turn_radius(def_of(host.world, host.id)).to_bits(), PC, true)?;
            }
            MoverCall::Blocked(blocker, facing) => {
                let words = collision_words(Some(*blocker));
                cont::save_collision(p, words, [facing[0].to_bits(), facing[1].to_bits()], PC)?;
                host.blocked_response(p);
            }
            MoverCall::Failed => {
                host.path_failed_52f260(false, p, a)?;
            }
        }
        Ok(())
    });
    if let Err(e) = result {
        ms.note(format!("element {id} mover: {e}"));
    }
    after_mind(ms, world, id, unpack, died);
}

/// The `aa_mind::element_update::World` of one delivery.
struct Update<'u, 'w> {
    host: &'u mut Host<'w>,
    p: &'u mut Payload,
    a: &'u mut Arena,
}
impl aa_mind::regiment_update::BaseWorld for Update<'_, '_> {
    type Actor = ActorId;
    type Kingdom = KingdomId;
    fn actor(&mut self, _: MindId) -> aa_mind::Result<ActorId> {
        Ok(self.host.id)
    }
    fn actor_has_state10(&mut self, _: ActorId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn current_owner108(&mut self, _: ActorId) -> aa_mind::Result<Option<KingdomId>> {
        Ok(None)
    }
    fn kingdom_byte239(&mut self, _: KingdomId) -> aa_mind::Result<u8> {
        Ok(0)
    }
    fn attached_to_organization(&mut self, _: ActorId) -> aa_mind::Result<bool> {
        Ok(self.host.org().is_some())
    }
    fn current_home_4d11a0(&mut self, _: ActorId) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn mind_virtual1a8(&mut self, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    fn timed_list_4d37e0(&mut self, _: ActorId) -> aa_mind::Result<()> {
        Ok(())
    }
}
impl aa_mind::element_update::World for Update<'_, '_> {
    type State = StateId;
    fn actor_flags8(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        Ok(u32::from(!alive(self.host.world, actor)))
    }
    fn actor_state_high(&mut self, _: ActorId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn expire_bad_targets(&mut self, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    fn current_state(&mut self, mind: MindId) -> aa_mind::Result<StateId> {
        self.a.mind(mind)?.stack.front().copied().flatten().ok_or_else(|| err("the Mind has no current state"))
    }
    fn think(&mut self, state: StateId) -> aa_mind::Result<()> {
        match self.host.offset_of(state) {
            offsets::MOVE => element_move::think(state, self.p, self.a, PC, self.host),
            // 0x52DF30: the embedded Move thinks.
            offsets::DEPLOY => {
                aa_mind::think_base(state, self.a)?;
                let mv = self.host.state(offsets::MOVE)?;
                element_move::think(mv, self.p, self.a, PC, self.host)
            }
            offsets::UNPACK => self.unpack_think(state),
            offsets::REATTACH => uai_reattach::think(state, self.a, &mut Stack { host: self.host, p: self.p }).map(|_| ()),
            offsets::SLEEP => uai_sleep::think(state, self.a, &mut Stack { host: self.host, p: self.p }, PC).map(|_| ()),
            other => Err(err(format!("Mind+{other:X} state Think is not hosted"))),
        }
    }
    fn mover_active_unlinked(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        Ok(self.host.ms.actors[actor as usize].mover.active && !self.host.world.is_scheduled(Sched::Mover(actor)))
    }
    /// Mind+88: the Move state's status.
    fn mind_word88(&mut self, _: MindId) -> aa_mind::Result<u32> {
        let mv = self.host.state(offsets::MOVE)?;
        Ok(self.a.state(mv)?.status)
    }
    fn deactivate_4cfc80(&mut self, _: ActorId) -> aa_mind::Result<()> {
        self.host.deactivate();
        Ok(())
    }
    fn updatable_linked(&mut self, _: MindId) -> aa_mind::Result<bool> {
        Ok(self.host.world.is_scheduled(Sched::Mind(self.host.id)))
    }
    fn active_component_linked(&mut self, actor: ActorId) -> aa_mind::Result<bool> {
        Ok(self.host.ms.actors[actor as usize].mover.active && self.host.world.is_scheduled(Sched::Mover(actor)))
    }
    fn transport_blocks(&mut self, _: MindId, _: ActorId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn reschedule(&mut self, _: MindId, delay: u32) -> aa_mind::Result<()> {
        self.host.schedule_mind(delay);
        Ok(())
    }
    fn summoned(&mut self, _: ActorId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn in_combat(&mut self, _: MindId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn summoned_dies(&mut self, _: ActorId) -> aa_mind::Result<()> {
        Ok(())
    }
}
/// `aa_mind::uai_thinks::ThinkWorld` for the Thinks of the hosted states.
impl ThinkWorld for Stack<'_, '_> {
    fn embedded_state(&mut self, _: MindId, offset: u32) -> aa_mind::Result<StateId> {
        self.host.state(offset)
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, a: &mut Arena) -> aa_mind::Result<()> {
        match offset {
            offsets::MOVE => {
                let mv = self.host.state(offsets::MOVE)?;
                element_move::think(mv, self.p, a, PC, self.host)
            }
            other => Err(err(format!("Mind+{other:X} state Think is not hosted"))),
        }
    }
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        element_move::World::resolve(&*self.host, gtid)
    }
    fn object_gtid(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(object + 1)
    }
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn mind_virtual_140(&mut self, _: MindId, _: u32, a: &mut Arena) -> aa_mind::Result<()> {
        let Stack { host, p } = self;
        host.completed(p, a).map(|_| ())
    }
    fn current_performer(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn locomotion(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(Some(self.host.id))
    }
    fn locomotion_follow(&mut self, _: u32, _: u32, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
}
impl uai_reattach::ReattachThinkWorld for Stack<'_, '_> {
    fn membership(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(self.host.org().map(|_| self.host.id))
    }
    fn membership_organization(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(self.host.org().map(|o| ORG | o))
    }
    fn membership_detached(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(self.host.attached())
    }
    fn membership_refresh(&mut self, _: u32) -> aa_mind::Result<()> {
        self.host.refresh_membership();
        Ok(())
    }
    fn mind_virtual_144(&mut self, _: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        let Stack { host, p } = self;
        host.completed(p, a).map(|_| ())
    }
    /// `0x532930(0, organization, 2, 0, null)`.
    fn move_to_object(&mut self, _: MindId, mode: u32, object: Option<u32>, a3: u32, a4: u32, point: [u32; 2], a: &mut Arena) -> aa_mind::Result<()> {
        let Some(object) = object else { return Ok(()) };
        let Stack { host, p } = self;
        let mv = host.state(offsets::MOVE)?;
        host.activate_to_object(mv, mode, object + 1, a3, a4, point, p, a)?;
        Ok(())
    }
}
impl uai_sleep::SleepThinkWorld for Stack<'_, '_> {
    /// Actor +110, the overlaps `0x4D67F0`, stays empty until M5.
    fn actor_word110(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn actor_flags100(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn actor_pending_point(&mut self, _: MindId) -> aa_mind::Result<Option<[u32; 2]>> {
        Ok(None)
    }
    fn move_activate(&mut self, _: MindId, request: aa_navigation::Activation, a: &mut Arena) -> aa_mind::Result<bool> {
        let Stack { host, p } = self;
        let mv = host.state(offsets::MOVE)?;
        host.activate_move(mv, request, p, a)
    }
    fn membership(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(self.host.org().map(|_| self.host.id))
    }
    fn membership_organization(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(self.host.org().map(|o| ORG | o))
    }
    /// The port reads "detached" as the byte: its step 3 runs on a clear one.
    fn membership_detached(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(self.host.attached())
    }
    fn membership_index(&mut self, _: u32) -> aa_mind::Result<u32> {
        let id = self.host.id;
        Ok(self.host.org().map_or(0, |o| member_slot(self.host.world, o, id) as u32))
    }
    /// Mind +0xFC `0x52F160`.
    fn mind_virtual_fc(&mut self, _: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        let Stack { host, p } = self;
        host.event_reattach(p, a).map(|_| ())
    }
    /// Actor +AC: 0 here; the pack and unpack postures of the regiment's other
    /// members (`0x52EC20`) are not hosted yet.
    fn actor_word_ac(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn object_word_ac(&mut self, _: u32) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn organization_leader(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn formation_offset(&mut self, _: u32, _: u32) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn world_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.host.world.clock.begin.0)
    }
    fn sleep_time(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn set_sleep_time(&mut self, _: StateId, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    fn leader_4d1080(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn leader_4d10a0(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn leader_4d0980(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn actor_4d10a0(&mut self, _: MindId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn definition_534_54(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn leader_virtual_0(&mut self, _: u32) -> aa_mind::Result<Binary> {
        Binary::from_f32_bits(0).map_err(err)
    }
    fn normalize(&mut self, vector: [u32; 2]) -> aa_mind::Result<[u32; 2]> {
        Ok(vector)
    }
    fn object_position(&mut self, object: u32) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(self.host.thing_position(object)))
    }
    fn issue_action(&mut self, _: MindId, _: &aa_mind::cai_thinks::unit_order::StackAction, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn resume_pending(&mut self, mind: MindId, a: &mut Arena) -> aa_mind::Result<bool> {
        command::resume_pending(mind, a, self, &mut Context::default())
    }
}
impl Update<'_, '_> {
    /// `0x5372B0` for the truck that is its organization's unit, or that has no
    /// organization: an active Move thinks, else the unpack (`0x575C80`) runs
    /// once the stores are back, then Mind +0x1B0 ends the state.
    fn unpack_think(&mut self, state: StateId) -> aa_mind::Result<()> {
        aa_mind::think_base(state, self.a)?;
        let mv = self.host.state(offsets::MOVE)?;
        if self.a.state(mv)?.status == 2 {
            return element_move::think(mv, self.p, self.a, PC, self.host);
        }
        let at = self.host.ms.actors[self.host.id as usize].unpack_at;
        self.host.unpack = Some(bits(at));
        aa_mind::pop(MindId(self.host.id), self.a, &mut Stack { host: self.host, p: self.p })
    }
}
