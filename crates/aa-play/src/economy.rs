//! Income, upkeep and stockpiles per kingdom, per minute like the original
//! (production applies only in supply; upkeep shortfalls). Owner: core.
//!
//! The original Kingdom economy, on the exact ports:
//! - contributions, Economy component refresh 0x570020: every economy
//!   thing keeps the provided and upkeep vectors it has added to its owner
//!   kingdom (`books::EconomyCache`); a refresh removes them and adds the new
//!   values only when they changed, through 0x4E4930 `adjust_providers` /
//!   0x4E48C0 `adjust_consumption` (aa-economy: per element, the shortage
//!   charges 0x4E1F60 removed and reapplied around each change, limited
//!   providers capped 0x4E3940), so the kingdom arrays drift exactly as the
//!   native ones. The values are the native getters (aa-actors
//!   `economy_values`): upkeep gate (member, destructed, unrecruitable
//!   organization), 56FB10 upkeep (an organization pays for every layout
//!   slot definition), 56F9A0 production eligibility and 56FC20 production
//!   (members through their organization, the packed form's values 56F960
//!   for an out-of-supply actor, RESOURCE_PRODUCTION modifier tables
//!   5019D0, NO_UPKEEP). The production queue reserves and releases its
//!   LIMITED_CONSUMED upkeep through the same 0x4E48C0 (`adjust_consumption`);
//! - GKingdom Tick 0x4E9C40 (aa-economy `Economy::tick`, PC53): stock +=
//!   (provided - consumed) * seconds/60 clamped [0, EconomyStockpileResourceMax],
//!   the shortage ratio and the kingdom_upkeep_shortage / repaired notices,
//!   the Commander's experience income 0x4CDFD0, then from world time 1.0 the
//!   victory checks 0x4E9960 / 0x4E9A40 (`crate::victory`);
//! - shortfall attrition 578660/562D10 (aa-combat `shortfall.rs`), in the
//!   organization's queue-1 maintenance tick 0x57C8E0: members of an
//!   out-of-supply supply_used organization of a kingdom in shortage lose
//!   elapsed * max_health * shortfall_damage_rate * (0.75 + u).
//!
//! The kingdom ticks are queue-0 deliveries (`crate::scheduler`), among the
//! production queues, newest registration first.
//!
//! Not native here: aa-play refreshes the contributions once per tick, before
//! the queue-0 deliveries, and after a production delivery that completed an
//! item, for every thing whose inputs changed since the last refresh, in
//! ascending native id order; the original refreshes at each state change as
//! it happens. The modifier context of an actor is its
//! definition's and map properties plus the kingdom technologies and
//! handicap, in that order.
use crate::data::{DefId, ResourceKind};
use crate::world::{ActorId, Event, KingdomId, OrgId, World};
use aa_actors::economy_values::{self as values, ActorSnapshot, Definition, GateWorld, Modifier, VectorKind};
use aa_body::F32;
use aa_economy::{Economy, F32Bits, X87Precision};

/// The simulation thread's x87 precision (PC53).
const PRECISION: aa_numeric::Precision = aa_numeric::Precision::Double53;

/// After the update's periodic deliveries (the kingdom ticks among them,
/// `crate::scheduler`): the HUD copy.
pub fn after_queue(world: &mut World) {
    for k in 0..world.kingdoms.len() as KingdomId {
        publish(world, k);
    }
}

// ---------------------------------------------------------------------------
// Kingdom arrays through aa-economy

fn catalog(world: &World) -> aa_economy::Catalog {
    let n = world.data.resources.len();
    aa_economy::Catalog {
        resources: world
            .data
            .resources
            .iter()
            .map(|r| aa_economy::ResourceDefinition {
                id: r.id.clone(),
                kind: match r.kind {
                    ResourceKind::Stockpiled => aa_economy::ResourceKind::Stockpiled,
                    ResourceKind::Upkeep => aa_economy::ResourceKind::Upkeep,
                    ResourceKind::LimitedConsumed => aa_economy::ResourceKind::LimitedConsumed,
                },
                limited_provider: r.limited_provider,
                purchase_cost: (0..n).map(|j| F32Bits(r.purchase_cost.get(j).copied().unwrap_or(0.0).to_bits())).collect(),
            })
            .collect(),
    }
}

fn limits(world: &World) -> aa_economy::Limits {
    aa_economy::Limits {
        // Native signed integer EconomyLimitedResourceMax.
        provided_maximum: world.data.rule_ci("EconomyLimitedResourceMax").map_or(i32::MAX, |v| v as i32),
        stockpiled_maximum: F32Bits(world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX).to_bits()),
    }
}

fn bits(v: &[f32]) -> Vec<F32Bits> {
    v.iter().map(|x| F32Bits(x.to_bits())).collect()
}

/// Run `f` on kingdom `k`'s arrays as an aa-economy `Economy` and store the
/// arrays back (the native stores are immediate, also after an error).
fn with_kingdom<R>(world: &mut World, k: KingdomId, f: impl FnOnce(&mut Economy) -> R) -> Option<R> {
    if k as usize >= world.kingdoms.len() {
        return None;
    }
    let n = world.data.resources.len();
    let catalog = catalog(world);
    let limits = limits(world);
    let kb = world.books.kingdom_mut(k, n);
    let state = aa_economy::State {
        raw_provided: bits(&kb.raw_provided[..n]),
        effective_provided: bits(&kb.provided[..n]),
        consumed: bits(&kb.consumed[..n]),
        stockpiled: bits(&kb.stockpiled[..n]),
    };
    let mut economy = Economy::new(catalog, state, limits, X87Precision::Double53).ok()?;
    let out = f(&mut economy);
    let s = economy.state();
    let kb = world.books.kingdom_mut(k, n);
    for (dst, src) in [
        (&mut kb.raw_provided, &s.raw_provided),
        (&mut kb.provided, &s.effective_provided),
        (&mut kb.consumed, &s.consumed),
        (&mut kb.stockpiled, &s.stockpiled),
    ] {
        for (d, v) in dst.iter_mut().zip(src) {
            *d = f32::from_bits(v.0);
        }
    }
    Some(out)
}

/// 0x4E48C0 on kingdom `k` (per element: shortage charges off, consumed +=
/// value, charges on). Used for actor upkeep and the production queue's
/// reservations.
pub fn adjust_consumption(world: &mut World, k: KingdomId, values: &[f32], add: bool) {
    let n = world.data.resources.len();
    let mut v = values.to_vec();
    v.resize(n, 0.0);
    with_kingdom(world, k, |e| e.adjust_consumption(&bits(&v), add).map(|_| ()));
}

/// 0x4E4930 on kingdom `k` (per element 0x4E3940).
pub fn adjust_providers(world: &mut World, k: KingdomId, values: &[f32], add: bool) {
    let n = world.data.resources.len();
    let mut v = values.to_vec();
    v.resize(n, 0.0);
    with_kingdom(world, k, |e| e.adjust_providers(&bits(&v), add).map(|_| ()));
}

fn add_upkeep_books(world: &mut World, k: KingdomId, values: &[f32], add: bool) {
    let n = world.data.resources.len();
    let kb = world.books.kingdom_mut(k, n);
    for (u, v) in kb.upkeep.iter_mut().zip(values) {
        *u = if add { *u + v } else { *u - v };
    }
}

// ---------------------------------------------------------------------------
// The value getters (aa-actors economy_values) over aa-play

/// The identity economy_values works with: actor `a` is `a + 1`, organization
/// `o` is `ORG | o` (never a native GTID).
const ORG: u32 = 0x8000_0000;
type Id = aa_actors::ActorId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Thing {
    Actor(ActorId),
    Org(OrgId),
}
impl Thing {
    fn id(self) -> Id {
        match self {
            Thing::Actor(a) => Id::from_raw(a + 1),
            Thing::Org(o) => Id::from_raw(ORG | o),
        }
    }
    fn of(id: Id) -> Option<Self> {
        match id.raw() {
            0 => None,
            r if r & ORG != 0 => Some(Thing::Org(r & !ORG)),
            r => Some(Thing::Actor(r - 1)),
        }
    }
}

fn err(m: &str) -> values::Error {
    values::Error::World(m.into())
}
fn f(v: f32) -> F32 {
    F32(v.to_bits())
}
fn has(d: &crate::data::Def, bit: u32) -> bool {
    d.components & (1 << bit) != 0
}
const ORGANIZATION_COMPONENT: u32 = 7;
const PROPERTIES_COMPONENT: u32 = 6;
const ECONOMY_COMPONENT: u32 = 9;
const SUPPLY_COMPONENT: u32 = 11;
const TRANSPORT_COMPONENT: u32 = 24;
/// Native actor state bits (aa-actors `state::DESCRIPTORS`).
const STATE_OUT_OF_SUPPLY: u64 = 1 << 19;
const STATE_DEAD: u64 = 1 << 23;
const STATE_DISABLED: u64 = 1 << 39;

struct Values<'a> {
    world: &'a World,
}

impl Values<'_> {
    fn living_member(&self, o: OrgId, slot: usize) -> Option<ActorId> {
        let w = self.world;
        w.orgs.get(o as usize)?.members.iter().copied().find(|&m| {
            w.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed && a.org == Some(o) && a.slot == Some(slot as u16))
        })
    }
    fn org_of(&self, raw: u32) -> Result<OrgId, values::Error> {
        match Thing::of(Id::from_raw(raw)) {
            Some(Thing::Org(o)) if (o as usize) < self.world.orgs.len() => Ok(o),
            _ => Err(err("not an organization")),
        }
    }
}

impl GateWorld for Values<'_> {
    type Definition = DefId;
    fn actor(&self, id: Id) -> Result<ActorSnapshot<DefId>, values::Error> {
        let w = self.world;
        match Thing::of(id) {
            Some(Thing::Actor(a)) => {
                let x = w.actors.get(a as usize).ok_or_else(|| err("no actor"))?;
                let d = w.data.def(x.def);
                let b = w.books.actor(a);
                let mut state = 0u64;
                if b.is_some_and(|b| b.out_of_supply) {
                    state |= STATE_OUT_OF_SUPPLY;
                }
                // Packing / unpacking disables the actor (queue pause reason 2).
                if b.is_some_and(|b| b.deploy.is_some()) {
                    state |= STATE_DISABLED;
                }
                if !x.alive {
                    state |= STATE_DEAD;
                }
                Ok(ActorSnapshot {
                    definition: x.def,
                    // Thing+8 bit 0: destructed.
                    flags: u32::from(!x.alive || x.removed),
                    state_bits: state,
                    element_organization: x.org.filter(|&o| (o as usize) < w.orgs.len()).map(|o| Thing::Org(o).id()),
                    organization: None,
                    economy: has(d, ECONOMY_COMPONENT).then_some(id.raw()),
                    constructor18: has(d, SUPPLY_COMPONENT).then(|| f(b.map_or(0.0, |b| b.supply_stock))),
                    transport: has(d, TRANSPORT_COMPONENT).then_some(id.raw()),
                })
            }
            Some(Thing::Org(o)) => {
                let org = w.orgs.get(o as usize).ok_or_else(|| err("no organization"))?;
                let d = w.data.def(org.def);
                let mut state = 0u64;
                if w.books.org(o).is_some_and(|b| b.out_of_supply) {
                    state |= STATE_OUT_OF_SUPPLY;
                }
                Ok(ActorSnapshot {
                    definition: org.def,
                    flags: u32::from(!org.alive),
                    state_bits: state,
                    element_organization: None,
                    organization: Some(id.raw()),
                    economy: has(d, ECONOMY_COMPONENT).then_some(id.raw()),
                    constructor18: None,
                    transport: None,
                })
            }
            None => Err(err("null actor")),
        }
    }
    fn definition(&self, id: &DefId) -> Result<Definition<DefId>, values::Error> {
        let d = self.world.data.def(*id);
        let n = self.world.data.resources.len();
        let v = |x: &[f32]| -> Vec<F32> { (0..n).map(|i| f(x.get(i).copied().unwrap_or(0.0))).collect() };
        Ok(Definition {
            flags174: d.components,
            class178: d.recruitable,
            word17c: u32::from(!d.recruited_at.is_empty()),
            layout_kind18: has(d, ORGANIZATION_COMPONENT).then(|| d.leader_slot.map_or(-1, i32::from)),
            supply_required: d.supply_required,
            packed_definition: d.packs_to,
            cost: v(&d.cost_all),
            upkeep: v(&d.upkeep_all),
            production: v(&d.production_all),
        })
    }
}

impl values::World for Values<'_> {
    fn resource_count(&self) -> Result<usize, values::Error> {
        Ok(self.world.data.resources.len())
    }
    /// 4CF110: an actor with a Properties component has a modifier context.
    fn actor_modifier(&self, id: Id) -> Result<Option<u32>, values::Error> {
        let w = self.world;
        let def = match Thing::of(id) {
            Some(Thing::Actor(a)) => w.actors.get(a as usize).map(|x| x.def),
            Some(Thing::Org(o)) => w.orgs.get(o as usize).map(|x| x.def),
            None => None,
        };
        Ok(def.filter(|&d| has(w.data.def(d), PROPERTIES_COMPONENT)).map(|_| id.raw()))
    }
    fn modifier(&self, context: u32) -> Result<Modifier, values::Error> {
        let w = self.world;
        let (def, kingdom, extra): (DefId, Option<KingdomId>, &[String]) = match Thing::of(Id::from_raw(context)) {
            Some(Thing::Actor(a)) => {
                let x = &w.actors[a as usize];
                (x.def, x.kingdom, w.books.actor(a).map_or(&[][..], |b| b.properties.as_slice()))
            }
            Some(Thing::Org(o)) => {
                let x = &w.orgs[o as usize];
                (x.def, Some(x.kingdom), &[][..])
            }
            None => return Err(err("null modifier context")),
        };
        let d = w.data.def(def);
        let n = w.data.resources.len();
        let has_property = |r: &String| d.properties.iter().chain(extra).any(|q| q.eq_ignore_ascii_case(r));
        let own = d
            .properties
            .iter()
            .chain(extra)
            .filter_map(|p| w.data.property(p))
            .filter(|p| p.required_properties.iter().all(|r| has_property(r)))
            .flat_map(|p| p.modifiers.iter());
        let techs: Vec<&crate::data::Modifier> = kingdom.map(|k| crate::production::technology_modifiers(w, k, &d.properties).collect()).unwrap_or_default();
        let mut flags = 0u32;
        let mut table: Vec<f32> = Vec::new();
        for m in own.chain(techs) {
            if m.kind == "FLAG" && m.flag.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("NO_UPKEEP")) {
                flags |= 1 << 9;
            }
            if m.kind == "RESOURCE_PRODUCTION" {
                table.resize(n, 0.0);
                for (t, v) in table.iter_mut().zip(&m.production) {
                    *t += *v;
                }
            }
        }
        Ok(Modifier { flags, factor4c: F32::ZERO, factor60: F32::ZERO, production_adjustments: table.into_iter().map(f).collect() })
    }
    /// TransportComponent +C0 (what a transport carries): aa-play transports
    /// carry nothing it accounts for.
    fn transport_vector(&self, _transport: u32, _kind: VectorKind) -> Result<Vec<F32>, values::Error> {
        Ok(vec![F32::ZERO; self.world.data.resources.len()])
    }
    fn layout_slot_count(&self, organization: u32) -> Result<usize, values::Error> {
        let o = self.org_of(organization)?;
        Ok(self.world.data.def(self.world.orgs[o as usize].def).slots.len())
    }
    fn layout_slot_definition(&self, organization: u32, index: usize) -> Result<Option<DefId>, values::Error> {
        let o = self.org_of(organization)?;
        Ok(self.world.data.def(self.world.orgs[o as usize].def).slots.get(index).map(|s| s.0).filter(|&d| d != crate::data::NO_DEF))
    }
    /// Organization +28: one member pointer per layout slot (null holes).
    fn member_count(&self, organization: u32) -> Result<usize, values::Error> {
        self.layout_slot_count(organization)
    }
    fn member(&self, organization: u32, index: usize) -> Result<Option<Id>, values::Error> {
        let o = self.org_of(organization)?;
        Ok(self.living_member(o, index).map(|m| Thing::Actor(m).id()))
    }
    fn store_member_production(&mut self, _economy: u32, _values: &[F32]) -> Result<(), values::Error> {
        Ok(())
    }
}

/// The 570020 inline upkeep gate: members (Element+10), destructed things
/// and organizations that are not recruitable pay nothing themselves.
fn upkeep_blocked(v: &Values<'_>, id: Id) -> bool {
    let Ok(a) = v.actor(id) else { return true };
    if a.element_organization.is_some() || a.flags & 1 != 0 {
        return true;
    }
    if a.organization.is_some() {
        return v.definition(&a.definition).ok().and_then(|d| values::recruitable(&d).ok()).is_none_or(|r| !r);
    }
    false
}

fn floats(v: &[F32]) -> Vec<f32> {
    v.iter().map(|x| f32::from_bits(x.0)).collect()
}

/// 4CCE70: numeric equality (signed zeros equal).
fn equal(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}

// ---------------------------------------------------------------------------
// 570020

fn cache(world: &World, t: Thing) -> crate::books::EconomyCache {
    match t {
        Thing::Actor(a) => world.books.actor(a).map(|b| b.economy.clone()).unwrap_or_default(),
        Thing::Org(o) => world.books.org(o).map(|b| b.economy.clone()).unwrap_or_default(),
    }
}
fn store(world: &mut World, t: Thing, c: crate::books::EconomyCache) {
    match t {
        Thing::Actor(a) => world.books.actor_mut(a).economy = c,
        Thing::Org(o) => world.books.org_mut(o).economy = c,
    }
}

/// Virtual +108: the owner kingdom.
fn owner(world: &World, t: Thing) -> Option<KingdomId> {
    let k = match t {
        Thing::Actor(a) => world.actors.get(a as usize)?.kingdom?,
        Thing::Org(o) => world.orgs.get(o as usize)?.kingdom,
    };
    ((k as usize) < world.kingdoms.len()).then_some(k)
}

/// The thing has an Economy component.
fn economy_thing(world: &World, t: Thing) -> bool {
    let def = match t {
        Thing::Actor(a) => world.actors[a as usize].def,
        Thing::Org(o) => world.orgs[o as usize].def,
    };
    has(world.data.def(def), ECONOMY_COMPONENT)
}

/// Fingerprint of what the thing's values depend on (aa-play's stand-in for
/// the native refresh triggers).
fn key(world: &World, t: Thing) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    let tech = |k: Option<KingdomId>| {
        k.and_then(|k| world.books.kingdom(k)).map_or(0, |b| b.technologies.len() as u64 | (u64::from(b.handicap.is_some()) << 32))
    };
    match t {
        Thing::Actor(a) => {
            let x = &world.actors[a as usize];
            let b = world.books.actor(a);
            mix(1);
            mix(u64::from(x.def));
            mix(x.kingdom.map_or(u64::MAX, u64::from));
            mix(u64::from(x.alive) | u64::from(x.removed) << 1);
            mix(x.org.map_or(u64::MAX, u64::from));
            mix(x.slot.map_or(u64::MAX, u64::from));
            mix(b.map_or(0, |b| u64::from(b.out_of_supply) | u64::from(b.supply_stock > 0.0) << 1 | u64::from(b.deploy.is_some()) << 2));
            mix(b.map_or(0, |b| b.properties.len() as u64));
            mix(tech(x.kingdom));
        }
        Thing::Org(o) => {
            let x = &world.orgs[o as usize];
            mix(2);
            mix(u64::from(x.def));
            mix(u64::from(x.kingdom));
            mix(u64::from(x.alive));
            mix(world.books.org(o).map_or(0, |b| u64::from(b.out_of_supply)));
            mix(tech(Some(x.kingdom)));
        }
    }
    h
}

fn remove_upkeep(world: &mut World, c: &mut crate::books::EconomyCache, k: Option<KingdomId>) {
    c.pays = false;
    if let Some(k) = k {
        adjust_consumption(world, k, &c.upkeep, false);
        add_upkeep_books(world, k, &c.upkeep, false);
        c.upkeep.iter_mut().for_each(|v| *v = 0.0);
    }
}
fn remove_provided(world: &mut World, c: &mut crate::books::EconomyCache, k: Option<KingdomId>) {
    c.provides = false;
    if let Some(k) = k {
        adjust_providers(world, k, &c.provided, false);
        c.provided.iter_mut().for_each(|v| *v = 0.0);
    }
}

/// 0x570020 for one thing.
fn refresh_thing(world: &mut World, t: Thing) {
    let mut c = cache(world, t);
    let owner = owner(world, t);
    // An ownership change: the vectors leave the kingdom they were added to.
    if c.kingdom.is_some() && c.kingdom != owner {
        let old = c.kingdom;
        if c.pays {
            remove_upkeep(world, &mut c, old);
        }
        if c.provides {
            remove_provided(world, &mut c, old);
        }
    }
    c.key = key(world, t);
    c.parent = match t {
        Thing::Actor(a) => world.actors[a as usize].org,
        Thing::Org(_) => None,
    };
    let Some(k) = owner else {
        store(world, t, c);
        return;
    };
    c.kingdom = Some(k);
    let id = t.id();
    let (blocked, upkeep, provides, provided) = {
        let mut v = Values { world };
        let blocked = upkeep_blocked(&v, id);
        let mut upkeep = Vec::new();
        if !blocked && values::cost_or_upkeep(&mut upkeep, Some(id), VectorKind::Upkeep, &v, PRECISION).is_err() {
            upkeep = vec![F32::ZERO; world.data.resources.len()];
        }
        let provides = values::eligible(id, &v).unwrap_or(false);
        let mut provided = Vec::new();
        if provides && values::production(&mut provided, Some(id), true, &mut v, PRECISION).is_err() {
            provided = vec![F32::ZERO; world.data.resources.len()];
        }
        (blocked, floats(&upkeep), provides, floats(&provided))
    };
    if blocked {
        if c.pays {
            remove_upkeep(world, &mut c, Some(k));
        }
    } else if !c.pays || !equal(&c.upkeep, &upkeep) {
        if c.pays {
            remove_upkeep(world, &mut c, Some(k));
        }
        c.upkeep = upkeep;
        adjust_consumption(world, k, &c.upkeep, true);
        add_upkeep_books(world, k, &c.upkeep, true);
        c.pays = true;
    }
    if provides {
        if !c.provides || !equal(&c.provided, &provided) {
            if c.provides {
                remove_provided(world, &mut c, Some(k));
            }
            c.provided = provided;
            adjust_providers(world, k, &c.provided, true);
            c.provides = true;
        }
    } else if c.provides {
        remove_provided(world, &mut c, Some(k));
    }
    store(world, t, c);
}

/// The contributions of every economy thing whose inputs changed since its
/// last refresh, in ascending native id order; a member's refresh is
/// followed by its organization's (4CF0F0), as natively.
pub fn refresh(world: &mut World) {
    let mut dirty: Vec<(u32, Thing)> = Vec::new();
    let slot = |world: &World, t: Thing| -> u32 {
        let id = match t {
            Thing::Actor(a) => world.ids.actor(a),
            Thing::Org(o) => world.ids.org(o),
        };
        id.map_or(u32::MAX, |g| u32::from(g.slot()))
    };
    for a in 0..world.actors.len() as ActorId {
        let t = Thing::Actor(a);
        let c = world.books.actor(a).map(|b| (b.economy.pays || b.economy.provides, b.economy.key));
        let applied = c.is_some_and(|c| c.0);
        let live = world.actors[a as usize].kingdom.is_some() && economy_thing(world, t) && !world.actors[a as usize].removed;
        if (applied || live) && c.is_none_or(|c| c.1 != key(world, t)) {
            dirty.push((slot(world, t), t));
        }
    }
    for o in 0..world.orgs.len() as OrgId {
        let t = Thing::Org(o);
        let c = world.books.org(o).map(|b| (b.economy.pays || b.economy.provides, b.economy.key));
        let applied = c.is_some_and(|c| c.0);
        let live = world.orgs[o as usize].alive && economy_thing(world, t);
        if (applied || live) && c.is_none_or(|c| c.1 != key(world, t)) {
            dirty.push((slot(world, t), t));
        }
    }
    dirty.sort();
    for (_, t) in dirty {
        let old_parent = cache(world, t).parent;
        refresh_thing(world, t);
        if let Thing::Actor(a) = t {
            let parent = world.actors[a as usize].org;
            for o in [parent, old_parent.filter(|p| Some(*p) != parent)].into_iter().flatten() {
                if (o as usize) < world.orgs.len() {
                    refresh_thing(world, Thing::Org(o));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 0x4E9C40

struct Leaves<'a> {
    world: &'a mut World,
    kingdom: KingdomId,
}

impl aa_economy::tick::TickWorld for Leaves<'_> {
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, aa_economy::EconomyError> {
        Ok(self.world.ids.thing(gtid).is_some())
    }
    fn upkeep_shortage(&mut self, percent: i32) -> Result<(), aa_economy::EconomyError> {
        self.world.books.notices.push(crate::books::Notice::UpkeepShortage { kingdom: self.kingdom, percent });
        Ok(())
    }
    fn upkeep_repaired(&mut self) -> Result<(), aa_economy::EconomyError> {
        self.world.books.notices.push(crate::books::Notice::UpkeepRepaired { kingdom: self.kingdom });
        Ok(())
    }
    fn team_victory_check(&mut self) -> Result<(), aa_economy::EconomyError> {
        crate::victory::team_check(self.world, self.kingdom);
        Ok(())
    }
    fn percentage_victory_check(&mut self) -> Result<(), aa_economy::EconomyError> {
        crate::victory::city_check(self.world, self.kingdom);
        Ok(())
    }
    fn statistics_snapshot(&mut self) -> Result<(), aa_economy::EconomyError> {
        // No statistics object is kept (TickParts::statistics is None).
        Ok(())
    }
}

/// Kingdom `k`'s economy Updatable delivered by the periodic queue 0
/// (0x4E5DF0 registration, Tick 0x4E9C40) with `elapsed_bits` = buckets x
/// step (the tick step).
pub fn kingdom_tick(world: &mut World, k: KingdomId, elapsed_bits: u32) {
    let n = world.data.resources.len();
    let seconds = F32Bits(elapsed_bits);
    let world_time = F32Bits(world.clock.begin.0);
    let rate = F32Bits(world.data.rule_ci("ExperienceFactionGainPeriodic").unwrap_or(1.0).to_bits());
    let catalog = catalog(world);
    let limits = limits(world);
    {
        if world.kingdoms.get(k as usize).is_none_or(|x| x.defeated) {
            return;
        }
        let kb = world.books.kingdom_mut(k, n).clone();
        let state = aa_economy::State {
            raw_provided: bits(&kb.raw_provided[..n]),
            effective_provided: bits(&kb.provided[..n]),
            consumed: bits(&kb.consumed[..n]),
            stockpiled: bits(&kb.stockpiled[..n]),
        };
        let Ok(mut economy) = Economy::new(catalog, state, limits, X87Precision::Double53) else { return };
        let mut tick_state = aa_economy::tick::TickState {
            // Kingdom +0x190 (independent) suppresses the overdraw flag.
            suppress_overdrawn: world.kingdoms[k as usize].independent,
            shortage_ratio: F32Bits(kb.shortage_ratio.to_bits()),
            announced_ratio: F32Bits(kb.announced_ratio.to_bits()),
            snapshot_timer: F32Bits::ZERO,
        };
        let mut singles = aa_economy::tick::RecruitSingles { charging: -1, items: Vec::new() };
        // Kingdom +0x230: the Commander of a kingdom with a general.
        let mut commander = world
            .special_ops
            .commanders
            .get(k as usize)
            .filter(|c| c.general.is_some())
            .map(|c| aa_economy::tick::CommanderExperience { balance: F32Bits(c.experience.to_bits()), maximum: F32Bits(c.maximum.to_bits()) });
        let parts = aa_economy::tick::TickParts {
            state: &mut tick_state,
            singles: &mut singles,
            commander: commander.as_mut().map(|experience| aa_economy::tick::CommanderParts { experience, owner_statistics: aa_economy::tick::OwnerStatistics::Absent }),
            statistics: None,
        };
        let input = aa_economy::tick::TickInput { seconds, world_time, commander_experience_rate: rate };
        let mut leaves = Leaves { world, kingdom: k };
        let _ = economy.tick(input, parts, &mut leaves);
        let s = economy.state();
        let kb = world.books.kingdom_mut(k, n);
        for (dst, src) in [
            (&mut kb.raw_provided, &s.raw_provided),
            (&mut kb.provided, &s.effective_provided),
            (&mut kb.consumed, &s.consumed),
            (&mut kb.stockpiled, &s.stockpiled),
        ] {
            for (d, v) in dst.iter_mut().zip(src) {
                *d = f32::from_bits(v.0);
            }
        }
        kb.shortage_ratio = f32::from_bits(tick_state.shortage_ratio.0);
        kb.announced_ratio = f32::from_bits(tick_state.announced_ratio.0);
        if let (Some(e), Some(c)) = (commander, world.special_ops.commanders.get_mut(k as usize)) {
            c.experience = f32::from_bits(e.balance.0);
        }
    }
}

/// 578660 shortfall attrition of organization `o` in its maintenance tick
/// (`crate::supply::org_maintenance`, `seconds` = its elapsed time; the
/// 57C8E0 gate: supply_used, out of supply, kingdom shortage ratio above
/// zero).
pub fn shortfall(world: &mut World, o: OrgId, seconds: f32) {
    let Some(org) = world.orgs.get(o as usize) else { return };
    if !org.alive {
        return;
    }
    let d = world.data.def(org.def);
    if !d.supply_used || !world.books.org(o).is_some_and(|b| b.out_of_supply) {
        return;
    }
    let ratio = world.books.kingdom(org.kingdom).map_or(0.0, |k| k.shortage_ratio);
    if ratio <= 0.0 {
        return;
    }
    let rate = d.shortfall_damage_rate;
    let members = org.members.clone();
    for m in members {
        let a = &world.actors[m as usize];
        if !a.alive || a.removed {
            continue;
        }
        let max = world.data.def(a.def).health;
        let base = seconds * max * rate;
        let u = world.random();
        let amount = (u - 0.25) * base + base;
        let a = &mut world.actors[m as usize];
        a.health = (a.health - amount).max(0.0);
        world.events.push(Event::Hit { target: m, damage: amount });
    }
}

fn index(world: &World, id: &str) -> Option<usize> {
    world.data.resource_index(id)
}

/// Copy the books into `Kingdom` for the HUD: money stock, per-minute net
/// rates (provided - consumed, shortage purchases included) of money, ammo
/// and oil, and the unit limit (provided cap) and units used.
pub fn publish(world: &mut World, k: KingdomId) {
    let Some(kb) = world.books.kingdom(k).cloned() else { return };
    let ids = [index(world, "money"), index(world, "ammo"), index(world, "oil")];
    let limit = index(world, "unit_limit_provided");
    let used = index(world, "unit_limit_consumed");
    let Some(kingdom) = world.kingdoms.get_mut(k as usize) else { return };
    for (slot, i) in ids.into_iter().enumerate() {
        if let Some(i) = i {
            kingdom.stockpile[slot] = kb.stockpiled.get(i).copied().unwrap_or(0.0);
            kingdom.income[slot] = kb.provided.get(i).copied().unwrap_or(0.0) - kb.consumed.get(i).copied().unwrap_or(0.0);
        }
    }
    kingdom.unit_limit = limit.and_then(|i| kb.provided.get(i).copied()).unwrap_or(0.0);
    kingdom.units_used = used.and_then(|i| kb.consumed.get(i).copied()).unwrap_or(0.0);
}

/// Per-minute (provided, consumed) of one resource for the HUD.
pub fn rates(world: &World, k: KingdomId, resource: &str) -> Option<(f32, f32)> {
    let i = index(world, resource)?;
    let kb = world.books.kingdom(k)?;
    Some((kb.provided.get(i).copied()?, kb.consumed.get(i).copied()?))
}
