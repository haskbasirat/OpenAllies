//! Original Mind virtual+B4 command-mask contributions. These mutate the current
//! actor's existing F0/F4 mask; the outer4D1350 coordinator owns clearing it.
use crate::{Arena, Error, MindId, Result, StateId};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ability {
    pub enabled: bool,
    pub flags: u32,
    pub required_targets: u32,
    pub effect_kinds: Vec<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum Capability {
    Any,
    Indirect,
    Direct,
    Repair,
}
///57E230/57E280/57E2F0/57E1C0 and4BE970, over the authored ordered ability list.
pub fn capability(abilities: &[Ability], kind: Capability) -> bool {
    abilities.iter().any(|a| {
        a.enabled
            && match kind {
                Capability::Any => true,
                Capability::Indirect => a.flags & 8 != 0,
                Capability::Direct => a.flags & 8 == 0,
                Capability::Repair => a.required_targets & 128 != 0 && a.effect_kinds.contains(&3),
            }
    })
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub flags: u32,
    pub element: Option<u32>,
    pub organization: Option<u32>,
    pub performer: Option<u32>,
    pub mover: bool,
    pub component9c: bool,
    pub word108: u32,
    pub word118: u32,
    pub componentac: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub struct RegimentStates {
    pub rout: StateId,
    pub engage: StateId,
}
/// Regiment virtual+5C51D450→52A8A0. Suspended exhausted states count too.
/// Element and Aircraft implementations are original6A71F0 and return false.
pub fn exhausted_in_stack(mind: MindId, exhausted: StateId, arena: &Arena) -> Result<bool> {
    Ok(arena.mind(mind)?.stack.contains(&Some(exhausted)))
}
pub trait World {
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn action_bits(&self, actor: u32) -> Result<u64>;
    /// Pure field store to the same actorF0/F4, with no implicit mask reset.
    fn set_action_bits(&mut self, actor: u32, bits: u64) -> Result<()>;
    fn element_organization(&self, element: u32) -> Result<Option<u32>>;
    fn abilities(&self, performer: u32) -> Result<Vec<Ability>>;
    fn resolve(&self, id: u32) -> Result<Option<u32>>;
    fn organization_link_d0(&self, organization: u32) -> Result<u32>;
    fn organization_status_578140(&self, organization: u32) -> Result<i32>;
    fn organization_values(&self, organization: u32) -> Result<[u32; 2]>;
    fn setting_cache108(&self) -> Result<bool>;
    fn setting_cached8(&self) -> Result<u32>;
    /// Historical name:4CF110 returns Actor.Properties+1C Modifier, not a
    /// player/Kingdom. A portable handle may identify its owning actor.
    fn actor_player(&mut self, actor: u32, arena: &mut Arena) -> Result<Option<u32>>;
    /// Flags from the actual Modifier returned by4CF110.
    fn player_flags(&self, player: u32) -> Result<u32>;
    /// Historical name: native Actor virtual+34=4D1F90 queries ACTION F0/F4,
    /// not actor-state100/104. Bind to the live action mask at this call point.
    fn actor_state(&mut self, actor: u32, index: u32, arena: &mut Arena) -> Result<bool>;
    /// Actual4D12C0. Called twice, with live actor lookup each time.
    fn actor_predicate_4d12c0(&mut self, actor: u32, arena: &mut Arena) -> Result<bool>;
    /// Actual575BB0/575C00 on actor+AC; both receive argument=true.
    fn component_ac_predicate(
        &mut self,
        component: u32,
        entry: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
}
fn owner(mind: MindId, a: &Arena) -> Result<u32> {
    Ok(a.mind(mind)?.actor)
}
fn set(w: &mut impl World, actor: u32, bit: u32, enabled: bool) -> Result<()> {
    let old = w.action_bits(actor)?;
    w.set_action_bits(
        actor,
        if enabled {
            old | (1u64 << bit)
        } else {
            old & !(1u64 << bit)
        },
    )
}
fn cap(w: &impl World, performer: Option<u32>, kind: Capability) -> Result<bool> {
    match performer {
        Some(id) => Ok(capability(&w.abilities(id)?, kind)),
        None => Ok(false),
    }
}
fn required(value: Option<u32>, name: &str) -> Result<u32> {
    value.ok_or_else(|| Error::World(format!("missing required {name}")))
}
fn active(mind: MindId, state: StateId, a: &Arena) -> Result<bool> {
    Ok(a.mind(mind)?.stack.front().copied().flatten() == Some(state))
}
///530510, Element Mind. A live Element component is a native prerequisite.
pub fn element(mind: MindId, arena: &mut Arena, w: &mut impl World) -> Result<()> {
    let actor = owner(mind, arena)?;
    let enabled = w.actor(actor)?.flags & (1 << 29) == 0;
    set(w, actor, 5, enabled)?;
    let actor = owner(mind, arena)?;
    if let Some(element) = w.actor(actor)?.element
        && w.element_organization(element)?.is_some()
    {
        set(w, actor, 2, enabled)?;
    }
    let actor = owner(mind, arena)?;
    let element = required(w.actor(actor)?.element, "Element component+80")?;
    let attached = w.element_organization(element)?.is_some();
    let allowed = enabled && attached && w.setting_cache108()?;
    set(w, actor, 9, allowed)?;
    let actor = owner(mind, arena)?;
    let allowed = attached && w.actor(actor)?.word108 != 0;
    set(w, actor, 4, allowed)?;
    let actor = owner(mind, arena)?;
    let allowed = cap(w, w.actor(actor)?.performer, Capability::Repair)?;
    set(w, owner(mind, arena)?, 3, allowed)?;
    let actor = owner(mind, arena)?;
    let allowed = cap(w, w.actor(actor)?.performer, Capability::Indirect)?;
    set(w, owner(mind, arena)?, 1, allowed)?;
    let actor = owner(mind, arena)?;
    let fields = w.actor(actor)?;
    let mut allowed = fields.mover && cap(w, fields.performer, Capability::Any)?;
    if allowed && w.actor_state(owner(mind, arena)?, 1, arena)? {
        allowed = cap(
            w,
            w.actor(owner(mind, arena)?)?.performer,
            Capability::Direct,
        )?;
    }
    set(w, owner(mind, arena)?, 0, allowed)?;
    let actor = owner(mind, arena)?;
    let fields = w.actor(actor)?;
    let allowed = fields.mover || cap(w, fields.performer, Capability::Any)?;
    set(w, owner(mind, arena)?, 6, allowed)?;
    let player = w.actor_player(owner(mind, arena)?, arena)?;
    let actor = owner(mind, arena)?;
    let fields = w.actor(actor)?;
    let mut allowed = !fields.component9c
        && !player
            .map(|p| w.player_flags(p).map(|f| f & 0x800 != 0))
            .transpose()?
            .unwrap_or(false);
    if allowed
        && let Some(element) = fields.element
        && let Some(leader) = w.element_organization(element)?
    {
        let org = required(
            w.actor(leader)?.organization,
            "leader Organization component+7C",
        )?;
        allowed = w.resolve(w.organization_link_d0(org)?)?.is_none();
    }
    set(w, actor, 10, allowed)?;
    if let Some(component) = w.actor(owner(mind, arena)?)?.componentac {
        let allowed = w.component_ac_predicate(component, 0x575bb0, arena)?;
        set(w, owner(mind, arena)?, 11, allowed)?;
        let component = required(
            w.actor(owner(mind, arena)?)?.componentac,
            "component+AC after575BB0",
        )?;
        let allowed = w.component_ac_predicate(component, 0x575c00, arena)?;
        set(w, owner(mind, arena)?, 12, allowed)?;
    }
    Ok(())
}
///518720. Independent bit stores preserve every unrelated action bit.
pub fn aircraft(mind: MindId, arena: &Arena, w: &mut impl World) -> Result<()> {
    for bit in [5, 0, 2] {
        set(w, owner(mind, arena)?, bit, true)?;
    }
    Ok(())
}
fn numeric<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("Mind command mask arithmetic: {e}")))
}
///521E10. Rout and Organization status0 clear only bits0..12; the other branch
/// re-evaluates actor predicates and active engage state at their native points.
pub fn regiment(
    mind: MindId,
    states: RegimentStates,
    arena: &mut Arena,
    w: &mut impl World,
    p: Precision,
) -> Result<()> {
    let routed = active(mind, states.rout, arena)?;
    let org = required(
        w.actor(owner(mind, arena)?)?.organization,
        "Organization component+7C",
    )?;
    let status_zero = w.organization_status_578140(org)? == 0;
    if routed || status_zero {
        let actor = owner(mind, arena)?;
        let bits = w.action_bits(actor)?;
        w.set_action_bits(actor, bits & !0x1fff)?;
        return Ok(());
    }
    let engaged = active(mind, states.engage, arena)?;
    let word118 = w.actor(owner(mind, arena)?)?.word118 != 0;
    let player = w.actor_player(owner(mind, arena)?, arena)?;
    let mut allowed = !engaged && !word118;
    if allowed {
        allowed = !w.actor_state(owner(mind, arena)?, 20, arena)?;
    }
    if allowed && let Some(player) = player {
        allowed = w.player_flags(player)? & 0x800 == 0;
    }
    if allowed {
        let org = required(
            w.actor(owner(mind, arena)?)?.organization,
            "Organization component+7C",
        )?;
        allowed = w.resolve(w.organization_link_d0(org)?)?.is_none();
    }
    set(w, owner(mind, arena)?, 10, allowed)?;
    let disabled = w.actor_predicate_4d12c0(owner(mind, arena)?, arena)?;
    set(w, owner(mind, arena)?, 7, !disabled)?;
    let disabled = w.actor_predicate_4d12c0(owner(mind, arena)?, arena)?;
    set(w, owner(mind, arena)?, 8, !disabled && engaged)?;
    if w.actor_state(owner(mind, arena)?, 7, arena)? && active(mind, states.engage, arena)? {
        let org = required(
            w.actor(owner(mind, arena)?)?.organization,
            "Organization component+7C",
        )?;
        let [value_b8, value_5c] = w.organization_values(org)?;
        let value = numeric(Binary::from_f32_bits(w.setting_cached8()?))?;
        let value = numeric(value.add(&numeric(Binary::from_f32_bits(value_b8))?, p))?;
        let value = numeric(value.add(&Binary::from_i32(1), p))?;
        if value.compare(&numeric(Binary::from_f32_bits(value_5c))?) == Ordering::Greater {
            set(w, owner(mind, arena)?, 7, false)?;
        }
    }
    Ok(())
}
