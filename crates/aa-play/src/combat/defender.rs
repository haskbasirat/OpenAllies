//! The DefenderComponent (actor `+0xB0`, slot 20, vtables 0xA3BBD4 / 0xA3BBAC):
//! the guns of a building, a corps or a bunker. Owner: combat.
//!
//! It acts on its ZOC being entered, not on a scan:
//!
//! * `0x567840`, reached from `0x514DA0` -> `0x514E40` when an enemy takes a tile
//!   of the zone, keeps the intruder on the `+0x1C` list if an ability could use
//!   it (`0x5673B0`), and with the list's first intruder the Updatable joins
//!   queue 0 (`0x50AC00(0)`, one draw) with a deferred update in one second
//!   (`0x50ACC0(1.0, 0)`).
//! * The deferred `0x567BF0` (Updatable `+0x18`, one second) drops the intruders
//!   out of the zone (virtual `+0x130(zone 4, 1)`) or out of reach (`0x5673B0`).
//!   An empty list clears every ability's target (`0x567140(ability, 0)`) and
//!   leaves the queues (`0x50B250`); otherwise it re-arms.
//! * The Tick `0x567960` (queue 0, every update) goes over the abilities: a target
//!   that went away or is refused (`0x4C8F20` other than busy, codes 7 and 8) is
//!   cleared; one that is missing or poses no threat (`0x566D40` below 2) is
//!   chosen again over the intruders (`0x567530`: the highest `0x566D40`, then
//!   the nearest, members standing for an organization); then the ability is
//!   used, after its turret has turned to it (`0x4C81D0`, `0x57E730`).
//! * `0x567140` writes the ability's target and, through `0x4D3990`, the
//!   defender (its organization if it has one, `0x4CE6C0`) on the `+0x118` of the
//!   target's organization or the target: [`DefenderState::engaged_with`].
//!
//! The Tick and `0x5673B0` run through the port `aa_combat::defender` over the
//! bindings [`Host`] and [`Reach`]. The admission `0x4C8F20` is
//! [`super::admission_with`]; the ability's use is [`super::use_weapon`]. No
//! getter of these has been met in a film yet, so the bindings are compared with
//! the code and not with a recording.
use super::{CombatState, TargetRef};
use crate::world::{ActorId, World};
use crate::zones::Key;

/// The component slot of the Defender.
pub const SLOT: u8 = 20;
/// `0x50ACC0(0x3F800000)`.
const LIST_SECONDS: f32 = 1.0;

/// One thing's Defender. `+0x1C` holds GTIDs; a dead one simply stops resolving,
/// so a dead thing here stands for an entry that no longer resolves.
#[derive(Clone, Debug, Default)]
pub struct DefenderState {
    /// `+0x1C`: the intruders, head first (`0x4D0900`).
    pub intruders: Vec<TargetRef>,
    /// Ability `+0x14` per ability of the definition's Defender list: the weapon
    /// index in `Def::weapons` and the target.
    pub targets: Vec<(usize, Option<ActorId>)>,
}

impl DefenderState {
    /// The `+0x118` engagements this Defender holds: what `0x567140` registered
    /// with `0x4D3990`, the target's organization or the target.
    pub fn engaged_with(&self, world: &World) -> Vec<TargetRef> {
        self.targets
            .iter()
            .filter_map(|&(_, t)| {
                let t = t?;
                Some(match world.actors.get(t as usize)?.org {
                    Some(o) => TargetRef::Org(o),
                    None => TargetRef::Actor(t),
                })
            })
            .collect()
    }
}

fn to_ref(k: Key) -> TargetRef {
    match k {
        Key::Actor(a) => TargetRef::Actor(a),
        Key::Org(o) => TargetRef::Org(o),
    }
}

/// `0x567BF0`'s and `0x567840`'s read of the abilities: the Defender list of
/// the definition (ActorData `+0x21C`, the constructor `0x567CD0`, source order).
fn abilities(world: &World, a: ActorId) -> Vec<usize> {
    let Some(x) = world.actors.get(a as usize) else { return Vec::new() };
    world.data.def(x.def).weapons.iter().enumerate().filter(|(_, w)| w.defender).map(|(i, _)| i).collect()
}

/// `0x5673B0` through its port (aa_combat::defender::reachable): `a`'s Defender
/// can reach `t`.
fn reachable(world: &World, st: &CombatState, a: ActorId, t: TargetRef) -> bool {
    let state = aa_combat::defender::Defender {
        owner: handle(a),
        abilities: abilities(world, a).into_iter().map(|w| w as u32).collect(),
        intruders: Vec::new(),
        radius: aa_body::F32::ZERO,
        radius_dirty: false,
    };
    let mut host = Reach { world, st, a, candidate: None };
    let candidate = resolves(world, ref_handle(t));
    aa_combat::defender::reachable(&state, candidate, &mut host, PC).unwrap_or(false)
}

/// The control word the battle runs under (53-bit significand).
const PC: aa_numeric::Precision = aa_numeric::Precision::Double53;
/// The mark of an organization in the handles given to the port.
const ORG: u32 = 0x8000_0000;

/// The port's name of actor `a`: nonzero, as an empty member entry is 0.
fn handle(a: ActorId) -> u32 {
    a + 1
}

fn ref_handle(t: TargetRef) -> u32 {
    match t {
        TargetRef::Actor(a) => handle(a),
        TargetRef::Org(o) => ORG | o,
    }
}

/// The actor behind a handle that is not an organization's.
fn actor_of(h: u32) -> Option<ActorId> {
    (h != 0 && h & ORG == 0).then(|| h - 1)
}

fn point_of(world: &World, h: u32) -> aa_combat::Result<[aa_body::F32; 2]> {
    let x = actor_of(h).and_then(|a| world.actors.get(a as usize)).ok_or(aa_combat::Error::Invalid("position of no actor"))?;
    Ok([aa_body::F32(x.pos.x.to_bits()), aa_body::F32(x.pos.y.to_bits())])
}

/// The handle table's answer: an actor that still stands (a dead thing stands
/// for a handle that no longer resolves), an organization that is alive.
fn resolves(world: &World, h: u32) -> Option<u32> {
    let known = match actor_of(h) {
        Some(a) => world.actors.get(a as usize).is_some_and(|x| x.alive),
        None => h & ORG != 0 && world.orgs.get((h & !ORG) as usize).is_some_and(|g| g.alive),
    };
    known.then_some(h)
}

/// Actor `+0x7C`: the members of an organization, in the order `World` keeps them.
fn members_of(world: &World, h: u32) -> Option<Vec<u32>> {
    if h & ORG == 0 {
        return None;
    }
    Some(world.orgs.get((h & !ORG) as usize).map_or_else(Vec::new, |g| g.members.iter().map(|&m| handle(m)).collect()))
}

fn weapon_of(world: &World, a: ActorId, weapon: usize) -> aa_combat::Result<&crate::data::Weapon> {
    world.data.def(world.actors[a as usize].def).weapons.get(weapon).ok_or(aa_combat::Error::Invalid("no such ability"))
}

/// The services of `0x5673B0` over the battle. An ability is its index in the
/// definition's weapons.
struct Reach<'a> {
    world: &'a World,
    st: &'a CombatState,
    a: ActorId,
    /// The candidate of the context `0x4C8740` was last asked with.
    candidate: Option<ActorId>,
}

impl aa_combat::defender::ReachWorld for Reach<'_> {
    // 0x6A71D0 for every actor class: true.
    fn qualifies(&mut self, _: u32) -> aa_combat::Result<bool> {
        Ok(true)
    }
    fn organization_members(&mut self, actor: u32) -> aa_combat::Result<Option<Vec<u32>>> {
        Ok(members_of(self.world, actor))
    }
    // +8 bit 14, the only bit asked for: targetable; a thing on its way out has none.
    fn flags(&self, actor: u32) -> aa_combat::Result<u32> {
        let x = actor_of(actor).and_then(|a| self.world.actors.get(a as usize)).filter(|x| x.alive && !x.removed);
        Ok(if x.is_some_and(|x| self.world.data.def(x.def).targetable) { 0x4000 } else { 0 })
    }
    fn position(&self, actor: u32) -> aa_combat::Result<[aa_body::F32; 2]> {
        point_of(self.world, actor)
    }
    fn admits(&mut self, ability: u32, context: &mut aa_combat::performer::Context) -> aa_combat::Result<bool> {
        self.candidate = context.target.and_then(actor_of);
        let Some(m) = self.candidate else { return Ok(false) };
        Ok(super::admits(self.world, self.st, self.a, weapon_of(self.world, self.a, ability as usize)?, m))
    }
    fn within(&mut self, ability: u32, _: u32, point: [aa_body::F32; 2]) -> aa_combat::Result<bool> {
        let point = glam::Vec2::new(f32::from_bits(point[0].0), f32::from_bits(point[1].0));
        Ok(super::in_turret_arc(self.world, self.a, weapon_of(self.world, self.a, ability as usize)?, point))
    }
    fn clear(&mut self, ability: u32, _: u32, _: [aa_body::F32; 2]) -> aa_combat::Result<bool> {
        let w = weapon_of(self.world, self.a, ability as usize)?;
        let Some(m) = self.candidate else { return Ok(false) };
        Ok(w.min_range <= 0.0 || super::in_range(self.world, self.a, m, w.min_range, true))
    }
    fn has_projectile(&self, ability: u32) -> aa_combat::Result<bool> {
        Ok(weapon_of(self.world, self.a, ability as usize)?.projectile.is_some())
    }
    // 0x4BE910: definition +0x24 through the modifiers (a projectile's 0x501D50,
    // flag bit 3 0x501DB0), without the fuzzy part. Each modifier result is taken
    // as its stored binary32 here.
    fn range(&mut self, ability: u32, _: u32) -> aa_combat::Result<aa_numeric::Fast> {
        let w = weapon_of(self.world, self.a, ability as usize)?;
        let key = Key::Actor(self.a);
        let clamp = self.world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0);
        let mut range = w.range;
        if w.projectile.is_some()
            && let Some(add) = crate::zones::modifier_add(self.world, key, "PROJECTILE_ABILITY_RANGE")
        {
            range = crate::zones::clamp_add(range, add, clamp);
        }
        if w.flags & 8 != 0
            && let Some(add) = crate::zones::modifier_add(self.world, key, "BOMBARD_ABILITY_RANGE")
        {
            range = crate::zones::clamp_add(range, add, clamp);
        }
        Ok(aa_numeric::Fast::from_f32(range)?)
    }
}

/// The services of the Tick `0x567960` (aa_combat::defender::tick) over the
/// battle. An ability is its row in [`DefenderState::targets`].
struct Host<'a> {
    world: &'a mut World,
    st: &'a mut CombatState,
    a: ActorId,
}

impl Host<'_> {
    fn row(&self, ability: u32) -> aa_combat::Result<(usize, Option<ActorId>)> {
        self.st.actors[self.a as usize].defender.targets.get(ability as usize).copied().ok_or(aa_combat::Error::Invalid("no such ability"))
    }
    fn actor(&self, h: u32) -> aa_combat::Result<ActorId> {
        actor_of(h).filter(|&a| (a as usize) < self.world.actors.len()).ok_or(aa_combat::Error::Invalid("not an actor"))
    }
}

impl aa_combat::defender::DefenderWorld for Host<'_> {
    fn resolve(&self, h: u32) -> aa_combat::Result<Option<u32>> {
        Ok(resolves(self.world, h))
    }
    fn removed(&self, actor: u32) -> aa_combat::Result<bool> {
        Ok(self.world.actors[self.actor(actor)? as usize].removed)
    }
    fn handle_of(&self, actor: u32) -> aa_combat::Result<u32> {
        Ok(actor)
    }
    fn position(&self, actor: u32) -> aa_combat::Result<[aa_body::F32; 2]> {
        point_of(self.world, actor)
    }
    fn ability_target(&self, ability: u32) -> aa_combat::Result<u32> {
        Ok(self.row(ability)?.1.map_or(0, handle))
    }
    // Definition +0x54 bit 3; nothing the battle's use reads from the context.
    fn ability_kind_flag(&self, ability: u32) -> aa_combat::Result<bool> {
        Ok(weapon_of(self.world, self.a, self.row(ability)?.0)?.flags & 8 != 0)
    }
    fn can_use(&mut self, ability: u32, context: &mut aa_combat::performer::Context, check_range: bool) -> aa_combat::Result<u32> {
        let (weapon, _) = self.row(ability)?;
        let Some(t) = context.target.and_then(actor_of) else { return Ok(1) };
        Ok(super::admission_with(self.world, self.st, self.a, weapon, t, check_range))
    }
    fn set_target(&mut self, ability: u32, target: Option<u32>) -> aa_combat::Result<()> {
        let target = target.and_then(actor_of);
        if let Some(row) = self.st.actors[self.a as usize].defender.targets.get_mut(ability as usize) {
            row.1 = target;
        }
        Ok(())
    }
    // 0x6A71D0 for every actor class: true.
    fn qualifies(&mut self, _: u32) -> aa_combat::Result<bool> {
        Ok(true)
    }
    fn organization_members(&mut self, actor: u32) -> aa_combat::Result<Option<Vec<u32>>> {
        Ok(members_of(self.world, actor))
    }
    fn aggressive(&mut self, actor: u32) -> aa_combat::Result<bool> {
        Ok(super::mind_aggressive(self.world, self.st, TargetRef::Actor(self.actor(actor)?)))
    }
    fn harmless(&mut self, actor: u32) -> aa_combat::Result<bool> {
        Ok(super::harmless(self.world, self.actor(actor)?))
    }
    // Body, ActorData +0x291 and not invulnerable (the definition's word here).
    fn body_takeable(&self, actor: u32) -> aa_combat::Result<bool> {
        let d = self.world.data.def(self.world.actors[self.actor(actor)? as usize].def);
        Ok(d.health > 0.0 && d.destructible && !d.invulnerable)
    }
    fn capturable(&mut self, actor: u32) -> aa_combat::Result<bool> {
        Ok(super::capturable(self.world, self.actor(actor)?))
    }
    // 0x4BE520: the definition's turret index is not -1.
    fn ability_has_turret(&self, ability: u32) -> aa_combat::Result<bool> {
        Ok(weapon_of(self.world, self.a, self.row(ability)?.0)?.turret != -1)
    }
    // The ability stands for its turret: `aim` finds it through the weapon.
    fn ability_turret(&self, ability: u32) -> aa_combat::Result<i32> {
        Ok(ability as i32)
    }
    fn aim(&mut self, _: u32, index: i32, _: [aa_body::F32; 2], handle: u32) -> aa_combat::Result<bool> {
        let (weapon, _) = self.row(index as u32)?;
        let target = self.actor(handle)?;
        Ok(super::turret_ready(self.world, self.st, self.a as usize, weapon, target))
    }
    fn use_ability(&mut self, _: u32, ability: u32, context: &mut aa_combat::performer::Context) -> aa_combat::Result<()> {
        let (weapon, _) = self.row(ability)?;
        let Some(t) = context.target.and_then(actor_of) else { return Ok(()) };
        super::use_weapon(self.world, self.st, self.a, weapon, t);
        Ok(())
    }
    // The Tick moves the zone itself before the port runs.
    fn refresh_radius(&mut self, _: u32, _: [aa_body::F32; 2], _: aa_body::F32) -> aa_combat::Result<()> {
        Ok(())
    }
}

/// `0x567840`: `owner`'s ZOC was entered by `who` (its organization when attached,
/// `0x514DA0`). The Defender of a dead thing does nothing, one without abilities
/// never admits. The first intruder starts the Updatable when it is on no queue
/// and the world is not loading (`0x4F0460`).
pub(crate) fn entered(world: &mut World, owner: Key, who: Key) {
    let Key::Actor(a) = owner else { return };
    let mut st = std::mem::take(&mut world.combat);
    entered_with(world, &mut st, a, who);
    world.combat = st;
}

fn entered_with(world: &mut World, st: &mut CombatState, a: ActorId, who: Key) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        return;
    }
    let t = to_ref(who);
    if st.actors.get(a as usize).is_some_and(|s| s.defender.intruders.contains(&t)) || !reachable(world, st, a, t) {
        return;
    }
    let row = abilities(world, a);
    let s = &mut st.actors[a as usize].defender;
    if s.targets.len() != row.len() {
        s.targets = row.into_iter().map(|w| (w, None)).collect();
    }
    s.intruders.insert(0, t);
    let owner = crate::scheduler::Owner::Component(a, SLOT);
    // 0x4F0460: not while the world loads (GWorld +28's mode, gone once it runs).
    if !world.is_scheduled(owner) && world.session_started {
        world.schedule_periodic_at(crate::trace::sites::DEFENDER_ENTERED, crate::scheduler::EVERY_TICK, owner, 0);
        world.schedule_after(crate::scheduler::DEFERRED, owner, 0, LIST_SECONDS);
    }
}

/// The deferred `0x567BF0` of actor `a`'s Defender.
pub(crate) fn deferred(world: &mut World, a: ActorId) {
    let mut st = std::mem::take(&mut world.combat);
    let mut list = st.actors.get_mut(a as usize).map_or_else(Vec::new, |s| std::mem::take(&mut s.defender.intruders));
    // zone 4 and virtual +0x130(zone, 1): the thing or a member stands in it
    let engine = world.fog.engine.as_ref();
    list.retain(|&t| target_exists(world, t) && engine.is_some_and(|e| e.in_zoc(world, key_of(t), a)) && reachable(world, &st, a, t));
    let empty = list.is_empty();
    if let Some(s) = st.actors.get_mut(a as usize) {
        s.defender.intruders = list;
        if empty {
            for row in &mut s.defender.targets {
                row.1 = None;
            }
        }
    }
    world.combat = st;
    let owner = crate::scheduler::Owner::Component(a, SLOT);
    if empty {
        world.unschedule(owner);
    } else {
        world.schedule_after(crate::scheduler::DEFERRED, owner, 0, LIST_SECONDS);
    }
}

fn key_of(t: TargetRef) -> Key {
    match t {
        TargetRef::Actor(a) => Key::Actor(a),
        TargetRef::Org(o) => Key::Org(o),
    }
}

fn target_exists(world: &World, t: TargetRef) -> bool {
    match t {
        TargetRef::Actor(a) => world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
        TargetRef::Org(o) => world.orgs.get(o as usize).is_some_and(|g| g.alive),
    }
}

/// The Tick `0x567960` of actor `a`'s Defender.
pub(crate) fn tick(world: &mut World, a: ActorId) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        return;
    }
    // Defender +0x30 (0x566F80: the modifiers changed, +0x2C the new radius): the first
    // Tick after it moves the ZOC (0x515F40 at the thing's position).
    if let Some(mut e) = crate::fog::take_engine(world) {
        let key = Key::Actor(a);
        let control = e.def_of(world, key).control;
        let radius = match crate::zones::modifier_add(world, key, "CONTROL_RANGE") {
            Some(add) => crate::zones::clamp_add(control, add, world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0)),
            None => control,
        };
        e.zoc_radius(world, key, radius);
        crate::fog::put_engine(world, e);
    }
    let mut st = std::mem::take(&mut world.combat);
    if st.actors.len() < world.actors.len() || st.orgs.len() < world.orgs.len() || st.rules.is_none() {
        st.prepare(world);
    }
    let mut state = aa_combat::defender::Defender {
        owner: handle(a),
        abilities: (0..st.actors[a as usize].defender.targets.len() as u32).collect(),
        intruders: st.actors[a as usize].defender.intruders.iter().map(|&t| ref_handle(t)).collect(),
        // +0x2C / +0x30: the zone refresh above stands for them
        radius: aa_body::F32::ZERO,
        radius_dirty: false,
    };
    let mut host = Host { world: &mut *world, st: &mut st, a };
    let done = aa_combat::defender::tick(&mut state, &mut host, PC);
    debug_assert!(done.is_ok(), "defender update: {done:?}");
    world.combat = st;
}
