//! Fog of war: per-kingdom tile visibility from the actors' detection
//! zones, the actors' detection words and the detection memory (ghost
//! records of static actors seen earlier). Owner: fog (UI developer).
//!
//! Behaviour-faithful reading of the original (the exact ports in aa-zones
//! `tile`, `zone`, `solver`, `component`, `detection`):
//!
//! * The FogOfWar world rule (Data/Game/world_rules_aa.tgi, enum NONE 0,
//!   EXPLORED 1 = the default, FULL 2) is GWorld `[[B39AB0]+0x108]`. At the
//!   world start `0x4FDCE0` runs `0x551FD0` for NONE (every tile explored
//!   and visible to every kingdom, every actor on a tile gets `+0x94` and
//!   `+0x98`: explored and visible) or `0x551F90` for EXPLORED (every tile
//!   explored, the static actors explored, `0x54F400`); FULL starts black.
//!   With NONE the fog is off (`0x54F3C0`/`0x4EA1D0` answer visible).
//! * Tile fog word (tile `+0x14`): bit k = explored by kingdom k (sticky),
//!   bit k + 16 = visible to kingdom k (cleared when no detecting zone of k
//!   covers the tile, `0x54F5E0`). Non-major (independent) kingdoms are never
//!   marked and always see.
//! * Detection component (actor `+0xB8`, tick `0x56F140`): a ZOD (zone type
//!   6) of the finalized ActorData `+0x1CC` radius (the largest of
//!   detection_range, the organization control_range, supply_range and the
//!   denizen guard ranges, `0x468B16`) reveals and explores for the owner;
//!   an RZOD (type 1) of ActorData `+0x1D0` (remote_detection_range, the
//!   control_range for an organization, `0x46889E`) reveals (without
//!   exploring) to the kingdoms of the reveal mask: the kingdoms engaging
//!   the actor (`0x56EDC0`) minus the owner (`0x56EFC0`). A dead actor keeps
//!   both zones where it died (types 1/6 survive the death flag, `0x514CD0`,
//!   and the tick returns early); a destroyed actor releases them.
//! * Coverage (`0x540020` fast circle builder, the SVar
//!   ZoneBuilderFastDetectionZones path): the zone sits on the tile of the
//!   clamped actor position (`0x515F40`, `0x552640`), its radius is raised so
//!   that `fmod(r, tile) >= tile / 4` (`0x53FE90`), the rows come from the
//!   midpoint circle `0x53F730` and a tile is covered when its centre lies
//!   within the radius (`0x53F6C0`); the last map column never receives a
//!   fast-built zone (native span clip, kept).
//! * Actor detection word (actor `+0x54`, same layout): visible to k while a
//!   tile the actor occupies is visible to k (`0x4EC010` reveal, `0x4EC1B0`
//!   hide, `0x4EC270` tile change); seen (explored) by k once revealed on an
//!   explored tile. Queries use the shared-vision mask (`0x4E0F90`: every
//!   allied major kingdom, itself included): `0x4EA1D0` visible, `0x4EA0D0`
//!   seen, `0x4D3B80` with `deep` for an organization (any member).
//! * Detection memory (`0x4EBC70`): a static actor that a kingdom has seen
//!   but no longer sees is remembered by that kingdom (a ghost record of how
//!   it was); the record goes when the kingdom sees the actor again.
//!
//! Plain `f32` arithmetic and our own layout; the per-tile detecting-zone
//! counts stand for the native "does another zone of the tile detect for k"
//! test (`0x515490`), with the same result.
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, KingdomId, OrgId, World};
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;

/// Bit offset of the visible half of a fog word (`0xA11740`).
pub const VISIBLE_SHIFT: u32 = 16;
/// Kingdom bits of a fog word.
pub const KINGDOM_BITS: u32 = 16;
const EXPLORED_ALL: u32 = 0xffff;
const VISIBLE_ALL: u32 = 0xffff_0000;
/// `0x552640` clamp margin (`CLAMP_MARGIN_BITS`, 1e-4).
const CLAMP_MARGIN: f32 = 1.0e-4;

/// The FogOfWar world rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FogMode {
    /// No fog: everything is explored and visible.
    None,
    /// The whole map and its static actors start explored (the AA default).
    #[default]
    Explored,
    /// Everything starts unexplored (black).
    Full,
}

impl FogMode {
    /// The rule's enum index (world rule value / SVar FogOfWar).
    pub fn from_index(v: i32) -> Self {
        match v {
            0 => FogMode::None,
            1 => FogMode::Explored,
            _ => FogMode::Full,
        }
    }
    /// The rule's EnumValue IDS (NONE, EXPLORED, FULL) or its index.
    pub fn from_ids(s: &str) -> Option<Self> {
        let t = s.trim().trim_matches('"');
        if let Ok(v) = t.parse::<f32>() {
            return Some(Self::from_index(v as i32));
        }
        match t.to_ascii_uppercase().as_str() {
            "NONE" => Some(FogMode::None),
            "EXPLORED" => Some(FogMode::Explored),
            "FULL" => Some(FogMode::Full),
            _ => None,
        }
    }
}

/// What a kingdom remembers of a static actor it no longer sees.
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    pub def: DefId,
    pub pos: Vec3,
    pub facing: f32,
    pub kingdom: Option<KingdomId>,
    pub health: f32,
}

/// What a tile looks like to a kingdom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileView {
    Visible,
    /// Explored but not visible: the grey fog.
    Explored,
    /// Never explored: the black fog.
    Unexplored,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Zone {
    x: i32,
    y: i32,
    /// Authored radius (m), compared for changes (`0x56EB40`).
    radius: f32,
    /// Kingdom bits the zone detects for.
    bits: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Zones {
    /// Type 6 (ZOD): reveals and explores for the owner.
    zod: Option<Zone>,
    /// Type 1 (RZOD): reveals to the reveal-mask kingdoms.
    rzod: Option<Zone>,
}

/// Per-definition detection ranges (ActorData +0x1CC / +0x1D0).
#[derive(Clone, Copy, Debug, Default)]
struct DefDetection {
    zod: f32,
    rzod: f32,
    /// Static actor: explored at the EXPLORED start, keeps memory records.
    keeps_memory: bool,
    /// Footprint half extents (m) for the tiles a structure occupies.
    half: Vec2,
}

/// Per-tick cost and work counters.
#[derive(Clone, Copy, Debug, Default)]
pub struct FogStats {
    /// Microseconds the last tick took.
    pub last_us: f32,
    /// Largest tick seen.
    pub max_us: f32,
    pub ticks: u64,
    /// Tile coverage changes of the last tick.
    pub tile_ops: u32,
    /// Zones moved / created / released in the last tick.
    pub zone_moves: u32,
}

/// Fog side state kept beside the battle (`World::fog`).
#[derive(Debug, Default)]
pub struct FogState {
    /// The FogOfWar rule; None until set (the world rule default is used).
    pub mode: Option<FogMode>,
    ready: bool,
    pub width: i32,
    pub height: i32,
    /// Tile edge (m): the map cell.
    pub tile: f32,
    /// Fog word per tile (row-major, y up).
    pub tiles: Vec<u32>,
    /// Detecting zones covering each tile, per kingdom bit.
    counts: Vec<Vec<u16>>,
    defs: Vec<DefDetection>,
    actor_zones: Vec<Zones>,
    org_zones: Vec<Zones>,
    /// Actor detection words (actor +0x54).
    pub words: Vec<u32>,
    /// Organization words (union of the members' visible bits).
    pub org_words: Vec<u32>,
    /// Shared-vision mask per kingdom bit (fog-word layout, `+0x228`).
    shared: [u32; KINGDOM_BITS as usize],
    /// Kingdom id -> fog bit (None: non-major or past 16 kingdoms).
    bits: Vec<Option<u32>>,
    /// Ghost records per kingdom bit.
    memory: Vec<BTreeMap<ActorId, Memory>>,
    /// Bumped whenever a tile word changes (renderers re-read the tiles).
    pub revision: u64,
    pub stats: FogStats,
}

impl FogState {
    pub fn enabled(&self) -> bool {
        self.mode != Some(FogMode::None)
    }
}

fn bit(i: u32) -> u32 {
    1u32.wrapping_shl(i)
}

/// `0x53FE90`: raise the radius so that its remainder modulo the integer
/// tile size is at least a quarter tile.
fn adjust_radius(r: f32, tile: f32) -> f32 {
    let q = tile.trunc().max(1.0);
    let a = q * 0.25;
    let c = r % q;
    if c < a { (a - c) + r } else { r }
}

/// `0x53F730` row half-widths: `hw[d]` for rows `cy +- d`, `d = 0..=rad`.
fn circle_rows(r_tiles: f32) -> Vec<i32> {
    let rad = r_tiles.trunc() as i32;
    let rr = (r_tiles * r_tiles).trunc() as i32;
    let mut hw = vec![0; rad.max(0) as usize + 1];
    let neg = -rad;
    if neg < 0 {
        let mut err = neg * neg - 1;
        let mut dx = 0i32;
        let mut dy = neg;
        loop {
            loop {
                let next = err - dx * 2 + 1;
                if next > rr {
                    break;
                }
                err = next;
                dx -= 1;
            }
            hw[(-dy) as usize] = -dx;
            err += dy * 2 + 1;
            dy += 1;
            if dy >= 0 {
                break;
            }
        }
    }
    hw[0] = rad.max(0);
    hw
}

/// Tiles of one zone circle: `(row half-widths, squared radius in tiles)`.
struct Circle {
    rows: Vec<i32>,
    r2_tiles: f32,
}

impl Circle {
    fn new(radius: f32, tile: f32) -> Self {
        let q = tile.trunc().max(1.0);
        let r = adjust_radius(radius, tile);
        Circle { rows: circle_rows(r / q), r2_tiles: (r * r) / (q * q) }
    }
    fn rad(&self) -> i32 {
        self.rows.len() as i32 - 1
    }
    /// The zone at `(zx, zy)` covers tile `(x, y)` (span bound and centre test).
    fn covers(&self, zx: i32, zy: i32, x: i32, y: i32) -> bool {
        let (dx, dy) = (x - zx, y - zy);
        let ady = dy.unsigned_abs() as usize;
        if ady >= self.rows.len() || dx.abs() > self.rows[ady] {
            return false;
        }
        (dx * dx + dy * dy) as f32 <= self.r2_tiles
    }
}

impl FogState {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height).then(|| (y * self.width + x) as usize)
    }

    fn tile_of(&self, p: Vec2) -> (i32, i32) {
        // 0x552640 clamp, then ftol >> shift
        let ex = self.width as f32 * self.tile;
        let ey = self.height as f32 * self.tile;
        let cx = p.x.clamp(self.tile, (ex - self.tile - CLAMP_MARGIN).max(self.tile));
        let cy = p.y.clamp(self.tile, (ey - self.tile - CLAMP_MARGIN).max(self.tile));
        ((cx / self.tile) as i32, (cy / self.tile) as i32)
    }

    fn counts_for(&mut self, b: u32) -> &mut Vec<u16> {
        let n = (self.width * self.height).max(0) as usize;
        let v = &mut self.counts[b as usize];
        if v.len() != n {
            *v = vec![0; n];
        }
        v
    }

    /// One coverage change of `bits` on tile `t`: +1 or -1 detecting zone.
    fn cover(&mut self, t: usize, bits: u32, add: bool, explore: bool) {
        let mut rest = bits;
        while rest != 0 {
            let b = rest.trailing_zeros();
            rest &= rest - 1;
            let c = &mut self.counts_for(b)[t];
            let before = *c;
            if add {
                *c = c.saturating_add(1);
            } else {
                *c = c.saturating_sub(1);
            }
            let after = *c;
            let w = &mut self.tiles[t];
            let old = *w;
            if after > 0 {
                *w |= bit(b + VISIBLE_SHIFT);
                if explore && add {
                    *w |= bit(b);
                }
            } else if before > 0 {
                *w &= !bit(b + VISIBLE_SHIFT);
            }
            if *w != old {
                self.revision = self.revision.wrapping_add(1);
            }
            self.stats.tile_ops += 1;
        }
    }

    /// Move a zone: tiles only in the new circle are added, tiles only in
    /// the old one removed; bits leaving or joining the zone (RZOD mask
    /// changes, `0x5406D0`) are removed / added over the whole circle.
    fn move_zone(&mut self, old: Option<Zone>, new: Option<Zone>, explore: bool) {
        if old == new {
            return;
        }
        self.stats.zone_moves += 1;
        let tile = self.tile;
        let co = old.map(|z| Circle::new(z.radius, tile));
        let cn = new.map(|z| Circle::new(z.radius, tile));
        let last = self.width - 1;
        // the union bounding box
        let mut lo = (i32::MAX, i32::MAX);
        let mut hi = (i32::MIN, i32::MIN);
        for (z, c) in [(old, &co), (new, &cn)] {
            if let (Some(z), Some(c)) = (z, c) {
                let r = c.rad();
                lo = (lo.0.min(z.x - r), lo.1.min(z.y - r));
                hi = (hi.0.max(z.x + r), hi.1.max(z.y + r));
            }
        }
        if lo.0 > hi.0 {
            return;
        }
        let (x0, y0) = (lo.0.max(0), lo.1.max(0));
        // native span clip: the end column is exclusive of width - 1
        let (x1, y1) = (hi.0.min(last - 1), hi.1.min(self.height - 1));
        let ob = old.map_or(0, |z| z.bits);
        let nb = new.map_or(0, |z| z.bits);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let in_old = match (old, &co) {
                    (Some(z), Some(c)) => c.covers(z.x, z.y, x, y),
                    _ => false,
                };
                let in_new = match (new, &cn) {
                    (Some(z), Some(c)) => c.covers(z.x, z.y, x, y),
                    _ => false,
                };
                if !in_old && !in_new {
                    continue;
                }
                let t = (y * self.width + x) as usize;
                let (remove, add) = match (in_old, in_new) {
                    (true, true) => (ob & !nb, nb & !ob),
                    (true, false) => (ob, 0),
                    (false, true) => (0, nb),
                    (false, false) => (0, 0),
                };
                if add != 0 {
                    self.cover(t, add, true, explore);
                }
                if remove != 0 {
                    self.cover(t, remove, false, explore);
                }
            }
        }
    }

    fn shared_mask(&self, bit_index: Option<u32>) -> Option<u32> {
        bit_index.map(|b| self.shared[b as usize])
    }

    /// Tiles a structure occupies (its footprint) or the tile under a unit.
    fn occupancy(&self, def: &DefDetection, pos: Vec2, facing: f32, out: &mut Vec<usize>) {
        out.clear();
        if def.half == Vec2::ZERO {
            let (x, y) = ((pos.x / self.tile).floor() as i32, (pos.y / self.tile).floor() as i32);
            out.extend(self.index(x, y));
            return;
        }
        let (s, c) = facing.sin_cos();
        let ext = Vec2::new((c * def.half.x).abs() + (s * def.half.y).abs(), (s * def.half.x).abs() + (c * def.half.y).abs());
        let lo = ((pos - ext) / self.tile).floor();
        let hi = ((pos + ext) / self.tile).floor();
        for y in lo.y as i32..=hi.y as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                out.extend(self.index(x, y));
            }
        }
    }

    fn prepare(&mut self, world: &World) {
        let map = &world.map;
        self.width = map.width as i32;
        self.height = map.height as i32;
        self.tile = if map.cell_size > 0.0 { map.cell_size } else { 4.0 };
        let n = (self.width * self.height).max(0) as usize;
        self.tiles = vec![0; n];
        self.counts = vec![Vec::new(); KINGDOM_BITS as usize];
        self.memory = vec![BTreeMap::new(); KINGDOM_BITS as usize];
        self.defs = world.data.defs.iter().map(def_detection).collect();
        self.actor_zones.clear();
        self.org_zones.clear();
        self.words.clear();
        self.org_words.clear();
        let mode = self.mode.unwrap_or_else(|| default_mode(world));
        self.mode = Some(mode);
        match mode {
            // 0x551FD0: every tile explored and visible, every actor too
            FogMode::None => {
                self.tiles.fill(EXPLORED_ALL | VISIBLE_ALL);
            }
            // 0x551F90: every tile explored, the static actors explored
            FogMode::Explored => {
                self.tiles.fill(EXPLORED_ALL);
            }
            FogMode::Full => {}
        }
        self.revision = self.revision.wrapping_add(1);
        self.ready = true;
        self.grow(world, true);
    }

    fn grow(&mut self, world: &World, start: bool) {
        let mode = self.mode.unwrap_or_default();
        while self.words.len() < world.actors.len() {
            let a = &world.actors[self.words.len()];
            let d = self.defs.get(a.def as usize).copied().unwrap_or_default();
            // 0x54F400 explores the start's static map objects (flag bit 20,
            // or bit 11 without bit 23): the neutral ones (cities, villages,
            // scenery). A side's base is not among them: the original shows
            // the other bases only once seen (compared with the running
            // original under EXPLORED).
            let neutral = a.kingdom.is_none_or(|k| world.kingdoms.get(k as usize).is_none_or(|x| x.independent));
            let w = match mode {
                FogMode::None => EXPLORED_ALL | VISIBLE_ALL,
                FogMode::Explored if start && d.keeps_memory && neutral => EXPLORED_ALL,
                _ => 0,
            };
            self.words.push(w);
            self.actor_zones.push(Zones::default());
        }
        while self.org_words.len() < world.orgs.len() {
            self.org_words.push(if mode == FogMode::None { EXPLORED_ALL | VISIBLE_ALL } else { 0 });
            self.org_zones.push(Zones::default());
        }
        if self.bits.len() != world.kingdoms.len() {
            self.bits = world.kingdoms.iter().map(|k| (!k.independent && k.id < KINGDOM_BITS).then_some(k.id)).collect();
        }
    }
}

/// The world rule default: FogOfWar in Data/Game/world_rules_aa.tgi is
/// EXPLORED; the SVars value (FogOfWar = 2) is only the fallback the world
/// rule overrides (SVars.tgi: "world rules override").
fn default_mode(_world: &World) -> FogMode {
    FogMode::Explored
}

fn def_detection(d: &crate::data::Def) -> DefDetection {
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
    DefDetection { zod, rzod, keeps_memory, half }
}

/// Set the FogOfWar rule (the setup screen's world rule). Before the first
/// tick it sets the start; later it restarts the fog.
pub fn set_mode(world: &mut World, mode: FogMode) {
    let st = &mut world.fog;
    if st.mode != Some(mode) {
        st.mode = Some(mode);
        st.ready = false;
    }
}

/// Build the fog state now if no tick has yet (the first frame of a battle
/// draws before the first tick runs).
pub fn ensure(world: &mut World) {
    if !world.fog.ready {
        tick(world);
    }
}

/// One fog tick (after the other systems of the battle tick).
pub fn tick(world: &mut World) {
    let started = std::time::Instant::now();
    let mut st = std::mem::take(&mut world.fog);
    if !st.ready || st.width != world.map.width as i32 || st.height != world.map.height as i32 {
        st.prepare(world);
    }
    st.grow(world, false);
    st.stats.tile_ops = 0;
    st.stats.zone_moves = 0;
    if st.enabled() {
        update(&mut st, world);
    }
    let us = started.elapsed().as_secs_f32() * 1.0e6;
    st.stats.last_us = us;
    st.stats.max_us = st.stats.max_us.max(us);
    st.stats.ticks += 1;
    world.fog = st;
}

fn update(st: &mut FogState, world: &World) {
    // shared vision (0x4E0F90): every allied major kingdom
    st.shared = [0; KINGDOM_BITS as usize];
    for (i, a) in world.kingdoms.iter().enumerate() {
        let Some(ba) = st.bits.get(i).copied().flatten() else { continue };
        let mut m = 0;
        for (j, b) in world.kingdoms.iter().enumerate() {
            let Some(bb) = st.bits.get(j).copied().flatten() else { continue };
            let r = world.books.relation(&world.kingdoms, Some(a.id), Some(b.id));
            if r == crate::books::SELF || r == crate::books::FRIEND {
                m |= bit(bb) | bit(bb + VISIBLE_SHIFT);
            }
        }
        st.shared[ba as usize] = m;
    }
    // reveal masks (0x56EDC0): the kingdoms whose actors engage the actor
    // or a member of the organization, minus the owner
    let mut actor_mask: Vec<u32> = vec![0; world.actors.len()];
    let mut org_mask: Vec<u32> = vec![0; world.orgs.len()];
    for a in world.actors.iter().filter(|a| a.alive && !a.removed) {
        let (Some(t), Some(k)) = (a.combat.target, a.kingdom) else { continue };
        let Some(b) = st.bits.get(k as usize).copied().flatten() else { continue };
        if let Some(m) = actor_mask.get_mut(t as usize) {
            *m |= bit(b);
        }
        if let Some(o) = world.actors.get(t as usize).and_then(|x| x.org) {
            if let Some(m) = org_mask.get_mut(o as usize) {
                *m |= bit(b);
            }
        }
    }
    let bits = st.bits.clone();
    let owner_bit = |k: Option<KingdomId>| k.and_then(|k| bits.get(k as usize).copied().flatten());
    // actor zones (0x56F140)
    for i in 0..world.actors.len() {
        let a = &world.actors[i];
        let cur = st.actor_zones[i];
        if a.removed {
            if cur.zod.is_some() || cur.rzod.is_some() {
                st.move_zone(cur.zod, None, true);
                st.move_zone(cur.rzod, None, false);
                st.actor_zones[i] = Zones::default();
            }
            continue;
        }
        if !a.alive {
            // a dead actor keeps its zones where it died
            continue;
        }
        let d = st.defs.get(a.def as usize).copied().unwrap_or_default();
        let own = owner_bit(a.kingdom);
        let (x, y) = st.tile_of(a.pos.truncate());
        let zod = (d.zod > 0.0).then_some(()).and(own).map(|b| Zone { x, y, radius: d.zod, bits: bit(b) });
        let mask = actor_mask[i] & !own.map_or(0, bit);
        let rzod = (d.rzod > 0.0 && mask != 0).then_some(Zone { x, y, radius: d.rzod, bits: mask });
        if cur.zod != zod {
            st.move_zone(cur.zod, zod, true);
        }
        if cur.rzod != rzod {
            st.move_zone(cur.rzod, rzod, false);
        }
        st.actor_zones[i] = Zones { zod, rzod };
    }
    // organization zones: the organization actor stands at its members'
    // centre
    for (i, o) in world.orgs.iter().enumerate() {
        let cur = st.org_zones[i];
        if !o.alive {
            if cur.zod.is_some() || cur.rzod.is_some() {
                st.move_zone(cur.zod, None, true);
                st.move_zone(cur.rzod, None, false);
                st.org_zones[i] = Zones::default();
            }
            continue;
        }
        let (mut sum, mut n) = (Vec2::ZERO, 0);
        for &m in &o.members {
            if let Some(a) = world.actors.get(m as usize).filter(|a| a.alive && !a.removed) {
                sum += a.pos.truncate();
                n += 1;
            }
        }
        let p = if n > 0 { sum / n as f32 } else { o.anchor };
        let d = st.defs.get(o.def as usize).copied().unwrap_or_default();
        let own = owner_bit(Some(o.kingdom));
        let (x, y) = st.tile_of(p);
        let zod = (d.zod > 0.0).then_some(()).and(own).map(|b| Zone { x, y, radius: d.zod, bits: bit(b) });
        let mask = org_mask[i] & !own.map_or(0, bit);
        let rzod = (d.rzod > 0.0 && mask != 0).then_some(Zone { x, y, radius: d.rzod, bits: mask });
        if cur.zod != zod {
            st.move_zone(cur.zod, zod, true);
        }
        if cur.rzod != rzod {
            st.move_zone(cur.rzod, rzod, false);
        }
        st.org_zones[i] = Zones { zod, rzod };
    }
    // actor words (0x4EC010 / 0x4EC1B0 / 0x4EC270): visible while a tile
    // the actor occupies is visible; seen once revealed on an explored tile
    let mut tiles = Vec::new();
    for w in st.org_words.iter_mut() {
        *w &= EXPLORED_ALL;
    }
    for i in 0..world.actors.len() {
        let a = &world.actors[i];
        if a.removed {
            continue;
        }
        let d = st.defs.get(a.def as usize).copied().unwrap_or_default();
        st.occupancy(&d, a.pos.truncate(), a.facing, &mut tiles);
        let (mut vis, mut exp) = (0u32, 0u32);
        for &t in &tiles {
            let w = st.tiles[t];
            vis |= w & VISIBLE_ALL;
            exp |= w & EXPLORED_ALL;
        }
        let old = st.words[i];
        let seen = (old & EXPLORED_ALL) | ((vis >> VISIBLE_SHIFT) & exp);
        let word = seen | vis;
        st.words[i] = word;
        if let Some(o) = a.org.filter(|_| a.alive) {
            if let Some(ow) = st.org_words.get_mut(o as usize) {
                *ow |= word;
            }
        }
        // detection memory (0x4EBC70) for static actors
        if d.keeps_memory {
            refresh_memory(st, world, i as ActorId, word);
        }
    }
    // ghosts of actors gone while remembered go once their spot is seen
    for b in 0..KINGDOM_BITS as usize {
        if st.memory[b].is_empty() {
            continue;
        }
        let shared = st.shared[b];
        let gone: Vec<ActorId> = st.memory[b]
            .iter()
            .filter(|(id, m)| {
                world.actors.get(**id as usize).is_none_or(|a| a.removed)
                    && st.index((m.pos.x / st.tile).floor() as i32, (m.pos.y / st.tile).floor() as i32).is_some_and(|t| st.tiles[t] & shared & VISIBLE_ALL != 0)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            st.memory[b].remove(&id);
        }
    }
}

/// `0x4EBC70`: per kingdom, a record while the actor is seen but not
/// visible (created while it is alive); deleted once it is visible again.
fn refresh_memory(st: &mut FogState, world: &World, id: ActorId, word: u32) {
    let a = &world.actors[id as usize];
    for b in 0..KINGDOM_BITS as usize {
        let shared = st.shared[b];
        if shared == 0 {
            continue;
        }
        let remembered = word & shared & VISIBLE_ALL == 0 && word & shared & EXPLORED_ALL != 0;
        let has = st.memory[b].contains_key(&id);
        if has {
            if !remembered {
                st.memory[b].remove(&id);
            }
        } else if remembered && a.alive {
            st.memory[b].insert(id, Memory { def: a.def, pos: a.pos, facing: a.facing, kingdom: a.kingdom, health: a.health });
        }
    }
}

// ---------------------------------------------------------------------------
// Queries.

fn viewer_bit(world: &World, viewer: Option<KingdomId>) -> Option<u32> {
    let v = viewer?;
    world.fog.bits.get(v as usize).copied().flatten().or_else(|| {
        // before the first fog tick: a major kingdom's id is its bit
        world.kingdoms.get(v as usize).filter(|k| !k.independent && k.id < KINGDOM_BITS).map(|k| k.id)
    })
}

/// The fog is on (FogOfWar not NONE).
pub fn enabled(world: &World) -> bool {
    world.fog.enabled()
}

/// The viewer's shared-vision mask; None = sees everything (fog off, no
/// viewer or a non-major viewer, `0x54EB70`).
fn mask(world: &World, viewer: Option<KingdomId>) -> Option<u32> {
    if !world.fog.enabled() || !world.fog.ready {
        return None;
    }
    world.fog.shared_mask(viewer_bit(world, viewer))
}

/// `0x4EA1D0`: the actor is visible to the viewer or a kingdom sharing its
/// vision (an organization member answers for itself).
pub fn actor_visible(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> bool {
    let Some(m) = mask(world, viewer) else { return true };
    world.fog.words.get(actor as usize).is_some_and(|w| w & m & VISIBLE_ALL != 0)
}

/// `0x4EA0D0`: the actor has been seen by the viewer's side.
pub fn actor_seen(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> bool {
    let Some(m) = mask(world, viewer) else { return true };
    world.fog.words.get(actor as usize).is_some_and(|w| w & m & EXPLORED_ALL != 0)
}

/// `0x4D3B80` with `deep`: an organization is visible while a member is.
pub fn org_visible(world: &World, org: OrgId, viewer: Option<KingdomId>) -> bool {
    let Some(m) = mask(world, viewer) else { return true };
    world.fog.org_words.get(org as usize).is_some_and(|w| w & m & VISIBLE_ALL != 0)
}

/// The viewer's ghost record of a static actor it no longer sees.
pub fn memory(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> Option<&Memory> {
    mask(world, viewer)?;
    let b = viewer_bit(world, viewer)?;
    world.fog.memory.get(b as usize)?.get(&actor)
}

/// Every ghost record of the viewer.
pub fn memories(world: &World, viewer: Option<KingdomId>) -> Vec<(ActorId, &Memory)> {
    if mask(world, viewer).is_none() {
        return Vec::new();
    }
    let Some(b) = viewer_bit(world, viewer) else { return Vec::new() };
    world.fog.memory.get(b as usize).map(|m| m.iter().map(|(k, v)| (*k, v)).collect()).unwrap_or_default()
}

/// What the actor looks like to the viewer: drawn live, drawn as the
/// viewer remembers it, or hidden.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sighting<'w> {
    Visible,
    Remembered(&'w Memory),
    Hidden,
}

/// The renderers' test: own and shared-vision actors and visible actors are
/// drawn; static actors the viewer has seen are drawn as remembered; the
/// rest is hidden. Unowned scenery is always drawn.
pub fn sighting(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> Sighting<'_> {
    let Some(m) = mask(world, viewer) else { return Sighting::Visible };
    let Some(a) = world.actors.get(actor as usize) else { return Sighting::Hidden };
    if a.kingdom.is_none() {
        return Sighting::Visible;
    }
    if let Some(kb) = a.kingdom.and_then(|k| world.fog.bits.get(k as usize).copied().flatten()) {
        if m & bit(kb) != 0 {
            return Sighting::Visible;
        }
    }
    let w = world.fog.words.get(actor as usize).copied().unwrap_or(0);
    if w & m & VISIBLE_ALL != 0 {
        return Sighting::Visible;
    }
    match memory(world, actor, viewer) {
        Some(r) => Sighting::Remembered(r),
        None => Sighting::Hidden,
    }
}

/// The viewer's view of tile `(x, y)` (shared vision).
pub fn tile_view(world: &World, viewer: Option<KingdomId>, x: i32, y: i32) -> TileView {
    let Some(m) = mask(world, viewer) else { return TileView::Visible };
    let Some(t) = world.fog.index(x, y) else { return TileView::Unexplored };
    let w = world.fog.tiles[t];
    if w & m & VISIBLE_ALL != 0 {
        TileView::Visible
    } else if w & m & EXPLORED_ALL != 0 {
        TileView::Explored
    } else {
        TileView::Unexplored
    }
}

/// The viewer's view at a ground point.
pub fn point_view(world: &World, viewer: Option<KingdomId>, p: Vec2) -> TileView {
    let tile = if world.fog.tile > 0.0 { world.fog.tile } else { world.map.cell_size.max(1.0) };
    tile_view(world, viewer, (p.x / tile).floor() as i32, (p.y / tile).floor() as i32)
}

/// The terrain fog texel values (`0x71BD00`): 0 visible, `gray` explored,
/// 255 unexplored, one per tile, row-major (y up). Fog off: all 0.
pub fn shade(world: &World, viewer: Option<KingdomId>, gray: u8, out: &mut Vec<u8>) {
    let st = &world.fog;
    let n = (st.width * st.height).max(0) as usize;
    out.clear();
    let Some(m) = mask(world, viewer) else {
        out.resize(n, 0);
        return;
    };
    out.extend(st.tiles.iter().map(|w| {
        if w & m & VISIBLE_ALL != 0 {
            0
        } else if w & m & EXPLORED_ALL != 0 {
            gray
        } else {
            255
        }
    }));
}
