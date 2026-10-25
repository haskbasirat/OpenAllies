//! The structure production queue on the exact ports (aa-production): the
//! queue (request 0x5836E0 / 0x582AE0 / 0x589E00, start 0x589420, process
//! 0x589F90, remove 0x5894C0, cancel 0x589BD0, duration 0x588D60), research
//! (request 0x584790, blocked 0x5844A0, completion 0x584120, refund 0x583F30,
//! requirements 0x5842F0 / 0x4C5250) and the HQ attach 0x574C10. `Host` binds
//! their callbacks to aa-play; the queue lives in `books` and is converted to
//! the native layout (binary32 words) around each call.
use super::{busy, can_afford, can_enroll, credit, fulfill as recruit_fulfill, hq_org, limited_part, recruit_cost, Vector};
use crate::books::{PauseReason, QueueItem};
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, KingdomId, OrgId, World};
use aa_numeric::{Binary, Precision};
use aa_production::research::{self, KingdomResearch, PropertyTarget, ResearchComponent, ResearchItem};
use aa_production::{Context, Queue};
use std::collections::BTreeMap;

type R<T> = aa_production::Result<T>;

/// The simulation thread's x87 precision.
pub(super) const PRECISION: Precision = Precision::Double53;

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}
fn floats(v: &[u32]) -> Vec<f32> {
    v.iter().map(|x| f32::from_bits(*x)).collect()
}
fn err(e: impl std::fmt::Display) -> aa_production::Error {
    aa_production::Error::World(e.to_string())
}

/// Native definition handles are never zero: aa-play definition `d` is `d+1`.
pub(super) fn def_handle(d: DefId) -> u32 {
    d + 1
}
fn def_of(handle: u32) -> Option<DefId> {
    handle.checked_sub(1)
}

/// A Property definition handle: its position in `GameData::properties` + 1.
pub(super) fn property_handle(world: &World, id: &str) -> u32 {
    world.data.properties.keys().position(|k| k.eq_ignore_ascii_case(id)).map_or(0, |i| i as u32 + 1)
}
fn property_of(world: &World, handle: u32) -> Option<&str> {
    world.data.properties.keys().nth(handle.checked_sub(1)? as usize).map(String::as_str)
}

/// A research record handle (the ActorData research list entry): the owning
/// definition and the entry index.
pub(super) fn record_handle(def: DefId, index: usize) -> u32 {
    (def + 1) << 12 | (index as u32 + 1)
}
fn record_of(handle: u32) -> Option<(DefId, usize)> {
    Some(((handle >> 12).checked_sub(1)?, ((handle & 0xfff).checked_sub(1)?) as usize))
}

/// The native research record of `def`'s research entry `index`.
pub(super) fn research_record(world: &World, def: DefId, index: usize) -> Option<ResearchItem> {
    let r = world.data.def(def).research.get(index)?;
    Some(ResearchItem {
        id: record_handle(def, index),
        definition: property_handle(world, &r.property),
        // record +0x2C: 0 LOCAL (actor), 1 SUPPORT (organization), 2 GLOBAL.
        kind: match r.kind.to_ascii_uppercase().as_str() {
            "LOCAL" => 0,
            "SUPPORT" => 1,
            _ => 2,
        },
        cost: bits(&r.cost),
        requirements: requirements(world, &r.requirements),
    })
}

/// A data Requirements object in native handles.
pub(super) fn requirements(world: &World, r: &crate::data::Requirements) -> research::Requirements {
    research::Requirements {
        properties: r.properties.iter().map(|p| property_handle(world, p)).filter(|&h| h != 0).collect(),
        any_property: r.any_property,
        actors: r.structures.iter().filter_map(|s| world.data.find(s)).map(def_handle).collect(),
        any_actor: r.any_structure,
    }
}

/// What the native item does not carry and aa-play's books need.
#[derive(Clone, Debug)]
pub(super) struct Extra {
    pub def: DefId,
    pub attaches: bool,
    pub research: Option<String>,
}

/// The callbacks of one building's queue.
pub(super) struct Host<'a> {
    pub world: &'a mut World,
    pub building: ActorId,
    /// Items by native id.
    pub extras: BTreeMap<u32, Extra>,
    /// The item the next id is given to (request paths).
    pub pending: Option<Extra>,
    /// Refusal facts for the command's reason.
    pub unaffordable: bool,
    pub blocked: Option<i32>,
}

impl<'a> Host<'a> {
    pub fn new(world: &'a mut World, building: ActorId) -> Self {
        Self { world, building, extras: BTreeMap::new(), pending: None, unaffordable: false, blocked: None }
    }
    fn kingdom(&self) -> Option<KingdomId> {
        self.world.actors.get(self.building as usize).and_then(|a| a.kingdom)
    }
    fn n(&self) -> usize {
        self.world.data.resources.len()
    }
    fn def(&self, handle: u32) -> R<&crate::data::Def> {
        def_of(handle).filter(|&d| (d as usize) < self.world.data.defs.len()).map(|d| self.world.data.def(d)).ok_or(aa_production::Error::Invalid("unknown definition"))
    }
    fn building_def(&self) -> DefId {
        self.world.actors[self.building as usize].def
    }
    /// The building's research record of `handle`, if it lists it.
    fn record(&self, handle: u32) -> Option<ResearchItem> {
        let (def, index) = record_of(handle)?;
        research_record(self.world, def, index)
    }
    fn item_extra(&self, item: &aa_production::QueueItem) -> Option<&Extra> {
        self.extras.get(&item.id)
    }
    /// The aa-play view of a native item (for the completion and refund).
    fn play_item(&self, item: &aa_production::QueueItem) -> QueueItem {
        let extra = self.item_extra(item);
        QueueItem {
            id: item.id,
            def: extra.map_or_else(|| def_of(item.actor_definition).unwrap_or(0), |e| e.def),
            paid: floats(&item.paid_cost),
            requested_upkeep: floats(&item.requested_upkeep),
            current_upkeep: floats(&item.current_upkeep),
            attaches: extra.is_some_and(|e| e.attaches),
            research: extra.and_then(|e| e.research.clone()),
            organization_slot: item.organization_slot,
        }
    }
    /// The building's research component (+0x10 completed list).
    fn component(&self) -> ResearchComponent {
        ResearchComponent { completed: self.world.books.actor(self.building).map(|b| b.research_completed.clone()).unwrap_or_default() }
    }
    fn store_component(&mut self, c: ResearchComponent) {
        self.world.books.actor_mut(self.building).research_completed = c.completed;
    }
}

/// A books queue in the native layout, with the extras of its items.
pub(super) fn load(world: &World, building: ActorId) -> (Queue, BTreeMap<u32, Extra>) {
    let mut extras = BTreeMap::new();
    let Some(b) = world.books.actor(building) else { return (Queue::default(), extras) };
    let q = &b.queue;
    let bdef = world.actors.get(building as usize).map_or(0, |a| a.def);
    let items = q
        .items
        .iter()
        .map(|it| {
            extras.insert(it.id, Extra { def: it.def, attaches: it.attaches, research: it.research.clone() });
            let research = it.research.as_ref().and_then(|p| world.data.def(bdef).research.iter().position(|r| r.property.eq_ignore_ascii_case(p)));
            aa_production::QueueItem {
                id: it.id,
                kind: if it.research.is_some() { 1 } else { 0 },
                actor_definition: if it.research.is_some() { 0 } else { def_handle(it.def) },
                organization_layout: 0,
                location_bits: [0; 2],
                property: research.map_or(0, |i| record_handle(bdef, i)),
                paid_cost: bits(&it.paid),
                requested_upkeep: bits(&it.requested_upkeep),
                current_upkeep: bits(&it.current_upkeep),
                organization_slot: it.organization_slot,
            }
        })
        .collect();
    let queue = Queue {
        items,
        deadline_bits: q.deadline.to_bits(),
        duration_bits: q.duration.to_bits(),
        paused_at_bits: q.paused_at.map_or(0xbf80_0000, f32::to_bits),
        pause_reason: match q.pause_reason {
            None => -1,
            Some(PauseReason::LimitedResource) => 0,
            Some(PauseReason::Supply) => 1,
            Some(PauseReason::Disabled) => 2,
        },
        scheduled: q.scheduled,
    };
    (queue, extras)
}

/// Store a native queue back into the books.
pub(super) fn store(world: &mut World, building: ActorId, queue: &Queue, extras: &BTreeMap<u32, Extra>) {
    let items = queue
        .items
        .iter()
        .map(|it| {
            let e = extras.get(&it.id);
            QueueItem {
                id: it.id,
                def: e.map_or_else(|| def_of(it.actor_definition).unwrap_or(0), |e| e.def),
                paid: floats(&it.paid_cost),
                requested_upkeep: floats(&it.requested_upkeep),
                current_upkeep: floats(&it.current_upkeep),
                attaches: e.is_some_and(|e| e.attaches),
                research: e.and_then(|e| e.research.clone()),
                organization_slot: it.organization_slot,
            }
        })
        .collect();
    let paused = f32::from_bits(queue.paused_at_bits);
    let q = &mut world.books.actor_mut(building).queue;
    q.items = items;
    q.deadline = f32::from_bits(queue.deadline_bits);
    q.duration = f32::from_bits(queue.duration_bits);
    q.paused_at = (paused >= 0.0).then_some(paused);
    q.pause_reason = match queue.pause_reason {
        0 => Some(PauseReason::LimitedResource),
        1 => Some(PauseReason::Supply),
        2 => Some(PauseReason::Disabled),
        _ => None,
    };
    q.scheduled = queue.scheduled;
}

/// The queue context: GWorld+0xE0 (the begin time 0x589420 / 0x589F90
/// read), the tick step, no instant production, PC53.
pub(super) fn context(world: &World) -> Context {
    Context { now_bits: world.clock.begin.0, delta_bits: world.clock.step.0, instant_production: false, precision: PRECISION }
}

/// Run `f` on `building`'s queue with its host; the queue goes back to the
/// books afterwards (also after an error: native stores are immediate).
pub(super) fn with_queue<T>(world: &mut World, building: ActorId, f: impl FnOnce(&mut Queue, Context, &mut Host<'_>) -> T) -> (T, bool, Option<i32>) {
    let (mut queue, extras) = load(world, building);
    let ctx = context(world);
    let mut host = Host::new(world, building);
    host.extras = extras;
    let out = f(&mut queue, ctx, &mut host);
    let (unaffordable, blocked) = (host.unaffordable, host.blocked);
    let extras = std::mem::take(&mut host.extras);
    store(world, building, &queue, &extras);
    (out, unaffordable, blocked)
}

// ---------------------------------------------------------------------------
// aa-production callbacks

impl aa_production::World for Host<'_> {
    fn actor_state(&mut self, _queue: &mut Queue, _state: u32, _enabled: bool) -> R<()> {
        // 4D64F0 structure_normal / _recruiting / _researching: aa-play keeps
        // no structure states.
        Ok(())
    }
    fn cancel_schedule(&mut self, queue: &mut Queue) -> R<()> {
        queue.scheduled = false;
        Ok(())
    }
    /// 0x50AC00(0): the periodic queue-0 registration draws its bucket from
    /// the world RNG (0x50B2C0: GetRNG then one draw, % 1 bucket).
    fn schedule_zero(&mut self, queue: &mut Queue) -> R<()> {
        self.world.random_u32();
        queue.scheduled = true;
        Ok(())
    }
    fn organization_definition(&mut self, actor_definition: u32) -> R<bool> {
        Ok(self.def(actor_definition)?.has_component("OrganizationComponent"))
    }
    fn organization_recruit_time(&mut self, actor_definition: u32) -> R<u32> {
        Ok(self.def(actor_definition)?.recruit_time.to_bits())
    }
    /// ElementComponent resupply_rate (ActorData +0x2CC).
    fn recruit_rate(&mut self, actor_definition: u32) -> R<Binary> {
        Ok(Binary::from_f32_bits(self.def(actor_definition)?.resupply_rate.to_bits())?)
    }
    fn research_duration(&mut self, property: u32) -> R<u32> {
        let (def, index) = record_of(property).ok_or(aa_production::Error::Invalid("research record"))?;
        Ok(self.world.data.def(def).research.get(index).map_or(0.0f32, |r| r.research_time).to_bits())
    }
    /// The LIMITED_CONSUMED part of the item's upkeep quote.
    fn limited_upkeep(&mut self, item: &aa_production::QueueItem) -> R<Vec<u32>> {
        Ok(bits(&limited_part(self.world, &floats(&item.requested_upkeep))))
    }
    fn can_reserve_upkeep(&mut self, _queue: &mut Queue, item: &aa_production::QueueItem, notify: bool) -> R<bool> {
        let Some(k) = self.kingdom() else { return Ok(false) };
        let limited = limited_part(self.world, &floats(&item.requested_upkeep));
        let current = floats(&item.current_upkeep);
        let Some(kb) = self.world.books.kingdom(k) else { return Ok(false) };
        let ok = self.world.data.resources.iter().enumerate().all(|(i, r)| {
            let need = limited.get(i).copied().unwrap_or(0.0) - current.get(i).copied().unwrap_or(0.0);
            if r.kind != crate::data::ResourceKind::LimitedConsumed || need <= 0.0 {
                return true;
            }
            let Some(p) = r.limited_by else { return true };
            kb.consumed.get(i).copied().unwrap_or(0.0) + need <= kb.provided.get(p).copied().unwrap_or(0.0)
        });
        if !ok && notify {
            self.world.books.notices.push(crate::books::Notice::ProductionPaused { building: self.building, reason: PauseReason::LimitedResource });
        }
        Ok(ok)
    }
    /// 0x4E48C0 on the owner kingdom.
    fn adjust_upkeep(&mut self, _queue: &mut Queue, values: &[u32], add: bool) -> R<()> {
        if let Some(k) = self.kingdom() {
            crate::economy::adjust_consumption(self.world, k, &floats(values), add);
            crate::economy::publish(self.world, k);
        }
        Ok(())
    }
    fn actor_disabled(&mut self, _queue: &mut Queue) -> R<bool> {
        Ok(busy(self.world, self.building))
    }
    fn actor_requires_supply(&mut self, _queue: &mut Queue) -> R<bool> {
        Ok(self.world.data.def(self.building_def()).supply_required)
    }
    fn actor_in_supply(&mut self, _queue: &mut Queue) -> R<bool> {
        Ok(!self.world.books.out_of_supply(self.building))
    }
    fn notify_pause(&mut self, _queue: &mut Queue, reason: i32) -> R<()> {
        let reason = match reason {
            0 => PauseReason::LimitedResource,
            1 => PauseReason::Supply,
            _ => PauseReason::Disabled,
        };
        self.world.books.notices.push(crate::books::Notice::ProductionPaused { building: self.building, reason });
        Ok(())
    }
    /// The organization item's HQ attachment reservation is released.
    fn detach_organization_slot(&mut self, _queue: &mut Queue, item: &aa_production::QueueItem) -> R<Option<u32>> {
        if self.item_extra(item).is_some_and(|e| e.attaches)
            && let Some(o) = hq_org(self.world, self.building)
        {
            let b = self.world.books.org_mut(o);
            b.reserved_attachments = b.reserved_attachments.saturating_sub(1);
            return Ok(Some(0));
        }
        Ok(None)
    }
    fn fulfill(&mut self, _queue: &mut Queue, item: &aa_production::QueueItem) -> R<bool> {
        if item.kind == 1 {
            let record = self.record(item.property).ok_or(aa_production::Error::Invalid("research record"))?;
            let mut component = self.component();
            let done = research::complete_research(&mut component, &record, PRECISION, self)?;
            self.store_component(component);
            return Ok(done);
        }
        let play = self.play_item(item);
        Ok(recruit_fulfill(self.world, self.building, &play))
    }
    fn refund(&mut self, _queue: &mut Queue, item: &aa_production::QueueItem) -> R<()> {
        if item.kind == 1 {
            if let Some(record) = self.record(item.property) {
                research::refund_research(&record, &item.paid_cost, self)?;
            }
            return Ok(());
        }
        if let Some(k) = self.kingdom() {
            credit(self.world, k, &floats(&item.paid_cost), 1.0);
        }
        Ok(())
    }
    fn release_item(&mut self, _queue: &mut Queue, item: &aa_production::QueueItem) -> R<()> {
        self.extras.remove(&item.id);
        Ok(())
    }
}

impl aa_production::Recruitment for Host<'_> {
    fn kingdom(&mut self) -> R<Option<u32>> {
        Ok(Host::kingdom(self))
    }
    fn can_enroll_organization(&mut self, definition: u32) -> R<bool> {
        Ok(def_of(definition).is_some_and(|d| can_enroll(self.world, self.building, d)))
    }
    /// ActorData +0x2F0: the definition's own layout (aa-play keeps one).
    fn default_layout(&mut self, _definition: u32) -> R<u32> {
        Ok(1)
    }
    /// 0x5827B0 -> 0x581F70.
    fn recruitability(&mut self, definition: u32, _layout: u32) -> R<i32> {
        let d = def_of(definition).ok_or(aa_production::Error::Invalid("definition"))?;
        recruitability(self.world, self.building, d)
    }
    fn quote_cost(&mut self, definition: u32, _layout: u32) -> R<Vec<u32>> {
        let d = def_of(definition).ok_or(aa_production::Error::Invalid("definition"))?;
        Ok(bits(&recruit_cost(self.world, self.building, d)))
    }
    fn can_afford(&mut self, kingdom: u32, cost: &[u32]) -> R<bool> {
        Ok(can_afford(self.world, kingdom, &floats(cost)))
    }
    fn notify_unaffordable(&mut self) -> R<()> {
        self.unaffordable = true;
        if let Some(k) = Host::kingdom(self) {
            self.world.books.notices.push(crate::books::Notice::Unaffordable { kingdom: k, building: self.building });
        }
        Ok(())
    }
    /// 0x4E1690.
    fn debit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        credit(self.world, kingdom, &floats(cost), -1.0);
        Ok(())
    }
    /// 4E2E00 / 4E4050: the kingdom recruit item reservation (aa-play keeps
    /// no recruit items).
    fn reserve_recruit(&mut self, _kingdom: u32, _definition: u32, _layout: u32) -> R<()> {
        Ok(())
    }
    /// 56F580: the definition's upkeep quote.
    fn quote_upkeep(&mut self, definition: u32, _layout: u32) -> R<Vec<u32>> {
        let d = def_of(definition).ok_or(aa_production::Error::Invalid("definition"))?;
        Ok(bits(&super::quote(self.world, d, Vector::Upkeep)))
    }
    fn next_item_id(&mut self) -> R<u32> {
        self.world.books.next_item += 1;
        let id = self.world.books.next_item;
        if let Some(e) = self.pending.take() {
            self.extras.insert(id, e);
        }
        Ok(id)
    }
    /// The HQ organization registers the organization item (its attachment).
    fn register_organization_item(&mut self, _queue: &mut Queue, item_id: u32) -> R<()> {
        if self.extras.get(&item_id).is_some_and(|e| e.attaches)
            && let Some(o) = hq_org(self.world, self.building)
        {
            self.world.books.org_mut(o).reserved_attachments += 1;
        }
        Ok(())
    }
}

/// 0x581F70 for the recruiting structure: an organization tests every
/// layout slot definition (the last failing code wins); a single actor its
/// ElementComponent Requirements through 0x4C5250 (0 met, 1 missing).
/// No kingdom restriction profile (0x4E65E0) and no campaign gate apply in
/// a custom battle.
pub(super) fn recruitability(world: &World, building: ActorId, d: DefId) -> R<i32> {
    let def = world.data.def(d);
    if def.has_component("OrganizationComponent") {
        let mut code = 0;
        for s in def.slots.iter().map(|s| s.0).filter(|&s| s != crate::data::NO_DEF) {
            let c = recruitability(world, building, s)?;
            if c != 0 {
                code = c;
            }
        }
        return Ok(code);
    }
    let req = requirements(world, &def.requirements);
    if req.properties.is_empty() && req.actors.is_empty() {
        return Ok(0);
    }
    let mut reader = Reader { world, building };
    let kingdom = reader.kingdom();
    let outcome = research::evaluate_requirements(&req, kingdom, None, Some(actor_handle(building)), &mut reader)?;
    Ok(if outcome.met { 0 } else { 1 })
}

/// 0x581F70 with its out lists (missing structure definitions, missing
/// property handles): an organization collects over its layout slots (the
/// last failing code wins, the lists keep their first appearance).
pub(super) fn recruit_missing(world: &World, building: ActorId, d: DefId) -> R<(i32, Vec<u32>, Vec<u32>)> {
    let def = world.data.def(d);
    if def.has_component("OrganizationComponent") {
        let (mut code, mut actors, mut properties) = (0, Vec::new(), Vec::new());
        for s in def.slots.iter().map(|s| s.0).filter(|&s| s != crate::data::NO_DEF) {
            let (c, a, p) = recruit_missing(world, building, s)?;
            if c != 0 {
                code = c;
            }
            for x in a {
                if !actors.contains(&x) {
                    actors.push(x);
                }
            }
            for x in p {
                if !properties.contains(&x) {
                    properties.push(x);
                }
            }
        }
        return Ok((code, actors, properties));
    }
    let req = requirements(world, &def.requirements);
    if req.properties.is_empty() && req.actors.is_empty() {
        return Ok((0, Vec::new(), Vec::new()));
    }
    let mut reader = Reader { world, building };
    let kingdom = reader.kingdom();
    let outcome = research::evaluate_requirements(&req, kingdom, None, Some(actor_handle(building)), &mut reader)?;
    Ok((if outcome.met { 0 } else { 1 }, outcome.missing_actors, outcome.missing_properties))
}

/// 0x5842F0 with the lists of 0x4C5250 for a research record of `building`.
pub(super) fn research_missing(world: &World, building: ActorId, record: &ResearchItem) -> R<(i32, Vec<u32>, Vec<u32>)> {
    let mut reader = Reader { world, building };
    let kingdom = reader.kingdom();
    let outcome = research::evaluate_requirements(&record.requirements, kingdom, None, Some(actor_handle(building)), &mut reader)?;
    let code = if outcome.met {
        0
    } else if outcome.properties_met {
        1
    } else {
        2
    };
    Ok((code, outcome.missing_actors, outcome.missing_properties))
}

/// Handles back to aa-play ids.
pub(super) fn defs_of(handles: &[u32]) -> Vec<DefId> {
    handles.iter().filter_map(|&h| def_of(h)).collect()
}
pub(super) fn properties_of(world: &World, handles: &[u32]) -> Vec<String> {
    handles.iter().filter_map(|&h| property_of(world, h).map(str::to_owned)).collect()
}

/// An actor handle for the requirement callbacks (never zero).
fn actor_handle(a: ActorId) -> u32 {
    a + 1
}

/// The requirement leaves (0x4C5250) over a read-only world.
pub(super) struct Reader<'a> {
    pub world: &'a World,
    pub building: ActorId,
}

impl Reader<'_> {
    fn kingdom(&self) -> Option<KingdomId> {
        self.world.actors.get(self.building as usize).and_then(|a| a.kingdom)
    }
    /// The building's properties (definition, map) contain `name`, or a
    /// technology the kingdom applies to it.
    fn has_property(&self, name: &str) -> bool {
        let a = &self.world.actors[self.building as usize];
        let d = self.world.data.def(a.def);
        let extra = self.world.books.actor(self.building).map_or(&[][..], |b| b.properties.as_slice());
        let own = d.properties.iter().chain(extra).any(|p| p.eq_ignore_ascii_case(name));
        own || a.kingdom.is_some_and(|k| {
            self.world.books.kingdom(k).is_some_and(|kb| kb.technologies.iter().any(|t| t.eq_ignore_ascii_case(name)))
                && self.world.data.property(name).is_some_and(|p| p.required_properties.iter().all(|r| d.properties.iter().chain(extra).any(|q| q.eq_ignore_ascii_case(r))))
        })
    }
}

impl research::RequirementWorld for Reader<'_> {
    fn properties_of(&mut self, owner: u32) -> R<Option<u32>> {
        Ok(Some(owner))
    }
    fn properties_contain(&mut self, _properties: u32, definition: u32) -> R<bool> {
        Ok(property_of(self.world, definition).is_some_and(|name| self.has_property(name)))
    }
    fn is_technology(&mut self, definition: u32) -> R<bool> {
        Ok(property_of(self.world, definition).and_then(|p| self.world.data.property(p)).is_some_and(|p| p.is_technology))
    }
    fn kingdom_knows(&mut self, kingdom: u32, definition: u32) -> R<bool> {
        let Some(name) = property_of(self.world, definition) else { return Ok(false) };
        Ok(self.world.books.kingdom(kingdom).is_some_and(|k| k.technologies.iter().any(|t| t.eq_ignore_ascii_case(name))))
    }
    fn settlement_component(&mut self, _settlement: u32) -> R<Option<u32>> {
        Ok(None)
    }
    fn settlement_has_actor(&mut self, _component: u32, _definition: u32) -> R<bool> {
        Ok(false)
    }
    /// 0x4E1480 -> 0x5890C0: a living structure of the kingdom with that
    /// definition, not packing / unpacking (complete).
    fn kingdom_has_actor(&mut self, kingdom: u32, definition: u32) -> R<bool> {
        let Some(d) = def_of(definition) else { return Ok(false) };
        let w = self.world;
        Ok(w.actors.iter().any(|a| a.alive && !a.removed && a.kingdom == Some(kingdom) && a.def == d && !busy(w, a.id)))
    }
    fn technology_allowed(&mut self, _kingdom: u32, _definition: u32) -> R<bool> {
        Ok(true)
    }
    fn describe_missing(&mut self, _properties: bool, _list: &[u32]) -> R<()> {
        Ok(())
    }
    fn missing_requirements_message(&mut self, _item: &ResearchItem) -> R<()> {
        Ok(())
    }
}
impl research::Research for Host<'_> {
    fn research_kingdom(&mut self) -> R<Option<u32>> {
        Ok(Host::kingdom(self))
    }
    /// Actor virtual +0x34 with state 0x23: a packing / unpacking building
    /// cannot research.
    fn research_state_active(&mut self, _state: u32) -> R<bool> {
        Ok(!busy(self.world, self.building))
    }
    fn organization(&mut self) -> R<Option<research::Organization>> {
        Ok(None)
    }
    fn kingdom_research(&mut self, kingdom: u32) -> R<&mut KingdomResearch> {
        let n = self.n();
        Ok(&mut self.world.books.kingdom_mut(kingdom, n).research)
    }
    fn organization_has_property(&mut self, _properties: u32, _definition: u32) -> R<bool> {
        Ok(false)
    }
    fn kingdom_has_property(&mut self, kingdom: u32, definition: u32) -> R<bool> {
        let mut reader = Reader { world: &*self.world, building: self.building };
        research::RequirementWorld::kingdom_knows(&mut reader, kingdom, definition)
    }
    fn research_requirements(&mut self, kingdom: Option<u32>, _organization: Option<research::Organization>, item: &ResearchItem, notify: bool) -> R<i32> {
        let mut reader = Reader { world: &*self.world, building: self.building };
        research::research_requirements(item, kingdom, None, Some(actor_handle(self.building)), notify, &mut reader)
    }
    fn research_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> R<bool> {
        Ok(can_afford(self.world, kingdom, &floats(cost)))
    }
    fn research_debit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        credit(self.world, kingdom, &floats(cost), -1.0);
        Ok(())
    }
    fn research_credit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        credit(self.world, kingdom, &floats(cost), 1.0);
        Ok(())
    }
    fn notify_research_blocked(&mut self, _item: &ResearchItem, _kingdom: Option<u32>) -> R<()> {
        self.blocked.get_or_insert(1);
        Ok(())
    }
    fn notify_research_unaffordable(&mut self, _item: &ResearchItem, _cost: &[u32], kingdom: Option<u32>) -> R<()> {
        self.unaffordable = true;
        if let Some(k) = kingdom {
            self.world.books.notices.push(crate::books::Notice::Unaffordable { kingdom: k, building: self.building });
        }
        Ok(())
    }
    fn next_research_item_id(&mut self) -> R<u32> {
        aa_production::Recruitment::next_item_id(self)
    }
    fn statistics(&mut self, _kingdom: u32) -> R<Option<&mut aa_economy::tick::Statistics>> {
        Ok(None)
    }
    /// 0x42D490(0xB): research_complete.
    fn research_complete_event(&mut self, kingdom: Option<u32>, item: &ResearchItem) -> R<()> {
        if let (Some(k), Some(p)) = (kingdom, property_of(self.world, item.definition).map(str::to_owned)) {
            self.world.books.notices.push(crate::books::Notice::Researched { kingdom: k, property: p });
        }
        Ok(())
    }
    /// LOCAL / SUPPORT properties (no shipped research item is either).
    fn apply_property(&mut self, _target: PropertyTarget, _definition: u32) -> R<()> {
        Ok(())
    }
    /// 0x4E3AD0: the kingdom gains the technology.
    fn kingdom_add_property(&mut self, kingdom: u32, definition: u32) -> R<()> {
        let Some(name) = property_of(self.world, definition).map(str::to_owned) else { return Err(err("unknown property")) };
        let n = self.n();
        let kb = self.world.books.kingdom_mut(kingdom, n);
        if !kb.technologies.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
            kb.technologies.push(name);
        }
        Ok(())
    }
    fn when_researched(&mut self, _item: &ResearchItem) -> R<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 0x574C10 HQ attach

/// The HQ actor's purchase: attaching organization `org` to it.
pub(super) struct Attach<'a> {
    pub world: &'a mut World,
    pub hq: ActorId,
    pub org: OrgId,
}

impl aa_production::purchases::Purchase for Attach<'_> {
    fn purchase_owner(&mut self) -> R<Option<u32>> {
        Ok(self.world.actors.get(self.hq as usize).and_then(|a| a.kingdom))
    }
    fn purchase_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> R<bool> {
        Ok(can_afford(self.world, kingdom, &floats(cost)))
    }
    fn purchase_debit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        credit(self.world, kingdom, &floats(cost), -1.0);
        Ok(())
    }
    fn purchase_credit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        credit(self.world, kingdom, &floats(cost), 1.0);
        Ok(())
    }
}

impl aa_production::purchases::HqAttach for Attach<'_> {
    /// 0x56FA30 (organization layout cost quote).
    fn organization_cost(&mut self, _organization: u32) -> R<Vec<u32>> {
        let def = self.world.orgs[self.org as usize].def;
        Ok(bits(&super::quote(self.world, def, Vector::Cost)))
    }
    /// HQAttachCostPercentage (cache +0xC0).
    fn attach_cost_factor(&self) -> u32 {
        self.world.data.rule_ci("HQAttachCostPercentage").unwrap_or(0.0).to_bits()
    }
    fn registry_count(&self) -> usize {
        self.world.data.resources.len()
    }
    fn precision(&self) -> aa_economy::X87Precision {
        aa_economy::X87Precision::Double53
    }
    fn attach_unaffordable(&mut self, _organization: u32, _cost: &[u32], kingdom: u32) -> R<()> {
        self.world.books.notices.push(crate::books::Notice::Unaffordable { kingdom, building: self.hq });
        Ok(())
    }
    /// 0x574810: the HQ takes another supply-using organization while it has
    /// a free attachment.
    fn attach_allowed(&mut self, _organization: u32, _notify: bool) -> R<bool> {
        let w = &*self.world;
        let o = &w.orgs[self.org as usize];
        let od = w.data.def(o.def);
        if od.max_attachments.is_some() || !od.supply_used || od.class != Some(DefClass::Organization) {
            return Ok(false);
        }
        let Some(hq_org) = hq_org(w, self.hq) else { return Ok(false) };
        let max = w.data.def(w.orgs[hq_org as usize].def).max_attachments.unwrap_or(0);
        let reserved = w.books.org(hq_org).map_or(0, |b| b.reserved_attachments);
        Ok(super::attachments(w, self.hq) + reserved < max)
    }
    fn create_attachment(&mut self, _organization: u32, _slot: u32) -> R<()> {
        Ok(())
    }
    /// organization +0x7C -> 0x5783B0: the organization's HQ.
    fn attach_to_organization(&mut self, _organization: u32) -> R<()> {
        self.world.orgs[self.org as usize].hq = Some(self.hq);
        self.world.books.notices.push(crate::books::Notice::Attached { org: self.org, hq: self.hq });
        Ok(())
    }
    /// 0x42D470(0x3C): the paid attach notification (the Attached notice).
    fn attach_event(&mut self, _organization: u32) -> R<()> {
        Ok(())
    }
}
