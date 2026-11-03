//! Supply: the supply components of HQs, depots, HQ trucks and cities, the
//! in-supply state (actor state 19) of the things that need supply,
//! regeneration and member replacement, and the city capture. Owner:
//! supplyfog.
//!
//! The supply rules run on the exact port of AA.exe (aa-economy `supply`,
//! proven against the native code, x87 arithmetic at PC53), bound to the
//! battle world by [`Binding`] and to the exact zones of [`crate::zones`]:
//! - SupplyComponent Tick 0x58AE10 ([`supplier_tick`], queue 1: one second in
//!   its bucket; InitExisting 0x58B0E0 runs it with 0 s when the supplier
//!   first appears): a relay (division HQ, depot, HQ truck) collects the
//!   supply providers of every kingdom allied with its owner whose zone
//!   covers its tile (its shape's tiles for a large structure, 0x554CC0 +
//!   0x54F8F0, newest found first) and links to the first one that is in
//!   supply and reaches a primary source (0x58ADB0); then the state 0x4D7CE0;
//!   a relay out of supply drains its stockpile by the elapsed time (its zone
//!   stays while stock is left, 0x4D0C90), a connected one recharges at
//!   SupplyStockpileRechargeRate up to stockpile_max, starting empty
//!   (constructor 0x58AA50); then the zone: 0x58ACE0 provides (a major
//!   kingdom's, not disabled (packing/unpacking), not under construction, not
//!   dead, a relay's stock, radius >= 0, a city not under siege) against the
//!   active zone 0: release 0x514CE0, (re)place 0x515F40 when it starts
//!   providing or its radius changed (0x58AA20: SUPPLY_RANGE modifiers, such
//!   as tech_improved_logistics +5, through 0x45D550 / 0x501BD0);
//! - in supply 0x4D0A80: a primary source always; an organization member
//!   with its supply_used organization's state; nothing of an independent
//!   kingdom; an organization only when nobody engages it (+0x118), it rides
//!   no transport (+0xD4) and it is in_supply_detached, attached to an HQ
//!   (+0xD0), an HQ itself (+0xD8) or SUPPLIED; SUPPLIED without an upkeep
//!   shortage is in supply; a relay while its link resolves; anything else
//!   when a supply zone of any kingdom allied with its owner, not its own,
//!   covers its tile (0x54F8B0, any shape tile for a large structure);
//! - the state 0x4D7CE0 is taken only by the things that need supply: the
//!   suppliers (their Tick), an EconomyComponent with supply_required (its
//!   queue-1 Tick 0x56F2B0, [`economy_tick`]), a supply_used organization
//!   (its TickMaintenance 0x57C8E0, [`org_maintenance`]) and a lone Body with
//!   unit_shortfall_damage_rate (0x562E10, [`body_supply_tick`]); members and
//!   the rest never leave supply;
//! - regeneration 0x57B5A0 behind the 0x579110 gate (supply_used, in supply,
//!   nobody engaging it, a resupply mind state): one slot per tick, a damaged
//!   primary first, then the first damaged member, then an absent primary,
//!   then the first empty slot with a Body; members heal resupply_rate *
//!   max_health * (1 - shortage) * dt, an empty slot builds up the same
//!   amount until it reaches the slot definition's health and the member is
//!   created (0x57AD30).
//!
//! [`flood_tiles`] (the earlier f32 flood) stays for the comparison tests.
use crate::books::SupplyZone;
use crate::world::{ActorId, KingdomId, OrgId, World};
use crate::zones::{Engine, Key};
use aa_economy::supply::{self as native, KingdomSupply, SupplyActor, SupplyComponent, SupplyDefinition, SupplyWorld};
use aa_economy::{EconomyError, F32Bits, X87Precision};
use glam::Vec2;
use std::collections::BTreeMap;

/// The EconomyComponent slot (aa-rules COMPONENTS bit 9).
pub const ECONOMY_SLOT: u8 = 9;
const BODY_SLOT: u32 = 0;
const PROPERTIES_SLOT: u32 = 6;
const ELEMENT_SLOT: u32 = 8;
const CITY_SLOT: u32 = 29;
const HQ_SLOT: u32 = 30;
/// The simulation thread's x87 precision.
const PRECISION: X87Precision = X87Precision::Double53;
/// Actor state bits (+0x100/+0x104): 19 out of supply, 20 under siege, 21
/// under construction, 39 disabled.
const STATE_OUT_OF_SUPPLY: u32 = 0x13;
const STATE_UNDER_SIEGE: u32 = 0x14;
const STATE_CONSTRUCTION: u32 = 0x15;
const STATE_DISABLED: u32 = 0x27;

fn live(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed)
}

/// The tile under a ground point (the `0x552640` clamp into the map, then
/// the tile index).
fn tile_index(map: &crate::map::GameMap, p: Vec2) -> Option<u32> {
    if map.width == 0 || map.height == 0 || map.cell_size <= 0.0 {
        return None;
    }
    let x = ((p.x / map.cell_size).floor() as i64).clamp(0, map.width as i64 - 1);
    let y = ((p.y / map.cell_size).floor() as i64).clamp(0, map.height as i64 - 1);
    Some((y * map.width as i64 + x) as u32)
}

/// ZoneSolver neighbour tables (`0xB322E0` deltas; `0xAB7804` next,
/// `0xAB7828` previous, `0xAB784C` / `0xAB7870` two steps on, `0xAB7894`
/// opposite): direction 0 is none, odd ones are diagonal.
const DELTA: [(i32, i32); 9] = [(0, 0), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1)];
const NEXT: [usize; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const PREVIOUS: [usize; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7];
const NEXT2: [usize; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2];
const PREVIOUS2: [usize; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6];
const OPPOSITE: [usize; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];

/// A ZoneSolver open-list entry, smallest cost first.
#[derive(PartialEq)]
struct Open {
    cost: f32,
    order: u32,
    x: i32,
    y: i32,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.total_cmp(&self.cost).then(other.order.cmp(&self.order))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The tiles a supply zone covers (zone type 0 is not fast-built): the
/// ZoneSolver Dijkstra flood `0x5402E0` (functor `0xA36910`, visit
/// `0x53F9F0`, aa-zones `flood::add`) from the zone's tile. A step costs one
/// tile, sqrt 2 diagonally (`0x53C5C0`); from a tile only the five
/// directions ahead of the one it was first reached from are tried
/// (`0x554ED0`), never an impassable tile (flags20 bit 0) nor a diagonal
/// past an impassable corner; a tile joins while its cost is within the
/// radius and the step does not cross between land and water (flags20 bit
/// 8), and spreads on while half a tile plus its cost is below the radius.
/// Supply zones pass every blocker (only ZOC zones stop at them). Sorted.
pub fn flood_tiles(map: &crate::map::GameMap, sx: i32, sy: i32, radius: f32) -> Vec<u32> {
    use crate::map::BLOCKED;
    let (w, h) = (map.width as i32, map.height as i32);
    let tile = map.cell_size;
    let flags = |x: i32, y: i32| if x < 0 || y < 0 || x >= w || y >= h { BLOCKED } else { map.flags[(y * w + x) as usize] };
    let index = |x: i32, y: i32| (y * w + x) as u32;
    let mut out = Vec::new();
    if w <= 0 || h <= 0 || tile <= 0.0 || map.flags.len() < (w * h) as usize || !(radius >= 0.0) || flags(sx, sy) & BLOCKED != 0 {
        return out;
    }
    // per reached tile: the direction it was first reached from, its cost
    let mut seen: std::collections::HashMap<u32, (usize, f32)> = std::collections::HashMap::new();
    seen.insert(index(sx, sy), (0, 0.0));
    out.push(index(sx, sy));
    let mut open = std::collections::BinaryHeap::new();
    let mut order = 0u32;
    open.push(Open { cost: 0.0, order, x: sx, y: sy });
    let half = tile * 0.5;
    let blocked = |x: i32, y: i32| flags(x, y) & BLOCKED != 0;
    while let Some(Open { cost, x, y, .. }) = open.pop() {
        let came = seen.get(&index(x, y)).map_or(0, |s| s.0);
        let valid = |dir: usize| {
            let (dx, dy) = DELTA[dir];
            if OPPOSITE[came] == dir || blocked(x + dx, y + dy) {
                return false;
            }
            if dir & 1 == 0 {
                return true;
            }
            let (px, py) = DELTA[PREVIOUS[dir]];
            let (nx, ny) = DELTA[NEXT[dir]];
            !blocked(x + px, y + py) && !blocked(x + nx, y + ny)
        };
        let (first, end) = if came == 0 { (1, 8) } else { (PREVIOUS2[came], NEXT2[came]) };
        let mut dirs = Vec::with_capacity(8);
        let mut dir = first;
        loop {
            if valid(dir) {
                dirs.push(dir);
            }
            if dir == end {
                break;
            }
            dir = NEXT[dir];
            if dir == first {
                break;
            }
        }
        for dir in dirs {
            let (dx, dy) = DELTA[dir];
            let (nx, ny) = (x + dx, y + dy);
            let nt = index(nx, ny);
            let d = if dir & 1 != 0 { std::f32::consts::SQRT_2 } else { 1.0 } * tile + cost;
            let mut within_before = false;
            match seen.get(&nt) {
                None => {
                    seen.insert(nt, (dir, d));
                }
                Some(&(first_dir, old)) => {
                    if radius < d || !(d + 1.0e-4 < old) {
                        continue;
                    }
                    within_before = old <= radius;
                    seen.insert(nt, (first_dir, d));
                }
            }
            let same_ground = ((flags(nx, ny) >> 8) ^ (flags(x, y) >> 8)) & 1 == 0;
            if !same_ground || radius < d {
                if let Some(s) = seen.get_mut(&nt) {
                    s.1 = f32::MAX;
                }
                continue;
            }
            if !within_before {
                out.push(nt);
            }
            if half + d < radius {
                order += 1;
                open.push(Open { cost: d, order, x: nx, y: ny });
            }
        }
    }
    out.sort_unstable();
    out
}

/// `books.zone_tiles` from the engine's supply zone tiles, in `books.zones`
/// order.
pub fn publish_zone_tiles(world: &mut World) {
    let Some(e) = world.fog.engine.as_ref() else { return };
    let tiles = world.books.zones.iter().map(|z| e.supply_tiles.get(&z.provider).cloned().unwrap_or_default()).collect();
    world.books.zone_tiles = tiles;
}

/// The native key of a thing (the aa-zones key: actor `i` -> `i + 1`,
/// organization `o` -> `ORG | (o + 1)`); it also stands for the GTID the
/// relay link (Supply +0x1C) holds.
fn raw(key: Key) -> u32 {
    match key {
        Key::Actor(a) => crate::zones::actor_key(a),
        Key::Org(o) => crate::zones::org_key(o),
    }
}

fn key_of(raw: u32) -> Option<Key> {
    if raw == 0 {
        None
    } else if raw & crate::zones::ORG != 0 {
        Some(Key::Org((raw & !crate::zones::ORG) - 1))
    } else {
        Some(Key::Actor(raw - 1))
    }
}

fn tile_xy(map: &crate::map::GameMap, p: Vec2) -> (i32, i32) {
    match tile_index(map, p) {
        Some(t) => ((t % map.width) as i32, (t / map.width) as i32),
        None => (0, 0),
    }
}

/// The supply component radius +0x10: the supply range ActorData +0x420
/// (0x45D550 without a context) through the modifier context (0x58AA20 ->
/// 0x45D550 -> 0x501BD0: the SUPPLY_RANGE ADD modifiers, at least the
/// smaller of the range and ModifierAttributeClampAdd, at least 0).
fn supply_radius(world: &World, a: ActorId) -> f32 {
    let d = world.data.def(world.actors[a as usize].def);
    match crate::zones::modifier_add(world, Key::Actor(a), "SUPPLY_RANGE") {
        Some(add) => crate::zones::clamp_add(d.supply_range, add, world.data.rule_ci("ModifierAttributeClampAdd").unwrap_or(1.0)),
        None => d.supply_range,
    }
}

/// ActorData fields of the supply code.
fn definition(d: &crate::data::Def) -> SupplyDefinition {
    SupplyDefinition {
        relay: d.supply_relay,
        stockpile_max: F32Bits::from_f32(d.supply_stockpile_max),
        // +0x348: the EconomyComponent's supply_required
        economy_ticks: d.supply_required,
        byte_31d: d.in_supply_detached,
        byte_2f4: d.supply_used,
    }
}

/// The native supply fields of one thing from the battle world and the zone
/// engine; None when it is not in the GTID table any more.
fn snapshot(world: &World, e: Option<&Engine>, key: Key) -> Option<SupplyActor> {
    let has = |d: &crate::data::Def, bit: u32| d.components & (1 << bit) != 0;
    match key {
        Key::Actor(a) => {
            let x = world.actors.get(a as usize).filter(|x| !x.removed)?;
            let d = world.data.def(x.def);
            let b = world.books.actor(a);
            let mut states = 0u64;
            let mut set = |s: u32, on: bool| {
                if on {
                    states |= 1u64 << s;
                }
            };
            set(STATE_OUT_OF_SUPPLY, b.is_some_and(|b| b.out_of_supply));
            // 0x4D6660
            set(STATE_UNDER_SIEGE, under_siege(world, a));
            // +0x100 bit 21: placed by BuildActor (an unpacking building is
            // bit 26, the film's st 0x4001000)
            set(STATE_CONSTRUCTION, b.is_some_and(|b| b.constructing));
            // +0x104 bit 7: packing and unpacking disable the actor
            set(STATE_DISABLED, b.is_some_and(|b| b.deploy.is_some()));
            let slot = e.and_then(|e| e.parts.actors.get(a as usize));
            let tile = e.and_then(|e| e.placement(key)).map(|(t, _)| t).unwrap_or_else(|| tile_xy(&world.map, x.pos.truncate()));
            Some(SupplyActor {
                gtid: raw(key),
                flags8: u32::from(!x.alive),
                states,
                definition: definition(d),
                // +0x8C: not assembled while unpacking (0x4D21A0)
                supply: (d.has_supply && !crate::zones::unpacking(world, a)).then(|| SupplyComponent {
                    radius: F32Bits(slot.and_then(|s| s.supply_radius).unwrap_or(d.supply_range.to_bits())),
                    dirty: slot.is_some_and(|s| s.supply_dirty),
                    stockpile: F32Bits::from_f32(b.map_or(0.0, |b| b.supply_stock)),
                    relay: b.and_then(|b| b.supply_relay).map_or(0, |r| raw(Key::Actor(r))),
                }),
                // +0x80 and its organization +0x10
                element: has(d, ELEMENT_SLOT).then(|| x.org.filter(|&o| world.orgs.get(o as usize).is_some_and(|g| g.alive)).map(|o| raw(Key::Org(o)))),
                organization_component: None,
                // Properties +0x1C bit 5: the SUPPLIED flag (Resupply)
                properties_flag20: has(d, PROPERTIES_SLOT).then(|| crate::special_ops::has_flag(world, a, "SUPPLIED")),
                // City +0x20: held under siege
                city_flag20: (has(d, CITY_SLOT) || d.city).then(|| b.is_some_and(|b| b.city_siege)),
                // +0x118 is read for an organization only (0x4D0A80)
                word_118: false,
                word_128: world.combat.actor(a).is_some_and(|c| !c.bombarding.is_empty()),
                word_d8: has(d, HQ_SLOT),
                gtid_d0: 0,
                kingdom: x.kingdom,
                position: [F32Bits(x.pos.x.to_bits()), F32Bits(x.pos.y.to_bits())],
                tile: [tile.0.max(0) as u32, tile.1.max(0) as u32],
                zone_word8: e.and_then(|e| e.supply_zone(a)).map(|z| F32Bits(z.radius_bits)),
            })
        }
        Key::Org(o) => {
            let g = world.orgs.get(o as usize).filter(|g| g.alive)?;
            let d = world.data.def(g.def);
            let mut states = 0u64;
            if world.books.org(o).is_some_and(|b| b.out_of_supply) {
                states |= 1u64 << STATE_OUT_OF_SUPPLY;
            }
            let tile = e.and_then(|e| e.placement(key)).map(|(t, _)| t).unwrap_or_else(|| tile_xy(&world.map, g.anchor));
            // +0x118: the organizations whose engage list holds it (0x4D3990)
            let engaged = world.combat.orgs.get(o as usize).is_some_and(|c| c.is_engaged());
            Some(SupplyActor {
                gtid: raw(key),
                flags8: 0,
                states,
                definition: definition(d),
                supply: None,
                element: None,
                // +0x7C and its +0xD4 transport: aa-play carries no organization
                // on a transport (economy.rs: transport_vector)
                organization_component: Some(false),
                properties_flag20: (has(d, PROPERTIES_SLOT) || d.components == 0).then(|| crate::special_ops::org_has_flag(world, o, "SUPPLIED")),
                city_flag20: None,
                word_118: engaged,
                word_128: world.combat.org(o).is_some_and(|c| !c.bombarding.is_empty()),
                // +0xD8: an HQ (its HQComponent)
                word_d8: has(d, HQ_SLOT),
                // +0xD0: the HQ it is attached to (0x44DEE0 resolves it)
                gtid_d0: g.hq.map_or(0, |h| raw(Key::Actor(h))),
                kingdom: Some(g.kingdom),
                position: [F32Bits(g.anchor.x.to_bits()), F32Bits(g.anchor.y.to_bits())],
                tile: [tile.0.max(0) as u32, tile.1.max(0) as u32],
                zone_word8: None,
            })
        }
    }
}

/// `0x4EE700`: Thing flags `0x20000400` (blocking, static) and `(int)tile <=
/// virtual +4 * 4` (0x9FD59C; the shape's +4 is the circle radius or the
/// rectangle's half diagonal, 0x46E0E0): its shape's tiles count.
fn large(world: &World, engine: Option<&Engine>, raw: u32) -> bool {
    let Some(key @ Key::Actor(a)) = key_of(raw) else { return false };
    let Some(x) = world.actors.get(a as usize) else { return false };
    let d = match engine {
        Some(e) => e.def_of(world, key),
        None => crate::zones::def_detection(world.data.def(x.def)),
    };
    crate::zones::multi_tile(&d, world.map.cell_size)
}

/// The supply port's world (aa-economy `SupplyWorld`) over the battle: the
/// native fields of the ticking thing and of every supplier, the zone
/// engine for the tile queries and the supply zone, and the SVars.
struct Binding<'a> {
    w: &'a World,
    e: Option<&'a mut Engine>,
    store: BTreeMap<u32, SupplyActor>,
    /// `0x50B1C0(1, 0)` asked: the Economy component leaves queue 1.
    stop: bool,
}

impl<'a> Binding<'a> {
    /// Only a relay's Tick reads other suppliers (the search and 0x58ADB0);
    /// everything else asks about the ticking thing and its organization.
    fn new(w: &'a World, e: Option<&'a mut Engine>, ticking: Key) -> Self {
        let mut store = BTreeMap::new();
        {
            let er = e.as_deref();
            let mut keys = vec![ticking];
            if let Key::Actor(a) = ticking
                && let Some(x) = w.actors.get(a as usize)
            {
                if let Some(o) = x.org {
                    keys.push(Key::Org(o));
                }
                let d = w.data.def(x.def);
                if d.has_supply && d.supply_relay {
                    keys.extend(w.actors.iter().filter(|x| !x.removed && w.data.def(x.def).has_supply).map(|x| Key::Actor(x.id)));
                }
            }
            for k in keys {
                if !store.contains_key(&raw(k))
                    && let Some(s) = snapshot(w, er, k)
                {
                    store.insert(raw(k), s);
                }
            }
        }
        Binding { w, e, store, stop: false }
    }
}

fn missing(actor: u32) -> EconomyError {
    EconomyError::World(format!("thing {actor:#x} is not in the supply snapshot"))
}

impl SupplyWorld for Binding<'_> {
    fn actor(&self, actor: u32) -> Result<&SupplyActor, EconomyError> {
        self.store.get(&actor).ok_or_else(|| missing(actor))
    }
    fn actor_mut(&mut self, actor: u32) -> Result<&mut SupplyActor, EconomyError> {
        self.store.get_mut(&actor).ok_or_else(|| missing(actor))
    }
    fn resolve(&self, gtid: u32) -> Option<u32> {
        // 0xB364C0: still in the GTID table (a dead thing until it is removed)
        match key_of(gtid)? {
            Key::Actor(a) => self.w.actors.get(a as usize).filter(|x| !x.removed).map(|_| gtid),
            Key::Org(o) => self.w.orgs.get(o as usize).filter(|g| g.alive).map(|_| gtid),
        }
    }
    fn kingdom(&self, kingdom: u32) -> Result<KingdomSupply, EconomyError> {
        let k = self.w.kingdoms.get(kingdom as usize).ok_or_else(|| EconomyError::World(format!("kingdom {kingdom}")))?;
        Ok(KingdomSupply {
            // +0x190: 1 for a non-major kingdom
            word_190: u32::from(k.independent || kingdom >= crate::zones::KINGDOM_BITS),
            shortage_ratio: F32Bits::from_f32(self.w.books.kingdom(kingdom).map_or(0.0, |b| b.shortage_ratio)),
        })
    }
    fn owner(&mut self, actor: u32) -> Result<Option<u32>, EconomyError> {
        Ok(self.actor(actor)?.kingdom)
    }
    fn world_suspended(&mut self) -> Result<bool, EconomyError> {
        // 0x4F0460: GWorld+0x28 +0xA8, the map mode (0x4F3640, 0x4EF6A0): a
        // battle is mode 0, the others are the map editor's
        Ok(false)
    }
    fn provide_under_siege(&self) -> bool {
        // SVar cache +0xFC (SVars_AA: true)
        self.w.data.rule_ci("SupplyProvideUnderSiege").is_some_and(|v| v != 0.0)
    }
    fn recharge_rate(&self) -> F32Bits {
        // SVar cache +0x100
        F32Bits::from_f32(self.w.data.rule_ci("SupplyStockpileRechargeRate").unwrap_or(1.0))
    }
    fn map_width(&self) -> u32 {
        self.w.map.width
    }
    fn large_actor(&mut self, actor: u32) -> Result<bool, EconomyError> {
        Ok(large(self.w, self.e.as_deref(), actor))
    }
    fn tile_supplied(&mut self, tile: u32, kingdom: Option<u32>, actor: u32) -> Result<bool, EconomyError> {
        let w = self.w;
        let (Some(e), Some(k)) = (self.e.as_deref_mut(), key_of(actor)) else { return Ok(false) };
        Ok(e.friendly_supply(w, tile as usize, kingdom, k))
    }
    fn covered_tiles(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError> {
        let (Some(e), Some(k)) = (self.e.as_deref(), key_of(actor)) else { return Ok(Vec::new()) };
        Ok(e.placement(k).map(|(_, t)| t.iter().map(|&t| t as u32).collect()).unwrap_or_default())
    }
    fn relay_candidates(&mut self, actor: u32, kingdom: Option<u32>) -> Result<Vec<u32>, EconomyError> {
        let w = self.w;
        let large = large(w, self.e.as_deref(), actor);
        let (Some(e), Some(Key::Actor(a))) = (self.e.as_deref_mut(), key_of(actor)) else { return Ok(Vec::new()) };
        let list = e.supply_candidates(w, a, kingdom, large);
        Ok(list.into_iter().map(|c| raw(Key::Actor(c))).filter(|c| self.store.contains_key(c)).collect())
    }
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<(), EconomyError> {
        let a = self.actor_mut(actor)?;
        if enabled {
            a.states |= 1u64 << state;
        } else {
            a.states &= !(1u64 << state);
        }
        Ok(())
    }
    fn refresh_actor(&mut self, _actor: u32) -> Result<(), EconomyError> {
        // 0x4D1350 recomputes the command mask +0xF0 / +0xF4 from the
        // components: the order panel's, which crate::player_orders asks of
        // the thing when it is shown
        Ok(())
    }
    fn refresh_economy(&mut self, _actor: u32) -> Result<(), EconomyError> {
        // 0x4CF0F0 -> 0x570020: crate::economy::refresh applies a changed
        // contribution at its next pass
        Ok(())
    }
    fn stop_economy_ticks(&mut self, _actor: u32) -> Result<(), EconomyError> {
        self.stop = true;
        Ok(())
    }
    fn clear_zone(&mut self, actor: u32) -> Result<(), EconomyError> {
        let w = self.w;
        if let (Some(e), Some(Key::Actor(a))) = (self.e.as_deref_mut(), key_of(actor)) {
            e.supply_release(w, a);
            let z = e.supply_zone(a).map(|z| F32Bits(z.radius_bits));
            if let Some(s) = self.store.get_mut(&actor) {
                s.zone_word8 = z;
            }
        }
        Ok(())
    }
    fn set_zone(&mut self, actor: u32, position: [F32Bits; 2], radius: F32Bits) -> Result<(), EconomyError> {
        let w = self.w;
        if let (Some(e), Some(Key::Actor(a))) = (self.e.as_deref_mut(), key_of(actor)) {
            e.supply_place(w, a, [position[0].0, position[1].0], radius.0);
            let z = e.supply_zone(a).map(|z| F32Bits(z.radius_bits));
            if let Some(s) = self.store.get_mut(&actor) {
                s.zone_word8 = z;
            }
        }
        Ok(())
    }
}

/// A refusal of the supply port: the battle keeps running on the fields
/// written so far; counted with the zone engine's refusals (`stats.refusals`).
fn refused(engine: Option<&mut Engine>, err: &EconomyError) {
    let text = format!("supply: {err}");
    if let Some(e) = engine {
        *e.parts.stats.refusals.entry(text.clone()).or_insert(0) += 1;
    }
    #[cfg(debug_assertions)]
    eprintln!("{text}");
    #[cfg(not(debug_assertions))]
    let _ = text;
}

/// The ticking thing's native fields back into the battle books: its state
/// and a supplier's stockpile, relay link, radius and dirty byte.
fn write_back(world: &mut World, e: Option<&mut Engine>, key: Key, s: &SupplyActor) {
    let oos = s.states & (1u64 << STATE_OUT_OF_SUPPLY) != 0;
    match key {
        Key::Actor(a) => {
            let b = world.books.actor_mut(a);
            b.out_of_supply = oos;
            if let Some(c) = &s.supply {
                b.supply_stock = c.stockpile.as_f32();
                b.supply_relay = match key_of(c.relay) {
                    Some(Key::Actor(r)) => Some(r),
                    _ => None,
                };
                b.zone_active = native::zone_active(s);
                if let Some(slot) = e.and_then(|e| e.supply_component(a)) {
                    slot.supply_radius = Some(c.radius.0);
                    slot.supply_dirty = c.dirty;
                }
            }
        }
        Key::Org(o) => world.books.org_mut(o).out_of_supply = oos,
    }
}

/// Run one native supply entry point for `key` over the battle world.
fn run(world: &mut World, key: Key, f: impl FnOnce(&mut Binding<'_>, u32) -> Result<(), EconomyError>) -> bool {
    let mut engine = crate::fog::take_engine(world);
    if let Some(e) = engine.as_mut() {
        e.observe_update(world);
        e.observe_key(world, key);
    }
    let (store, stop, failed) = {
        let mut b = Binding::new(world, engine.as_mut(), key);
        let failed = f(&mut b, raw(key)).err();
        (b.store, b.stop, failed)
    };
    if let Some(err) = failed {
        refused(engine.as_mut(), &err);
    }
    if let Some(s) = store.get(&raw(key)) {
        write_back(world, engine.as_mut(), key, s);
    }
    if let Some(e) = engine {
        crate::fog::put_engine(world, e);
    }
    stop
}

/// SupplyComponent Tick 0x58AE10 (aa-economy `supply_tick`) for supplier
/// `a`: its queue-1 Tick (`elapsed` = 1 s, in the bucket its InitCreate
/// registration or the saved list gave it, `crate::scheduler`) and the
/// immediate call of its InitExisting 0x58B0E0 (`elapsed` = 0). First the
/// radius +0x10 as the modifier context gives it (0x58AA20 marks the zone
/// dirty when SUPPLY_RANGE modifiers change it; the constructor 0x58AA50
/// reads it without marking).
pub fn supplier_tick(world: &mut World, a: ActorId, elapsed: f32) {
    let Some(x) = world.actors.get(a as usize) else { return };
    if x.removed || !world.data.def(x.def).has_supply {
        return;
    }
    let first = !world.books.actor(a).is_some_and(|b| b.supply_started);
    let r = supply_radius(world, a).to_bits();
    if let Some(mut e) = crate::fog::take_engine(world) {
        e.observe_update(world);
        e.observe_key(world, Key::Actor(a));
        // 0x58B0E0: CreateZone(0) when the zone vector has no zone 0 (inactive,
        // radius -1, until it provides), then 0x58AE10(0)
        if first {
            e.supply_create(world, a);
        }
        if let Some(s) = e.supply_component(a) {
            match s.supply_radius {
                None => s.supply_radius = Some(r),
                Some(old) if old != r => {
                    s.supply_radius = Some(r);
                    s.supply_dirty = true;
                }
                _ => {}
            }
        }
        crate::fog::put_engine(world, e);
    }
    let seconds = F32Bits(elapsed.to_bits());
    run(world, Key::Actor(a), |b, k| native::supply_tick(b, k, seconds, PRECISION));
    world.books.actor_mut(a).supply_started = true;
}

/// EconomyComponent::Tick 0x56F2B0 (its queue-1 Updatable, registered at
/// InitCreate 0x56F250 for a definition requiring supply): the state
/// 0x4D7CE0; without supply_required (+0x348) the component logs and leaves
/// queue 1 (0x50B1C0(1, 0)).
pub fn economy_tick(world: &mut World, key: Key) {
    let present = match key {
        Key::Actor(a) => world.actors.get(a as usize).is_some_and(|x| !x.removed),
        Key::Org(o) => world.orgs.get(o as usize).is_some_and(|g| g.alive),
    };
    if !present {
        return;
    }
    if run(world, key, |b, k| native::economy_component_tick(b, k)) {
        world.unschedule(match key {
            Key::Actor(a) => crate::scheduler::Owner::Component(a, ECONOMY_SLOT),
            Key::Org(o) => crate::scheduler::Owner::OrgComponent(o, ECONOMY_SLOT),
        });
    }
}

/// 0x4D7CE0 on one thing: the state 19 follows 0x4D0A80.
fn update_state(world: &mut World, key: Key) {
    run(world, key, |b, k| native::update_supply_state(b, k).map(|_| ()));
}

/// 0x4D6660 as it stands now: state 20 (under siege) is `+0x128` not empty, or
/// the last hit within UnderSiegeRecentlyDamagedSeconds (0x4D0440: the time of
/// Body +0x28's record, organization +0x90), or City +0x20, for a living thing.
/// The Denizen's guard zone part (0x569540, zone 5) is not bound. The original
/// takes the state at its refresh points (a hit 0x5625C0, the Element Tick
/// 0x570B90, the City Tick 0x5647A0, 0x566400, a death 0x4D7960) and keeps it
/// between them; this answers at the time of asking.
pub fn under_siege(world: &World, a: ActorId) -> bool {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive) {
        return false;
    }
    let c = world.combat.actor(a);
    if world.books.actor(a).is_some_and(|b| b.city_siege || b.saved_siege) || c.is_some_and(|c| !c.bombarding.is_empty()) {
        return true;
    }
    c.and_then(|c| c.last_hit).is_some_and(|(_, t)| {
        let seconds = world.data.rule_ci("UnderSiegeRecentlyDamagedSeconds").unwrap_or(0.0);
        t != 0.0 && ((world.time as f64 - t as f64) as f32) < seconds
    })
}

/// 0x561C60, the gate of the Body Updatable 0x562E10: UnitSupplyEnabled, not
/// an organization, unit_shortfall_damage_rate (ActorData +0x2A8) above 0
/// (0x561B40), not an organization member; the `+0x16C` GTID, the truck a
/// trailer belongs to (0x4D6F10, 0x4D5EA0), is unresolved: aa-play keeps no
/// trailer as an actor.
pub fn body_supplies(world: &World, a: ActorId) -> bool {
    let Some(x) = world.actors.get(a as usize).filter(|x| !x.removed) else { return false };
    let d = world.data.def(x.def);
    world.data.rule_ci("UnitSupplyEnabled").is_some_and(|v| v != 0.0)
        && d.class != Some(crate::data::DefClass::Organization)
        && d.components & (1 << BODY_SLOT) != 0
        && d.unit_shortfall_damage_rate > 0.0
        && x.org.is_none()
}

/// Body Updatable +0x14 0x562E10 (queue 1 when its InitCreate 0x562310
/// registers it, 0x561B40): the state 0x4D7CE0 of a lone Body that bleeds out
/// of supply (the shortfall damage 0x562D10 that follows belongs to the
/// economy).
pub fn body_supply_tick(world: &mut World, a: ActorId) {
    if body_supplies(world, a) {
        update_state(world, Key::Actor(a));
    }
}

/// `books.zones`: the active supply zones (zone 0 radius not -1) by
/// provider, then their tiles.
fn publish_zones(world: &mut World) {
    let Some(mut e) = world.fog.engine.take() else {
        world.books.zones.clear();
        world.books.zone_tiles.clear();
        return;
    };
    let mut zones = Vec::new();
    for a in 0..world.actors.len() as ActorId {
        let x = &world.actors[a as usize];
        if x.removed || !world.data.def(x.def).has_supply {
            continue;
        }
        let (Some(z), Some(k)) = (e.supply_zone(a).filter(|z| z.active()), x.kingdom) else { continue };
        zones.push(SupplyZone { provider: a, kingdom: k, centre: x.pos.truncate(), radius: f32::from_bits(z.radius_bits) });
    }
    e.refresh_supply_tiles();
    world.fog.engine = Some(e);
    world.books.zones = zones;
    publish_zone_tiles(world);
}

/// The per-update supply pass: the tile changes of this update's moves, the
/// Element Ticks of this update, the InitExisting 0x58B0E0 of the suppliers
/// new this update (creation order, each seeing the zones of those before it;
/// an unpacking building has no Supply until the deferred end of the unpacking
/// assembles it, 0x576319 -> 0x4D3068, `crate::deploy`), the Body supply ticks
/// of lone bleeding Bodies without a queue-1 registration (every update until
/// their Body registers) and `books.zones`. The city zones take candidates in
/// as they are covered ([`city_entered`]).
pub fn tick(world: &mut World) {
    crate::fog::observe_now(world);
    crate::fog::element_pass(world);
    relay_links_follow_the_swap(world);
    for a in 0..world.actors.len() as ActorId {
        let x = &world.actors[a as usize];
        if !x.removed && world.data.def(x.def).has_supply && !world.books.actor(a).is_some_and(|b| b.supply_started) && !crate::zones::unpacking(world, a) {
            supplier_tick(world, a, 0.0);
        }
    }
    for a in 0..world.actors.len() as ActorId {
        if body_supplies(world, a) && !world.is_scheduled(crate::scheduler::Owner::Component(a, BODY_SLOT as u8)) {
            body_supply_tick(world, a);
        }
    }
    publish_zones(world);
}

/// An unpack gives the building the truck's GTID (0x4EFB90): a relay's link
/// Supply +0x1C to the truck goes on resolving, to the building. The links here
/// name the actor, and the zone keys are identities only, so the swap has to
/// reach them.
fn relay_links_follow_the_swap(world: &mut World) {
    let swaps: Vec<(ActorId, ActorId)> = world
        .books
        .notices
        .iter()
        .filter_map(|n| match n {
            crate::books::Notice::Replaced { old, new } => Some((*old, *new)),
            _ => None,
        })
        .collect();
    for (old, new) in swaps {
        for b in world.books.actors.iter_mut() {
            if b.supply_relay == Some(old) {
                b.supply_relay = Some(new);
            }
        }
    }
}

/// 0x564300, reached from the city's zone 8 (0x514E20 when the zone covers a
/// tile with the thing on it, 0x515620 when the thing's tile change enters
/// one; the film: 0x4D6350 -> 0x4CFEA0 -> 0x515620): an admitted candidate
/// (0x5640A0) not listed yet joins the head of City+18, and a City not on a
/// queue registers on queue 1 (0x50AC60: one draw).
pub(crate) fn city_entered(world: &mut World, city: ActorId, who: Key) {
    let who = match who {
        Key::Actor(a) => crate::books::CityCandidateRef::Actor(a),
        Key::Org(o) => crate::books::CityCandidateRef::Org(o),
    };
    if !admitted(world, city, who) || world.books.actor(city).is_some_and(|b| b.city_candidates.contains(&who)) {
        return;
    }
    world.books.actor_mut(city).city_candidates.insert(0, who);
    let owner = crate::scheduler::Owner::Component(city, crate::scheduler::CITY_SLOT);
    if !world.is_scheduled(owner) {
        world.schedule_periodic_at(crate::trace::sites::CITY_ADMISSION, crate::scheduler::EVERY_SECOND, owner, 0);
    }
}

/// KKC_OrganizationComponent::TickMaintenance 0x57C8E0 for organization
/// `o` (queue 1, `elapsed` = 1 s): after the zone update 0x515F40 the
/// organization follows its moving leader (0x57C800, movement owner); a
/// supply_used one (ActorData +0x2F4) takes its supply state 0x4D7CE0 (on a
/// change 0x4D1350 refreshes every member's modifiers), then the shortfall
/// attrition 0x578660 of an out-of-supply organization of a kingdom short of
/// upkeep; then behind the 0x579110 gate the regeneration 0x57B5A0 of one
/// slot.
pub fn org_maintenance(world: &mut World, o: OrgId, elapsed: f32) {
    if !world.orgs.get(o as usize).is_some_and(|x| x.alive) {
        return;
    }
    crate::morale::maintenance_delivery(world, o, elapsed);
    if !world.orgs.get(o as usize).is_some_and(|x| x.alive) {
        return;
    }
    crate::movement::org_follow_leader(world, o);
    if !world.orgs.get(o as usize).is_some_and(|x| x.alive) {
        return;
    }
    if world.data.def(world.orgs[o as usize].def).supply_used {
        update_state(world, Key::Org(o));
    }
    crate::economy::shortfall(world, o, elapsed);
    regenerate(world, o, elapsed);
    // +0xB0: the ZOC at the radius +0xAC its modifiers now give (0x577FD0).
    if world.combat.org(o).is_some()
        && let Some(mut e) = crate::fog::take_engine(world)
    {
        let radius = crate::combat::control_radius(world, &world.combat, o);
        e.zoc_radius(world, Key::Org(o), radius);
        crate::fog::put_engine(world, e);
    }
    // +0x8E: the behaviour has just turned aggressive: the effects of the ZOC
    // over what already stands in it (0x5147C0).
    if let Some(s) = world.combat.orgs.get_mut(o as usize)
        && std::mem::take(&mut s.rescan_8e)
        && let Some(mut e) = crate::fog::take_engine(world)
    {
        e.zoc_effects(world, Key::Org(o));
        crate::fog::put_engine(world, e);
    }
}

/// One city zone candidate: a lone actor or an organization (as the native
/// organization actor at its own position).
struct CityCandidate {
    kingdom: KingdomId,
    /// 4D0180: the current value the city tick weighs.
    current: f32,
}

/// Performer +60 (57E430): the best 45E090 value over the Performer and the
/// Defender abilities of a definition with a PerformerComponent
/// ([`crate::combat::performer_values`]), None without one.
pub(crate) fn performer_value(world: &World, def: crate::data::DefId) -> Option<f32> {
    let d = world.data.def(def);
    if !d.has_component("PerformerComponent") {
        return None;
    }
    Some(world.combat.extras.as_ref().map_or(0.0, |e| e.def(def).performer_value))
}

/// Performer +60 of actor `a` (57E430 at its creation), [`performer_value`] until
/// combat has seen it.
fn actor_performer_value(world: &World, a: ActorId) -> Option<f32> {
    let def = world.actors[a as usize].def;
    let value = performer_value(world, def)?;
    Some(world.combat.actor(a).filter(|c| c.initialized).map_or(value, |c| c.performer_value))
}

/// 57DC40: the Body health ratio (Body +10 / +14), 1 without a Body.
pub(crate) fn health_ratio(world: &World, a: ActorId) -> f32 {
    let actor = &world.actors[a as usize];
    let d = world.data.def(actor.def);
    if !d.has_component("BodyComponent") {
        return 1.0;
    }
    let max = world
        .combat
        .actors
        .get(a as usize)
        .filter(|c| c.initialized && c.max_health > 0.0)
        .map_or_else(|| crate::production::max_health(world, actor.kingdom, actor.def), |c| c.max_health);
    if max > 0.0 { actor.health / max } else { 1.0 }
}

/// 5640A0 kingdom gate: owned by a major (non-independent) kingdom
/// (kingdom +400 == 0).
fn major(world: &World, k: Option<KingdomId>) -> Option<KingdomId> {
    k.filter(|&k| world.kingdoms.get(k as usize).is_some_and(|x| !x.independent))
}

/// One thing's standing against a city, when 0x5640A0 can admit it: not dead,
/// owned by a major kingdom, not an organization member (4549C0: the
/// organization stands for it), thing flag bit 12 (flying, 0x1000) clear,
/// actor value 4D01E0(0) > 0, and without a Body or captureable or
/// destructible. 4D01E0 is Organization +6C (5788B0 maximum: the members'
/// Performer +60 plus the empty layout slots' ActorData +1E4) + Settlement
/// +34 + Performer +60 + Denizen +14; 4D0180, the weight, uses Organization
/// +68 (the members' Performer +60 times their health ratio), Settlement
/// +30, 57DC40 (Performer +60 times the health ratio) and Denizen +10.
/// aa-play has no Settlement occupants: that term is 0.
fn candidate(world: &World, who: crate::books::CityCandidateRef) -> Option<CityCandidate> {
    if let crate::books::CityCandidateRef::Actor(a) = who
        && world.actors.get(a as usize)?.org.is_some()
    {
        return None;
    }
    candidate_of_load(world, who)
}

/// Whether `who` is a candidate during a saved game's record pass: a member's publish
/// 4FB514 comes before its organization holds it (4549C0 reads no organization yet).
pub(crate) fn admitted_at_load(world: &World, who: crate::books::CityCandidateRef) -> bool {
    candidate_of_load(world, who).is_some()
}

fn candidate_of_load(world: &World, who: crate::books::CityCandidateRef) -> Option<CityCandidate> {
    match who {
        crate::books::CityCandidateRef::Actor(a) => {
            let actor = world.actors.get(a as usize)?;
            if !actor.alive || actor.removed {
                return None;
            }
            let kingdom = major(world, actor.kingdom)?;
            let d = world.data.def(actor.def);
            if d.flying || (d.has_component("BodyComponent") && !d.captureable && !d.destructible) {
                return None;
            }
            // Performer +60 and Denizen +14 (the value), 57DC40 and Denizen
            // +10 (the weight).
            let performer = actor_performer_value(world, a);
            let (dv, dmax) = world.books.actor(a).and_then(|b| b.denizen.as_ref()).map_or((0.0, 0.0), |d| (d.value, d.value_max));
            let value = performer.unwrap_or(0.0) + dmax;
            if !(value > 0.0) {
                return None;
            }
            let current = performer.map_or(0.0, |v| v * health_ratio(world, a)) + dv;
            Some(CityCandidate { kingdom, current })
        }
        crate::books::CityCandidateRef::Org(o) => {
            let org = world.orgs.get(o as usize)?;
            if !org.alive || !org.members.iter().any(|&m| live(world, m)) {
                return None;
            }
            let kingdom = major(world, Some(org.kingdom))?;
            let d = world.data.def(org.def);
            if d.flying || (d.has_component("BodyComponent") && !d.captureable && !d.destructible) {
                return None;
            }
            let (mut maximum, mut current) = (0.0f32, 0.0f32);
            for (slot, (slot_def, _)) in d.slots.iter().enumerate() {
                let member = org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16));
                match member {
                    Some(m) => {
                        if let Some(v) = actor_performer_value(world, m) {
                            current += health_ratio(world, m) * v;
                            maximum += v;
                        }
                    }
                    None => {
                        if *slot_def != crate::data::NO_DEF
                            && let Some(v) = performer_value(world, *slot_def)
                        {
                            maximum += v;
                        }
                    }
                }
            }
            if maximum <= 0.0 {
                return None;
            }
            Some(CityCandidate { kingdom, current })
        }
    }
}

/// 0x5640A0 for `who` against `city` (itself excluded): City+10, the blocks
/// that go with it, also refuses.
fn admitted(world: &World, city: ActorId, who: crate::books::CityCandidateRef) -> bool {
    if let crate::books::CityCandidateRef::Actor(a) = who
        && world.books.actor(city).is_some_and(|b| b.city_blocks.contains(&a))
    {
        return false;
    }
    who != crate::books::CityCandidateRef::Actor(city) && candidate(world, who).is_some()
}

/// The world start's 0x4F6B50 -> 0x4F6980 (from 0x4FB9E0, once the zones stand):
/// every city (kingdom +0x30C, a thing with a zone 8) takes the things on its
/// zone's tiles that are its own owner's, have a Body that can be neither
/// destroyed (+0x291) nor taken (0x4D1120), and a Structure not yet attached,
/// and that are no city themselves: 0x5642A0 lists the thing on City+10 and writes
/// the city to its Structure +0x34 (0x588AB0). City+10 changes hands with the
/// city (0x5643A0), and such a block is no target (0x522CC0).
pub(crate) fn attach_city_blocks(world: &mut World) {
    let Some(engine) = world.fog.engine.as_ref() else { return };
    let cities: Vec<(ActorId, Vec<ActorId>)> = world
        .actors
        .iter()
        .filter(|a| a.alive && !a.removed && world.data.def(a.def).city)
        .map(|a| (a.id, engine.things_in_city_zone(a.id)))
        .filter(|(_, l)| !l.is_empty())
        .collect();
    for (city, things) in cities {
        let owner = world.actors[city as usize].kingdom;
        for b in things {
            let x = &world.actors[b as usize];
            let d = world.data.def(x.def);
            if !x.alive
                || x.removed
                || x.kingdom != owner
                || d.city
                || !d.structure
                || d.health <= 0.0
                || d.destructible
                || crate::combat::capturable(world, b)
                || world.books.actor(b).is_some_and(|k| k.block_of.is_some())
            {
                continue;
            }
            world.books.actor_mut(city).city_blocks.insert(0, b);
            world.books.actor_mut(b).block_of = Some(city);
        }
    }
}

/// City tick 0x5647A0 (queue 1, the City component) for city `c`. Against
/// the current owner: the value of the owner's own candidates in the zone
/// defends (actor virtual +0x138 allied and +0x134 owned: a friend's value
/// does not defend), the value of its enemies (+0x140) attacks. Attackers
/// with no defender (or no owner at all) let the zone decide: the first
/// kingdom in kingdom order that has value there, no value of kingdoms it
/// regards as enemies, and no friend with more value or under threat takes
/// the city (0x4D5230 owner change, reason 10). Otherwise the city is under
/// siege (City +0x20 = 1, reset at each tick and when the zone empties,
/// 0x564C10) unless the attack is below the summed value of the allied
/// candidates of City+18 (0x5648BC, so friends alone hold it in peace); a
/// city under siege provides no supply (0x58ACE0). The neutral buildings of
/// the old owner around the city change hands with it.
pub fn city_tick(world: &mut World, c: ActorId) {
    use crate::books::{ENEMY, FRIEND};
    let n = world.kingdoms.len();
    {
        let Some(city) = world.actors.get(c as usize) else { return };
        if !city.alive || city.removed {
            return;
        }
        let def = world.data.def(city.def);
        if !def.city || def.city_zone_radius <= 0.0 {
            return;
        }
    }
    // 0x564C10: the City+18 candidates still admitted (0x5640A0) and in the
    // zone (virtual +0x130(zone 8, 0)) stay; an empty list leaves the queue
    // (0x50B250) until the next admission (0x564300) registers the City again.
    {
        if let Some(mut e) = crate::fog::take_engine(world) {
            e.observe_update(world);
            crate::fog::put_engine(world, e);
        }
        let mut list = std::mem::take(&mut world.books.actor_mut(c).city_candidates);
        let engine = world.fog.engine.as_ref();
        let inside = |who: crate::books::CityCandidateRef| {
            let key = match who {
                crate::books::CityCandidateRef::Actor(a) => Key::Actor(a),
                crate::books::CityCandidateRef::Org(o) => Key::Org(o),
            };
            engine.is_some_and(|e| e.in_city_zone(key, c))
        };
        list.retain(|&who| admitted(world, c, who) && inside(who));
        let empty = list.is_empty();
        world.books.actor_mut(c).city_candidates = list;
        if empty {
            // City +0x20 cleared (then the siege state 0x4D6660)
            world.books.actor_mut(c).city_siege = false;
            world.unschedule(crate::scheduler::Owner::Component(c, crate::scheduler::CITY_SLOT));
            return;
        }
    }
    // 0x5647A0: City +0x20 = 0 first
    world.books.actor_mut(c).city_siege = false;
    {
        let city = &world.actors[c as usize];
        let owner = city.kingdom;
        // 0x5647A0 over the City+18 candidates (the city itself never is one:
        // it has no Performer and belongs to its owner).
        let listed = world.books.actor(c).map(|b| b.city_candidates.clone()).unwrap_or_default();
        let candidates: Vec<CityCandidate> = listed.iter().filter_map(|&who| candidate(world, who)).collect();
        let mut present = vec![0.0f32; n];
        let mut any = false;
        for cand in &candidates {
            if (cand.kingdom as usize) < n {
                present[cand.kingdom as usize] += cand.current;
                any = true;
            }
        }
        let rel = |a: usize, b: usize| world.books.relation(&world.kingdoms, Some(a as u32), Some(b as u32));
        let (decide, siege) = match owner {
            None => (any, false),
            Some(o) => {
                // over City+18 head first: an allied candidate (+0x138: owned
                // or a friend) adds its value to the allied sum (0x564846 ..
                // 0x564854 `fadd`/`fstp` of the same local) and to the defence
                // when owned (+0x134); an enemy (+0x140) adds to the attack
                let o = o as usize;
                let (mut defend, mut attack, mut ally) = (0.0f32, 0.0f32, 0.0f32);
                for cand in &candidates {
                    let k = cand.kingdom as usize;
                    if k >= n {
                        continue;
                    }
                    let r = rel(k, o);
                    if k == o || r == FRIEND {
                        ally += cand.current;
                        if k == o {
                            defend += cand.current;
                        }
                    } else if r == ENEMY {
                        attack += cand.current;
                    }
                }
                let decide = attack > 0.0 && defend <= 0.0;
                // 0x5648BC: under siege unless the attack is below the allied sum
                (decide, !decide && !(attack < ally))
            }
        };
        if !decide {
            // 0x5648E8: held under siege
            world.books.actor_mut(c).city_siege = siege;
            return;
        }
        // threat[k]: the value of the candidates whose kingdom k regards as
        // an enemy (kingdom virtual +0x140)
        let threat: Vec<f32> = (0..n).map(|k| (0..n).filter(|&e| e != k && rel(k, e) == ENEMY).map(|e| present[e]).sum()).collect();
        let winner = (0..n).find(|&k| {
            threat[k] <= 0.0
                && present[k] > 0.0
                && !(0..n).any(|f| f != k && rel(k, f) == FRIEND && (present[k] < present[f] || threat[f] > 0.0))
        });
        let Some(w) = winner.map(|k| k as u32) else { return };
        if Some(w) == owner {
            return;
        }
        world.actors[c as usize].kingdom = Some(w);
        world.events.push(crate::world::Event::Captured { actor: c, kingdom: w });
        // 0x4D8450 -> 0x5643A0: City+10, the blocks, go with it.
        for b in world.books.actor(c).map(|k| k.city_blocks.clone()).unwrap_or_default() {
            if let Some(a) = world.actors.get_mut(b as usize).filter(|a| !a.removed) {
                a.kingdom = Some(w);
            }
        }
        // 0x4D5230 -> 0x4CF0F0: the contribution moves to the new owner at once.
        crate::economy::refresh(world);
    }
}
/// The organization's own position (its GActor +20/+24): set at its
/// creation and moved after its leader only by its maintenance tick
/// 0x57C8E0 -> 0x57C800 -> 0x57C680 (the Berlin film: a new regiment stays
/// at its creation spot while its truck stands 10 m away). The movement
/// owner keeps it in `Organization::anchor`.
pub fn org_position(world: &World, o: OrgId) -> Vec2 {
    world.orgs[o as usize].anchor
}

/// 0x579110 gate inputs owned by combat: somebody engages the organization (actor
/// +0x118, its engagers' list), or the ActorStateMind it reports (0x4D49B0(4)) has
/// `resupply = false` (+0x1C, Game/actor_states.tgi): chasing, engaged, routing, bombard,
/// regrouping, rampaging, unpacking, packing. What its members shoot at is no input.
fn engaged(world: &World, o: OrgId) -> bool {
    use crate::morale::RegimentState as S;
    crate::combat::org_state(world, o).is_some_and(|c| c.is_engaged())
        || matches!(crate::morale::regiment_state(world, o), S::Chasing | S::Engage | S::Rout | S::Bombard | S::Regroup | S::Rampage | S::Unpack | S::Pack)
}

/// 0x57B5A0 behind the 0x579110 gate for organization `o`, `dt` = the
/// maintenance tick's elapsed time.
fn regenerate(world: &mut World, o: OrgId, dt: f32) {
    {
        let org = &world.orgs[o as usize];
        if !org.alive {
            return;
        }
        let def = world.data.def(org.def);
        if !def.supply_used || world.books.org(o).is_none_or(|b| b.out_of_supply) || engaged(world, o) {
            return;
        }
        let shortage = world.books.kingdom(org.kingdom).map_or(0.0, |k| k.shortage_ratio);
        let factor = if shortage > 0.0 { 1.0 - shortage } else { 1.0 };
        let slots = def.slots.len();
        let member_at = |slot: usize| {
            org.members.iter().copied().find(|&m| live(world, m) && world.actors[m as usize].slot == Some(slot as u16))
        };
        // 5617C0 against Body +14, the actual maximum (handicap, technology
        // and experience HEALTH modifiers).
        let damaged = |m: ActorId| world.actors[m as usize].health < member_max_health(world, m);
        let leader = def.leader_slot.map(usize::from).filter(|&l| l < slots);
        let choice = leader
            .filter(|&l| member_at(l).is_some_and(damaged))
            // 0x57B5A0's loop: a damaged member only while it is attached (Element +1C).
            .or_else(|| (0..slots).find(|&s| member_at(s).is_some_and(|m| damaged(m) && crate::movement::element_is_attached(world, m))))
            .or_else(|| leader.filter(|&l| member_at(l).is_none() && def.slots[l].0 != u32::MAX))
            .or_else(|| (0..slots).find(|&s| member_at(s).is_none() && def.slots[s].0 != u32::MAX && world.data.def(def.slots[s].0).has_component("BodyComponent")));
        let Some(slot) = choice else { return };
        if let Some(m) = member_at(slot) {
            // Element +20 (5704D0: resupply_rate through the member's
            // RESUPPLY_RATE modifier) * Body +14 * (1 - shortage) * delta,
            // one member at a time until it is full (57B5A0 / 562BA0).
            let md = world.data.def(world.actors[m as usize].def);
            let rate = md.resupply_rate * crate::production::multiply_factor(world, org.kingdom, &md.properties, "RESUPPLY_RATE");
            let rate = rate + md.resupply_rate * (crate::special_ops::org_resupply_multiplier(world, o) - 1.0);
            let max = member_max_health(world, m);
            let amount = rate * max * factor * dt;
            let a = &mut world.actors[m as usize];
            a.health = (a.health + amount).min(max);
            return;
        }
        let slot_def = def.slots[slot].0;
        let sd = world.data.def(slot_def);
        // 0x5014E0: the organization modifier context multiplies +2CC
        // (technologies such as tech_plentiful_ammunition: RESUPPLY_RATE).
        let rate = sd.resupply_rate * crate::production::multiply_factor(world, org.kingdom, &def.properties, "RESUPPLY_RATE");
        // (combat owner) the enchantments' RESUPPLY_RATE (Resupply special
        // operations) in the same 502A70 sum
        let rate = rate + sd.resupply_rate * (crate::special_ops::org_resupply_multiplier(world, o) - 1.0);
        let amount = sd.health * rate * factor * dt;
        let b = world.books.org_mut(o);
        if b.progress.len() < slots {
            b.progress.resize(slots, 0.0);
        }
        b.progress[slot] += amount;
        if b.progress[slot] >= sd.health {
            b.progress[slot] = 0.0;
            // 57AD30 -> 4FBF40: out of the nearest recruiting structure's
            // door when one is within control range, else at the slot.
            crate::production::spawn_member(world, o, slot);
        }
    }
}

/// Body +14 of a member: the combat maximum (with the experience level),
/// else the definition health with the kingdom HEALTH modifiers.
fn member_max_health(world: &World, m: ActorId) -> f32 {
    let a = &world.actors[m as usize];
    world
        .combat
        .actors
        .get(m as usize)
        .filter(|c| c.initialized && c.max_health > 0.0)
        .map_or_else(|| crate::production::max_health(world, a.kingdom, a.def), |c| c.max_health)
}
