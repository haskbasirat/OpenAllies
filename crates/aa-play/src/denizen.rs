//! The DenizenComponent (Denizen.cpp, component slot 10) of airfields and
//! carriers: the aircraft inventory behind the air attack and air recon
//! orders. Owner: core.
//!
//! - Constructor 0x56B720, InitCreate 0x568750 and the InitExisting 0x5687E0
//!   a created actor also runs ([`created`]): one slot per inventory count,
//!   head first. Each slot gets the full health of its definition (unless
//!   the actor is under construction). The Updatable registers on queue 1
//!   (one world draw) when the guard range is positive or the inventory is
//!   not empty. The last release is -delay, and the behaviours (+0x38) are
//!   the OR of the Organization and DenizenGroup behaviours.
//! - The queue-1 Tick 0x56E860 ([`tick`]): the occupants' value 0x569E40,
//!   the deployed missions 0x56E480 (members that left are pruned; a mission
//!   without members ends), the slot regeneration 0x569440 / 0x568C80, and
//!   the readiness 0x5696B0.
//! - The command mask 0x569C70 ([`enabled`]): air_attack (0x19) needs the
//!   AIR_OFFENSIVE behaviour both possible and ready, air_recon (0x1A)
//!   AIR_RECON. [`ready`] is +0x3C for the HUD.
//! - The purchases 0x56D5A0 (attack an actor), 0x56D7C0 (attack a point) and
//!   0x56DA00 (recon a point) ([`purchase`]) on aa-production
//!   `purchase_denizen`. Then CreateDenizen 0x56CC10 ([`create`]): the
//!   aircraft are created in the hangar (the Runway frame 0x517DF0 /
//!   0x517EE0) and grouped into a mission (0x568180). The mission's use
//!   0x568480 hands each member the command.
//! - The runway queue 0x5689A0 and its deferred release 0x5695B0
//!   ([`runway_release`]): with a positive Runway delay the aircraft wait in
//!   the hangar and leave one per delay.
//!
//! Not yet ported:
//! - TickGuard 0x56A7E0 (the guard zone, its intruders, under_siege, and the
//!   militia / AIR_DEFENSIVE scramble 0x5688F0 -> 0x56D290);
//! - the worker and settlement lists 0x56A910;
//! - the escort missions 0x56E390 / 0x56AEC0;
//! - an aircraft landing back into its slot (OnIdentityTaken 0x56A980);
//! - the aircraft Mind (AAI.cpp).
//!
//! The orders stay refused until the aircraft fly.
use crate::books::{AirOrder, DenizenBooks, DenizenSlot, Mission, Notice};
use crate::data::{DefId, DenizenDef};
use crate::scheduler::{DEFERRED, Owner};
use crate::world::{ActorId, KingdomId, World};
use aa_production::purchases::{Denizen, DenizenPurchase, Purchase};
use glam::{Vec2, Vec3};

/// The component slot (aa-rules COMPONENTS).
pub const SLOT: u8 = 10;
/// Behaviour bits (0x463520).
pub const WORKER: u32 = 0x02;
pub const MILITIA_OFFENSIVE: u32 = 0x04;
pub const MILITIA_DEFENSIVE: u32 = 0x08;
pub const AIR_OFFENSIVE: u32 = 0x10;
pub const AIR_DEFENSIVE: u32 = 0x20;
pub const AIR_RECON: u32 = 0x40;
pub const AIR_ESCORT: u32 = 0x80;
/// The actor commands of 0x56E730 (aa-rules PREDEFINED_ACTOR_COMMANDS).
pub const COMMAND_AIR_ATTACK: u32 = 0x19;
pub const COMMAND_AIR_RECON: u32 = 0x1A;

const BODY: u32 = 0;
const PERFORMER: u32 = 5;
const ELEMENT: u32 = 8;
const AIRCRAFT: u32 = 23;

fn has(world: &World, def: DefId, bit: u32) -> bool {
    world.data.def(def).components & (1 << bit) != 0
}

fn denizen_def(world: &World, a: ActorId) -> Option<&DenizenDef> {
    world.actors.get(a as usize).and_then(|x| world.data.def(x.def).denizen.as_ref())
}

fn books(world: &World, a: ActorId) -> Option<&DenizenBooks> {
    world.books.actor(a).and_then(|b| b.denizen.as_ref())
}

fn books_mut(world: &mut World, a: ActorId) -> Option<&mut DenizenBooks> {
    world.books.actors.get_mut(a as usize).and_then(|b| b.denizen.as_mut())
}

/// 0x45D730(0): the full health of a definition (no modifier context).
fn full_health(world: &World, def: DefId) -> f32 {
    world.data.def(def).health
}

/// 0x56B720 + 0x568750 + 0x5687E0 for the created actor `a`: its denizen
/// books. Returns true when its Updatable registers on queue 1 (0x50AC60).
pub fn created(world: &mut World, a: ActorId) -> bool {
    let Some(dd) = denizen_def(world, a).cloned() else { return false };
    // 0x568750: not under construction (actor +0x100 bit 21) -> every slot
    // takes 0x45D730(0) of its definition.
    let constructing = world.books.actor(a).is_some_and(|b| b.constructing);
    let slots = dd
        .slots
        .iter()
        .map(|&def| DenizenSlot { def, occupant: None, health: if constructing { 0.0 } else { full_health(world, def) }, worker: false, defensive: false })
        .collect();
    let delay = dd.runway.as_ref().map_or(0.0, |r| r.delay);
    let behaviors = dd.organization_behaviors.iter().fold(0, |b, x| b | x) | dd.groups.iter().fold(0, |b, g| b | g.behavior);
    world.books.actor_mut(a).denizen = Some(DenizenBooks { slots, last_release: -delay, behaviors, ..Default::default() });
    registers(&dd)
}

/// 0x568750's registration test: guard_range > 0 or a non-empty inventory.
pub fn registers(dd: &DenizenDef) -> bool {
    dd.guard_range > 0.0 || dd.inventory
}

/// +0x3C: the behaviours the inventory can field now (0x10 air attack,
/// 0x40 air recon; 0 without a denizen).
pub fn ready(world: &World, a: ActorId) -> u32 {
    books(world, a).map_or(0, |d| d.ready)
}

/// 0x569C70: whether `command` (0x19 air_attack, 0x1A air_recon) is
/// enabled on `a`. Every denizen command is off while it is razing (state
/// 30, which aa-play does not have).
pub fn enabled(world: &World, a: ActorId, command: u32) -> bool {
    let Some(d) = books(world, a) else { return false };
    let bit = match command {
        COMMAND_AIR_ATTACK => AIR_OFFENSIVE,
        COMMAND_AIR_RECON => AIR_RECON,
        _ => return false,
    };
    d.behaviors & bit != 0 && d.ready & bit != 0
}

/// 0x568A40: the housed slots of `def` ready to leave (an aircraft at full
/// health, anything else alive).
pub fn ready_count(world: &World, a: ActorId, def: DefId) -> usize {
    let Some(d) = books(world, a) else { return 0 };
    let aircraft = has(world, def, AIRCRAFT);
    let full = full_health(world, def);
    d.slots
        .iter()
        .filter(|s| s.occupant.is_none() && s.def == def)
        .filter(|s| if aircraft { s.health >= full } else { s.health > 0.0 })
        .count()
}

/// 0x568AA0: the first housed slot of `def` at full health, else the alive
/// one with the most health.
fn pick(world: &World, a: ActorId, def: DefId) -> Option<usize> {
    let d = books(world, a)?;
    let full = full_health(world, def);
    let mut best: Option<usize> = None;
    for (i, s) in d.slots.iter().enumerate() {
        if s.occupant.is_some() || s.def != def || !(s.health > 0.0) {
            continue;
        }
        if s.health >= full {
            return Some(i);
        }
        if best.is_none_or(|b| d.slots[b].health < s.health) {
            best = Some(i);
        }
    }
    best
}

/// The Denizen Tick 0x56E860 (queue 1; `dt` the elapsed seconds).
pub fn tick(world: &mut World, a: ActorId, dt: f32) {
    if books(world, a).is_none() || !world.actors.get(a as usize).is_some_and(|x| x.alive) {
        return;
    }
    values(world, a);
    update_deployed(world, a);
    // State 20 (under_siege) scales the regeneration by
    // DenizenResupplyRateUnderSiege; aa-play sets no under_siege (TickGuard).
    regenerate(world, a, dt);
    refresh_ready(world, a);
}

/// 0x569E40: +0x10 / +0x14, the occupants' current and full value. A housed
/// slot of a performer counts its definition's combat value (ActorData
/// +0x1E4) times its health ratio; a deployed one its actor (0x57DC40).
fn values(world: &mut World, a: ActorId) {
    let Some(d) = books(world, a) else { return };
    let (mut value, mut max) = (0.0f32, 0.0f32);
    for s in &d.slots {
        match s.occupant.filter(|&o| world.actor(o).is_some()) {
            None => {
                if !has(world, s.def, PERFORMER) {
                    continue;
                }
                let cv = crate::supply::performer_value(world, s.def).unwrap_or(0.0);
                let ratio = if has(world, s.def, BODY) { s.health / full_health(world, s.def) } else { 1.0 };
                value += ratio * cv;
                max += cv;
            }
            Some(o) => {
                let def = world.actors[o as usize].def;
                if let Some(cv) = crate::supply::performer_value(world, def) {
                    value += cv * crate::supply::health_ratio(world, o);
                    max += cv;
                }
            }
        }
    }
    if let Some(d) = books_mut(world, a) {
        d.value = value;
        d.value_max = max;
    }
}

/// UpdateDeployedDenizens 0x56E480, the part without escorts: each listed
/// mission (held by a local reference) is pruned (group virtual +0x14,
/// 0x5691A0); an empty one leaves the list (its node's reference and the
/// local one released, 0x56A190).
fn update_deployed(world: &mut World, a: ActorId) {
    let list = books(world, a).map(|d| d.missions.clone()).unwrap_or_default();
    for id in list {
        if !world.books.groups.contains_key(&id) {
            continue;
        }
        retain_group(world, id);
        if group_disbanded(world, id) {
            if let Some(d) = books_mut(world, a) {
                d.missions.retain(|&m| m != id);
            }
            release_group(world, id);
        }
        release_group(world, id);
    }
}

/// 0x568180: the mean of the first min(members, positions) FormationPositions
/// (binary32 sums times 1/used; NaN without positions, as 0 * (1/0)).
fn mean_created(formation: &[[f32; 3]], members: usize) -> [f32; 3] {
    let used = members.min(formation.len());
    let mut sum = [0.0f32; 3];
    for p in formation.iter().take(used) {
        for k in 0..3 {
            sum[k] += p[k];
        }
    }
    let scale = 1.0f32 / used as f32;
    [scale * sum[0], scale * sum[1], scale * sum[2]]
}

/// 0x5691A0's recomputation: the positions of the first min(members,
/// positions) survivors, times 1/members.
fn mean_pruned(formation: &[[f32; 3]], members: usize) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    for p in formation.iter().take(members) {
        for k in 0..3 {
            sum[k] += p[k];
        }
    }
    let scale = 1.0f32 / members as f32;
    [sum[0] * scale, sum[1] * scale, sum[2] * scale]
}

/// Group virtual +0x14 (0x5691A0): members whose GTID no longer resolves
/// are removed (0x50C510); when one was and members remain, +0x14 is
/// recomputed. True when no member is left.
pub fn group_disbanded(world: &mut World, id: u32) -> bool {
    let Some(m) = world.books.groups.get(&id) else { return true };
    let (base, group) = (m.base, m.group);
    let members = m.members.clone();
    let live: Vec<ActorId> = members.iter().copied().filter(|&x| world.actor(x).is_some()).collect();
    let formation = world.actors.get(base as usize).and_then(|b| world.data.def(b.def).denizen.as_ref()).and_then(|d| d.groups.get(group)).map(|g| g.formation.clone());
    if let Some(m) = world.books.groups.get_mut(&id) {
        let removed = live.len() != members.len();
        m.members = live;
        if removed && !m.members.is_empty()
            && let Some(f) = formation
        {
            m.offset = mean_pruned(&f, m.members.len());
        }
        m.members.is_empty()
    } else {
        true
    }
}

/// Group +4 increment.
pub fn retain_group(world: &mut World, id: u32) {
    if let Some(m) = world.books.groups.get_mut(&id) {
        m.refs += 1;
    }
}

/// Group +4 decrement; at zero the deleting destructor 0x569180 -> 0x5683E0
/// frees the member nodes and releases the escort group (+0x28).
pub fn release_group(world: &mut World, id: u32) {
    let Some(m) = world.books.groups.get_mut(&id) else { return };
    m.refs = m.refs.saturating_sub(1);
    if m.refs == 0 {
        let escort = m.escort;
        world.books.groups.remove(&id);
        if let Some(e) = escort {
            release_group(world, e);
        }
    }
}

/// Group +0xC: the members in list order (creation order), as actors.
pub fn group_members(world: &World, id: u32) -> Vec<ActorId> {
    world.books.groups.get(&id).map(|m| m.members.clone()).unwrap_or_default()
}

/// Group +8 (the DenizenGroup's FormationPositions; None for a null +8)
/// and the +0x14 mean offset.
pub fn group_layout(world: &World, id: u32) -> Option<(Option<Vec<[f32; 3]>>, [f32; 3])> {
    let m = world.books.groups.get(&id)?;
    let formation = world.actors.get(m.base as usize).and_then(|b| world.data.def(b.def).denizen.as_ref()).and_then(|d| d.groups.get(m.group)).map(|g| g.formation.clone());
    Some((formation, m.offset))
}

/// 0x569440: one housed Element slot of each 0x568880(2) flag heals per
/// tick, a damaged one (alive, below full) before a dead one; the worker
/// slot heals first.
fn regenerate(world: &mut World, a: ActorId, dt: f32) {
    let Some(d) = books(world, a) else { return };
    let eligible = |s: &DenizenSlot| s.occupant.is_none() && has(world, s.def, ELEMENT);
    let (mut plain, mut worker): (Option<usize>, Option<usize>) = (None, None);
    for (i, s) in d.slots.iter().enumerate() {
        if eligible(s) && s.health != 0.0 && s.health < full_health(world, s.def) {
            let pick = if s.worker { &mut worker } else { &mut plain };
            if pick.is_none() {
                *pick = Some(i);
            }
            if plain.is_some() && worker.is_some() {
                break;
            }
        }
    }
    if plain.is_none() || worker.is_none() {
        for (i, s) in d.slots.iter().enumerate() {
            if eligible(s) && s.health == 0.0 {
                let pick = if s.worker { &mut worker } else { &mut plain };
                if pick.is_none() {
                    *pick = Some(i);
                }
                if plain.is_some() && worker.is_some() {
                    break;
                }
            }
        }
    }
    let kingdom = world.actors[a as usize].kingdom;
    for i in [worker, plain].into_iter().flatten() {
        heal_slot(world, a, kingdom, i, dt);
    }
}

/// 0x568C80: health += full * resupply_rate (ActorData +0x2CC, 0x45D5B0)
/// * (1 - the owner's shortage ratio, Kingdom +0x1F0, when positive) * dt.
fn heal_slot(world: &mut World, a: ActorId, kingdom: Option<KingdomId>, i: usize, dt: f32) {
    let shortage = kingdom.and_then(|k| world.books.kingdom(k)).map_or(0.0, |k| k.shortage_ratio);
    let factor = if shortage > 0.0 { 1.0 - shortage } else { 1.0 };
    let Some(def) = books(world, a).and_then(|d| d.slots.get(i)).map(|s| s.def) else { return };
    let rate = world.data.def(def).resupply_rate;
    let full = full_health(world, def);
    if let Some(s) = books_mut(world, a).and_then(|d| d.slots.get_mut(i)) {
        s.health += full * rate * factor * dt;
    }
}

/// 0x5696B0: +0x3C = the groups with at least their minimum size ready,
/// plus every Organization behaviour.
fn refresh_ready(world: &mut World, a: ActorId) {
    let Some(dd) = denizen_def(world, a).cloned() else { return };
    let mut ready = 0;
    for g in &dd.groups {
        if ready_count(world, a, g.actor) as f32 >= g.size[0] {
            ready |= g.behavior;
        }
    }
    for &b in &dd.organization_behaviors {
        ready |= b;
    }
    if let Some(d) = books_mut(world, a) {
        d.ready = ready;
    }
}

/// CreateDenizen 0x56CC10(mode, 1, 1) on `base`: the first DenizenGroup of
/// `mode` with at least its minimum size ready fields min(size max, ready)
/// aircraft as a new mission (its id), or None.
pub fn create(world: &mut World, base: ActorId, mode: u32) -> Option<u32> {
    let dd = denizen_def(world, base)?.clone();
    for (gi, group) in dd.groups.iter().enumerate() {
        if group.behavior & mode == 0 {
            continue;
        }
        let ready = ready_count(world, base, group.actor);
        // fild / fcomp: fewer than the minimum (or an unordered one) skips.
        if !(ready as f32 >= group.size[0]) {
            continue;
        }
        // 0x96CC4C: the maximum truncated.
        let n = (group.size[1] as i32).min(ready as i32);
        let mut members = Vec::new();
        for _ in 0..n.max(0) {
            let Some(slot) = pick(world, base, group.actor) else { break };
            members.push(launch(world, base, &dd, slot, mode));
        }
        if members.is_empty() {
            continue;
        }
        // 0x568180: +8 the group, +0x20 its behaviour, +0x24 the base, the
        // members (the creation list reversed twice: creation order) and the
        // mean offset; each member Mind's +0x30 takes it (0x52A020).
        world.books.next_group = world.books.next_group.max(1);
        let id = world.books.next_group;
        world.books.next_group += 1;
        let offset = mean_created(&group.formation, members.len());
        world.books.groups.insert(id, Mission { id, base, refs: 0, members: members.clone(), group: gi, offset, mode: group.behavior, order: AirOrder::Recon(Vec2::ZERO), escort: None });
        for &m in &members {
            crate::combat::air::set_group(world, m, Some(id));
        }
        // The base's +0x40 list node holds a reference (head insertion).
        retain_group(world, id);
        if let Some(d) = books_mut(world, base) {
            d.missions.insert(0, id);
        }
        return Some(id);
    }
    None
}

/// One aircraft of 0x56CC10 from `slot`: created (0x4F9C10) at the hangar
/// (base frame 0x517DF0 of Runway hangar_offset, plus the aircraft's
/// taxi_height) facing the rotated hangar_orientation (0x517EE0), the slot
/// occupied with its health, home and mode on its Mind (0x52C280,
/// 0x51C220), and queued for the runway when the delay is positive
/// (0x5689A0).
fn launch(world: &mut World, base: ActorId, dd: &DenizenDef, slot: usize, mode: u32) -> ActorId {
    let (def, kingdom, origin, facing) = {
        let b = &world.actors[base as usize];
        (world.books.actor(base).and_then(|x| x.denizen.as_ref()).map_or(0, |d| d.slots[slot].def), b.kingdom, b.pos, b.facing)
    };
    let dir = Vec2::new(facing.cos(), facing.sin());
    let (q, o) = dd.runway.as_ref().map_or(([0.0; 3], [0.0; 2]), |r| (r.hangar_offset, r.hangar_orientation));
    let mut pos = origin + Vec3::new(dir.y, -dir.x, 0.0) * q[0] + Vec3::new(dir.x, dir.y, 0.0) * q[1] + Vec3::Z * q[2];
    pos.z += world.data.def(def).aircraft.as_ref().map_or(0.0, |x| x.taxi_height);
    let heading = Vec2::new(o[0] * dir.y + o[1] * dir.x, -dir.x * o[0] + o[1] * dir.y);
    let facing = if heading.length_squared() > 0.0 { heading.y.atan2(heading.x) } else { facing };
    let id = world.spawn_actor(def, kingdom, pos, facing);
    let now = world.time;
    world.books.actor_mut(id).created_at = now;
    world.actors[id as usize].health = crate::production::max_health(world, kingdom, def);
    let health = world.actors[id as usize].health;
    if let Some(s) = books_mut(world, base).and_then(|d| d.slots.get_mut(slot)) {
        s.occupant = Some(id);
        s.health = health;
    }
    // Actor +0x3C = ActorData +0x408 (taxi_scale), flag +8 bit 2; the Mind's
    // home (virtual +0x40, 0x52C280) and mode (virtual +0x3C, 0x51C220).
    let scale = world.data.def(def).aircraft.as_ref().map_or(1.0, |x| x.taxi_scale);
    if let Some(x) = world.books.actor_mut(id).aircraft.as_mut() {
        x.scale = scale;
        x.denizen = true;
    }
    crate::combat::air::set_home(world, id, Some(base));
    crate::combat::air::set_mode(world, id, mode);
    let delay = dd.runway.as_ref().map_or(0.0, |r| r.delay);
    if delay > 0.0 {
        let empty = books(world, base).is_some_and(|d| d.runway.is_empty());
        if empty {
            world.schedule_after(DEFERRED, Owner::Component(base, SLOT), 0, 0.0);
        }
        if let Some(d) = books_mut(world, base) {
            d.runway.push(id);
        }
        if let Some(x) = world.books.actor_mut(id).aircraft.as_mut() {
            x.hidden = true;
        }
    }
    id
}

/// The deferred release 0x5695B0: once the delay has passed since the last
/// release, the first aircraft still there leaves the hangar (its Mind
/// +0x184, 0x4D7EA0). The queue waits again while aircraft remain.
pub fn runway_release(world: &mut World, base: ActorId) {
    let Some(dd) = denizen_def(world, base).cloned() else { return };
    let delay = dd.runway.as_ref().map_or(0.0, |r| r.delay);
    let now = world.time;
    let Some(d) = books(world, base) else { return };
    if d.runway.is_empty() {
        return;
    }
    let mut next = delay + d.last_release;
    if next <= now {
        while let Some(first) = books_mut(world, base).and_then(|d| (!d.runway.is_empty()).then(|| d.runway.remove(0))) {
            if world.actor(first).is_some_and(|x| x.alive) && has(world, world.actors[first as usize].def, ELEMENT) {
                // Mind +0x184 (0x518980: takeoff), then 0x4D7EA0.
                crate::combat::air::release(world, first);
                if let Some(x) = world.books.actor_mut(first).aircraft.as_mut() {
                    x.hidden = false;
                }
                next = delay + now;
                if let Some(d) = books_mut(world, base) {
                    d.last_release = now;
                }
                break;
            }
        }
    }
    if books(world, base).is_some_and(|d| !d.runway.is_empty()) {
        world.schedule_after(DEFERRED, Owner::Component(base, SLOT), 0, next - now);
    }
}

/// 0x568480: the mission's +0x20 mode, then the command to every member
/// with a Mind (0x4D9B60: the kill command 0x4CA330 at an actor or a
/// point, the move command 0x4CA760 for recon).
fn use_mission(world: &mut World, mission: u32, order: AirOrder, mode: u32) {
    let Some(m) = world.books.groups.get_mut(&mission) else { return };
    m.mode = mode;
    m.order = order;
    for x in m.members.clone() {
        if world.actor(x).is_none() {
            continue;
        }
        crate::combat::air::command(world, x, order);
    }
}

struct Host<'a> {
    world: &'a mut World,
    base: ActorId,
    order: AirOrder,
}

type R<T> = aa_production::Result<T>;

fn floats(v: &[u32]) -> Vec<f32> {
    v.iter().map(|x| f32::from_bits(*x)).collect()
}

impl Purchase for Host<'_> {
    fn purchase_owner(&mut self) -> R<Option<u32>> {
        Ok(self.world.actors[self.base as usize].kingdom)
    }
    fn purchase_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> R<bool> {
        Ok(crate::production::can_afford(self.world, kingdom, &floats(cost)))
    }
    fn purchase_debit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        crate::production::credit(self.world, kingdom, &floats(cost), -1.0);
        Ok(())
    }
    fn purchase_credit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        crate::production::credit(self.world, kingdom, &floats(cost), 1.0);
        Ok(())
    }
}

impl Denizen for Host<'_> {
    /// ActorData +0x3F0 (AirAttackCost) / +0x3FC (AirReconCost), zero when
    /// not authored (0x45F180).
    fn denizen_cost(&mut self, purchase: DenizenPurchase) -> R<Vec<u32>> {
        let def = self.world.data.def(self.world.actors[self.base as usize].def);
        let cost = match purchase {
            DenizenPurchase::AirRecon => def.air_recon_cost.clone(),
            _ => def.air_attack_cost.clone(),
        };
        let n = self.world.data.resources.len();
        Ok(cost.unwrap_or_else(|| vec![0.0; n]).iter().map(|v| v.to_bits()).collect())
    }
    fn denizen_unaffordable(&mut self, purchase: DenizenPurchase, cost: &[u32], kingdom: u32) -> R<()> {
        let what = if purchase == DenizenPurchase::AirRecon { "denizen_air_recon" } else { "denizen_air_attack" };
        self.world.books.notices.push(Notice::DenizenUnaffordable { kingdom, building: self.base, what: what.into(), cost: floats(cost) });
        Ok(())
    }
    fn create_denizen(&mut self, _purchase: DenizenPurchase, mode: u32) -> R<Option<u32>> {
        Ok(create(self.world, self.base, mode))
    }
    fn use_denizen(&mut self, _purchase: DenizenPurchase, created: u32, mode: u32) -> R<()> {
        use_mission(self.world, created, self.order, mode);
        Ok(())
    }
}

/// 0x56D5A0 (attack an actor), 0x56D7C0 (attack a point) or 0x56DA00 (recon
/// a point) on `base`: false when unaffordable (a
/// `Notice::DenizenUnaffordable`). When affordable it returns true even if
/// no aircraft could be created; only a creation is paid for. Returns the
/// new mission when there is one.
pub fn purchase(world: &mut World, base: ActorId, order: AirOrder) -> (bool, Option<u32>) {
    let kind = match order {
        AirOrder::AttackActor(_) => DenizenPurchase::AirAttackTarget,
        AirOrder::AttackPoint(_) => DenizenPurchase::AirAttackLocation,
        _ => DenizenPurchase::AirRecon,
    };
    let before = world.books.next_group.max(1);
    let mut host = Host { world, base, order };
    let paid = aa_production::purchases::purchase_denizen(kind, &mut host).unwrap_or(false);
    let after = world.books.next_group.max(1);
    (paid, (after != before).then_some(before))
}

/// 0x4D7960 -> 0x568B10: a deployed denizen that dies (its home still
/// there) frees its slot with no health, to be rebuilt from nothing
/// (0x569440's second pass).
pub fn died(world: &mut World, a: ActorId) {
    let Some(home) = crate::combat::air::home(world, a) else { return };
    if let Some(s) = books_mut(world, home).and_then(|d| d.slots.iter_mut().find(|s| s.occupant == Some(a))) {
        s.occupant = None;
        s.health = 0.0;
    }
}

/// 0x4CE9C0 -> 0x569740: a denizen aircraft entering its base. Its slot is
/// housed again with full * (its Body health ratio). Returns false when no
/// slot holds it; the original then adopts it into a new slot (0x568C20 /
/// 0x568B80, not ported). The caller removes the actor (0x4F6760).
pub fn returned(world: &mut World, home: ActorId, a: ActorId) -> bool {
    let ratio = crate::supply::health_ratio(world, a);
    let Some(i) = books(world, home).and_then(|d| d.slots.iter().position(|s| s.occupant == Some(a))) else { return false };
    let full = books(world, home).map_or(0.0, |d| full_health(world, d.slots[i].def));
    if let Some(s) = books_mut(world, home).and_then(|d| d.slots.get_mut(i)) {
        s.occupant = None;
        s.health = full * ratio;
    }
    true
}

/// The live group (mission) `id`.
pub fn mission(world: &World, id: u32) -> Option<&Mission> {
    world.books.groups.get(&id)
}

/// The missions listed on `base` (+0x40), head first.
pub fn missions(world: &World, base: ActorId) -> Vec<u32> {
    books(world, base).map(|d| d.missions.clone()).unwrap_or_default()
}
