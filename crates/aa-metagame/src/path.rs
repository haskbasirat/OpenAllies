//! Metagame territory pathfinder: the generic greedy search used by army move
//! and attack validation/processing and by the MGAI, plus the army entry
//! points 93FDC0 / 93FCC0 / 93FAD0 and the moves-used count 93EAE0.
//!
//! Ported from the original machine code (addresses of the 32-bit AA.exe);
//! proven by `tests/native_path.rs` against `research/phase8-metagame/
//! native-path.json` (original code under Unicorn at x87 PC24/PC53/PC64).
//!
//! Native objects (the search framework is generic; only the metagame
//! instance is ported):
//! - path object (ctor 961550, vtable AC7E94, 0x50 bytes): +0x08 open set,
//!   +0x0C node store, +0x10 best node, +0x14 best heuristic (binary32),
//!   +0x18 popped node, +0x1C popped heap entry, +0x2C/+0x30 neighbour
//!   (index/territory), +0x34 start, +0x38 goal, +0x3C kingdom actor,
//!   +0x40 budget. Slot 0 = 93ED00 (expansion), slot 1 = 7C7840 (always 3 =
//!   continue), slot 2 = 93E9B0 (-> store 960CC0, set parent), slot 3 =
//!   4C9FA0 (0). Destroyed by 93E930.
//! - node store (961230, vtable ACDD68, 0x28 bytes): +0x08 start index,
//!   +0x0C goal index, +0x10 kingdom actor, +0x14 budget, +0x18 parent array
//!   (one i32 per MG territory, -1 = undiscovered), +0x1C/+0x20 result path
//!   (MG territories). Virtuals used: 9612F0 is-goal, 960CE0 discovered,
//!   960D00 blocked, 960CC0 set parent, 9616C0 build path (960DB0 reverse),
//!   960E00 depth limit, 53E5A0 entry depth, 960E20 link cost, 960E70
//!   heuristic, 960D40/960D50 set start/goal, 960F30 preconditions, 960D60
//!   trivial (start == goal or adjacent), 960DF0 clear path.
//! - open set (961430: vtable ACDDC0, inner binary heap 53E7B0 with capacity
//!   = territory count and maximum `+0x0C = 0` = unlimited): 16-byte entries
//!   {node, depth, key binary32, link cost binary32}; push 53F570/53F350, pop
//!   53F400/53F410, clear 9615D0/961620/961670.
//! - 960FB0 (search loop) is guarded by the global byte B3D43D (re-entrancy:
//!   a nested search returns 0). Every exit clears it, so it is not state of
//!   a completed search and is not reproduced.
//!
//! Every float rule takes the x87 precision explicitly; the game passes
//! [`crate::GAME_PRECISION`]. Only the heuristic (960E70) and the key sum
//! (53F570) are float; heap compares are exact binary32 compares.
//!
//! MG territories are referenced by index (`MgState::territories`); links are
//! map definitions (`TerritoryDefinition::links`, link `+0x04` owner = the
//! definition holding the link (9583C0 -> 962530), `+0x08` target).
use crate::battle::{KingdomActor, KingdomRelations};
use crate::data::{MapDefinition, MetaGameData, TerritoryKind};
use crate::state::{ArmyId, MgState};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Heuristic scale `0x3D888889` (about 1/15) read from AA0E34 by 960E70.
pub const HEURISTIC_SCALE_BITS: u32 = 0x3d88_8889;
/// 961350 result: path found (only result accepted by 93FCC0/93FAD0).
pub const SEARCH_FOUND: i32 = 1;
/// 961350 result: failure (preconditions, exhausted open set, depth limit
/// without progress, or a found path longer than the budget).
pub const SEARCH_FAILED: i32 = 0;
/// 961350 result: depth limit reached after the best node moved off the start.
pub const SEARCH_DEPTH_LIMIT: i32 = 2;
/// Budget value meaning "no depth limit" (960E00, 961350 compare with -1).
pub const UNLIMITED: i32 = -1;

/// Result of one search (961550 + 961350).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathSearch {
    /// 961350 return value ([`SEARCH_FOUND`], [`SEARCH_FAILED`], [`SEARCH_DEPTH_LIMIT`]).
    pub code: i32,
    /// Node store path `+0x1C` after the run: MG territories from the first
    /// step to the goal (the start is not included; empty when start ==
    /// goal). 961350 can reject a found path (code 0) without clearing it.
    pub path: Vec<usize>,
}

/// One open-set entry (16 bytes, 53F570 layout).
#[derive(Clone, Copy, Debug)]
struct OpenEntry {
    node: usize,
    depth: i32,
    key_bits: u32,
    #[allow(dead_code)]
    cost_bits: u32,
}

/// Numeric order of two finite binary32 values (both loaded exactly by
/// `fld dword` / `fcomp dword`); signed zeros compare equal.
fn f32_order(a: u32, b: u32) -> Ordering {
    let key = |bits: u32| -> i64 {
        let magnitude = i64::from(bits & 0x7fff_ffff);
        if bits >> 31 != 0 { -magnitude } else { magnitude }
    };
    key(a).cmp(&key(b))
}

/// The binary heap of 53E7B0 (0x10-byte elements).
#[derive(Default)]
struct OpenHeap(Vec<OpenEntry>);

impl OpenHeap {
    /// `FUN_0053F350`: append, then move the hole up while the new key is
    /// `<=` the parent key (`fcomp`, `test ah,0x41`, `jp` stops on greater or
    /// unordered). Never full: the 961430 heap maximum `+0x0C` is 0.
    fn push(&mut self, entry: OpenEntry) {
        let mut hole = self.0.len();
        self.0.push(entry);
        while hole > 0 {
            let parent = (hole - 1) / 2;
            if f32_order(entry.key_bits, self.0[parent].key_bits) == Ordering::Greater {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = entry;
    }

    /// `FUN_0053F410`: take the root; the last entry sifts down from the
    /// root over the remaining `count - 1` entries: the right child is
    /// chosen unless left < right (`test ah,5`, `jp`), the child moves up
    /// while last > child (stop on `<`, `==` or unordered).
    fn pop(&mut self) -> Option<OpenEntry> {
        let top = *self.0.first()?;
        let last_index = self.0.len() - 1;
        let last = self.0[last_index];
        let mut hole = 0usize;
        if last_index > 1 {
            let mut left = 1usize;
            loop {
                let right = 2 * hole + 2;
                let child = if right < last_index
                    && f32_order(self.0[left].key_bits, self.0[right].key_bits) != Ordering::Less
                {
                    right
                } else {
                    left
                };
                if f32_order(last.key_bits, self.0[child].key_bits) != Ordering::Greater {
                    break;
                }
                self.0[hole] = self.0[child];
                hole = child;
                left = 2 * child + 1;
                if left >= last_index {
                    break;
                }
            }
        }
        self.0[hole] = last;
        self.0.truncate(last_index);
        Some(top)
    }
}

/// MG territory graph of one state: map definitions, and the MG territory
/// of each definition as the state territory hash (`MGState+0x34`, filled by
/// InitCreate, looked up by 939B60 -> 4154D0 with `_wcsicmp` 96CAD6 on the
/// definition IDS) resolves it.
pub struct PathGraph<'a> {
    state: &'a MgState,
    map: &'a MapDefinition,
    /// Definition index -> MG territory: `Ok(t)`, or why the native lookup
    /// cannot be reproduced (miss = native null dereference, ambiguous IDS).
    by_definition: Vec<std::result::Result<usize, &'static str>>,
}

impl<'a> PathGraph<'a> {
    pub fn new(state: &'a MgState, data: &'a MetaGameData) -> Result<Self> {
        let campaign = data
            .campaigns
            .get(state.campaign)
            .ok_or_else(|| Error::Unsupported(format!("campaign {}", state.campaign)))?;
        let map = campaign
            .map
            .and_then(|m| data.maps.get(m))
            .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
        // _wcsicmp in the C locale folds only 'A'..='Z'.
        let fold = |s: &str| s.to_ascii_lowercase();
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (t, territory) in state.territories.iter().enumerate() {
            let def = map.territories.get(territory.definition).ok_or_else(|| {
                Error::Unsupported(format!("MG territory {t} definition out of range"))
            })?;
            index.entry(fold(&def.ids)).or_default().push(t);
        }
        let by_definition = map
            .territories
            .iter()
            .map(|d| match index.get(&fold(&d.ids)).map(Vec::as_slice) {
                Some([t]) => Ok(*t),
                None => Err("939B60 lookup miss (native null dereference)"),
                Some(_) => Err("939B60 lookup with several MG territories of one IDS (hash order)"),
            })
            .collect();
        Ok(Self { state, map, by_definition })
    }

    fn definition(&self, territory: usize) -> usize {
        self.state.territories[territory].definition
    }

    fn check(&self, territory: usize) -> Result<()> {
        if territory < self.state.territories.len() {
            Ok(())
        } else {
            Err(Error::Unsupported(format!("MG territory {territory} out of range")))
        }
    }

    /// `FUN_00939B60` (by definition IDS) then `+0x04` (index).
    pub fn territory_of_definition(&self, definition: usize) -> Result<usize> {
        match self.by_definition.get(definition) {
            Some(Ok(t)) => Ok(*t),
            Some(Err(why)) => Err(Error::Unsupported((*why).into())),
            None => Err(Error::Unsupported(format!("definition {definition} out of range"))),
        }
    }

    fn links(&self, territory: usize) -> &'a [crate::data::TerritoryLink] {
        &self.map.territories[self.definition(territory)].links
    }

    fn kind(&self, territory: usize) -> TerritoryKind {
        self.map.territories[self.definition(territory)].kind
    }

    /// `FUN_00957910` (first link of `from`'s definition whose target is
    /// `to`'s definition) then `FUN_00962310` (link owner definition type !=
    /// ALIAS): 1, 0 for links out of an alias; `None` without a link.
    fn link_cost(&self, from: usize, to: usize) -> Option<i32> {
        let target = self.definition(to);
        self.links(from)
            .iter()
            .any(|l| l.target == target)
            .then(|| i32::from(self.kind(from) != TerritoryKind::Alias))
    }

    /// `FUN_00960E70`: 0.0 (9F32DC) at the goal, else
    /// `(|x - gx| + |y - gy|) * 0x3D888889` on the definition locations
    /// `+0x14/+0x18`, every x87 op
    /// rounded to `precision` (`fsub`, `fcom 0` + `fchs`, `faddp`, `fmul`);
    /// returned in ST0, unrounded to binary32 (callers store it with `fstp
    /// dword` except the best-node compare in 960FB0).
    pub fn heuristic(&self, node: usize, goal: usize, precision: Precision) -> Result<Binary> {
        self.check(node)?;
        self.check(goal)?;
        if node == goal {
            return Ok(Binary::from_f32_bits(0)?);
        }
        let a = self.map.territories[self.definition(node)].location;
        let b = self.map.territories[self.definition(goal)].location;
        let abs_diff = |x: u32, y: u32| -> Result<Binary> {
            let d = Binary::from_f32_bits(x)?.sub(&Binary::from_f32_bits(y)?, precision)?;
            Ok(if d.is_negative_nonzero() { d.negated() } else { d })
        };
        let sum = abs_diff(a[0], b[0])?.add(&abs_diff(a[1], b[1])?, precision)?;
        Ok(sum.mul(&Binary::from_f32_bits(HEURISTIC_SCALE_BITS)?, precision)?)
    }

    /// `FUN_00954B30` on `territory` with `actor`: no owner, or the owner's
    /// kingdom actor regards `actor` as an enemy (vtable+0x140).
    pub fn hostile<R: KingdomRelations>(&self, relations: &R, territory: usize, actor: KingdomActor) -> Result<bool> {
        match self.state.territories[territory].owner {
            None => Ok(true),
            Some(k) => {
                let owner = self.state.kingdoms[k].actor.ok_or_else(|| {
                    Error::Unsupported("954B30: owner kingdom without actor (native null call)".into())
                })?;
                Ok(relations.enemy(owner, actor))
            }
        }
    }

    /// `FUN_00960D60`: start == goal, or the start definition has a link to
    /// the goal definition (957970, identity of the target definition).
    fn trivial(&self, start: usize, goal: usize) -> bool {
        start == goal || {
            let target = self.definition(goal);
            self.links(start).iter().any(|l| l.target == target)
        }
    }

    /// `FUN_009616C0` with `arg`: clear (+0x20 = 0); trivial: `[goal]`, or
    /// empty when start == goal; otherwise only when `arg` is the goal, the
    /// parent chain from the goal up to the start (excluded), reversed
    /// (960DB0).
    fn build_path(&self, parent: &[Option<usize>], start: usize, goal: usize, arg: usize) -> Result<Vec<usize>> {
        if self.trivial(start, goal) {
            return Ok(if goal == start { Vec::new() } else { vec![goal] });
        }
        let mut path = Vec::new();
        if arg != goal {
            return Ok(path);
        }
        let mut t = goal;
        while t != start {
            path.push(t);
            t = parent[t].ok_or_else(|| {
                Error::Unsupported("9616C0: parent -1 in the chain (native reads territory -1)".into())
            })?;
        }
        path.reverse();
        Ok(path)
    }

    /// The search as 93FCC0/93FAD0 run it: `961550(start, goal,
    /// kingdom_actor, budget)` then `961350`.
    ///
    /// 961350: start/goal/actor/budget into the node store; preconditions
    /// 960F30 (budget 0 requires start == goal; both definitions need at
    /// least one link) else 0; search 960FB0; a result whose path count is
    /// greater than the budget (signed, budget != -1) becomes 0 (the path is
    /// left in the store).
    pub fn search<R: KingdomRelations>(
        &self,
        relations: &R,
        start: usize,
        goal: usize,
        kingdom_actor: Option<KingdomActor>,
        budget: i32,
        precision: Precision,
    ) -> Result<PathSearch> {
        self.check(start)?;
        self.check(goal)?;
        if (budget == 0 && start != goal) || self.links(start).is_empty() || self.links(goal).is_empty() {
            return Ok(PathSearch { code: SEARCH_FAILED, path: Vec::new() });
        }
        let (mut code, path) = self.run(relations, start, goal, kingdom_actor, budget, precision)?;
        if path.len() as i64 > i64::from(budget) && budget != UNLIMITED {
            code = SEARCH_FAILED;
        }
        Ok(PathSearch { code, path })
    }

    /// `FUN_00960FB0`, greedy best-first search keyed by `f32(link cost +
    /// f32(h))` (53F570 sums at `precision`, stores binary32):
    ///
    /// 1. parent[start] = start (93E9B0); trivial (960D60) or start == goal
    ///    (9612F0) -> path 9616C0, result 1.
    /// 2. push {start, depth 0, cost 0.0 (4DB6B0), h(start)}; best = start,
    ///    best h = binary32(h(start)).
    /// 3. pop (empty -> 9616C0(best), result 0); the popped node becomes
    ///    best when its unrounded h < best h (`fcom dword`, then stored as
    ///    binary32); 7C7840 returns 3 (continue); depth > budget (960E00,
    ///    not for -1) -> result 0 when best is the start (4F0590), else
    ///    9616C0(best) and 2.
    /// 4. for every link of the popped definition in order (957960, target
    ///    MG territory via 939B60), 93ED00: skip discovered (960CE0); set
    ///    parent (960CC0, also for blocked ones); the goal -> 9616C0, result 1
    ///    (goal test before the blocked test); skip blocked (960D00: actor
    ///    given, not the goal, 954B30 hostile); push {n, depth + 1, cost
    ///    (960E20, `fild`), binary32(h(n))} (never full; the native insert
    ///    failure result 2 is unreachable).
    fn run<R: KingdomRelations>(
        &self,
        relations: &R,
        start: usize,
        goal: usize,
        kingdom_actor: Option<KingdomActor>,
        budget: i32,
        precision: Precision,
    ) -> Result<(i32, Vec<usize>)> {
        let mut parent: Vec<Option<usize>> = vec![None; self.state.territories.len()];
        parent[start] = Some(start);
        if self.trivial(start, goal) || start == goal {
            return Ok((SEARCH_FOUND, self.build_path(&parent, start, goal, start)?));
        }
        let mut heap = OpenHeap::default();
        let h_start = self.heuristic(start, goal, precision)?.store_f32()?;
        let zero = Binary::from_f32_bits(0)?;
        heap.push(OpenEntry {
            node: start,
            depth: 0,
            key_bits: zero.add(&Binary::from_f32_bits(h_start)?, precision)?.store_f32()?,
            cost_bits: 0,
        });
        let mut best = start;
        let mut best_bits = h_start;
        loop {
            let Some(item) = heap.pop() else {
                return Ok((SEARCH_FAILED, self.build_path(&parent, start, goal, best)?));
            };
            let node = item.node;
            let h = self.heuristic(node, goal, precision)?;
            if h.compare(&Binary::from_f32_bits(best_bits)?) == Ordering::Less {
                best_bits = h.store_f32()?;
                best = node;
            }
            if budget != UNLIMITED && item.depth > budget {
                if best == start {
                    return Ok((SEARCH_FAILED, Vec::new()));
                }
                return Ok((SEARCH_DEPTH_LIMIT, self.build_path(&parent, start, goal, best)?));
            }
            for link in self.links(node) {
                let next = self.territory_of_definition(link.target)?;
                if parent[next].is_some() {
                    continue;
                }
                parent[next] = Some(node);
                if next == goal {
                    return Ok((SEARCH_FOUND, self.build_path(&parent, start, goal, next)?));
                }
                // 960D00
                if let Some(actor) = kingdom_actor
                    && self.hostile(relations, next, actor)?
                {
                    continue;
                }
                let h_bits = self.heuristic(next, goal, precision)?.store_f32()?;
                let cost = self.link_cost(node, next).ok_or_else(|| {
                    Error::Unsupported("960E20: no link to the neighbour (native null call)".into())
                })?;
                let cost_bits = Binary::from_i32(cost).store_f32()?;
                let key_bits = Binary::from_f32_bits(cost_bits)?
                    .add(&Binary::from_f32_bits(h_bits)?, precision)?
                    .store_f32()?;
                heap.push(OpenEntry {
                    node: next,
                    depth: item.depth.wrapping_add(1),
                    key_bits,
                    cost_bits,
                });
            }
        }
    }

    /// `FUN_0093EAE0`: over consecutive pairs of the army path (component
    /// `+0x24/+0x28`), add 962310 of the link from the earlier definition to
    /// the later one when it exists (957910): 1, or 0 out of an alias.
    pub fn moves_used(&self, army: ArmyId) -> Result<i32> {
        let path = &self.state.army(army).path;
        let mut used = 0i32;
        for pair in path.windows(2) {
            self.check(pair[0])?;
            self.check(pair[1])?;
            if let Some(cost) = self.link_cost(pair[0], pair[1]) {
                used = used.wrapping_add(cost);
            }
        }
        Ok(used)
    }

    /// Army type movement: Thing (`actor+4`, 529C30) `+0x5B8 -> +0x04`.
    fn movement(data: &MetaGameData, state: &MgState, army: ArmyId) -> Result<i32> {
        let definition = state.army(army).definition;
        data.armies
            .get(definition)
            .map(|a| a.movement)
            .ok_or_else(|| Error::Unsupported(format!("army type {definition} not loaded")))
    }

    /// The 93FCC0 budget: 0 when the current territory is hostile to the
    /// army kingdom (954B30) or the army fought this turn (`+0x10 > 0`),
    /// else movement - moves used (may be negative; -1 means unlimited).
    pub fn army_budget<R: KingdomRelations>(&self, data: &MetaGameData, relations: &R, army: ArmyId) -> Result<i32> {
        let a = self.state.army(army);
        let current = a
            .territory
            .ok_or_else(|| Error::Unsupported("93FCC0: army without territory (native null)".into()))?;
        self.check(current)?;
        if self.hostile(relations, current, a.owner)? || a.fought > 0 {
            return Ok(0);
        }
        Ok(Self::movement(data, self.state, army)?.wrapping_sub(self.moves_used(army)?))
    }

    /// `FUN_0093FCC0`: search from the current territory with
    /// [`Self::army_budget`] and the army kingdom; `Some(path)` iff result 1
    /// (path copied by 4A0A40).
    pub fn army_search<R: KingdomRelations>(
        &self,
        data: &MetaGameData,
        relations: &R,
        army: ArmyId,
        target: usize,
        precision: Precision,
    ) -> Result<Option<Vec<usize>>> {
        let budget = self.army_budget(data, relations, army)?;
        let a = self.state.army(army);
        let current = a.territory.expect("checked by army_budget");
        let r = self.search(relations, current, target, Some(a.owner), budget, precision)?;
        Ok((r.code == SEARCH_FOUND).then_some(r.path))
    }

    /// `FUN_0093FAD0`: search from the first territory of the army path with
    /// the full movement. On result 1 the output is the army path walked
    /// back from `path[n-2]` down to `path[1]`, stopping at the first one
    /// that occurs in the new path P (then P from that occurrence), else
    /// followed by `path[0]` and all of P.
    pub fn army_fallback<R: KingdomRelations>(
        &self,
        data: &MetaGameData,
        relations: &R,
        army: ArmyId,
        target: usize,
        precision: Precision,
    ) -> Result<Option<Vec<usize>>> {
        let a = self.state.army(army);
        let first = *a
            .path
            .first()
            .ok_or_else(|| Error::Unsupported("93FAD0 reads path[0] of an empty path".into()))?;
        self.check(first)?;
        let movement = Self::movement(data, self.state, army)?;
        let r = self.search(relations, first, target, Some(a.owner), movement, precision)?;
        if r.code != SEARCH_FOUND {
            return Ok(None);
        }
        let mut out = Vec::new();
        let mut from = None;
        let mut k = a.path.len() as i64 - 2;
        while k > 0 {
            let t = a.path[k as usize];
            if let Some(j) = r.path.iter().position(|&x| x == t) {
                from = Some(j);
                break;
            }
            out.push(t);
            k -= 1;
        }
        let from = match from {
            Some(j) => j,
            None => {
                out.push(first);
                0
            }
        };
        out.extend_from_slice(&r.path[from..]);
        Ok(Some(out))
    }

    /// `FUN_0093FDC0`: an ALIAS target (definition `+0x0C == 4`) is replaced
    /// by the MG territory of its partner (`+0x44`, 939B60); then 93FCC0,
    /// and 93FAD0 when it fails. `None` = false with an empty output list.
    pub fn army_path<R: KingdomRelations>(
        &self,
        data: &MetaGameData,
        relations: &R,
        army: ArmyId,
        target: usize,
        precision: Precision,
    ) -> Result<Option<Vec<usize>>> {
        self.check(target)?;
        let def = &self.map.territories[self.definition(target)];
        let target = if def.kind == TerritoryKind::Alias {
            let partner = def.alias_partner.ok_or_else(|| {
                Error::Unsupported("93FDC0: alias without partner (native null dereference)".into())
            })?;
            self.territory_of_definition(partner)?
        } else {
            target
        };
        if let Some(path) = self.army_search(data, relations, army, target, precision)? {
            return Ok(Some(path));
        }
        self.army_fallback(data, relations, army, target, precision)
    }
}

/// [`PathGraph::search`] on a fresh graph (961550 + 961350).
#[allow(clippy::too_many_arguments)]
pub fn find_path<R: KingdomRelations>(
    state: &MgState,
    data: &MetaGameData,
    relations: &R,
    start: usize,
    goal: usize,
    kingdom_actor: Option<KingdomActor>,
    budget: i32,
    precision: Precision,
) -> Result<PathSearch> {
    PathGraph::new(state, data)?.search(relations, start, goal, kingdom_actor, budget, precision)
}

/// [`PathGraph::moves_used`] (93EAE0).
pub fn moves_used(state: &MgState, data: &MetaGameData, army: ArmyId) -> Result<i32> {
    PathGraph::new(state, data)?.moves_used(army)
}

/// [`PathGraph::army_path`] (93FDC0).
pub fn army_path<R: KingdomRelations>(
    state: &MgState,
    data: &MetaGameData,
    relations: &R,
    army: ArmyId,
    target: usize,
    precision: Precision,
) -> Result<Option<Vec<usize>>> {
    PathGraph::new(state, data)?.army_path(data, relations, army, target, precision)
}
