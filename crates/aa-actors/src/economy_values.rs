//! Native per-actor resource gates and values. Registry order and all source
//! definitions/component data are explicit, with no zero-production fallback.
use crate::ActorId;
use aa_body::F32;
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("economy value service failed: {0}")]
    World(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Body(#[from] aa_body::Error),
    #[error("could not allocate resource vector")]
    Allocation,
    #[error("Organization resource traversal contains an actor cycle at {0:?}")]
    Cycle(ActorId),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition<D> {
    pub flags174: u32,
    /// Native recruitable NO=0, YES=1, WITH_PRIMARY=2.
    pub class178: u32,
    /// Native ActorData+17C recruitment-backreference head; zero means NULL.
    /// Live adapters may use the lazy World getter instead of this snapshot.
    pub word17c: u32,
    /// Actual signed LayoutDefinition+18; required for class2 Organization data.
    pub layout_kind18: Option<i32>,
    pub supply_required: bool,
    /// Reverse MobileDeployment unpack_to_actor_ids pairing written by4605B0.
    pub packed_definition: Option<D>,
    /// Definition resource vectors324,330,33C in registry order.
    pub cost: Vec<F32>,
    pub upkeep: Vec<F32>,
    pub production: Vec<F32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorSnapshot<D> {
    pub definition: D,
    pub flags: u32,
    pub state_bits: u64,
    /// Element+10 owning Organization actor, or None if unattached/no Element.
    pub element_organization: Option<ActorId>,
    pub organization: Option<u32>,
    pub economy: Option<u32>,
    /// ConstructorComponent+18. None means the component is genuinely absent.
    pub constructor18: Option<F32>,
    /// TransportComponent actor+C0, providing cached cost/upkeep vectors.
    pub transport: Option<u32>,
}
/// Pure coherent lookups: these eligibility functions execute no world callback.
pub trait GateWorld {
    type Definition: Clone;
    fn actor(&self, actor: ActorId) -> Result<ActorSnapshot<Self::Definition>>;
    fn definition(&self, definition: &Self::Definition) -> Result<Definition<Self::Definition>>;
    /// Native inverse ActorData+548 is read only by the unsupplied branches.
    /// The default supports complete immutable fixture definitions. A runtime
    /// whose inverse binding is unresolved must override this getter and fail
    /// on that read, rather than expose unknown as a genuine null reference.
    fn packed_definition(&self, definition: &Self::Definition) -> Result<Option<Self::Definition>> {
        Ok(self.definition(definition)?.packed_definition)
    }
    /// Native4D0C90 is read at its original branch. Runtime adapters may resolve
    ///Supply+18 lazily so earlier eligibility exits do not read that field.
    fn supplied(&self, actor: ActorId) -> Result<bool> {
        Ok(supplied(&self.actor(actor)?))
    }
}
///45E5A0. Class2 requires Organization flag7 and nonnegative layout field18.
pub fn recruitable<D>(definition: &Definition<D>) -> Result<bool> {
    recruitable_fields(definition.class178, definition.flags174, || {
        definition
            .layout_kind18
            .ok_or_else(|| Error::World("class2 Organization lacks layout field18".into()))
    })
}
///45E5A0 over its exact field slice. The Layout+18 getter is reached only
///for WITH_PRIMARY with the actual Organization component flag.
pub fn recruitable_fields(
    class178: u32,
    flags174: u32,
    primary_slot: impl FnOnce() -> Result<i32>,
) -> Result<bool> {
    match class178 {
        1 => Ok(true),
        2 if flags174 & (1 << 7) != 0 => Ok(primary_slot()? >= 0),
        _ => Ok(false),
    }
}
///4D0C90. Ordered positive comparison preserves signedzero/infinity/NaN gates.
pub fn supplied<D>(actor: &ActorSnapshot<D>) -> bool {
    if actor.state_bits & (1 << 19) == 0 {
        return true;
    }
    actor.constructor18.is_some_and(|v| {
        v.0 >> 31 == 0 && (v.0 & 0x7fff_ffff) > 0 && (v.0 & 0x7fff_ffff) <= 0x7f80_0000
    })
}
///56F960: unsupplied actor whose packed replacement has Economy flag9.
pub fn packed_supply(actor: ActorId, w: &impl GateWorld) -> Result<bool> {
    if w.supplied(actor)? {
        return Ok(false);
    }
    let actor = w.actor(actor)?;
    match w.packed_definition(&actor.definition)? {
        Some(id) => Ok(w.definition(&id)?.flags174 & (1 << 9) != 0),
        None => Ok(false),
    }
}
///56F9A0 production eligibility, preserving every original early gate.
pub fn eligible(actor: ActorId, w: &impl GateWorld) -> Result<bool> {
    let value = w.actor(actor)?;
    if value.element_organization.is_some()
        || value.flags & 1 != 0
        || value.state_bits & ((1u64 << 39) | (1 << 21)) != 0
    {
        return Ok(false);
    }
    let definition = w.definition(&value.definition)?;
    if value.organization.is_some() {
        return recruitable(&definition);
    }
    if packed_supply(actor, w)? {
        return Ok(true);
    }
    Ok(!definition.supply_required || w.supplied(actor)?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Modifier {
    pub flags: u32,
    pub factor4c: F32,
    pub factor60: F32,
    /// Actual numeric Modifier+80 table; opaque pointer IDs are not values.
    pub production_adjustments: Vec<F32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorKind {
    Cost,
    Upkeep,
    Production,
}
/// All getters are coherent pure snapshots. The sole mutation is the explicit
/// Economy+44 cache write; it must not modify definitions, ownership or members.
pub trait World: GateWorld {
    fn resource_count(&self) -> Result<usize>;
    /// Actual4CF110 Modifier/Properties context identity, or native null.
    fn actor_modifier(&self, actor: ActorId) -> Result<Option<u32>>;
    fn modifier(&self, context: u32) -> Result<Modifier>;
    ///501730 reads ActorData+17C only after45E5A0 returns false. Keep the
    ///source dependency lazy: upkeep, production, Construction cost and
    ///recruitable cost do not read it. Fixture snapshots retain their default.
    fn recruit_backref_present(&self, definition: &Self::Definition) -> Result<bool> {
        Ok(self.definition(definition)?.word17c != 0)
    }
    ///5019D0 is only read for Production; Cost/Upkeep never inspect its table.
    fn production_delta(&self, context: u32, index: usize) -> Result<aa_numeric::Binary> {
        Ok(aa_body::economy_modifiers::production_delta(
            &self.modifier(context)?.production_adjustments,
            self.resource_count()?,
            index,
        )?)
    }
    fn transport_vector(&self, transport: u32, kind: VectorKind) -> Result<Vec<F32>>;
    /// Layout descriptor+2C count of ordered definition slot indices.
    fn layout_slot_count(&self, organization: u32) -> Result<usize>;
    /// Resolve layout slot index through actual parameter table, including nulls.
    fn layout_slot_definition(
        &self,
        organization: u32,
        index: usize,
    ) -> Result<Option<Self::Definition>>;
    fn member_count(&self, organization: u32) -> Result<usize>;
    fn member(&self, organization: u32, index: usize) -> Result<Option<ActorId>>;
    fn store_member_production(&mut self, economy: u32, values: &[F32]) -> Result<()>;
}
fn reset(out: &mut Vec<F32>, count: usize) -> Result<()> {
    out.try_reserve(count.saturating_sub(out.len()))
        .map_err(|_| Error::Allocation)?;
    out.resize(count, F32::ZERO);
    out.fill(F32::ZERO);
    Ok(())
}
fn copy(out: &mut Vec<F32>, values: &[F32]) -> Result<()> {
    reset(out, values.len())?;
    out.copy_from_slice(values);
    Ok(())
}
fn add(out: &mut [F32], values: &[F32], p: Precision) -> Result<()> {
    //4CCD80 only visits min(destination_count,source_count), storing each sum.
    for (dst, src) in out.iter_mut().zip(values) {
        *dst = F32(aa_numeric::Binary::from_f32_bits(src.0)?
            .add(&aa_numeric::Binary::from_f32_bits(dst.0)?, p)?
            .store_f32()?);
    }
    Ok(())
}
///45F4D0/45F570/45F680 over actual source resource arrays.
pub fn definition_values<W: World>(
    out: &mut Vec<F32>,
    id: &W::Definition,
    kind: VectorKind,
    context: Option<u32>,
    w: &W,
    p: Precision,
) -> Result<()> {
    let d = w.definition(id)?;
    let source = match kind {
        VectorKind::Cost => &d.cost,
        VectorKind::Upkeep => &d.upkeep,
        VectorKind::Production => &d.production,
    };
    copy(out, source)?;
    let Some(context) = context else {
        return Ok(());
    };
    let modifier = w.modifier(context)?;
    match kind {
        VectorKind::Cost => {
            for value in out {
                let adjusted = if d.flags174 & (1 << 15) != 0 {
                    aa_body::economy_modifiers::special_cost(
                        *value,
                        modifier.factor4c,
                        d.flags174,
                        p,
                    )?
                } else {
                    //501730 calls45E5A0 once per scalar, then reads+17C
                    //only on false. Do not eagerly resolve this source graph.
                    let is_recruitable = recruitable(&d)?;
                    let backref = !is_recruitable && w.recruit_backref_present(id)?;
                    aa_body::economy_modifiers::cost(
                        *value,
                        modifier.factor60,
                        is_recruitable,
                        u32::from(backref),
                        p,
                    )?
                };
                *value = F32(adjusted.store_f32()?);
            }
        }
        VectorKind::Upkeep => {
            if modifier.flags & (1 << 9) != 0 {
                out.fill(F32::ZERO);
            }
        }
        VectorKind::Production => {
            for (index, value) in out.iter_mut().enumerate() {
                let delta = w.production_delta(context, index)?;
                *value = F32(delta
                    .add(&aa_numeric::Binary::from_f32_bits(value.0)?, p)?
                    .store_f32()?);
            }
        }
    }
    Ok(())
}

///56F850/56F580/56F660: base definition plus ordered authored layout entries.
pub fn layout_values<W: World>(
    out: &mut Vec<F32>,
    definition: Option<&W::Definition>,
    organization: Option<u32>,
    context: Option<u32>,
    kind: VectorKind,
    w: &W,
    p: Precision,
) -> Result<()> {
    reset(out, w.resource_count()?)?;
    if let Some(id) = definition
        && w.definition(id)?.flags174 & (1 << 9) != 0
    {
        definition_values(out, id, kind, context, w, p)?;
    }
    if let Some(org) = organization {
        let count = w.layout_slot_count(org)?;
        let mut temporary = Vec::new();
        for index in 0..count {
            if let Some(id) = w.layout_slot_definition(org, index)?
                && w.definition(&id)?.flags174 & (1 << 9) != 0
            {
                // Cost includes owner context for layout entries; Upkeep and
                // Production deliberately use null context in the native helper.
                let member_context = if kind == VectorKind::Cost {
                    context
                } else {
                    None
                };
                definition_values(&mut temporary, &id, kind, member_context, w, p)?;
                add(out, &temporary, p)?;
            }
        }
    }
    Ok(())
}
///56FA30 and56FB10. Transport flag24 overrides the ordinary actor checks first.
pub fn cost_or_upkeep<W: World>(
    out: &mut Vec<F32>,
    actor: Option<ActorId>,
    kind: VectorKind,
    w: &W,
    p: Precision,
) -> Result<()> {
    if kind == VectorKind::Production {
        return Err(Error::World(
            "production requires the member traversal entry".into(),
        ));
    }
    reset(out, w.resource_count()?)?;
    let Some(actor) = actor else { return Ok(()) };
    let a = w.actor(actor)?;
    let d = w.definition(&a.definition)?;
    if d.flags174 & (1 << 24) != 0 {
        let transport = a
            .transport
            .ok_or_else(|| Error::World("flag24 actor lacks Transport".into()))?;
        return copy(out, &w.transport_vector(transport, kind)?);
    }
    if kind == VectorKind::Upkeep
        && (a.element_organization.is_some()
            || a.flags & 1 != 0
            || a.organization.is_some() && !recruitable(&d)?)
    {
        return Ok(());
    }
    if a.economy.is_some() {
        definition_values(out, &a.definition, kind, w.actor_modifier(actor)?, w, p)?;
    }
    if let Some(org) = w.actor(actor)?.organization {
        let mut values = Vec::new();
        layout_values(
            &mut values,
            None,
            Some(org),
            w.actor_modifier(actor)?,
            kind,
            w,
            p,
        )?;
        add(out, &values, p)?;
    }
    Ok(())
}
fn production_start<W: World>(
    out: &mut Vec<F32>,
    actor: ActorId,
    w: &W,
    p: Precision,
) -> Result<Option<u32>> {
    reset(out, w.resource_count()?)?;
    let a = w.actor(actor)?;
    let _d = w.definition(&a.definition)?;
    if !w.supplied(actor)?
        && let Some(id) = &w.packed_definition(&a.definition)?
        && w.definition(id)?.flags174 & (1 << 9) != 0
    {
        layout_values(
            out,
            Some(id),
            a.organization,
            w.actor_modifier(actor)?,
            VectorKind::Production,
            w,
            p,
        )?;
        return Ok(None);
    }
    if a.economy.is_some() {
        definition_values(
            out,
            &a.definition,
            VectorKind::Production,
            w.actor_modifier(actor)?,
            w,
            p,
        )?;
    }
    Ok(w.actor(actor)?.organization)
}
struct Frame {
    actor: ActorId,
    organization: u32,
    next: usize,
    output: Vec<F32>,
    waiting_component: Option<u32>,
}
///56FC20. An iterative equivalent of native recursive member traversal avoids a
/// host call-stack limit. Members/length are reread and null/no-Economy slots are
/// skipped; only actual Economy+44 caches are updated when requested.
pub fn production<W: World>(
    out: &mut Vec<F32>,
    actor: Option<ActorId>,
    cache_members: bool,
    w: &mut W,
    p: Precision,
) -> Result<()> {
    reset(out, w.resource_count()?)?;
    let Some(actor) = actor else { return Ok(()) };
    let Some(org) = production_start(out, actor, w, p)? else {
        return Ok(());
    };
    let mut frames = Vec::new();
    frames.try_reserve(1).map_err(|_| Error::Allocation)?;
    frames.push(Frame {
        actor,
        organization: org,
        next: 0,
        output: std::mem::take(out),
        waiting_component: None,
    });
    let mut active = BTreeSet::new();
    active.insert(actor);
    let result = (|| {
        loop {
            let last = frames.len() - 1;
            let org = frames[last].organization;
            let index = frames[last].next;
            if index >= w.member_count(org)? {
                let complete = frames.pop().expect("nonempty frame stack");
                active.remove(&complete.actor);
                if let Some(parent) = frames.last_mut() {
                    let component = parent
                        .waiting_component
                        .take()
                        .expect("child has caller component");
                    if cache_members {
                        w.store_member_production(component, &complete.output)?;
                    }
                    add(&mut parent.output, &complete.output, p)?;
                    parent.next += 1;
                    continue;
                }
                *out = complete.output;
                return Ok(());
            }
            let Some(child) = w.member(org, index)? else {
                frames[last].next += 1;
                continue;
            };
            let Some(component) = w.actor(child)?.economy else {
                frames[last].next += 1;
                continue;
            };
            if active.contains(&child) {
                return Err(Error::Cycle(child));
            }
            let mut values = Vec::new();
            let child_org = production_start(&mut values, child, w, p)?;
            if let Some(child_org) = child_org {
                frames.try_reserve(1).map_err(|_| Error::Allocation)?;
                frames[last].waiting_component = Some(component);
                active.insert(child);
                frames.push(Frame {
                    actor: child,
                    organization: child_org,
                    next: 0,
                    output: values,
                    waiting_component: None,
                });
            } else {
                if cache_members {
                    w.store_member_production(component, &values)?;
                }
                add(&mut frames[last].output, &values, p)?;
                frames[last].next += 1;
            }
        }
    })();
    if result.is_err() && !frames.is_empty() {
        *out = std::mem::take(&mut frames[0].output);
    }
    result
}
