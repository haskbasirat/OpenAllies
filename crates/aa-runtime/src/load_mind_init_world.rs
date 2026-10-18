//! Native saved Element529EC0/Regiment51D1B0 initialization on the held owner.
//! Core/states move into one scoped Arena, then return to their same component
//! BEFORE any Actor callback, so reentry observes the real owner. No ActorStore
//! is fabricated and no stack, command, RNG or component is reconstructed.
use crate::{
    catalog::{Catalog, RestoredMindParts},
    live_object_init::{Record, Source},
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
    state,
};
use aa_mapworld::{
    WorldInventory,
    restore::{RestoredComponent, RestoredRecord},
};
use aa_mind::{Arena, MindId, construct::Kind};
use aa_numeric::Precision;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn required(value: &'static str) -> Error {
    Error::Unsupported(value)
}

/// Reached state/reset children require their actual world implementation.
/// The constructed-flag paths do not invoke enter/leave. No default succeeds.
pub trait Effects {
    /// Executes the real source/held-owner action adapter. Isolated native
    /// prefix tests may stop at this child entry instead of claiming its effects.
    fn rebuild_actions(
        &mut self,
        catalog: &Catalog<'_>,
        inventory: &WorldInventory,
        record: &mut Record<'_>,
        pc: Precision,
    ) -> Result<()> {
        crate::saved_actor_actions::rebuild(catalog, inventory, record, pc).map_err(error)
    }
    fn enter(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _state: &'static state::Descriptor,
        _old: Option<&'static state::Descriptor>,
        _pc: Precision,
    ) -> Result<()> {
        Err(required(
            "4D4A90 actual Mind Init Actor-state entry effects",
        ))
    }
    fn leave(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _state: &'static state::Descriptor,
        _pc: Precision,
    ) -> Result<()> {
        Err(required(
            "4D16B0 actual Mind Init Actor-state leave effects",
        ))
    }
    fn reset_default(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _kind: Kind,
        _pc: Precision,
    ) -> Result<()> {
        Err(required(
            "actual derived Mind virtual1A0 default reset services",
        ))
    }
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {}

fn parts<'a>(record: &'a Record<'_>) -> Result<&'a RestoredMindParts> {
    let component = record
        .shell
        .actor
        .as_ref()
        .ok_or_else(|| required("Mind Init actual Actor shell"))?
        .components
        .get(ComponentSlot::Mind);
    match component {
        Some(RestoredComponent::Mind(m)) => m
            .state
            .as_any()
            .downcast_ref::<RestoredMindParts>()
            .ok_or_else(|| required("Mind Init actual constructed payload")),
        _ => Err(required("Mind Init actual typed slot4")),
    }
}
fn parts_mut<'a>(record: &'a mut Record<'_>) -> Result<&'a mut RestoredMindParts> {
    let component = record
        .shell
        .actor
        .as_mut()
        .ok_or_else(|| required("Mind Init actual Actor shell"))?
        .components
        .get_mut(ComponentSlot::Mind);
    match component {
        Some(RestoredComponent::Mind(m)) => m
            .state
            .as_any_mut()
            .downcast_mut::<RestoredMindParts>()
            .ok_or_else(|| required("Mind Init actual constructed payload")),
        _ => Err(required("Mind Init actual typed slot4")),
    }
}
fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<(MindId, Kind, usize)> {
    let source = Source::bind(catalog, record.definition)?;
    source.validate_record(record)?;
    if !matches!(record.definition.native_class, 3 | 4)
        || !record
            .registry
            .get(record.owner)
            .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
    {
        return Err(required("Mind Init actual registered held Actor"));
    }
    let mask = catalog
        .canonical_component_mask(&record.definition.proof.id, source.source_precision())
        .map_err(error)?;
    if record.definition.component_mask != Some(mask) {
        return Err(required("Mind Init source component mask differs"));
    }
    let kind = crate::catalog::mind_kind(mask)
        .ok_or_else(|| required("Mind Init source factory has no concrete Mind"))?;
    let p = parts(record)?;
    let Some(RestoredComponent::Mind(m)) = record
        .shell
        .actor
        .as_ref()
        .unwrap()
        .components
        .get(ComponentSlot::Mind)
    else {
        unreachable!()
    };
    if p.installed.kind != kind
        || m.required != crate::catalog::mind_requirement(kind).map_err(error)?
        || p.installed.mind_id.0 == 0
        || p.mind
            .as_ref()
            .is_none_or(|v| v.actor != record.owner.raw())
    {
        return Err(required("Mind Init factory/core owner identity differs"));
    }
    let id = p.installed.mind_id;
    let mut used = std::collections::BTreeSet::new();
    for state in p.installed.state_bindings.values() {
        if !used.insert(*state) || p.states.get(state).is_none_or(|s| s.parent != id) {
            return Err(required("Mind Init actual embedded state binding differs"));
        }
    }
    if p.mind
        .as_ref()
        .unwrap()
        .stack
        .iter()
        .flatten()
        .any(|s| !used.contains(s))
    {
        return Err(required("Mind Init stack has a foreign state allocation"));
    }
    Ok((id, kind, p as *const _ as usize))
}
fn take_into(record: &mut Record<'_>, arena: &mut Arena, id: MindId, pointer: usize) -> Result<()> {
    if !arena.minds.is_empty() || !arena.states.is_empty() {
        return Err(required("Mind Init scoped arena is already owned"));
    }
    let owner = record.owner;
    let p = parts_mut(record)?;
    if p as *const _ as usize != pointer || p.installed.mind_id != id {
        return Err(required(
            "Mind Init callback replaced the actual allocation",
        ));
    }
    let core = p
        .mind
        .take()
        .ok_or_else(|| required("Mind Init core is already leased"))?;
    if core.actor != owner.raw() {
        p.mind = Some(core);
        return Err(required("Mind Init core owner changed"));
    }
    arena.minds.insert(id, core);
    arena.states = std::mem::take(&mut p.states);
    Ok(())
}
fn return_to(record: &mut Record<'_>, arena: &mut Arena, id: MindId, pointer: usize) -> Result<()> {
    let p = parts_mut(record)?;
    if p as *const _ as usize != pointer || p.installed.mind_id != id {
        return Err(required(
            "Mind Init callback replaced the actual allocation",
        ));
    }
    if arena.minds.is_empty() && arena.states.is_empty() && p.mind.is_some() {
        return Ok(());
    }
    if p.mind.is_some() || !p.states.is_empty() {
        return Err(required("Mind Init has two mutable core/state owners"));
    }
    p.mind = arena.minds.remove(&id);
    p.states = std::mem::take(&mut arena.states);
    if p.mind.is_none() {
        return Err(required("Mind Init scoped core disappeared"));
    }
    Ok(())
}
struct Scope<'a, 'r> {
    record: &'a mut Record<'r>,
    arena: Arena,
    id: MindId,
    pointer: usize,
}
impl Drop for Scope<'_, '_> {
    fn drop(&mut self) {
        let _ = return_to(self.record, &mut self.arena, self.id, self.pointer);
    }
}

fn index(record: &Record<'_>, arena: &Arena, id: MindId, kind: Kind) -> Result<u8> {
    let p = parts(record)?;
    let state = arena
        .mind(id)
        .map_err(error)?
        .stack
        .front()
        .copied()
        .flatten()
        .ok_or_else(|| required("native Mind sync dereferences a null active state"))?;
    if arena.state(state).map_err(error)?.parent != id {
        return Err(required("Mind Init active state has another parent"));
    }
    let offset = p
        .installed
        .state_bindings
        .iter()
        .find_map(|(o, s)| (*s == state).then_some(*o))
        .ok_or_else(|| required("Mind Init active state has no native embedded binding"))?;
    let value = match (kind, offset) {
        (Kind::Element, 0x34) | (Kind::Regiment, 0x34 | 0x270) => 12,
        (Kind::Element, 0x48 | 0x7c) => 8,
        (Kind::Element, 0x68) => {
            if record.shell.actor.as_ref().unwrap().components.state.high & 2 != 0 {
                33
            } else {
                12
            }
        }
        (Kind::Element, 0xf8 | 0x12c) | (Kind::Regiment, 0x1cc) => 4,
        (Kind::Element, 0x164) | (Kind::Regiment, 0x24c) => 7,
        (Kind::Element, 0x178) => 9,
        (Kind::Regiment, 0x54) => {
            let disable = *p
                .installed
                .state_bindings
                .get(&0x34)
                .ok_or_else(|| required("Regiment Disable binding"))?;
            //51D690 delegates to native Disable+20 (12) only at status2;
            //the native immediate return at51D6A1 is17 for every other status.
            if arena.state(disable).map_err(error)?.status == 2 {
                12
            } else {
                17
            }
        }
        (Kind::Regiment, 0x120) => 3,
        (Kind::Regiment, 0x1a8) => 10,
        (Kind::Regiment, 0x1b8) => 15,
        (Kind::Regiment, 0x1f0) => 13,
        (Kind::Regiment, 0x20c) => 14,
        (Kind::Regiment, 0x220) => 16,
        (Kind::Regiment, 0x2a4) => 35,
        _ => {
            return Err(error(format!(
                "Mind Init actual state virtual20 is required(kind={kind:?},offset={offset:X})"
            )));
        }
    };
    Ok(value)
}

struct StateEffects<'a, 'r, 's, S> {
    catalog: &'a Catalog<'s>,
    record: &'a mut Record<'r>,
    inventory: &'a WorldInventory,
    effects: &'a mut S,
    pc: Precision,
}
impl<S> StateEffects<'_, '_, '_, S> {
    fn commit(&mut self, f: &state::Fields) {
        let a = self.record.shell.actor.as_mut().unwrap();
        a.components.state.low = f.bits as u32;
        a.components.state.high = (f.bits >> 32) as u32;
        self.record.shell.thing.flags = f.thing_flags;
    }
    fn reread(&self, f: &mut state::Fields) {
        let s = self.record.shell.actor.as_ref().unwrap().components.state;
        f.bits = u64::from(s.low) | (u64::from(s.high) << 32);
        f.thing_flags = self.record.shell.thing.flags;
    }
}
impl<S: Effects> state::Effects for StateEffects<'_, '_, '_, S> {
    fn enter(
        &mut self,
        f: &mut state::Fields,
        d: &'static state::Descriptor,
        old: Option<&'static state::Descriptor>,
    ) -> state::Result<()> {
        self.commit(f);
        let r = self
            .effects
            .enter(self.catalog, self.record, d, old, self.pc);
        self.reread(f);
        r.map_err(|e| state::Error::Callback(e.to_string()))
    }
    fn leave(&mut self, f: &mut state::Fields, d: &'static state::Descriptor) -> state::Result<()> {
        self.commit(f);
        let r = self.effects.leave(self.catalog, self.record, d, self.pc);
        self.reread(f);
        r.map_err(|e| state::Error::Callback(e.to_string()))
    }
    fn rebuild_abilities(&mut self, f: &mut state::Fields) -> state::Result<()> {
        self.commit(f);
        let r = self
            .effects
            .rebuild_actions(self.catalog, self.inventory, self.record, self.pc);
        self.reread(f);
        r.map_err(|e| state::Error::Callback(e.to_string()))
    }
}
fn set<S: Effects>(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    effects: &mut S,
    index: u8,
    enabled: bool,
    pc: Precision,
) -> Result<()> {
    let s = record.shell.actor.as_ref().unwrap().components.state;
    let mut f = state::Fields {
        bits: u64::from(s.low) | (u64::from(s.high) << 32),
        thing_flags: record.shell.thing.flags,
    };
    let mut e = StateEffects {
        catalog,
        record,
        inventory,
        effects,
        pc,
    };
    let r = state::set(&mut f, index, enabled, &mut e);
    e.commit(&f);
    r.map_err(error)
}
struct World<'a, 'r, 's, S> {
    catalog: &'a Catalog<'s>,
    record: &'a mut Record<'r>,
    inventory: &'a WorldInventory,
    effects: &'a mut S,
    id: MindId,
    kind: Kind,
    pointer: usize,
    pc: Precision,
}
impl<S: Effects> crate::mind_world::InitExistingWorld for World<'_, '_, '_, S> {
    fn reset_default(&mut self, _: MindId, arena: &mut Arena) -> aa_mind::Result<()> {
        return_to(self.record, arena, self.id, self.pointer)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?;
        let r = self
            .effects
            .reset_default(self.catalog, self.record, self.kind, self.pc);
        let back = take_into(self.record, arena, self.id, self.pointer);
        match (r, back) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(aa_mind::Error::World(e.to_string())),
            (Err(e), Err(back)) => Err(aa_mind::Error::World(format!("{e}; ownership:{back}"))),
        }
    }
    fn update_actor_state(&mut self, _: MindId, arena: &mut Arena) -> aa_mind::Result<()> {
        let next = index(self.record, arena, self.id, self.kind)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?;
        return_to(self.record, arena, self.id, self.pointer)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?;
        let r = (|| {
            set(
                self.catalog,
                self.inventory,
                self.record,
                self.effects,
                next,
                true,
                self.pc,
            )?;
            if self.kind == Kind::Regiment {
                //4D7DA0 rereads actual Org presence/state before conditional+118.
                let a = self.record.shell.actor.as_ref().unwrap();
                let enabled = if a.components.present(ComponentSlot::Organization)
                    && a.components.state.low & 0x10 == 0
                {
                    *a.words
                        .get(&0x118)
                        .ok_or_else(|| required("4D7DA0 actual Actor+118"))?
                        != 0
                } else {
                    false
                };
                set(
                    self.catalog,
                    self.inventory,
                    self.record,
                    self.effects,
                    40,
                    enabled,
                    self.pc,
                )?;
            }
            Ok(())
        })();
        let back = take_into(self.record, arena, self.id, self.pointer);
        match (r, back) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(aa_mind::Error::World(e.to_string())),
            (Err(e), Err(back)) => Err(aa_mind::Error::World(format!("{e}; ownership:{back}"))),
        }
    }
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    pc: Precision,
    effects: &mut impl Effects,
) -> Result<bool> {
    let (id, kind, pointer) = checked(catalog, record)?;
    if !matches!(kind, Kind::Element | Kind::Regiment) {
        return Err(required("actual Aircraft517C60 flight initialization"));
    }
    let mut scope = Scope {
        record,
        arena: Arena::default(),
        id,
        pointer,
    };
    take_into(scope.record, &mut scope.arena, id, pointer)?;
    let r = crate::mind_world::init_existing(
        id,
        &mut scope.arena,
        &mut World {
            catalog,
            record: &mut *scope.record,
            inventory,
            effects,
            id,
            kind,
            pointer,
            pc,
        },
    );
    let back = return_to(scope.record, &mut scope.arena, id, pointer);
    match (r, back) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(error(e)),
        (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(back)) => Err(error(format!("{e}; ownership:{back}"))),
    }
}
/// Every concrete Mind after-all-components hook is native4B2830 return.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}

#[cfg(test)]
#[path = "load_mind_init_world_tests.rs"]
mod tests;
