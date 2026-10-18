//! Original research admission, payment, queueing and completion.
//!
//! * `0x584790` ResearchComponent request, `0x5845F0` affordability,
//!   `0x5844A0` blocked reason, `0x584030` component already-researched test.
//! * `0x589F00`/`0x589830` research queue item creation (kind 1).
//! * `0x588A40` structure queue contains research item.
//! * `0x4E2310`/`0x4E2360`/`0x4E2390` kingdom-level research in progress list
//!   (kingdom+0x324 head, +0x328 tail).
//! * `0x583F30` research refund (credit and reservation release).
//! * `0x584120` research completion (score 2, completed list, event 0xB,
//!   property application by research level, reservation release, trigger).
//! * `0x4C3D70` is a plain copy of the item's ResearchCost vector: no modifier
//!   or price formula is applied to research costs.
//!
//! Requirement evaluation (`0x5842F0`), property containers (`0x5032D0` via
//! `0x581280`/`0x4E0FB0`), property application (`0x4CEF10`), kingdom
//! technology add (`0x4E3AD0`), messages and triggers are explicit callbacks.
use crate::{Context, Error, Queue, QueueItem, Result, World, enqueue_prepared};
use aa_economy::tick::{SCORE_TECHS_RESEARCHED, Statistics, add_score};
use aa_economy::{F32Bits, X87Precision};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// ActorData research list entry. `id` is the native record identity (the
/// pointer the queue item stores at +0x18); `definition` is record+0 (property
/// definition, 0 = null); `kind` is record+0x2C (0 actor, 1 organization, 2
/// kingdom); `cost` is the ResearchCost vector at record+0x18.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchItem {
    pub id: u32,
    pub definition: u32,
    pub kind: u32,
    pub cost: Vec<u32>,
    /// Record+4 requirements evaluated by `0x5842F0`.
    #[serde(default)]
    pub requirements: Requirements,
}

/// Requirements object (research record+4, also used by recruitment): property
/// definitions at +0 (list, OR mode byte +8) and actor definitions at +0xC
/// (list, OR mode byte +0x14). Lists keep native order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirements {
    pub properties: Vec<u32>,
    pub any_property: bool,
    pub actors: Vec<u32>,
    pub any_actor: bool,
}

/// Result of `0x4C5250` with the lists `0x4C5630` hands to the text builder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementOutcome {
    pub met: bool,
    /// The `known` out byte: property requirements satisfied.
    pub properties_met: bool,
    pub missing_properties: Vec<u32>,
    pub missing_actors: Vec<u32>,
}

/// Leaves used by `0x4C5250`/`0x5842F0`.
pub trait RequirementWorld {
    /// `[x+0x78]` Properties component of an actor or settlement.
    fn properties_of(&mut self, owner: u32) -> Result<Option<u32>>;
    /// `properties.0x581280(definition, 0)`.
    fn properties_contain(&mut self, properties: u32, definition: u32) -> Result<bool>;
    /// Property definition virtual +0x18 (technology).
    fn is_technology(&mut self, definition: u32) -> Result<bool>;
    /// `kingdom.0x4E0FB0(definition, 0)`.
    fn kingdom_knows(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// `[settlement+0x98]` SettlementComponent.
    fn settlement_component(&mut self, settlement: u32) -> Result<Option<u32>>;
    /// `component.0x585500(definition, 1, 0)`.
    fn settlement_has_actor(&mut self, component: u32, definition: u32) -> Result<bool>;
    /// `kingdom.0x4E1480(definition, 1, 0)`.
    fn kingdom_has_actor(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// `kingdom.0x4E1890(definition)`: team and kingdom technology lists.
    fn technology_allowed(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// `0x4C5560` text builder over a missing list (properties first).
    fn describe_missing(&mut self, properties: bool, list: &[u32]) -> Result<()>;
    /// `0x405D60` research_error_missing_requirements|ss into the caller's
    /// message (only when the caller passed a message buffer).
    fn missing_requirements_message(&mut self, item: &ResearchItem) -> Result<()>;
}

/// `0x4C5250` (as called by `0x4C5630` with skip = 0).
pub fn evaluate_requirements(
    requirements: &Requirements,
    kingdom: Option<u32>,
    settlement: Option<u32>,
    actor: Option<u32>,
    world: &mut impl RequirementWorld,
) -> Result<RequirementOutcome> {
    let actor_properties = match actor {
        Some(a) => world.properties_of(a)?,
        None => None,
    };
    let settlement_properties = match settlement {
        Some(s) => world.properties_of(s)?,
        None => None,
    };
    let mut out = RequirementOutcome::default();
    let mut any = requirements.properties.is_empty();
    let mut all = true;
    for &property in &requirements.properties {
        let mut known = false;
        if let Some(p) = actor_properties {
            known = world.properties_contain(p, property)?;
        }
        if !known && let Some(p) = settlement_properties {
            known = world.properties_contain(p, property)?;
        }
        if !known
            && world.is_technology(property)?
            && let Some(k) = kingdom
        {
            known = world.kingdom_knows(k, property)?;
        }
        if known {
            if requirements.any_property {
                any = true;
                break;
            }
            continue;
        }
        if !out.missing_properties.contains(&property) {
            out.missing_properties.push(property);
        }
        all = false;
    }
    out.properties_met = if requirements.any_property { any } else { all };
    let component = match settlement {
        Some(s) => world.settlement_component(s)?,
        None => None,
    };
    let mut any = requirements.actors.is_empty();
    let mut all = true;
    for &definition in &requirements.actors {
        let mut present = false;
        if let Some(c) = component {
            present = world.settlement_has_actor(c, definition)?;
        }
        if !present && let Some(k) = kingdom {
            present = world.kingdom_has_actor(k, definition)?;
        }
        if present {
            any = true;
            if requirements.any_actor {
                break;
            }
            continue;
        }
        if !out.missing_actors.contains(&definition) {
            out.missing_actors.push(definition);
        }
        all = false;
    }
    let actors_met = if requirements.any_actor { any } else { all };
    out.met = out.properties_met && actors_met;
    Ok(out)
}

/// `0x5842F0(kingdom, settlement, actor, item, message)`: 0 met, 1 property
/// requirements met but actors missing (message filled when requested), 2 not
/// allowed or property requirements missing.
pub fn research_requirements(
    item: &ResearchItem,
    kingdom: Option<u32>,
    settlement: Option<u32>,
    actor: Option<u32>,
    message: bool,
    world: &mut impl RequirementWorld,
) -> Result<i32> {
    if let Some(k) = kingdom
        && !world.technology_allowed(k, item.definition)?
    {
        return Ok(2);
    }
    let outcome = evaluate_requirements(&item.requirements, kingdom, settlement, actor, world)?;
    if outcome.properties_met {
        // 0x4C5630 builds the missing-list text whenever the known byte is set.
        world.describe_missing(false, &outcome.missing_actors)?;
        world.describe_missing(true, &outcome.missing_properties)?;
    }
    if outcome.met {
        return Ok(0);
    }
    if !outcome.properties_met {
        return Ok(2);
    }
    if message {
        world.missing_requirements_message(item)?;
    }
    Ok(1)
}

/// ResearchComponent+0x10/+0x14: completed research items, head first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchComponent {
    pub completed: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    pub item: u32,
    pub definition: u32,
}

/// GKingdom+0x324/+0x328: kingdom-level research in progress, head first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomResearch {
    pub in_progress: Vec<Reservation>,
}

impl KingdomResearch {
    /// `0x4E2310`: push front only for a non-null kingdom-level definition.
    pub fn reserve(&mut self, item: &ResearchItem) -> Result<()> {
        if item.definition != 0 && item.kind == 2 {
            self.in_progress
                .try_reserve(1)
                .map_err(|_| Error::Allocation)?;
            self.in_progress.insert(
                0,
                Reservation {
                    item: item.id,
                    definition: item.definition,
                },
            );
        }
        Ok(())
    }
    /// `0x4E2360` -> `0x4F1430`: remove the first node holding this record.
    pub fn release(&mut self, item: &ResearchItem) {
        if item.definition != 0
            && item.kind == 2
            && let Some(index) = self.in_progress.iter().position(|r| r.item == item.id)
        {
            self.in_progress.remove(index);
        }
    }
    /// `0x4E2390`: kingdom-level records compare by definition, not identity.
    pub fn contains(&self, item: &ResearchItem) -> bool {
        item.kind == 2
            && self
                .in_progress
                .iter()
                .any(|r| r.definition == item.definition)
    }
}

/// `0x588A40`: a queued research (kind 1) item for the same record.
pub fn queue_contains_research(queue: &Queue, item: &ResearchItem) -> bool {
    queue
        .items
        .iter()
        .any(|queued| queued.kind == 1 && queued.property == item.id)
}

/// Organization returned by `0x4CED40`; `properties` is organization+0x78
/// (0 = null), the container `0x581280` queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: u32,
    pub properties: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyTarget {
    /// `actor.0x4CEF10(definition, 2, 0)`.
    Actor,
    /// `organization.0x4CEF10(definition, 8, actor)`.
    Organization(u32),
}

/// Required research leaves. Kingdom identities are opaque and must be the
/// actual owner returned by the actor's virtual +0x108 at each native call.
pub trait Research: World {
    fn research_kingdom(&mut self) -> Result<Option<u32>>;
    /// Actor virtual +0x34 with state 0x23.
    fn research_state_active(&mut self, state: u32) -> Result<bool>;
    fn organization(&mut self) -> Result<Option<Organization>>;
    fn kingdom_research(&mut self, kingdom: u32) -> Result<&mut KingdomResearch>;
    fn organization_has_property(&mut self, properties: u32, definition: u32) -> Result<bool>;
    fn kingdom_has_property(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// `0x5842F0(kingdom, organization, actor, item, notify)`: 0 ok, 1 missing
    /// requirements, 2 not on the allowed list.
    fn research_requirements(
        &mut self,
        kingdom: Option<u32>,
        organization: Option<Organization>,
        item: &ResearchItem,
        notify: bool,
    ) -> Result<i32>;
    fn research_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool>;
    fn research_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    /// `kingdom.0x4E39D0(cost)` stockpile credit with clamping.
    fn research_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    /// research_error_generic|s through `0x40C230` (message kingdom reread).
    fn notify_research_blocked(&mut self, item: &ResearchItem, kingdom: Option<u32>) -> Result<()>;
    /// research_error_affording|ss through `0x40C230`.
    fn notify_research_unaffordable(
        &mut self,
        item: &ResearchItem,
        cost: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()>;
    /// `++world+0x198` inside `0x589830`.
    fn next_research_item_id(&mut self) -> Result<u32>;
    fn statistics(&mut self, kingdom: u32) -> Result<Option<&mut Statistics>>;
    /// `0x42D490(0xB, kingdom, actor, name)`: research_complete.
    fn research_complete_event(&mut self, kingdom: Option<u32>, item: &ResearchItem) -> Result<()>;
    fn apply_property(&mut self, target: PropertyTarget, definition: u32) -> Result<()>;
    /// `kingdom.0x4E3AD0(definition, 2, 0, notify=0)`.
    fn kingdom_add_property(&mut self, kingdom: u32, definition: u32) -> Result<()>;
    /// `0x42A590(definition+0x54, actor, 0)`: the property's researched trigger.
    fn when_researched(&mut self, item: &ResearchItem) -> Result<()>;
}

fn x87(precision: Precision) -> X87Precision {
    match precision {
        Precision::Single24 => X87Precision::Single24,
        Precision::Double53 => X87Precision::Double53,
        Precision::Extended64 => X87Precision::Extended64,
    }
}

/// `0x584030`.
pub fn component_completed(component: &ResearchComponent, item: &ResearchItem) -> bool {
    component.completed.contains(&item.id)
}

/// `0x5844A0`: 0 allowed, 2 research state inactive or requirement reason,
/// 4 already queued/in progress/known; other values from `0x5842F0`.
pub fn research_blocked(
    component: &ResearchComponent,
    queue: &Queue,
    item: &ResearchItem,
    notify: bool,
    world: &mut impl Research,
) -> Result<i32> {
    if !world.research_state_active(0x23)? {
        return Ok(2);
    }
    let kingdom = world.research_kingdom()?;
    if queue_contains_research(queue, item) {
        return Ok(4);
    }
    if let Some(kingdom) = kingdom
        && world.kingdom_research(kingdom)?.contains(item)
    {
        return Ok(4);
    }
    let organization = world.organization()?;
    let known = match item.kind {
        0 => component_completed(component, item),
        1 => match organization {
            Some(org) if org.properties != 0 => {
                world.organization_has_property(org.properties, item.definition)?
            }
            _ => false,
        },
        2 => match kingdom {
            Some(kingdom) => world.kingdom_has_property(kingdom, item.definition)?,
            None => false,
        },
        _ => false,
    };
    if known {
        return Ok(4);
    }
    let organization = world.organization()?;
    let kingdom = world.research_kingdom()?;
    world.research_requirements(kingdom, organization, item, notify)
}

/// `0x5845F0`: one copy of ResearchCost, `0x4E20A0`, optional message.
pub fn research_affordable(
    item: &ResearchItem,
    notify: bool,
    world: &mut impl Research,
) -> Result<bool> {
    let Some(kingdom) = world.research_kingdom()? else {
        return Ok(false);
    };
    let cost = item.cost.clone();
    if world.research_can_afford(kingdom, &cost)? {
        return Ok(true);
    }
    if notify {
        let message_kingdom = world.research_kingdom()?;
        world.notify_research_unaffordable(item, &cost, message_kingdom)?;
    }
    Ok(false)
}

/// `0x584790`. Payment uses the kingdom captured first; queueing (which may
/// start and process the head) precedes the kingdom-level reservation. No
/// rollback is performed when a later required callback fails.
pub fn request_research(
    component: &ResearchComponent,
    queue: &mut Queue,
    item: &ResearchItem,
    context: Context,
    world: &mut impl Research,
) -> Result<bool> {
    let Some(kingdom) = world.research_kingdom()? else {
        return Ok(false);
    };
    if item.id == 0 {
        return Err(Error::Invalid(
            "research request requires a research record",
        ));
    }
    if research_blocked(component, queue, item, false, world)? != 0 {
        let message_kingdom = world.research_kingdom()?;
        world.notify_research_blocked(item, message_kingdom)?;
        return Ok(false);
    }
    if !research_affordable(item, true, world)? {
        return Ok(false);
    }
    let cost = item.cost.clone();
    world.research_debit(kingdom, &cost)?;
    let id = world.next_research_item_id()?;
    let queued = QueueItem {
        id,
        kind: 1,
        actor_definition: 0,
        organization_layout: 0,
        location_bits: [0; 2],
        property: item.id,
        paid_cost: cost,
        // 0x4CD440 sizes both upkeep vectors to the resource registry, zeroed.
        requested_upkeep: vec![0; item.cost.len()],
        current_upkeep: vec![0; item.cost.len()],
        organization_slot: 0,
    };
    enqueue_prepared(queue, queued, context, world)?;
    world.kingdom_research(kingdom)?.reserve(item)?;
    Ok(true)
}

/// `0x583F30`: research refund used by cancellation (`0x589BD0`) and failed
/// completion (`0x589F90`). Credits the paid cost and releases a kingdom-level
/// reservation on the owner queried now; nothing happens without an owner.
pub fn refund_research(
    item: &ResearchItem,
    paid_cost: &[u32],
    world: &mut impl Research,
) -> Result<()> {
    if let Some(kingdom) = world.research_kingdom()? {
        world.research_credit(kingdom, paid_cost)?;
        world.kingdom_research(kingdom)?.release(item);
    }
    Ok(())
}

/// `0x584120`, always returns true natively. A null owner kingdom with a
/// kingdom-level record would dereference null in `0x4E2360`; that is an error.
pub fn complete_research(
    component: &mut ResearchComponent,
    item: &ResearchItem,
    precision: Precision,
    world: &mut impl Research,
) -> Result<bool> {
    let kingdom = world.research_kingdom()?;
    if let Some(kingdom) = kingdom
        && let Some(statistics) = world.statistics(kingdom)?
    {
        add_score(
            statistics,
            SCORE_TECHS_RESEARCHED,
            F32Bits(0x3f80_0000),
            x87(precision),
        )?;
    }
    component
        .completed
        .try_reserve(1)
        .map_err(|_| Error::Allocation)?;
    component.completed.insert(0, item.id);
    let event_kingdom = world.research_kingdom()?;
    world.research_complete_event(event_kingdom, item)?;
    match item.kind {
        0 => world.apply_property(PropertyTarget::Actor, item.definition)?,
        1 => {
            if let Some(organization) = world.organization()? {
                world.apply_property(
                    PropertyTarget::Organization(organization.id),
                    item.definition,
                )?;
            }
        }
        2 => {
            if let Some(owner) = world.research_kingdom()? {
                world.kingdom_add_property(owner, item.definition)?;
            }
        }
        _ => {}
    }
    match kingdom {
        Some(kingdom) => world.kingdom_research(kingdom)?.release(item),
        None if item.definition != 0 && item.kind == 2 => {
            return Err(Error::Invalid(
                "kingdom-level research completed without an owner kingdom",
            ));
        }
        None => {}
    }
    world.when_researched(item)?;
    Ok(true)
}

/// Property instance record (0x18 bytes, constructor `0x5030F0`) in the
/// kingdom property state vector at GKingdom+0x318.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyRecord {
    /// +4 world time at creation (world+0xE0).
    pub time: u32,
    /// +8 source code (2 = research).
    pub source: u32,
    /// +0xC property definition.
    pub definition: u32,
    /// +0x10 duplicate flag (an earlier record has the same definition).
    pub duplicate: bool,
    /// +0x14 GTID of the origin object (0 when none).
    pub origin: u32,
}

/// GKingdom+0x318 property state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomProperties {
    pub records: Vec<PropertyRecord>,
}

impl KingdomProperties {
    /// `0x5032D0(container, definition, 0, 0)` as used by `0x4E0FB0`: any
    /// record (duplicates included) for the definition.
    pub fn contains(&self, definition: u32) -> bool {
        self.records.iter().any(|r| r.definition == definition)
    }
}

/// Leaves of GKingdom::AddProperty `0x4E3AD0`.
pub trait KingdomPropertyWorld {
    /// Property definition virtual +0x18 (technology).
    fn is_technology(&mut self, definition: u32) -> Result<bool>;
    /// Definition+0x5C byte: restricted by the allowed lists.
    fn restricted(&mut self, definition: u32) -> Result<bool>;
    /// `kingdom.0x4E1890(definition)`.
    fn technology_allowed(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// "Technology '%s' not on kingdom '%s' allowed list" (`0x404F60`).
    fn log_not_allowed(&mut self, kingdom: u32, definition: u32) -> Result<()>;
    /// world+0xE0.
    fn world_time(&mut self) -> Result<u32>;
    /// GKingdom+0x284 actor list, in list order.
    fn kingdom_actors(&mut self, kingdom: u32) -> Result<Vec<u32>>;
    /// `actor.0x4CEF10(definition, source, origin actor)` (property effects;
    /// owned by aa-body, see requests-to-lead).
    fn apply_property(
        &mut self,
        actor: u32,
        definition: u32,
        source: u32,
        origin: Option<u32>,
    ) -> Result<()>;
    /// Generation-checked lookup of a record's origin GTID (`0x5031E0`).
    fn resolve(&self, gtid: u32) -> Option<u32>;
    /// `0x42D4B0(5, kingdom, name)` technology_gained.
    fn technology_gained(&mut self, kingdom: u32, definition: u32) -> Result<()>;
}

/// `0x4E3AD0(definition, source, origin_gtid, notify)`. The record is always
/// appended; only the first record of a definition propagates to the
/// kingdom's actors and raises event 5. Returns whether it was the first.
pub fn add_kingdom_property(
    properties: &mut KingdomProperties,
    kingdom: u32,
    definition: u32,
    source: u32,
    origin: u32,
    notify: bool,
    world: &mut impl KingdomPropertyWorld,
) -> Result<bool> {
    if world.is_technology(definition)?
        && world.restricted(definition)?
        && !world.technology_allowed(kingdom, definition)?
    {
        world.log_not_allowed(kingdom, definition)?;
        return Ok(false);
    }
    let duplicate = properties.contains(definition);
    let record = PropertyRecord {
        time: world.world_time()?,
        source,
        definition,
        duplicate,
        origin,
    };
    properties
        .records
        .try_reserve(1)
        .map_err(|_| Error::Allocation)?;
    properties.records.push(record);
    if duplicate {
        return Ok(false);
    }
    for actor in world.kingdom_actors(kingdom)? {
        let origin_actor = world.resolve(origin);
        world.apply_property(actor, definition, source, origin_actor)?;
    }
    if notify {
        world.technology_gained(kingdom, definition)?;
    }
    Ok(true)
}

/// Leaves of GKingdom::RemoveProperty `0x4E23D0` beyond `KingdomPropertyWorld`.
pub trait KingdomPropertyRemoval: KingdomPropertyWorld {
    /// "unable to remove property" diagnostic of `0x5034F0` (`0x404F90`).
    fn log_missing_property(&mut self, definition: u32) -> Result<()>;
    /// `actor.0x4CEF70(definition, origin actor)`.
    fn remove_property_effect(
        &mut self,
        actor: u32,
        definition: u32,
        origin: Option<u32>,
    ) -> Result<()>;
    /// `0x42D4B0(6, kingdom, name)` technology_lost.
    fn technology_lost(&mut self, kingdom: u32, definition: u32) -> Result<()>;
}

/// `0x4E23D0(definition, origin actor)` with `0x5034F0`: removes the first
/// record whose definition matches and whose origin GTID resolves to
/// `origin`. Removing the active record promotes the next record of the same
/// definition (effects stay); otherwise effects are removed from every kingdom
/// actor and event 6 is raised. Returns whether effects were removed.
pub fn remove_kingdom_property(
    properties: &mut KingdomProperties,
    kingdom: u32,
    definition: u32,
    origin: Option<u32>,
    world: &mut impl KingdomPropertyRemoval,
) -> Result<bool> {
    let index = properties
        .records
        .iter()
        .position(|r| r.definition == definition && world.resolve(r.origin) == origin);
    let Some(index) = index else {
        world.log_missing_property(definition)?;
        return Ok(false);
    };
    let removed = properties.records.remove(index);
    if removed.duplicate {
        return Ok(false);
    }
    if let Some(next) = properties
        .records
        .iter_mut()
        .find(|r| r.definition == definition)
    {
        next.duplicate = false;
        return Ok(false);
    }
    for actor in world.kingdom_actors(kingdom)? {
        let origin_actor = world.resolve(removed.origin);
        world.remove_property_effect(actor, definition, origin_actor)?;
    }
    world.technology_lost(kingdom, definition)?;
    Ok(true)
}
