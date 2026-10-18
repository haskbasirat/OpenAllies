//! Original SAI goal-engine actor assignment (Source/Sai/SAIGoalEngine.cpp).
//!
//! `7A93B0` runs two per-goal passes over the priority-ordered goals:
//! evaluate `7A9150` (gather actors, then recompute the current priority and
//! the selected status) and assign `7A7E30` (commit actors from the pools and
//! trade actors with the other selected goals). Their callees are ported here:
//! `7A35D0`/`7A3740` (goals that name one required actor), `7A7EB0` (gather
//! from the pool, then from lower and higher priority goals: `7A3E10`,
//! `7A3F30`), `7A6C70` (assign from the pool, then trade: `7A50E0`, swap
//! `7A33E0`), the player's pool release `7AFA40` (mask test `7A1120`, list
//! removal `56A1F0`) and the pool compaction `7A6480` with predicate `7A2FB0`.
//!
//! Goal and actor virtuals are world services, called in native order with
//! the native arguments. The base goal implementations of add/remove actor
//! (`7A6930`/`7A6A90`) mutate the lists in `Roster`; every list walk here
//! follows node handles exactly like the native pointer walk (the next node is
//! read before or after the calls exactly where the original reads it).
//!
//! Actors are intrusive reference-counted objects natively (`+4` count,
//! destructor at vtable `+0`). The retain/release pairs around each walk only
//! keep the actor alive for the duration of the calls; ownership here is the
//! caller's arena, so they are not modeled (the oracle verifies no destructor
//! runs). Native assertion handlers (`855D30`) are reported as errors.
use super::engine::{Controller, GoalId};
use crate::{
    Error, Result,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub type ActorId = u32;
/// Available-actor pools at player `+54` (indexed by actor category).
pub const CATEGORIES: usize = 17;
/// Goal virtual `+8` result for goals that take no actors from a pool.
pub const NO_CATEGORY: u32 = 0x11;

pub type NodeId = usize;
#[derive(Clone, Copy, Debug)]
struct Node {
    actor: Option<ActorId>,
    next: Option<NodeId>,
}

/// Native `{head, tail, count}` list of `{actor, next}` nodes (goal `+0C`,
/// pool lists). Unlinked nodes stay addressable with their last contents, as
/// a freed native node is until its memory is reused.
#[derive(Clone, Debug, Default)]
pub struct ActorList {
    nodes: Vec<Node>,
    head: Option<NodeId>,
    tail: Option<NodeId>,
    /// `+8` count; maintained by the callers exactly as natively.
    pub count: i32,
}
impl ActorList {
    /// Builds a list in order (head first) with an explicit count word.
    pub fn from_actors(actors: impl IntoIterator<Item = Option<ActorId>>, count: i32) -> Self {
        let mut list = ActorList {
            count,
            ..Default::default()
        };
        for actor in actors {
            let id = list.nodes.len();
            list.nodes.push(Node { actor, next: None });
            match list.tail {
                Some(t) => list.nodes[t].next = Some(id),
                None => list.head = Some(id),
            }
            list.tail = Some(id);
        }
        list
    }
    pub fn head(&self) -> Option<NodeId> {
        self.head
    }
    pub fn actor(&self, node: NodeId) -> Option<ActorId> {
        self.nodes[node].actor
    }
    pub fn next(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node].next
    }
    /// Linked contents, head first.
    pub fn actors(&self) -> Vec<Option<ActorId>> {
        let mut out = Vec::new();
        let mut node = self.head;
        while let Some(i) = node {
            out.push(self.nodes[i].actor);
            node = self.nodes[i].next;
        }
        out
    }
    /// Actor held by the tail node (`None` when the tail pointer is null).
    pub fn tail_actor(&self) -> Option<Option<ActorId>> {
        self.tail.map(|t| self.nodes[t].actor)
    }
    /// `4B9E60` (new head node) followed by the callers' `tail = node when
    /// tail is null`. The count is the caller's.
    pub fn push_front(&mut self, actor: Option<ActorId>) {
        let id = self.nodes.len();
        self.nodes.push(Node {
            actor,
            next: self.head,
        });
        self.head = Some(id);
        if self.tail.is_none() {
            self.tail = Some(id);
        }
    }
    /// `56A1F0`: unlinks the first node holding `actor`; the count is the
    /// caller's.
    pub fn remove_first(&mut self, actor: Option<ActorId>) -> bool {
        let Some(head) = self.head else {
            return false;
        };
        if self.nodes[head].actor == actor {
            self.head = self.nodes[head].next;
            if self.head.is_none() {
                self.tail = None;
            }
            return true;
        }
        let mut prev = head;
        while let Some(node) = self.nodes[prev].next {
            if self.nodes[node].actor == actor {
                self.nodes[prev].next = self.nodes[node].next;
                if self.tail == Some(node) {
                    self.tail = Some(prev);
                }
                return true;
            }
            prev = node;
        }
        false
    }
    /// Clears a node's actor pointer (release then store 0).
    pub fn clear_actor(&mut self, node: NodeId) {
        self.nodes[node].actor = None;
    }
    /// `7A6480` with predicate `7A2FB0` (node actor is null): drops null nodes
    /// from the head, then from the rest, decrementing the count per node.
    pub fn remove_null(&mut self) {
        while let Some(head) = self.head {
            if self.nodes[head].actor.is_some() {
                break;
            }
            self.unlink_head();
            self.count -= 1;
        }
        let Some(mut keep) = self.head else {
            return;
        };
        let mut probe = self.nodes[keep].next;
        while let Some(node) = probe {
            probe = self.nodes[node].next;
            if self.nodes[node].actor.is_none() {
                self.count -= 1;
                self.unlink_after(keep);
            } else {
                keep = self.nodes[keep].next.expect("linked node");
            }
        }
    }
    /// `7A6480` head step: `head = head.next`, clearing the tail with the
    /// last node. The count is the caller's.
    pub fn unlink_head(&mut self) {
        if let Some(head) = self.head {
            self.head = self.nodes[head].next;
            if self.head.is_none() {
                self.tail = None;
            }
        }
    }
    /// `7A6480` inner step: unlinks `keep.next`, moving the tail back to
    /// `keep` when it was the tail. The count is the caller's.
    pub fn unlink_after(&mut self, keep: NodeId) {
        if let Some(node) = self.nodes[keep].next {
            self.nodes[keep].next = self.nodes[node].next;
            if self.tail == Some(node) {
                self.tail = Some(keep);
            }
        }
    }
    /// Whether any linked node holds `actor` (pointer search).
    pub fn contains(&self, actor: Option<ActorId>) -> bool {
        self.actors().contains(&actor)
    }
}
impl PartialEq for ActorList {
    fn eq(&self, other: &Self) -> bool {
        self.actors() == other.actors()
            && self.tail_actor() == other.tail_actor()
            && self.count == other.count
    }
}

/// SAI actor fields: `+10` owning goal, `+14` cached travel time (binary32,
/// -1 when invalid), `+18/+1C` category mask, `+30` cached route object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaiActor {
    pub owner: Option<GoalId>,
    pub categories: u64,
    pub travel_cache: u32,
    pub route: Option<u32>,
}

/// Player and goal actor bookkeeping.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Roster {
    /// Goal `+0C/+10/+14` actor lists.
    pub goal_actors: BTreeMap<GoalId, ActorList>,
    /// Player `+54`: one pool pointer per category (`None` = null pointer).
    pub pools: Vec<Option<ActorList>>,
    /// Player `+38`: actors withheld from the pools (`7AFB00` skips them).
    pub reserved: ActorList,
    pub actors: BTreeMap<ActorId, SaiActor>,
}
impl Roster {
    pub fn goal_list(&self, goal: GoalId) -> Result<&ActorList> {
        self.goal_actors
            .get(&goal)
            .ok_or_else(|| Error::World(format!("SAI goal {goal} has no actor list")))
    }
    pub fn goal_list_mut(&mut self, goal: GoalId) -> Result<&mut ActorList> {
        self.goal_actors
            .get_mut(&goal)
            .ok_or_else(|| Error::World(format!("SAI goal {goal} has no actor list")))
    }
    pub fn pool_mut(&mut self, category: u32) -> Result<&mut ActorList> {
        self.pools
            .get_mut(category as usize)
            .ok_or_else(|| {
                Error::World(format!(
                    "actor category {category} is outside the player pool array"
                ))
            })?
            .as_mut()
            .ok_or_else(|| Error::World(format!("null player pool {category}")))
    }
    pub fn actor_mut(&mut self, actor: Option<ActorId>) -> Result<&mut SaiActor> {
        let actor = actor.ok_or_else(|| Error::World("null SAI actor dereference".into()))?;
        self.actors
            .get_mut(&actor)
            .ok_or_else(|| Error::World(format!("missing SAI actor {actor}")))
    }
    pub fn actor(&self, actor: Option<ActorId>) -> Result<&SaiActor> {
        let actor = actor.ok_or_else(|| Error::World("null SAI actor dereference".into()))?;
        self.actors
            .get(&actor)
            .ok_or_else(|| Error::World(format!("missing SAI actor {actor}")))
    }
}

/// Goal fields and actor bookkeeping visible to the services.
pub struct State<'a> {
    pub ctl: &'a mut Controller,
    pub roster: &'a mut Roster,
}

/// The resource-array argument of add/remove actor: null, the caller's
/// running array, or the player's economy array (`economy+1C4`).
#[derive(Clone, Copy, Debug)]
pub enum Resources<'r> {
    Null,
    Running(&'r [u32]),
    Economy,
}

/// Goal and actor virtuals and engine services, in native call order.
/// Values are the unstored x87 results.
pub trait AssignWorld {
    /// Goal `+8`: pool category (`NO_CATEGORY` for none).
    fn category(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<u32>;
    /// Goal `+C`: has enough actors.
    fn satisfied(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<bool>;
    /// Goal `+14`.
    fn expired(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<bool>;
    /// Goal `+24(gained, lost, 1)`: value of trading `lost` for `gained`.
    fn swap_value(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        gained: Option<ActorId>,
        lost: Option<ActorId>,
    ) -> Result<Binary>;
    /// Goal `+28(actor, 1)`: value of the actor for the goal.
    fn value(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<ActorId>) -> Result<Binary>;
    /// Goal `+2C(actor)`: value of an actor the goal already holds.
    fn held_value(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
    ) -> Result<Binary>;
    /// Goal `+30`: nonzero when lower goals may take its actors.
    fn spare(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<i32>;
    /// Goal `+38`: the one actor the goal requires, if any.
    fn required_actor(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<Option<ActorId>>;
    /// Goal `+44`.
    fn critical(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<bool>;
    /// Goal `+64(actor, resources, a, b)` (base `7A6930`).
    fn add_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        a: bool,
        b: bool,
    ) -> Result<()>;
    /// Goal `+68(actor, resources, a, b, c)` (base `7A6A90`).
    #[allow(clippy::too_many_arguments)]
    fn remove_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        a: bool,
        b: bool,
        c: bool,
    ) -> Result<()>;
    /// Goal `+6C(actor)`.
    fn excluded(&mut self, s: &mut State<'_>, goal: GoalId, actor: Option<ActorId>)
    -> Result<bool>;
    /// Goal `+80` (base `7A5600`).
    fn deselect(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<()>;
    /// Actor `+C`.
    fn actor_dead(&mut self, s: &mut State<'_>, actor: ActorId) -> Result<bool>;
    /// Actor `+20(goal, 1)`.
    fn actor_can_serve(&mut self, s: &mut State<'_>, actor: ActorId, goal: GoalId) -> Result<bool>;
    /// `7A3860(resources, relaxed)` (ported as `goal_priority::affordable`).
    fn affordable(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        running: &[u32],
        relaxed: bool,
    ) -> Result<bool>;
    /// `7A3B00(0, 0)`: the actor/travel adjusted priority.
    fn priority_with_actors(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<Binary>;
    /// `79DF70(0)` on the SAI singleton.
    fn yield_point(&mut self, s: &mut State<'_>) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn positive(v: &Binary) -> bool {
    v.compare(&Binary::from_i32(0)) == Ordering::Greater
}
/// `fcomp dword`/`test ah,41`: strictly greater than an f32 (false unordered).
fn greater_bits(v: &Binary, bits: u32) -> Result<bool> {
    if host(bits).is_nan() {
        return Ok(false);
    }
    Ok(v.compare(&n(bits)?) == Ordering::Greater)
}
/// `fcomp dword`/`test ah,5`/`jp`: strictly less than an f32 (false unordered).
fn less_bits(v: &Binary, bits: u32) -> Result<bool> {
    if host(bits).is_nan() {
        return Ok(false);
    }
    Ok(v.compare(&n(bits)?) == Ordering::Less)
}
fn status(s: &State<'_>, goal: GoalId) -> Result<u32> {
    Ok(s.ctl.goal(goal)?.status)
}
fn current(s: &State<'_>, goal: GoalId) -> Result<u32> {
    Ok(s.ctl.goal(goal)?.current)
}
fn base(s: &State<'_>, goal: GoalId) -> Result<u32> {
    Ok(s.ctl.goal(goal)?.base)
}
fn owner(s: &State<'_>, actor: ActorId) -> Result<Option<GoalId>> {
    Ok(s.roster.actor(Some(actor))?.owner)
}
fn list_head(s: &State<'_>, goal: GoalId) -> Result<Option<NodeId>> {
    Ok(s.roster.goal_list(goal)?.head())
}
fn list_node(
    s: &State<'_>,
    goal: GoalId,
    node: NodeId,
) -> Result<(Option<ActorId>, Option<NodeId>)> {
    let list = s.roster.goal_list(goal)?;
    Ok((list.actor(node), list.next(node)))
}
fn pool_node(
    s: &mut State<'_>,
    category: u32,
    node: NodeId,
) -> Result<(Option<ActorId>, Option<NodeId>)> {
    let pool = s.roster.pool_mut(category)?;
    Ok((pool.actor(node), pool.next(node)))
}
fn order_goal(order: &[GoalId], k: i64) -> Result<GoalId> {
    usize::try_from(k)
        .ok()
        .and_then(|k| order.get(k).copied())
        .ok_or_else(|| Error::World(format!("goal order index {k} out of range")))
}
/// `fld [goal+34]`/`fcomp 0`/`test ah,41`/`jnp`: current <= 0 (ordered).
fn not_positive(bits: u32) -> bool {
    host(bits) <= 0.0
}

/// Recomputes `+34` the way `7A9150`, `7A6C70`, `7A6930` and `7A6A90` do:
/// `affordable(resources, 0) ? (float)7A3B00(0, 0) : 0`.
fn refresh_current(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    running: &[u32],
) -> Result<u32> {
    let value = if world.affordable(s, goal, running, false)? {
        store(world.priority_with_actors(s, goal)?)?
    } else {
        0
    };
    s.ctl.goal_mut(goal)?.current = value;
    Ok(value)
}

/// Complete `7AFA40` (player method): removes `actor` from every pool whose
/// category bit (`7A1120`) is set, except the goal's own category when a goal
/// is given (native flag 1; flag 0 skips nothing, i.e. category `0x11`).
pub fn release_from_pools(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    actor: Option<ActorId>,
    goal: Option<GoalId>,
) -> Result<()> {
    let skip = match goal {
        Some(goal) => world.category(s, goal)?,
        None => NO_CATEGORY,
    };
    for i in 0..CATEGORIES as u32 {
        if i == skip {
            continue;
        }
        let mask = s.roster.actor(actor)?.categories;
        if mask & (1_u64 << i) == 0 {
            continue;
        }
        let pool = s.roster.pool_mut(i)?;
        if pool.remove_first(actor) {
            pool.count -= 1;
        }
    }
    Ok(())
}

/// Complete `7A9150` (goal evaluate pass of `7A93B0`).
pub fn evaluate(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    running: &mut [u32],
    order: &[GoalId],
    index: usize,
) -> Result<()> {
    if world.expired(s, goal)? {
        return world.deselect(s, goal);
    }
    if world.required_actor(s, goal)?.is_some() {
        evaluate_required(world, s, goal)?;
    } else if world.category(s, goal)? != NO_CATEGORY {
        gather(world, s, goal, order, index)?;
    }
    if host(base(s, goal)?) > 0.0 {
        let value = refresh_current(world, s, goal, running)?;
        if not_positive(value) && status(s, goal)? == 2 {
            world.deselect(s, goal)?;
        }
    }
    if not_positive(current(s, goal)?) {
        world.deselect(s, goal)?;
    } else {
        s.ctl.goal_mut(goal)?.status = 2;
    }
    world.yield_point(s)
}

/// Complete `7A7E30` (goal assign pass of `7A93B0`); `p` is the x87
/// precision of the swap sums in `7A33E0`.
pub fn assign(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    running: &mut [u32],
    order: &[GoalId],
    index: usize,
    p: Precision,
) -> Result<()> {
    if world.expired(s, goal)? {
        return world.deselect(s, goal);
    }
    if world.required_actor(s, goal)?.is_some() {
        assign_required(world, s, goal, running, order)?;
        return world.yield_point(s);
    }
    if world.category(s, goal)? != NO_CATEGORY {
        assign_from_pool(world, s, goal, running, order, index, p)?;
    }
    world.yield_point(s)
}

/// Complete `7A35D0`: claim the required actor when it is free, or take it
/// from its owner when the owner values it below this goal's base priority.
pub fn evaluate_required(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
) -> Result<()> {
    let Some(req) = world.required_actor(s, goal)? else {
        return Ok(());
    };
    let holder = owner(s, req)?;
    if world.actor_dead(s, req)? || holder == Some(goal) {
        return Ok(());
    }
    if !positive(&world.value(s, goal, Some(req))?) {
        return Ok(());
    }
    match holder {
        None => {
            release_from_pools(world, s, Some(req), None)?;
            world.add_actor(s, goal, Some(req), Resources::Null, false, false)
        }
        Some(holder) => {
            let held = world.held_value(s, holder, Some(req))?;
            if !less_bits(&held, base(s, goal)?)? {
                return Ok(());
            }
            world.remove_actor(s, holder, Some(req), Resources::Economy, true, false, false)?;
            world.add_actor(s, goal, Some(req), Resources::Null, false, false)?;
            if not_positive(current(s, holder)?) || world.critical(s, holder)? {
                world.deselect(s, holder)?;
            }
            Ok(())
        }
    }
}

/// Complete `7A3740`: hand the required actor to a selected goal that values
/// it above this goal's base priority (this goal then deselects).
pub fn assign_required(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    running: &mut [u32],
    order: &[GoalId],
) -> Result<()> {
    let Some(req) = world.required_actor(s, goal)? else {
        return Ok(());
    };
    let mut k = 0;
    while owner(s, req)? == Some(goal) && k < order.len() {
        let other = order[k];
        if other != goal && status(s, other)? == 2 && !world.excluded(s, other, Some(req))? {
            let v = world.value(s, other, Some(req))?;
            if positive(&v) && greater_bits(&v, base(s, goal)?)? {
                world.remove_actor(s, goal, Some(req), Resources::Null, false, false, false)?;
                world.add_actor(
                    s,
                    other,
                    Some(req),
                    Resources::Running(running),
                    true,
                    false,
                )?;
                world.deselect(s, goal)?;
            }
        }
        k += 1;
    }
    Ok(())
}

/// Complete `7A7EB0` (evaluate-time gathering): take positive-value actors
/// from the goal's pool, then from lower goals (`7A3E10`, last to `index+1`)
/// and higher goals (`7A3F30`, `index-1` to first) until satisfied.
pub fn gather(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    order: &[GoalId],
    index: usize,
) -> Result<()> {
    let category = world.category(s, goal)?;
    let mut node = s.roster.pool_mut(category)?.head();
    while !world.satisfied(s, goal)? {
        let Some(at) = node else { break };
        let (actor, _) = pool_node(s, category, at)?;
        if actor.is_none() {
            return Err(Error::World(
                "SAIGoalEngine assertion (855D30): null actor in the goal's pool".into(),
            ));
        }
        if !world.excluded(s, goal, actor)? && positive(&world.value(s, goal, actor)?) {
            s.roster.pool_mut(category)?.clear_actor(at);
            release_from_pools(world, s, actor, Some(goal))?;
            world.add_actor(s, goal, actor, Resources::Null, false, false)?;
        }
        node = pool_node(s, category, at)?.1;
    }
    s.roster.pool_mut(category)?.remove_null();
    let index = index as i64;
    let mut k = order.len() as i64 - 1;
    while !world.satisfied(s, goal)? && k > index {
        take_from_lower(world, s, goal, order_goal(order, k)?)?;
        k -= 1;
    }
    let mut k = index - 1;
    while !world.satisfied(s, goal)? && k >= 0 {
        take_from_higher(world, s, goal, order_goal(order, k)?)?;
        k -= 1;
    }
    Ok(())
}

/// Complete `7A3E10`: take every positive-value actor from a lower priority
/// goal that offers spare actors (`+30`).
pub fn take_from_lower(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    other: GoalId,
) -> Result<()> {
    if world.spare(s, other)? == 0 {
        return Ok(());
    }
    let mut node = list_head(s, other)?;
    while !world.satisfied(s, goal)? {
        let Some(at) = node else { break };
        let (actor, next) = list_node(s, other, at)?;
        node = next;
        let Some(id) = actor else { continue };
        if world.actor_dead(s, id)? || world.excluded(s, goal, actor)? {
            continue;
        }
        if positive(&world.value(s, goal, actor)?) {
            world.remove_actor(s, other, actor, Resources::Null, false, false, false)?;
            world.add_actor(s, goal, actor, Resources::Null, false, false)?;
        }
    }
    Ok(())
}

/// Complete `7A3F30`: take actors from a higher priority goal while each one
/// is worth more here than there; the first actor that is not ends the scan.
pub fn take_from_higher(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    other: GoalId,
) -> Result<()> {
    let mut node = list_head(s, other)?;
    if world.satisfied(s, goal)? {
        return Ok(());
    }
    while let Some(at) = node {
        let (actor, next) = list_node(s, other, at)?;
        node = next;
        if !world.excluded(s, goal, actor)? {
            let v = world.value(s, goal, actor)?;
            let stored = store(v.clone())?;
            if positive(&v) {
                let held = world.held_value(s, other, actor)?;
                if !less_bits(&held, stored)? {
                    return Ok(());
                }
                world.remove_actor(s, other, actor, Resources::Null, false, false, false)?;
                world.add_actor(s, goal, actor, Resources::Null, false, false)?;
                if world.critical(s, other)? {
                    world.deselect(s, goal)?;
                }
            }
        }
        if world.satisfied(s, goal)? {
            return Ok(());
        }
    }
    Ok(())
}

/// Complete `7A6C70` (assign-time commit): take positive-value actors from the
/// pool, refresh the current priority, then trade with every later selected
/// goal in both directions (`7A50E0`) while this goal stays selected.
pub fn assign_from_pool(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    goal: GoalId,
    running: &mut [u32],
    order: &[GoalId],
    index: usize,
    p: Precision,
) -> Result<()> {
    let first = world.category(s, goal)?;
    if s.roster.pool_mut(first)?.head().is_some() {
        let category = world.category(s, goal)?;
        let mut node = s.roster.pool_mut(category)?.head();
        let mut any = false;
        while let Some(at) = node {
            let (actor, _) = pool_node(s, category, at)?;
            if !world.excluded(s, goal, actor)? && positive(&world.value(s, goal, actor)?) {
                s.roster.pool_mut(category)?.clear_actor(at);
                release_from_pools(world, s, actor, Some(goal))?;
                world.add_actor(s, goal, actor, Resources::Running(running), false, false)?;
                any = true;
            }
            node = pool_node(s, category, at)?.1;
        }
        if any {
            if host(base(s, goal)?) > 0.0 {
                let value = refresh_current(world, s, goal, running)?;
                if not_positive(value) && status(s, goal)? == 2 {
                    world.deselect(s, goal)?;
                }
            }
            s.roster.pool_mut(category)?.remove_null();
        }
    }
    let mut k = index + 1;
    while status(s, goal)? == 2 && k < order.len() {
        let other = order[k];
        if status(s, other)? == 2 && world.category(s, other)? != NO_CATEGORY {
            trade(world, s, goal, other, running, p)?;
            world.yield_point(s)?;
            trade(world, s, other, goal, running, p)?;
            world.yield_point(s)?;
            if not_positive(current(s, goal)?) || world.critical(s, goal)? {
                world.deselect(s, goal)?;
            }
            if not_positive(current(s, other)?) || world.critical(s, other)? {
                world.deselect(s, other)?;
            }
        }
        k += 1;
    }
    Ok(())
}

/// Complete `7A50E0`: move each of `from`'s actors to `to` when `to` values it
/// positively and above `from`'s held value; otherwise try a swap (`7A33E0`).
pub fn trade(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    from: GoalId,
    to: GoalId,
    running: &mut [u32],
    p: Precision,
) -> Result<()> {
    let mut node = list_head(s, from)?;
    while let Some(at) = node {
        let (actor, next) = list_node(s, from, at)?;
        node = next;
        if !world.excluded(s, to, actor)? {
            let v = world.value(s, to, actor)?;
            let stored = store(v.clone())?;
            if positive(&v) {
                let held = world.held_value(s, from, actor)?;
                if less_bits(&held, stored)? {
                    world.remove_actor(
                        s,
                        from,
                        actor,
                        Resources::Running(running),
                        true,
                        true,
                        false,
                    )?;
                    world.add_actor(s, to, actor, Resources::Running(running), true, true)?;
                    continue;
                }
            }
        }
        swap(world, s, from, to, actor, running, p)?;
    }
    Ok(())
}

/// Complete `7A33E0` at the given x87 precision: exchange `actor` (held by
/// `a`) for the actor of `b` with the largest positive combined trade value.
pub fn swap(
    world: &mut impl AssignWorld,
    s: &mut State<'_>,
    a: GoalId,
    b: GoalId,
    actor: Option<ActorId>,
    running: &mut [u32],
    p: Precision,
) -> Result<bool> {
    let id = actor.ok_or_else(|| Error::World("null SAI actor dereference".into()))?;
    if !world.actor_can_serve(s, id, b)? {
        return Ok(false);
    }
    let mut best = 0_u32;
    let mut candidate: Option<ActorId> = None;
    let mut node = list_head(s, b)?;
    while let Some(at) = node {
        let (other, _) = list_node(s, b, at)?;
        if let Some(other_id) = other
            && !world.actor_dead(s, other_id)?
            && !world.excluded(s, a, other)?
            && !world.excluded(s, b, actor)?
        {
            let gain_b = store(world.swap_value(s, b, actor, other)?)?;
            let gain_a = world.swap_value(s, a, other, actor)?;
            let sum = num(gain_a.add(&n(gain_b)?, p))?;
            if greater_bits(&sum, best)? {
                best = store(sum)?;
                candidate = other;
            }
        }
        node = list_node(s, b, at)?.1;
    }
    if candidate.is_none() {
        return Ok(false);
    }
    world.remove_actor(s, a, actor, Resources::Running(running), true, true, false)?;
    world.remove_actor(
        s,
        b,
        candidate,
        Resources::Running(running),
        true,
        true,
        false,
    )?;
    world.add_actor(s, a, candidate, Resources::Running(running), true, true)?;
    world.add_actor(s, b, actor, Resources::Running(running), true, true)?;
    Ok(true)
}
