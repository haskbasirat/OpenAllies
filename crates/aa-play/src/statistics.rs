//! The kingdoms' statistics and their snapshots, the score screen's data. Owner:
//! core. The objects and their arithmetic are aa_economy's
//! (`aa_economy::statistics`, GKingdom +0x32C and +0x340); this module holds them
//! per kingdom, runs the writers aa-play reaches, and keeps the +0x338 list.
//!
//! - World start 0x4F1E10: `start_world`.
//! - Economy tick 0x4E9C40: production totals 0x58ED00, Commander experience and
//!   the 60 s snapshot timer 0x4E37D0 (`crate::economy::kingdom_tick`).
//! - Snapshot 0x58F530 (`Snapshot::take`), read back by 0x58F3C0 (`Snapshot::value`).
//! - Recruit 0x582E80 (`recruited`), death 0x4D57C0 (`died`), research 0x584120
//!   (`aa_production::research` through the host's `statistics`).
//!
//! A labelled rendition: the OWNED scores 47..=55. The original keeps them with
//! 0x58F010 / 0x58EBA0 whenever a thing joins or leaves a kingdom, on its category
//! word 0x4CEEC0. `Snapshot::take` counts them from the world instead: the same
//! number for what a snapshot shows, none of the bookkeeping.
use crate::world::{ActorId, KingdomId, OrgId, World};
use aa_economy::statistics::{KingdomStatistics, Statistics};
use aa_economy::{F32Bits, X87Precision};
use serde::{Deserialize, Serialize};

/// `aa_rules::native::statistics::PREDEFINED_STATISTICS` indices the writers use.
pub const ORGANIZATIONS_RECRUITED: usize = 3;
pub const ORGANIZATIONS_KILLED: usize = 4;
pub const ORGANIZATIONS_LOST: usize = 5;
pub const UNITS_PRODUCED: usize = 8;
pub const UNITS_KILLED: usize = 9;
pub const UNITS_LOST: usize = 10;
pub const TECH_UNITS_PRODUCED: usize = 11;
pub const TECH_UNITS_KILLED: usize = 12;
pub const TECH_UNITS_LOST: usize = 13;
pub const STRUCTURES_KILLED: usize = 21;
pub const STRUCTURES_LOST: usize = 22;
pub const ORGANIZATIONS_OWNED: usize = 47;
pub const STRUCTURES_OWNED: usize = 48;
pub const BUILDINGS_OWNED: usize = 53;
pub const CITIES_OWNED: usize = 55;
/// 0x58D8C0 favourites: by definition.
pub const FAVORITE_UNIT: usize = 57;
pub const FAVORITE_ORGANIZATION: usize = 61;
/// 0x58D950 favourites: by kingdom.
pub const FAVORITE_ENEMY: usize = 60;

const PC: X87Precision = X87Precision::Double53;

/// Native inline cap of the snapshot's rows (`0x58F530`: `min(count, 10)`).
const SNAPSHOT_RESOURCES: usize = 10;

/// The kingdom's statistics (`KingdomBooks::statistics`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KingdomStats {
    pub objects: KingdomStatistics,
    /// +0x334: the snapshot timer.
    pub timer: u32,
    /// +0x338: the snapshots, oldest first.
    pub snapshots: Vec<Snapshot>,
}

/// One `0x58F530` object, what `0x58F3C0` reads.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// +4: production totals, resource order (10 rows inline).
    pub production: Vec<f32>,
    /// +0x2C: the scores 0..=55.
    pub scores: Vec<f32>,
    /// +0x10C: the rate of each resource (`0x58F530`).
    pub rates: Vec<f32>,
    /// +0x134: kingdom +0x218 (statistic 56, HERO_INVENTORY_SIZE).
    pub word_134: f32,
    /// +0x138..: the derived scores `0x58F420` wrote.
    pub derived: Vec<f32>,
}

impl Snapshot {
    /// 0x58F3C0 by StatData index. An index of 57..=61 lands on +0x138.. as well:
    /// the native arithmetic `+0x54 + i * 4` reads the first derived scores.
    pub fn value(&self, table: &crate::data::StatisticsTable, index: usize) -> f32 {
        if index >= table.derived_start {
            return self.derived.get(index - table.derived_start).copied().unwrap_or(0.0);
        }
        if index >= table.rate_start {
            return self.rates.get(index - table.rate_start).copied().unwrap_or(0.0);
        }
        if index >= table.production_start {
            return self.production.get(index - table.production_start).copied().unwrap_or(0.0);
        }
        if index == 0x38 {
            return self.word_134;
        }
        if index > 0x38 {
            return self.derived.get(index - 0x39).copied().unwrap_or(0.0);
        }
        self.scores.get(index).copied().unwrap_or(0.0)
    }
}

fn f(bits: F32Bits) -> f32 {
    f32::from_bits(bits.0)
}

/// Every kingdom's 0x4F1E10 at world start: empty objects, the mission scores zero.
pub fn start_world(world: &mut World) {
    let n = world.data.resources.len();
    for k in 0..world.kingdoms.len() as KingdomId {
        world.books.kingdom_mut(k, n).statistics.objects.start_world(n);
    }
}

/// The last loop of 0x4F1E10: a kingdom without snapshots takes one at once
/// (`0x4E37D0(0, force = 1)`: the timer is +0, then 0x58F530), the graph's first
/// point. The economy of the placed things stands by then in the original (the
/// actors' PostLoad comes first, 0x4FD3F3), so this runs after `economy::publish`.
pub fn start_snapshots(world: &mut World) {
    let n = world.data.resources.len();
    for k in 0..world.kingdoms.len() as KingdomId {
        let books = world.books.kingdom_mut(k, n);
        if !books.statistics.snapshots.is_empty() {
            continue;
        }
        books.statistics.timer = 0;
        let Some(statistics) = books.statistics.objects.active.take() else { continue };
        take_snapshot(world, k, &statistics);
        world.books.kingdom_mut(k, n).statistics.objects.active = Some(statistics);
    }
}

fn active(world: &mut World, k: KingdomId) -> Option<&mut Statistics> {
    world.books.kingdoms.get_mut(k as usize)?.statistics.objects.active.as_mut()
}

/// 0x58E870.
pub fn add(world: &mut World, k: KingdomId, index: usize, amount: f32) {
    if let Some(s) = active(world, k) {
        let _ = s.add_score(index, F32Bits(amount.to_bits()), PC);
    }
}

/// 0x58D8C0 (`def` is the opaque key: aa-play's definition id + 1, never 0).
pub fn add_by_definition(world: &mut World, k: KingdomId, index: usize, def: crate::data::DefId, amount: f32) {
    if let Some(r) = world.books.kingdoms.get_mut(k as usize).and_then(|b| b.statistics.objects.relations.as_mut()) {
        let _ = r.add_by_definition(index, Some(def + 1), F32Bits(amount.to_bits()), PC);
    }
}

/// 0x58D950 (`key` is a kingdom id + 1).
fn add_by_kingdom(world: &mut World, k: KingdomId, index: usize, key: KingdomId, amount: f32) {
    if let Some(r) = world.books.kingdoms.get_mut(k as usize).and_then(|b| b.statistics.objects.relations.as_mut()) {
        let _ = r.add(index, Some(key + 1), F32Bits(amount.to_bits()), PC);
    }
}

/// The leader of a by-definition favourite (`0x58F820` -> `0x58D9D0`).
pub fn favorite_definition(world: &World, k: KingdomId, index: usize) -> Option<crate::data::DefId> {
    let r = world.books.kingdom(k)?.statistics.objects.relations.as_ref()?;
    r.definition_entry(index).ok()?.leader?.checked_sub(1)
}

/// The leader of FAVORITE_ENEMY (`0x58F820` -> `0x58D9D0`).
pub fn favorite_enemy(world: &World, k: KingdomId) -> Option<KingdomId> {
    let r = world.books.kingdom(k)?.statistics.objects.relations.as_ref()?;
    r.entry(FAVORITE_ENEMY).ok()?.leader?.checked_sub(1)
}

/// 0x58EDD0: an element whose definition has a list at +0x2C8 (requirements).
fn tech_unit(world: &World, def: crate::data::DefId) -> bool {
    let d = world.data.def(def);
    !d.requirements.properties.is_empty() || !d.requirements.structures.is_empty()
}

/// 0x582E80 when a recruit has made its thing.
pub fn recruited(world: &mut World, k: KingdomId, def: crate::data::DefId, org: Option<OrgId>) {
    let is_org = world.data.def(def).has_component("OrganizationComponent") && !world.data.def(def).has_component("HQComponent");
    if is_org {
        add(world, k, ORGANIZATIONS_RECRUITED, 1.0);
        // The elements' favourites: every layout entry the set names, then the
        // organization.
        if let Some(o) = org {
            let d = world.data.def(def);
            let mut seen = Vec::new();
            let entries: Vec<(u32, usize)> = d.slot_entries.iter().copied().zip(0..).collect();
            for (entry, slot) in entries {
                if seen.contains(&entry) {
                    continue;
                }
                seen.push(entry);
                if let Some(e) = crate::production::slot_element(world, o, slot).filter(|&e| e != crate::data::NO_DEF) {
                    add_by_definition(world, k, FAVORITE_UNIT, e, 1.0);
                }
            }
        }
        add_by_definition(world, k, FAVORITE_ORGANIZATION, def, 1.0);
    } else if world.data.def(def).has_component("ElementComponent") {
        add(world, k, UNITS_PRODUCED, 1.0);
        add_by_definition(world, k, FAVORITE_UNIT, def, 1.0);
        if tech_unit(world, def) {
            add(world, k, TECH_UNITS_PRODUCED, 1.0);
        }
    }
}

/// 0x4D57C0's scores for a dying actor: `killer` is the kingdom the kill is
/// credited to (hostile, within KillCreditTimeout).
pub fn actor_died(world: &mut World, a: ActorId, killer: Option<KingdomId>) {
    let actor = &world.actors[a as usize];
    let (def, owner) = (actor.def, actor.kingdom);
    let d = world.data.def(def);
    let hq = d.has_component("HQComponent");
    if d.has_component("ElementComponent") && !hq {
        if let Some(k) = owner {
            add(world, k, UNITS_LOST, 1.0);
        }
        if let Some(k) = killer {
            add(world, k, UNITS_KILLED, 1.0);
        }
        // 0x58D950(0x3C): each side's favourite enemy.
        if let (Some(o), Some(k)) = (owner, killer) {
            add_by_kingdom(world, o, FAVORITE_ENEMY, k, 1.0);
            add_by_kingdom(world, k, FAVORITE_ENEMY, o, 1.0);
        }
        if tech_unit(world, def) {
            if let Some(k) = owner {
                add(world, k, TECH_UNITS_LOST, 1.0);
            }
            if let Some(k) = killer {
                add(world, k, TECH_UNITS_KILLED, 1.0);
            }
        }
    }
    let d = world.data.def(def);
    if d.has_component("StructureComponent") {
        if let Some(k) = owner {
            add(world, k, STRUCTURES_LOST, 1.0);
        }
        if let Some(k) = killer {
            add(world, k, STRUCTURES_KILLED, 1.0);
        }
    }
}

/// 0x4D57C0's scores for a dying organization.
pub fn org_died(world: &mut World, o: OrgId, killer: Option<KingdomId>) {
    let org = &world.orgs[o as usize];
    let (def, owner) = (org.def, org.kingdom);
    if world.data.def(def).has_component("HQComponent") {
        return;
    }
    add(world, owner, ORGANIZATIONS_LOST, 1.0);
    if let Some(k) = killer {
        add(world, k, ORGANIZATIONS_KILLED, 1.0);
    }
}

/// 0x58F530 for kingdom `k`, pushed on +0x338 (0x45F240). `statistics` is the +0x32C
/// object, which the economy tick holds at this moment.
pub fn take_snapshot(world: &mut World, k: KingdomId, statistics: &Statistics) {
    let n = world.data.resources.len();
    let kb = world.books.kingdom_mut(k, n).clone();
    let mut scores: Vec<f32> = statistics.scores.iter().map(|s| f(*s)).collect();
    owned_scores(world, k, &mut scores);
    let production: Vec<f32> = statistics.production.iter().map(|s| f(*s)).collect();
    // +0x10C, `min(count, 10)` rows: a LIMITED_CONSUMED row (type 2) its consumed,
    // a provider row (+0x2C) its provided, else provided minus consumed for types
    // 0 and 1.
    let rates: Vec<f32> = world
        .data
        .resources
        .iter()
        .enumerate()
        .take(SNAPSHOT_RESOURCES)
        .map(|(i, r)| {
            let provided = kb.provided.get(i).copied().unwrap_or(0.0);
            let consumed = kb.consumed.get(i).copied().unwrap_or(0.0);
            match r.kind {
                crate::data::ResourceKind::LimitedConsumed => consumed,
                _ if r.limited_provider => provided,
                _ => provided - consumed,
            }
        })
        .collect();
    // +0x134: kingdom +0x218, statistic HERO_INVENTORY_SIZE: aa-play has no hero
    // inventory, and no shipped map of the corpus a hero.
    let mut snapshot = Snapshot { production, scores, rates, word_134: 0.0, derived: Vec::new() };
    // 0x58F420: each score's terms over what the snapshot has so far, in the
    // native list order; a score can read an earlier one.
    let table = world.data.statistics.clone();
    for terms in &table.derived {
        let mut sum = 0.0f32;
        for &(index, weight) in terms {
            sum = (f64::from(snapshot.value(&table, index as usize)) * f64::from(weight) + f64::from(sum)) as f32;
        }
        snapshot.derived.push(sum);
    }
    world.books.kingdom_mut(k, n).statistics.snapshots.push(snapshot);
}

/// The rendition of the OWNED scores: see the module documentation.
fn owned_scores(world: &World, k: KingdomId, scores: &mut [f32]) {
    let mut count = |index: usize, n: usize| {
        if let Some(s) = scores.get_mut(index) {
            *s = n as f32;
        }
    };
    count(ORGANIZATIONS_OWNED, world.orgs.iter().filter(|o| o.alive && o.kingdom == k && !world.data.def(o.def).has_component("HQComponent")).count());
    let owned: Vec<&crate::data::Def> = world.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom == Some(k)).map(|a| world.data.def(a.def)).collect();
    count(STRUCTURES_OWNED, owned.iter().filter(|d| d.has_component("StructureComponent")).count());
    count(BUILDINGS_OWNED, owned.iter().filter(|d| d.has_component("ConstructionComponent")).count());
    count(CITIES_OWNED, owned.iter().filter(|d| d.has_component("CityComponent")).count());
}
