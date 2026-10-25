//! The original zone engine (aa-zones: KKC_Zone, the interned tile zone
//! sets, the ZoneSolver fast circle builder and Dijkstra flood, the tile fog
//! and supply words, the detection component and the actor detection word)
//! running over the battle world. Owner: fog (UI developer).
//!
//! aa-zones is the oracle-proven port of AA.exe (research/phase8-zones). This
//! module implements its world traits over [`World`] and calls it at the
//! native points, at PC53 (the simulation thread):
//!
//! * actor tile change (`0x4D6350`): the tile actor lists, `0x4EC270`
//!   ([`detection::tile_changed`]) and the zone loop
//!   ([`component::move_actor_zones`]: every active zone, the supply zone
//!   included, follows the actor) - [`Engine::observe`];
//! * the supply component zone (`0x58AE10` tail: `0x58ACE0` provides vs the
//!   active slot 0, `0x514CE0` release, `0x515F40` move when it starts
//!   providing or its radius was recomputed, `0x58AA20` dirty byte), zone
//!   type 0 (ZOS) flooded by the ZoneSolver - [`Engine::supply_tick`];
//! * the detection component tick (`0x56F140`, [`component::tick`]) of every
//!   actor and organization with detection ranges: the ZOD (type 6) and RZOD
//!   (type 1) zones and the reveal mask - [`Engine::detection_tick`];
//! * the world start (`0x551F90` / `0x551FD0`) - [`Engine::start_fog`];
//! * an ownership change (`0x4D8450` zone part, a captured city or
//!   building): every active zone released under the old owner and put back
//!   on its tile for the new one (`0x4D5DE0` / `0x4CFD30`);
//! * destruction: every zone of a removed actor is released (`0x514CE0`).
//!
//! The tile callbacks (`+0x64` reveal, `+0x68` hide, `+0x6C` unexplore) run
//! [`detection::reveal`] / [`detection::hide`] / [`detection::unexplore`] on
//! the actor's detection word and its memory.
//!
//! World facts the traits need and how they are answered (native offsets in
//! the aa-zones trait docs):
//! * keys: actor `i` -> `i + 1`, organization `o` -> `ORG | (o + 1)`,
//!   kingdom `k` -> `KINGDOM + k` (index `+0x18C` = `k`, non-major `+0x190`
//!   for an independent kingdom or one past the 16 fog bits);
//! * allied (`+0x138`) = relation self or friend, enemy (`+0x140`) = enemy
//!   ([`crate::books::Books::relation`], the native 4E52D0 values);
//! * actor `+8` bit 0 = dead; `+0x100` bit 21 (0x200000) = under
//!   construction: a building still unpacking from its truck (its type-6
//!   zone then takes ActorData `+0x1D0`, `0x56F140`);
//! * the `+0x80` component of an organization member points at its living
//!   organization; its byte `+0x1C` (`0x5709D0`) is set for the
//!   organization's leader and for members inside the organization's ZOC
//!   (zone type 4 of ActorData `+0x2FC`, not bound: the control_range circle
//!   around the organization stands for its tiles). Such a member carries
//!   no zones of its own (`0x56EA30` / `0x56EC10`) and adds its engagers to
//!   the organization's reveal mask (`0x56EFC0`);
//! * ActorData `+0x1CC` / `+0x1D0` = the finalized detection / remote ranges
//!   ([`def_detection`]); reveal sources (`0x56EDC0`) = the kingdoms whose
//!   actors target the actor; the kingdom reveal byte (`0x4CF110`) bit 7 =
//!   the REVEAL_ENEMY_RZOD enchantment flag;
//! * the tile actor list (`+8`) holds every actor that is not removed on the
//!   tiles it occupies (a structure's footprint, a unit's tile) and every
//!   living organization on its own tile, newest first;
//! * memory (`+0x8C`) is kept by static actors; records are snapshots of
//!   the actor when created (the record object `0x4EB9F0` is not ported).
//!
//! Not bound (no source in the battle world yet): zone types 2 (discovery
//! markers), 3, 4 (ZOC), 5 (ZOG), 7, 8 and the actor-entered effects
//! (`0x54EF10`), containers and carriers, first-sighting messages, the SAI
//! region counters (the hooks keep the fog-word mirror only), the detection
//! range modifiers (`0x501C30`) and the updatable schedule of the component
//! ticks (every component ticks every battle tick, actors then
//! organizations in index order).
use crate::books::{ENEMY, FRIEND, SELF};
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, KingdomId, OrgId, World};
use aa_numeric::Precision;
use aa_zones::component::{self, ActorZones, DetectionComponent, DetectionComponentWorld};
use aa_zones::detection::{self, ActorDetectionWorld};
use aa_zones::flood::FloodWorld;
use aa_zones::query::{Kingdom, VisitWorld, ZoneActors};
use aa_zones::tile::{AllianceWorld, ComponentEffect, MapGeometry, SaiHook, ZoneMap, ZoneWorld};
use aa_zones::zone::{self, Zone, ZoneObjectWorld};
use aa_zones::{ActorKey, Error, Result, SetRef};
use glam::{Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// The simulation thread's x87 precision control.
pub const PC: Precision = Precision::Double53;
/// Organization keys.
pub const ORG: u32 = 0x8000_0000;
/// Kingdom key base.
pub const KINGDOM: u32 = 0x0001_0000;
/// Kingdom bits of the fog / supply / reveal words (`0xA11740`).
pub const KINGDOM_BITS: u32 = aa_zones::query::KINGDOM_BITS;
/// Zone type 0 (ZOS supply).
pub const ZOS: u32 = 0;
const EXPLORED_ALL: u32 = 0xffff;
const VISIBLE_ALL: u32 = 0xffff_0000;

/// What a kingdom remembers of a static actor it no longer sees.
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    pub def: DefId,
    pub pos: Vec3,
    pub facing: f32,
    pub kingdom: Option<KingdomId>,
    pub health: f32,
}

/// Per-definition detection data (ActorData +0x1CC / +0x1D0 and what the
/// world callbacks need).
#[derive(Clone, Copy, Debug, Default)]
pub struct DefDetection {
    /// ActorData +0x1CC (finalized detection range, m).
    pub zod: f32,
    /// ActorData +0x1D0 (remote detection range, m).
    pub rzod: f32,
    /// The definition has a detection component (actor +0xB8).
    pub component: bool,
    /// Static actor: explored at the EXPLORED start, keeps memory records.
    pub keeps_memory: bool,
    /// A building, city or other structure (footprint tiles, memory).
    pub structure: bool,
    /// Footprint half extents (m) for the tiles a structure occupies.
    pub half: Vec2,
    /// Actor virtual +0 (size radius, m).
    pub size: f32,
    /// ActorData +0x2FC (control range, m): the organization ZOC radius.
    pub control: f32,
}

/// `0x468B16` / `0x46889E`: the finalized ranges of a definition.
pub fn def_detection(d: &crate::data::Def) -> DefDetection {
    let has = d.detection_range > 0.0 || d.remote_detection_range > 0.0;
    let org = d.class == Some(DefClass::Organization);
    // 0x468B16: the largest of the contributing ranges
    let zod = if has {
        let mut r = d.detection_range.max(d.supply_range);
        if org {
            r = r.max(d.control_range);
        }
        r
    } else {
        0.0
    };
    // 0x46889E: an organization's remote range is its control range
    let rzod = if has {
        if org && d.remote_detection_range >= 0.0 { d.control_range } else { d.remote_detection_range }
    } else {
        0.0
    };
    let structure = matches!(d.class, Some(DefClass::Building)) || d.city || d.structure;
    let keeps_memory = structure || (d.class == Some(DefClass::Other) && d.actor_data && d.speed_max <= 0.0 && !d.invisible);
    let half = if structure { d.blocking_size.map_or(Vec2::ZERO, |s| Vec2::new(s[0], s[1]) * 0.5) } else { Vec2::ZERO };
    let size = match (d.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref(), d.blocking_size) {
        (Some("CIRCLE"), Some(s)) => s[0].max(0.0),
        (Some(_), Some(s)) => (s[0] * s[0] + s[1] * s[1]).sqrt() * 0.5,
        _ => 0.0,
    };
    DefDetection { zod, rzod, component: has, keeps_memory, structure, half, size, control: d.control_range }
}

/// Zone state of one actor or organization.
#[derive(Clone, Debug, Default)]
pub struct Slot {
    /// Actor +0x148 zone vector.
    pub zones: ActorZones,
    /// Actor +0xB8 detection component.
    pub comp: Option<DetectionComponent>,
    /// Actor +0x54 detection word.
    pub word: u32,
    /// The tiles the actor is listed on (row-major).
    pub tiles: Vec<usize>,
    /// The tile under the actor (actor +0x44 / +0x48); None before placing.
    pub tile: Option<(i32, i32)>,
    /// Destroyed: zones released, off the tiles, never placed again.
    pub gone: bool,
    /// The pose (position x, y and facing bits) the tiles were computed for.
    pose: Option<[u32; 3]>,
    /// The owner the zones were placed for (`+0xE8`), None before placing.
    owner: Option<Option<KingdomId>>,
    /// Supply component +0x10 (radius bits) and dirty byte +0x14.
    supply_radius: Option<u32>,
    supply_dirty: bool,
}

impl Slot {
    pub fn zone(&self, index: u32) -> Option<Zone> {
        self.zones.slots.get(index as usize).copied().flatten()
    }
    fn supply_active(&self) -> bool {
        self.zone(ZOS).is_some_and(|z| z.active())
    }
}

/// Work and refusal counters.
#[derive(Clone, Debug, Default)]
pub struct EngineStats {
    pub tile_changes: u64,
    pub component_ticks: u64,
    pub supply_moves: u64,
    pub releases: u64,
    /// Microseconds of the last observe / detection tick / supply tick.
    pub observe_us: f32,
    pub detection_us: f32,
    pub supply_us: f32,
    /// aa-zones refusals (explicit native faults / unported paths) by text.
    pub refusals: BTreeMap<String, u64>,
}

/// Everything the trait context owns besides the zone map.
#[derive(Debug)]
pub struct Parts {
    pub geometry: MapGeometry,
    pub actors: Vec<Slot>,
    pub orgs: Vec<Slot>,
    /// Tile +8 actor lists in insertion order (the head is the last).
    lists: Vec<Vec<ActorKey>>,
    /// Tile +0x14 fog words, mirrored through the SAI hooks for the reads
    /// made while aa-zones holds the map (`0x4EC1B0`).
    mirror: Vec<u32>,
    /// Fog bit changes reported by the hooks (renderers re-read on change).
    pub fog_changes: u64,
    /// Kingdom fields by kingdom id.
    pub kingdoms: Vec<Kingdom>,
    pub fog_on: bool,
    fast_zones: bool,
    /// Ghost records per kingdom index.
    pub memory: Vec<BTreeMap<ActorId, Memory>>,
    /// Reveal sources (0x56EDC0) per actor key of this tick.
    engaged: Vec<u32>,
    /// Kingdom reveal bytes (0x4CF110) of this tick, by kingdom id.
    reveal_bytes: Vec<u8>,
    /// Organization membership per actor (+0x80 target, byte +0x1C).
    member: Vec<Option<(OrgId, bool)>>,
    /// The owner an actor answers while its zones are released for an
    /// ownership change (`0x4D8450` releases under the old owner).
    owner_override: Option<(ActorKey, Option<KingdomId>)>,
    next_zone_key: u32,
    /// Supply providers whose zone changed since the last tile scan.
    pub supply_changed: BTreeSet<ActorId>,
    pub stats: EngineStats,
    defs: Vec<DefDetection>,
}

/// The zone engine of one battle.
#[derive(Debug)]
pub struct Engine {
    pub map: ZoneMap,
    pub parts: Parts,
    /// The tiles of each provider's supply zone (sorted), from the tile sets.
    pub supply_tiles: BTreeMap<ActorId, Arc<Vec<u32>>>,
}

/// Identity of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Actor(ActorId),
    Org(OrgId),
}

pub fn actor_key(a: ActorId) -> ActorKey {
    a + 1
}
pub fn org_key(o: OrgId) -> ActorKey {
    ORG | (o + 1)
}
pub fn kingdom_key(k: KingdomId) -> u32 {
    KINGDOM + k
}
fn decode(key: ActorKey) -> Result<Key> {
    if key == 0 {
        return Err(Error::NativeNull("actor key 0"));
    }
    Ok(if key & ORG != 0 { Key::Org((key & !ORG) - 1) } else { Key::Actor(key - 1) })
}

/// `log2(tile)` when the tile edge is a power of two (GMap +4 / +8).
fn shift_of(tile: f32) -> Option<u32> {
    let t = tile as u32;
    (t > 0 && t.is_power_of_two() && t as f32 == tile).then(|| t.trailing_zeros())
}

/// The map geometry of the zone code, None for a map the GMap shift cannot
/// describe (no tiles, or a tile edge that is not a power of two).
pub fn geometry(m: &crate::map::GameMap) -> Option<MapGeometry> {
    let shift = shift_of(m.cell_size)?;
    // a map without tile flags (synthetic test maps) is all passable land
    if m.width == 0 || m.height == 0 || (!m.flags.is_empty() && m.flags.len() < (m.width * m.height) as usize) {
        return None;
    }
    let tile = m.cell_size;
    Some(MapGeometry {
        shift,
        tile_size_bits: tile.to_bits(),
        extent_bits: [(m.width as f32 * tile).to_bits(), (m.height as f32 * tile).to_bits()],
        width: m.width as i32,
        height: m.height as i32,
    })
}

impl Engine {
    /// A fresh engine for the world's map (None: see [`geometry`]).
    pub fn new(world: &World) -> Option<Engine> {
        let geometry = geometry(&world.map)?;
        let map = ZoneMap::new(geometry).ok()?;
        let n = (world.map.width * world.map.height) as usize;
        let fast_zones = world.data.rule_ci("ZoneBuilderFastDetectionZones").is_none_or(|v| v != 0.0);
        Some(Engine {
            map,
            parts: Parts {
                geometry,
                actors: Vec::new(),
                orgs: Vec::new(),
                lists: vec![Vec::new(); n],
                mirror: vec![0; n],
                fog_changes: 0,
                kingdoms: Vec::new(),
                fog_on: true,
                fast_zones,
                memory: vec![BTreeMap::new(); KINGDOM_BITS as usize],
                engaged: Vec::new(),
                reveal_bytes: Vec::new(),
                member: Vec::new(),
                owner_override: None,
                next_zone_key: 0x4000_0000,
                supply_changed: BTreeSet::new(),
                stats: EngineStats::default(),
                defs: world.data.defs.iter().map(def_detection).collect(),
            },
            supply_tiles: BTreeMap::new(),
        })
    }

    /// The engine still fits the world's map.
    pub fn fits(&self, world: &World) -> bool {
        geometry(&world.map) == Some(self.parts.geometry)
    }

    /// The tile fog word (tile +0x14).
    pub fn tile_fog(&self, t: usize) -> u32 {
        self.map.tiles.get(t).map_or(0, |x| x.fog)
    }

    /// Run `f` with the trait context; a refusal is counted and logged once.
    fn with<R>(&mut self, world: &World, f: impl FnOnce(&mut ZoneMap, &mut Ctx<'_>) -> Result<R>) -> Option<R> {
        let mut ctx = Ctx { w: world, p: &mut self.parts };
        match f(&mut self.map, &mut ctx) {
            Ok(r) => Some(r),
            Err(e) => {
                let text = e.to_string();
                let n = self.parts.stats.refusals.entry(text.clone()).or_insert(0);
                if *n == 0 {
                    log_refusal(&text);
                }
                *n += 1;
                None
            }
        }
    }

    /// The detection data of a key's definition.
    pub fn def_of(&self, world: &World, key: Key) -> DefDetection {
        let def = match key {
            Key::Actor(a) => world.actors.get(a as usize).map(|x| x.def),
            Key::Org(o) => world.orgs.get(o as usize).map(|x| x.def),
        };
        def.and_then(|d| self.parts.defs.get(d as usize).copied()).unwrap_or_default()
    }

    fn grow(&mut self, world: &World) {
        if self.parts.actors.len() < world.actors.len() {
            self.parts.actors.resize_with(world.actors.len(), Slot::default);
        }
        if self.parts.orgs.len() < world.orgs.len() {
            self.parts.orgs.resize_with(world.orgs.len(), Slot::default);
        }
        if self.parts.defs.len() != world.data.defs.len() {
            self.parts.defs = world.data.defs.iter().map(def_detection).collect();
        }
    }

    /// Kingdom fields and their vision / supply masks (`0x4E0F90`).
    fn refresh_kingdoms(&mut self, world: &World) {
        let ks: Vec<Kingdom> = world
            .kingdoms
            .iter()
            .map(|k| Kingdom {
                key: kingdom_key(k.id),
                index: k.id,
                nonmajor: u32::from(k.independent || k.id >= KINGDOM_BITS),
                shared_mask: 0,
                supply_mask: 0,
            })
            .collect();
        self.parts.kingdoms = ks.clone();
        for (i, k) in ks.iter().enumerate() {
            if let Some((shared, supply)) = self.with(world, |_, c| aa_zones::tile::kingdom_masks(c, k)) {
                self.parts.kingdoms[i].shared_mask = shared;
                self.parts.kingdoms[i].supply_mask = supply;
            }
        }
    }

    /// `0x5709D0` for every member: the `+0x80` target and byte `+0x1C`.
    fn refresh_membership(&mut self, world: &World) {
        let mut member = vec![None; world.actors.len()];
        for (o, g) in world.orgs.iter().enumerate() {
            if !g.alive {
                continue;
            }
            let o = o as OrgId;
            let at = crate::supply::org_position(world, o);
            let leader = leader_of(world, o);
            let control = self.parts.defs.get(g.def as usize).map_or(0.0, |d| d.control);
            for &m in &g.members {
                let Some(a) = world.actors.get(m as usize).filter(|a| a.alive && !a.removed) else { continue };
                let inside = Some(m) == leader || a.pos.truncate().distance_squared(at) <= control * control;
                member[m as usize] = Some((o, inside));
            }
        }
        self.parts.member = member;
    }

    /// The start of the battle (`0x551F90` EXPLORED: every tile explored and
    /// the start's static neutral actors explored, `0x54F400`; `0x551FD0`
    /// NONE: every tile and actor explored and visible, fog off; FULL:
    /// black).
    pub fn start_fog(&mut self, world: &World, explored: bool, visible: bool, explored_actor: impl Fn(ActorId) -> bool) {
        self.observe(world);
        let word = if explored { EXPLORED_ALL } else { 0 } | if visible { VISIBLE_ALL } else { 0 };
        for t in self.map.tiles.iter_mut() {
            t.fog = word;
        }
        self.parts.mirror.fill(word);
        self.parts.fog_changes += 1;
        self.parts.fog_on = !(explored && visible);
        for i in 0..world.actors.len() {
            if self.parts.actors[i].gone || world.actors[i].removed {
                continue;
            }
            let key = actor_key(i as ActorId);
            if visible {
                self.with(world, |_, c| {
                    detection::set_all_explored(c, key)?;
                    detection::set_all_visible(c, key)
                });
            } else if explored && explored_actor(i as ActorId) {
                self.with(world, |_, c| detection::set_all_explored(c, key));
            }
        }
        if visible {
            for o in 0..world.orgs.len() {
                if world.orgs[o].alive {
                    let key = org_key(o as OrgId);
                    self.with(world, |_, c| {
                        detection::set_all_explored(c, key)?;
                        detection::set_all_visible(c, key)
                    });
                }
            }
        }
    }

    /// The tile changes since the last call (`0x4D6350` per actor whose tile
    /// or footprint changed, placement of new actors and organizations) and
    /// the destruction of removed ones.
    pub fn observe(&mut self, world: &World) {
        let started = std::time::Instant::now();
        self.observe_inner(world);
        self.parts.stats.observe_us = started.elapsed().as_secs_f32() * 1.0e6;
    }

    fn observe_inner(&mut self, world: &World) {
        self.grow(world);
        self.refresh_kingdoms(world);
        self.refresh_membership(world);
        for i in 0..world.actors.len() {
            let id = i as ActorId;
            let key = actor_key(id);
            if self.parts.actors[i].gone {
                continue;
            }
            let a = &world.actors[i];
            if a.removed {
                self.destroy(world, key);
                continue;
            }
            if let Some(old) = self.parts.actors[i].owner
                && old != a.kingdom
            {
                self.change_kingdom(world, key, old);
            }
            self.parts.actors[i].owner = Some(a.kingdom);
            // the tiles only depend on the pose: skip unmoved actors
            let pose = [a.pos.x.to_bits(), a.pos.y.to_bits(), a.facing.to_bits()];
            if self.parts.actors[i].pose == Some(pose) {
                continue;
            }
            self.parts.actors[i].pose = Some(pose);
            let d = self.def_of(world, Key::Actor(id));
            let s = &self.parts.actors[i];
            if d.half == Vec2::ZERO && s.tile.is_some() && s.tile == Some(tile_of(&self.parts.geometry, a.pos.truncate())) {
                continue;
            }
            let (tile, tiles) = occupancy(&self.parts.geometry, &d, a.pos.truncate(), a.facing);
            let fresh = s.tile.is_none();
            if fresh || s.tile != Some(tile) || s.tiles != tiles {
                if fresh && d.component {
                    self.parts.actors[i].comp = Some(DetectionComponent { actor: key, reveal_mask: 0, zod_radius_bits: d.zod.to_bits() });
                }
                self.tile_change(world, key, tile, tiles, fresh);
            }
        }
        for o in 0..world.orgs.len() {
            let id = o as OrgId;
            let key = org_key(id);
            if self.parts.orgs[o].gone {
                continue;
            }
            if !world.orgs[o].alive {
                self.destroy(world, key);
                continue;
            }
            let owner = Some(world.orgs[o].kingdom);
            if let Some(old) = self.parts.orgs[o].owner
                && old != owner
            {
                self.change_kingdom(world, key, old);
            }
            self.parts.orgs[o].owner = Some(owner);
            let p = crate::supply::org_position(world, id);
            let s = &self.parts.orgs[o];
            let fresh = s.tile.is_none();
            if fresh || s.tile != Some(tile_of(&self.parts.geometry, p)) {
                let (tile, tiles) = occupancy(&self.parts.geometry, &DefDetection::default(), p, 0.0);
                let d = self.def_of(world, Key::Org(id));
                if fresh && d.component {
                    self.parts.orgs[o].comp = Some(DetectionComponent { actor: key, reveal_mask: 0, zod_radius_bits: d.zod.to_bits() });
                }
                self.tile_change(world, key, tile, tiles, fresh);
            }
        }
    }

    /// The detection component ticks (`0x56F140`) of this battle tick.
    pub fn detection_tick(&mut self, world: &World) {
        let started = std::time::Instant::now();
        self.detection_inner(world);
        self.parts.stats.detection_us = started.elapsed().as_secs_f32() * 1.0e6;
    }

    fn detection_inner(&mut self, world: &World) {
        self.parts.engaged = engagements(world);
        self.parts.reveal_bytes = world
            .kingdoms
            .iter()
            .map(|k| if crate::special_ops::kingdom_has_flag(world, k.id, "REVEAL_ENEMY_RZOD") { 0x80 } else { 0 })
            .collect();
        for i in 0..world.actors.len().min(self.parts.actors.len()) {
            let s = &self.parts.actors[i];
            if s.gone || s.comp.is_none() || s.tile.is_none() {
                continue;
            }
            let key = actor_key(i as ActorId);
            self.parts.stats.component_ticks += 1;
            self.with(world, |map, c| component::tick(map, c, key, PC));
        }
        for o in 0..world.orgs.len().min(self.parts.orgs.len()) {
            let s = &self.parts.orgs[o];
            if s.gone || s.comp.is_none() || s.tile.is_none() {
                continue;
            }
            let key = org_key(o as OrgId);
            self.parts.stats.component_ticks += 1;
            self.with(world, |map, c| component::tick(map, c, key, PC));
        }
    }

    fn slot_mut(&mut self, key: ActorKey) -> Option<&mut Slot> {
        match decode(key).ok()? {
            Key::Actor(a) => self.parts.actors.get_mut(a as usize),
            Key::Org(o) => self.parts.orgs.get_mut(o as usize),
        }
    }

    /// `0x4D6350`: the actor moved to another tile (or was placed).
    fn tile_change(&mut self, world: &World, key: ActorKey, tile: (i32, i32), tiles: Vec<usize>, fresh: bool) {
        self.parts.stats.tile_changes += 1;
        let Some(s) = self.slot_mut(key) else { return };
        let old = std::mem::take(&mut s.tiles);
        let supply = s.supply_active();
        for t in old {
            if let Some(l) = self.parts.lists.get_mut(t)
                && let Some(i) = l.iter().rposition(|k| *k == key)
            {
                l.remove(i);
            }
        }
        for &t in &tiles {
            if let Some(l) = self.parts.lists.get_mut(t) {
                l.push(key);
            }
        }
        if let Some(s) = self.slot_mut(key) {
            s.tiles = tiles;
            s.tile = Some(tile);
        }
        self.with(world, |map, c| detection::tile_changed(map, c, key));
        if !fresh {
            if supply && let Ok(Key::Actor(a)) = decode(key) {
                self.parts.supply_changed.insert(a);
            }
            self.with(world, |map, c| component::move_actor_zones(map, c, key, PC));
        }
    }

    /// The zone part of an ownership change (`0x4D8450`): `0x4D5DE0` keeps
    /// the radius of every active zone and releases it (`0x514CE0`, still
    /// under the old owner), the owner changes, then `0x4CFD30` puts every
    /// zone whose kept radius is `>= 0` back on its tile (`0x515970`) for
    /// the new owner.
    fn change_kingdom(&mut self, world: &World, key: ActorKey, old: Option<KingdomId>) {
        let Some(s) = self.slot_mut(key) else { return };
        let n = s.zones.slots.len();
        let mut kept = vec![zone::INACTIVE; n];
        self.parts.owner_override = Some((key, old));
        for (i, r) in kept.iter_mut().enumerate() {
            let Some(z) = self.slot_mut(key).and_then(|s| s.zone(i as u32)).filter(|z| z.active()) else { continue };
            *r = z.radius_bits;
            self.parts.stats.releases += 1;
            if i as u32 == ZOS
                && let Ok(Key::Actor(a)) = decode(key)
            {
                self.parts.supply_changed.insert(a);
            }
            self.with(world, |map, c| c.release(map, key, i as u32));
        }
        self.parts.owner_override = None;
        for (i, &r) in kept.iter().enumerate() {
            // DAT_009f32dc (0.0) <= kept radius
            if !(f32::from_bits(r) >= 0.0) {
                continue;
            }
            self.with(world, |map, c| {
                let mut z = c.zone_at(key, i as u32)?;
                let (x, y) = (z.x, z.y);
                let res = zone::move_to_tile(map, c, &mut z, x, y, r, PC);
                c.set_zone(key, i as u32, z)?;
                res
            });
        }
    }

    /// A destroyed actor: release every zone (`0x514CE0`), leave the tiles.
    fn destroy(&mut self, world: &World, key: ActorKey) {
        let Some(s) = self.slot_mut(key) else { return };
        s.gone = true;
        let n = s.zones.slots.len();
        for i in 0..n as u32 {
            let active = self.slot_mut(key).and_then(|s| s.zone(i)).is_some_and(|z| z.active());
            if !active {
                continue;
            }
            self.parts.stats.releases += 1;
            if i == ZOS
                && let Ok(Key::Actor(a)) = decode(key)
            {
                self.parts.supply_changed.insert(a);
            }
            self.with(world, |map, c| c.release(map, key, i));
        }
        let tiles = self.slot_mut(key).map(|s| std::mem::take(&mut s.tiles)).unwrap_or_default();
        for t in tiles {
            if let Some(l) = self.parts.lists.get_mut(t) {
                l.retain(|k| *k != key);
            }
        }
    }

    /// The supply component zones (`0x58B0E0` / `0x58AE10` tail) of this
    /// tick: `providing` lists the live providers for which `0x58ACE0`
    /// holds, with their radius (component +0x10, `0x58AA20`). Every other
    /// actor with an active supply zone stops providing.
    pub fn supply_tick(&mut self, world: &World, providing: &[(ActorId, f32)]) {
        let started = std::time::Instant::now();
        self.supply_inner(world, providing);
        self.parts.stats.supply_us = started.elapsed().as_secs_f32() * 1.0e6;
    }

    fn supply_inner(&mut self, world: &World, providing: &[(ActorId, f32)]) {
        self.grow(world);
        let on: BTreeMap<ActorId, f32> = providing.iter().copied().collect();
        for i in 0..world.actors.len() {
            let id = i as ActorId;
            let key = actor_key(id);
            let s = &mut self.parts.actors[i];
            if s.gone || s.tile.is_none() {
                continue;
            }
            let provides = on.get(&id).copied();
            if let Some(r) = provides
                && s.supply_radius != Some(r.to_bits())
            {
                // 0x58AA20: the radius (re)computed marks the zone dirty
                s.supply_radius = Some(r.to_bits());
                s.supply_dirty = true;
            }
            let active = s.supply_active();
            if provides.is_none() && !active {
                continue;
            }
            let dirty = s.supply_dirty;
            let radius = s.supply_radius.unwrap_or(zone::INACTIVE);
            // 0x58B0E0: CreateZone(0) when the vector has no zone 0
            if s.zone(ZOS).is_none() {
                self.with(world, |_, c| component::create_zone(c, key, ZOS));
            }
            let mv = match (provides.is_some(), active) {
                (true, false) => true,
                (false, true) => {
                    self.parts.stats.releases += 1;
                    self.parts.supply_changed.insert(id);
                    self.with(world, |map, c| c.release(map, key, ZOS));
                    dirty
                }
                _ => dirty,
            };
            if mv {
                self.parts.stats.supply_moves += 1;
                self.parts.supply_changed.insert(id);
                let pos = world.actors[i].pos.truncate();
                let bits = [pos.x.to_bits(), pos.y.to_bits()];
                self.with(world, |map, c| {
                    let mut z = c.zone_at(key, ZOS)?;
                    let r = zone::move_to_position(map, c, &mut z, bits, radius, PC);
                    c.set_zone(key, ZOS, z)?;
                    r
                });
                self.parts.actors[i].supply_dirty = false;
            }
        }
    }

    /// Re-read the tiles of every provider whose supply zone changed: the
    /// tiles whose zone set holds its type-0 entry (`0x54EF40`), searched
    /// over its previous tiles and the square the zone can reach (a flood
    /// step costs at least one tile edge).
    pub fn refresh_supply_tiles(&mut self) {
        let changed = std::mem::take(&mut self.parts.supply_changed);
        let g = self.parts.geometry;
        let tile = f32::from_bits(g.tile_size_bits);
        for a in changed {
            let key = actor_key(a);
            let Some(z) = self.parts.actors.get(a as usize).and_then(|s| s.zone(ZOS)).filter(|z| z.active()) else {
                self.supply_tiles.remove(&a);
                continue;
            };
            let reach = (f32::from_bits(z.radius_bits) / tile).ceil().max(0.0) as i32 + 1;
            let mut cache: HashMap<SetRef, bool> = HashMap::new();
            let map = &self.map;
            let mut holds = |t: usize| -> bool {
                let Some(set) = map.tiles[t].set else { return false };
                *cache.entry(set).or_insert_with(|| aa_zones::query::contains(&map.pool, set, key, 1).unwrap_or(false))
            };
            let mut out: BTreeSet<u32> = BTreeSet::new();
            for &t in self.supply_tiles.get(&a).map_or(&[][..], |v| v.as_slice()) {
                if holds(t as usize) {
                    out.insert(t);
                }
            }
            for y in (z.y - reach).max(0)..=(z.y + reach).min(g.height - 1) {
                for x in (z.x - reach).max(0)..=(z.x + reach).min(g.width - 1) {
                    let t = (y * g.width + x) as usize;
                    if holds(t) {
                        out.insert(t as u32);
                    }
                }
            }
            self.supply_tiles.insert(a, Arc::new(out.into_iter().collect()));
        }
    }

    /// The tile holds `provider`'s supply zone (its type-0 tile set entry).
    pub fn covers(&self, provider: ActorId, t: usize) -> bool {
        let Some(set) = self.map.tiles.get(t).and_then(|x| x.set) else { return false };
        aa_zones::query::contains(&self.map.pool, set, actor_key(provider), 1).unwrap_or(false)
    }

    /// A kingdom's fields (None for an unknown id).
    pub fn kingdom(&self, k: KingdomId) -> Option<&Kingdom> {
        self.parts.kingdoms.get(k as usize)
    }

    /// The actor's detection word (0 before it is placed).
    pub fn actor_word(&self, a: ActorId) -> u32 {
        self.parts.actors.get(a as usize).map_or(0, |s| s.word)
    }

    /// The organization's own detection word.
    pub fn org_word(&self, o: OrgId) -> u32 {
        self.parts.orgs.get(o as usize).map_or(0, |s| s.word)
    }

    /// `0x4D3B80` (`deep`) for a shared-vision mask: the organization word,
    /// or a living member's word, visible.
    pub fn org_visible(&self, world: &World, o: OrgId, mask: u32) -> bool {
        if self.org_word(o) & mask & VISIBLE_ALL != 0 {
            return true;
        }
        world.orgs.get(o as usize).is_some_and(|g| {
            g.members
                .iter()
                .any(|&m| world.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed) && self.actor_word(m) & mask & VISIBLE_ALL != 0)
        })
    }
}

fn log_refusal(text: &str) {
    // the battle keeps running; the refusal is counted in the stats
    #[cfg(debug_assertions)]
    eprintln!("aa-zones refused: {text}");
    #[cfg(not(debug_assertions))]
    let _ = text;
}

/// Actor `+0x100` bit 21 (0x200000, aa-production `construction.rs`): under
/// construction. A building unpacking from its truck is built over its
/// unpacking length.
fn under_construction(world: &World, a: ActorId) -> bool {
    world.books.actor(a).and_then(|b| b.deploy.as_ref()).is_some_and(|d| d.kind == crate::books::DeployKind::Unpacking)
}

/// The member whose position the organization takes (57C800: the leader
/// slot when alive, else the first living member).
fn leader_of(world: &World, o: OrgId) -> Option<ActorId> {
    let org = &world.orgs[o as usize];
    let live = |m: ActorId| world.actors.get(m as usize).is_some_and(|x| x.alive && !x.removed);
    let leader = world.data.def(org.def).leader_slot;
    org.members
        .iter()
        .copied()
        .find(|&m| live(m) && leader.is_some() && world.actors[m as usize].slot == leader)
        .or_else(|| org.members.iter().copied().find(|&m| live(m)))
}

/// The tile under a point (actor +0x44 / +0x48).
fn tile_of(g: &MapGeometry, pos: Vec2) -> (i32, i32) {
    let tile = f32::from_bits(g.tile_size_bits);
    ((pos.x / tile).floor() as i32, (pos.y / tile).floor() as i32)
}

/// The tile under an actor (actor +0x44 / +0x48) and the tiles it occupies
/// (a structure's footprint rectangle, a unit's own tile), row-major.
pub fn occupancy(g: &MapGeometry, d: &DefDetection, pos: Vec2, facing: f32) -> ((i32, i32), Vec<usize>) {
    let tile = f32::from_bits(g.tile_size_bits);
    let index = |x: i32, y: i32| (x >= 0 && y >= 0 && x < g.width && y < g.height).then(|| (y * g.width + x) as usize);
    let at = tile_of(g, pos);
    let mut out = Vec::new();
    if d.half == Vec2::ZERO {
        out.extend(index(at.0, at.1));
        return (at, out);
    }
    let (s, c) = facing.sin_cos();
    let ext = Vec2::new((c * d.half.x).abs() + (s * d.half.y).abs(), (s * d.half.x).abs() + (c * d.half.y).abs());
    let lo = ((pos - ext) / tile).floor();
    let hi = ((pos + ext) / tile).floor();
    for y in lo.y as i32..=hi.y as i32 {
        for x in lo.x as i32..=hi.x as i32 {
            out.extend(index(x, y));
        }
    }
    (at, out)
}

/// `0x56EDC0`: per actor, the kingdom bits of the actors engaging it.
fn engagements(world: &World) -> Vec<u32> {
    let mut out = vec![0u32; world.actors.len()];
    for a in world.actors.iter().filter(|a| a.alive && !a.removed) {
        let (Some(t), Some(k)) = (a.combat.target, a.kingdom) else { continue };
        if k >= KINGDOM_BITS {
            continue;
        }
        if let Some(m) = out.get_mut(t as usize) {
            *m |= 1u32 << k;
        }
    }
    out
}

/// The trait context: the world (read) and the engine parts (write).
pub struct Ctx<'a> {
    pub w: &'a World,
    pub p: &'a mut Parts,
}

impl Ctx<'_> {
    fn slot(&self, key: ActorKey) -> Result<&Slot> {
        match decode(key)? {
            Key::Actor(a) => self.p.actors.get(a as usize),
            Key::Org(o) => self.p.orgs.get(o as usize),
        }
        .ok_or_else(|| Error::World(format!("unknown actor key {key:#x}")))
    }
    fn slot_mut(&mut self, key: ActorKey) -> Result<&mut Slot> {
        match decode(key)? {
            Key::Actor(a) => self.p.actors.get_mut(a as usize),
            Key::Org(o) => self.p.orgs.get_mut(o as usize),
        }
        .ok_or_else(|| Error::World(format!("unknown actor key {key:#x}")))
    }
    fn zone_at(&self, key: ActorKey, index: u32) -> Result<Zone> {
        self.slot(key)?.zone(index).ok_or(Error::NativeNull("actor zone slot"))
    }
    fn set_zone(&mut self, key: ActorKey, index: u32, z: Zone) -> Result<()> {
        if let Some(slot) = self.slot_mut(key)?.zones.slots.get_mut(index as usize) {
            *slot = Some(z);
        }
        Ok(())
    }
    /// `0x514CE0` on the actor's zone `index`.
    fn release(&mut self, map: &mut ZoneMap, key: ActorKey, index: u32) -> Result<()> {
        let mut z = self.zone_at(key, index)?;
        let r = zone::release(map, self, &mut z, PC);
        self.set_zone(key, index, z)?;
        r
    }
    fn kingdom_of_key(&self, key: u32) -> Option<KingdomId> {
        (key >= KINGDOM).then(|| key - KINGDOM)
    }
    fn owner_id(&self, key: ActorKey) -> Result<Option<KingdomId>> {
        if let Some((k, owner)) = self.p.owner_override
            && k == key
        {
            return Ok(owner);
        }
        Ok(match decode(key)? {
            Key::Actor(a) => self.w.actors.get(a as usize).and_then(|x| x.kingdom),
            Key::Org(o) => self.w.orgs.get(o as usize).map(|x| x.kingdom),
        })
    }
    fn relation(&self, a: Option<KingdomId>, b: Option<KingdomId>) -> u8 {
        self.w.books.relation(&self.w.kingdoms, a, b)
    }
    fn def(&self, key: ActorKey) -> Result<DefDetection> {
        let d = match decode(key)? {
            Key::Actor(a) => self.w.actors.get(a as usize).map(|x| x.def),
            Key::Org(o) => self.w.orgs.get(o as usize).map(|x| x.def),
        };
        Ok(d.and_then(|d| self.p.defs.get(d as usize).copied()).unwrap_or_default())
    }
    fn dead(&self, key: ActorKey) -> Result<bool> {
        Ok(match decode(key)? {
            Key::Actor(a) => self.w.actors.get(a as usize).is_none_or(|x| !x.alive || x.removed),
            Key::Org(o) => self.w.orgs.get(o as usize).is_none_or(|x| !x.alive),
        })
    }
    /// The organization an actor is a member of, with byte +0x1C.
    fn membership(&self, key: ActorKey) -> Result<Option<(OrgId, bool)>> {
        Ok(match decode(key)? {
            Key::Actor(a) => self.p.member.get(a as usize).copied().flatten(),
            Key::Org(_) => None,
        })
    }
    fn members(&self, key: ActorKey, holes: bool) -> Result<Option<Vec<ActorKey>>> {
        Ok(match decode(key)? {
            Key::Org(o) => self.w.orgs.get(o as usize).map(|g| {
                g.members
                    .iter()
                    .filter_map(|&m| {
                        let live = self.w.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed);
                        if live { Some(actor_key(m)) } else { holes.then_some(0) }
                    })
                    .collect()
            }),
            Key::Actor(_) => None,
        })
    }
}

impl ZoneActors for Ctx<'_> {
    fn owner(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(self.owner_id(actor)?.map_or(0, kingdom_key))
    }
    fn allied(&mut self, actor: ActorKey, kingdom: u32) -> Result<bool> {
        let r = self.relation(self.owner_id(actor)?, self.kingdom_of_key(kingdom));
        Ok(r == SELF || r == FRIEND)
    }
    fn enemy(&mut self, actor: ActorKey, kingdom: u32) -> Result<bool> {
        Ok(self.relation(self.owner_id(actor)?, self.kingdom_of_key(kingdom)) == ENEMY)
    }
    fn zone_slot(&mut self, actor: ActorKey, index: u32) -> Result<u32> {
        Ok(self.slot(actor)?.zone(index).map_or(0, |z| z.key))
    }
    fn word4(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(match decode(actor)? {
            Key::Actor(a) => self.w.actors.get(a as usize).map_or(0, |x| x.def + 1),
            Key::Org(o) => self.w.orgs.get(o as usize).map_or(0, |x| x.def + 1),
        })
    }
    fn component_b0(&mut self, _actor: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn reveal_mask(&mut self, actor: ActorKey) -> Result<Option<u32>> {
        Ok(self.slot(actor)?.comp.map(|c| c.reveal_mask))
    }
}

impl VisitWorld for Ctx<'_> {
    fn kingdom(&mut self, key: u32) -> Result<Kingdom> {
        let k = self.kingdom_of_key(key).ok_or(Error::NativeNull("kingdom key"))?;
        self.p.kingdoms.get(k as usize).copied().ok_or_else(|| Error::World(format!("unknown kingdom {k}")))
    }
    fn kingdom_slot(&mut self, index: u32) -> Result<u32> {
        // GWorld+0x154[index] (0 past the count)
        Ok(self.p.kingdoms.get(index as usize).map_or(0, |k| k.key))
    }
}

impl ZoneWorld for Ctx<'_> {
    fn fog_enabled(&mut self) -> Result<bool> {
        Ok(self.p.fog_on)
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        // the SAI region counters are not ported; the hooks keep the fog
        // word mirror (every fog bit change of reveal/hide/restore reports)
        Ok(true)
    }
    fn sai(&mut self, hook: SaiHook, tile: usize, kingdom: u32) -> Result<()> {
        let Some(k) = self.kingdom_of_key(kingdom).and_then(|k| self.p.kingdoms.get(k as usize).copied()) else { return Ok(()) };
        if k.nonmajor != 0 {
            return Ok(());
        }
        let Some(w) = self.p.mirror.get_mut(tile) else { return Ok(()) };
        let b = |i: u32| 1u32.wrapping_shl(i);
        match hook {
            SaiHook::ExploredInc => *w |= b(k.index),
            SaiHook::ExploredDec => *w &= !b(k.index),
            SaiHook::VisibleInc => *w |= b(k.index.wrapping_add(aa_zones::tile::VISIBLE_SHIFT)),
            SaiHook::VisibleDec => *w &= !b(k.index.wrapping_add(aa_zones::tile::VISIBLE_SHIFT)),
            SaiHook::OccupiedInc | SaiHook::OccupiedDec => return Ok(()),
        }
        self.p.fog_changes += 1;
        Ok(())
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(self.p.kingdoms.iter().map(|k| k.key).collect())
    }
    fn kingdom_count(&mut self) -> Result<usize> {
        Ok(self.p.kingdoms.len())
    }
    fn kingdom_at(&mut self, i: usize) -> Result<u32> {
        self.p.kingdoms.get(i).map(|k| k.key).ok_or_else(|| Error::World(format!("kingdom list index {i}")))
    }
    fn tile_actors(&mut self, tile: usize) -> Result<Vec<ActorKey>> {
        Ok(self.p.lists.get(tile).map(|l| l.iter().rev().copied().collect()).unwrap_or_default())
    }
    fn actor_reveal(&mut self, actor: ActorKey, kingdom: u32, explored: bool, visible: bool) -> Result<()> {
        detection::reveal(self, actor, kingdom, explored, visible)
    }
    fn actor_hide(&mut self, actor: ActorKey, kingdom: u32) -> Result<()> {
        let g = self.p.geometry;
        detection::hide(self, &g, actor, kingdom, PC)
    }
    fn actor_unexplore(&mut self, actor: ActorKey, kingdom: u32) -> Result<()> {
        detection::unexplore(self, actor, kingdom)
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn actor_b4(&mut self, actor: ActorKey) -> Result<bool> {
        Ok(!self.slot(actor)?.gone)
    }
    fn actor_multi_tile(&mut self, actor: ActorKey) -> Result<bool> {
        Ok(self.slot(actor)?.tiles.len() > 1)
    }
    fn actor_tile(&mut self, actor: ActorKey) -> Result<i64> {
        let (x, y) = self.slot(actor)?.tile.unwrap_or((0, 0));
        Ok(i64::from(y) * i64::from(self.p.geometry.width) + i64::from(x))
    }
    fn actor_flags(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(u32::from(self.dead(actor)?))
    }
    fn actor_redirect(&mut self, actor: ActorKey) -> Result<Option<ActorKey>> {
        Ok(self.membership(actor)?.filter(|m| m.1).map(|m| org_key(m.0)))
    }
    fn component(&mut self, actor: ActorKey, offset: u32) -> Result<u32> {
        Ok(match offset {
            // actor +0x100 flags: bit 21 (0x200000) under construction, a
            // building still unpacking from its truck
            0x100 => match decode(actor)? {
                Key::Actor(a) if under_construction(self.w, a) => 0x20_0000,
                _ => 0,
            },
            _ => 0,
        })
    }
    fn component_effect(&mut self, effect: ComponentEffect, _component: u32, _target: ActorKey) -> Result<()> {
        Err(Error::World(format!("component effect {effect:?} is not bound")))
    }
}

impl FloodWorld for Ctx<'_> {
    fn tile_flags(&mut self, tile: usize) -> Result<u32> {
        let f = &self.w.map.flags;
        if f.is_empty() && tile < self.p.lists.len() {
            return Ok(0);
        }
        f.get(tile).copied().ok_or(Error::TileOutside(tile as i64))
    }
    fn tile_blocker(&mut self, _tile: usize) -> Result<u32> {
        Ok(0)
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        Ok(true)
    }
}

impl ZoneObjectWorld for Ctx<'_> {
    fn solver_present(&mut self, _release: bool) -> Result<bool> {
        Ok(true)
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        Ok(self.p.fast_zones)
    }
    fn presentation(&mut self, _owner: u32, _x: u32, _y: u32, _r: u32, _flags: u16) -> Result<()> {
        Ok(())
    }
    fn log_implicit_clear(&mut self, _actor_id: u32, _flags: u16) -> Result<()> {
        Ok(())
    }
    fn actor_id(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(actor)
    }
}

impl DetectionComponentWorld for Ctx<'_> {
    fn detection_component(&mut self, actor: ActorKey) -> Result<&mut DetectionComponent> {
        self.slot_mut(actor)?.comp.as_mut().ok_or(Error::NativeNull("actor+0xB8 detection component"))
    }
    fn zones(&mut self, actor: ActorKey) -> Result<&mut ActorZones> {
        Ok(&mut self.slot_mut(actor)?.zones)
    }
    fn new_zone_key(&mut self, _actor: ActorKey, _index: u32) -> Result<u32> {
        self.p.next_zone_key = self.p.next_zone_key.wrapping_add(0x18);
        Ok(self.p.next_zone_key)
    }
    fn log_unexpected_zone(&mut self, _actor: ActorKey, _index: u32) -> Result<()> {
        Ok(())
    }
    fn actor_position(&mut self, actor: ActorKey) -> Result<[u32; 2]> {
        let p = match decode(actor)? {
            Key::Actor(a) => self.w.actors.get(a as usize).map(|x| x.pos.truncate()),
            Key::Org(o) => ((o as usize) < self.w.orgs.len()).then(|| crate::supply::org_position(self.w, o)),
        }
        .ok_or_else(|| Error::World("position of an unknown actor".into()))?;
        Ok([p.x.to_bits(), p.y.to_bits()])
    }
    fn actor_virtual_100(&mut self, _actor: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn actor_virtual_158(&mut self, _actor: ActorKey) -> Result<bool> {
        Ok(false)
    }
    fn actor_80(&mut self, actor: ActorKey) -> Result<(u32, u32, u8)> {
        Ok(match self.membership(actor)? {
            Some((o, inside)) => (1, org_key(o), u8::from(inside)),
            None => (0, 0, 0),
        })
    }
    fn data_word(&mut self, actor: ActorKey, offset: u32) -> Result<u32> {
        let d = self.def(actor)?;
        Ok(match offset {
            0x1cc => d.zod.to_bits(),
            0x1d0 => d.rzod.to_bits(),
            _ => 0,
        })
    }
    fn modified_detection_range(&mut self, _actor: ActorKey, _context: u32) -> Result<u32> {
        Err(Error::World("detection range modifier context (0x501C30) is not bound".into()))
    }
    fn reveal_sources(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(match decode(actor)? {
            Key::Actor(a) => self.p.engaged.get(a as usize).copied().unwrap_or(0),
            Key::Org(_) => 0,
        })
    }
    fn organization_slots(&mut self, actor: ActorKey) -> Result<Option<Vec<ActorKey>>> {
        self.members(actor, true)
    }
    fn kingdom_enemy(&mut self, kingdom: u32, owner: u32) -> Result<bool> {
        Ok(self.relation(self.kingdom_of_key(kingdom), self.kingdom_of_key(owner)) == ENEMY)
    }
    fn kingdom_reveal_byte(&mut self, kingdom: u32) -> Result<Option<u8>> {
        let Some(k) = self.kingdom_of_key(kingdom) else { return Ok(None) };
        Ok(Some(match self.p.reveal_bytes.get(k as usize) {
            Some(&b) => b,
            None => {
                if crate::special_ops::kingdom_has_flag(self.w, k, "REVEAL_ENEMY_RZOD") {
                    0x80
                } else {
                    0
                }
            }
        }))
    }
    fn schedule_tick(&mut self, _actor: ActorKey, _delay_bits: u32) -> Result<()> {
        Ok(())
    }
}

impl ActorDetectionWorld for Ctx<'_> {
    fn detection_word(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(self.slot(actor)?.word)
    }
    fn set_detection_word(&mut self, actor: ActorKey, word: u32) -> Result<()> {
        self.slot_mut(actor)?.word = word;
        Ok(())
    }
    fn actor_container(&mut self, _actor: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn actor_size_bits(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(self.def(actor)?.size.to_bits())
    }
    fn shape_tiles(&mut self, actor: ActorKey) -> Result<Vec<usize>> {
        Ok(self.slot(actor)?.tiles.clone())
    }
    fn tile_fog(&mut self, tile: usize) -> Result<u32> {
        self.p.mirror.get(tile).copied().ok_or(Error::TileOutside(tile as i64))
    }
    fn occupancy_tiles(&mut self, actor: ActorKey) -> Result<Vec<usize>> {
        Ok(self.slot(actor)?.tiles.clone())
    }
    fn keeps_memory(&mut self, actor: ActorKey) -> Result<bool> {
        Ok(matches!(decode(actor)?, Key::Actor(_)) && self.def(actor)?.keeps_memory)
    }
    fn memory_eligible(&mut self, _actor: ActorKey) -> Result<bool> {
        Ok(false)
    }
    fn memory_slot(&mut self, actor: ActorKey, index: u32) -> Result<u32> {
        let Key::Actor(a) = decode(actor)? else { return Ok(0) };
        Ok(u32::from(self.p.memory.get(index as usize).is_some_and(|m| m.contains_key(&a))))
    }
    fn memory_slots(&mut self, actor: ActorKey) -> Result<Vec<u32>> {
        let Key::Actor(a) = decode(actor)? else { return Ok(Vec::new()) };
        let n = self.p.kingdoms.len().min(KINGDOM_BITS as usize);
        Ok((0..n).map(|i| u32::from(self.p.memory[i].contains_key(&a))).collect())
    }
    fn create_memory(&mut self, actor: ActorKey, kingdom: u32) -> Result<()> {
        let Key::Actor(a) = decode(actor)? else { return Ok(()) };
        let Some(k) = self.kingdom_of_key(kingdom).filter(|k| *k < KINGDOM_BITS) else { return Ok(()) };
        if let Some(x) = self.w.actors.get(a as usize) {
            let m = Memory { def: x.def, pos: x.pos, facing: x.facing, kingdom: x.kingdom, health: x.health };
            self.p.memory[k as usize].insert(a, m);
        }
        Ok(())
    }
    fn delete_memory(&mut self, actor: ActorKey, kingdom: u32) -> Result<()> {
        let Key::Actor(a) = decode(actor)? else { return Ok(()) };
        if let Some(k) = self.kingdom_of_key(kingdom).filter(|k| *k < KINGDOM_BITS) {
            self.p.memory[k as usize].remove(&a);
        }
        Ok(())
    }
    fn clear_memory(&mut self, actor: ActorKey) -> Result<()> {
        let Key::Actor(a) = decode(actor)? else { return Ok(()) };
        for m in self.p.memory.iter_mut() {
            m.remove(&a);
        }
        Ok(())
    }
    fn announces_first_sighting(&mut self, _actor: ActorKey) -> Result<bool> {
        Ok(false)
    }
    fn first_sighting(&mut self, _actor: ActorKey, _kingdom: u32) -> Result<()> {
        Ok(())
    }
    fn carrier(&mut self, _actor: ActorKey) -> Result<u32> {
        Ok(0)
    }
    fn carrier_visible(&mut self, _carrier: u32, _kingdom: u32) -> Result<bool> {
        Ok(false)
    }
    fn organization_members(&mut self, actor: ActorKey) -> Result<Option<Vec<ActorKey>>> {
        self.members(actor, false)
    }
    fn component_80_target(&mut self, actor: ActorKey) -> Result<u32> {
        Ok(self.membership(actor)?.map_or(0, |m| org_key(m.0)))
    }
    fn virtual_seen(&mut self, actor: ActorKey, kingdom: u32, deep: bool, explored: bool) -> Result<bool> {
        if explored { detection::explored_by(self, actor, kingdom, deep) } else { detection::visible_to(self, actor, kingdom, deep) }
    }
}

impl AllianceWorld for Ctx<'_> {
    fn kingdom_allied(&mut self, kingdom: u32, other: u32) -> Result<bool> {
        let r = self.relation(self.kingdom_of_key(kingdom), self.kingdom_of_key(other));
        Ok(r == SELF || r == FRIEND)
    }
}
