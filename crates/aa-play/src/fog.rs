//! Fog of war: per-kingdom tile visibility from the actors' detection
//! zones, the actors' detection words and the detection memory (ghost
//! records of static actors seen earlier). Owner: fog (UI developer).
//!
//! The fog runs on the exact port of the original (aa-zones, bound to the
//! battle in [`crate::zones`]): the tile fog words, the detection component
//! zones and the actor words are AA.exe's, computed with its x87 arithmetic
//! at PC53 and its tie and order rules. This module keeps the battle-facing
//! API (the queries the renderers, the minimap and the AI use) over it.
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
//! * Detection component (`0x56F140`): ZOD (type 6, ActorData `+0x1CC`) and
//!   RZOD (type 1, `+0x1D0`, revealed to the reveal-mask kingdoms); see
//!   [`crate::zones`] for the bindings.
//! * Actor detection word (actor `+0x54`): `0x4EC010` reveal, `0x4EC1B0`
//!   hide, `0x4EC270` tile change; the queries use the shared-vision mask
//!   (`0x4E0F90`): `0x4EA1D0` visible, `0x4EA0D0` seen, `0x4D3B80` with
//!   `deep` for an organization (its own word or any member).
//! * Detection memory (`0x4EBC70`): a static actor that a kingdom has seen
//!   but no longer sees is remembered by that kingdom; the record goes when
//!   the kingdom sees the actor again. A record of an actor destroyed out of
//!   sight goes once its spot is seen (the record object `0x4EB9F0` is not
//!   ported).
use crate::world::{ActorId, KingdomId, OrgId, World};
use crate::zones::Engine;
use glam::Vec2;

pub use crate::zones::Memory;

/// Bit offset of the visible half of a fog word (`0xA11740`).
pub const VISIBLE_SHIFT: u32 = 16;
/// Kingdom bits of a fog word.
pub const KINGDOM_BITS: u32 = 16;
const EXPLORED_ALL: u32 = 0xffff;
const VISIBLE_ALL: u32 = 0xffff_0000;

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

/// What a tile looks like to a kingdom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileView {
    Visible,
    /// Explored but not visible: the grey fog.
    Explored,
    /// Never explored: the black fog.
    Unexplored,
}

/// Per-tick cost and work counters.
#[derive(Clone, Copy, Debug, Default)]
pub struct FogStats {
    /// Microseconds the last tick took.
    pub last_us: f32,
    /// Largest tick seen.
    pub max_us: f32,
    pub ticks: u64,
    /// Fog bit changes of the last tick.
    pub tile_ops: u32,
    /// Actor tile changes (`0x4D6350`) of the last tick.
    pub zone_moves: u32,
}

/// Fog side state kept beside the battle (`World::fog`), with the zone
/// engine that also holds the supply zones.
#[derive(Debug, Default)]
pub struct FogState {
    /// The FogOfWar rule; None until set (the world rule default is used).
    pub mode: Option<FogMode>,
    ready: bool,
    pub width: i32,
    pub height: i32,
    /// Tile edge (m): the map cell.
    pub tile: f32,
    /// Bumped whenever a tile word changes (renderers re-read the tiles).
    pub revision: u64,
    pub stats: FogStats,
    /// The aa-zones engine (fog and supply zones); None before the first
    /// tick, or for a map whose tile edge is not a power of two.
    pub engine: Option<Engine>,
    /// The engine's fog change count at the last revision bump.
    seen_changes: u64,
    /// Engine counters at the end of the last tick.
    seen_tile_changes: u64,
}

impl FogState {
    pub fn enabled(&self) -> bool {
        self.mode != Some(FogMode::None)
    }
}

/// The world rule default: FogOfWar in Data/Game/world_rules_aa.tgi is
/// EXPLORED; the SVars value (FogOfWar = 2) is only the fallback the world
/// rule overrides (SVars.tgi: "world rules override").
fn default_mode(_world: &World) -> FogMode {
    FogMode::Explored
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

/// The zone engine, created for the world's map when missing (the supply
/// zones may need it before the first fog tick). None for a map aa-zones
/// cannot describe.
pub fn take_engine(world: &mut World) -> Option<Engine> {
    match world.fog.engine.take() {
        Some(e) if e.fits(world) => Some(e),
        _ => Engine::new(world),
    }
}

/// Put the engine back after [`take_engine`].
pub fn put_engine(world: &mut World, engine: Engine) {
    world.fog.engine = Some(engine);
}

/// The zone engine (read).
pub fn engine(world: &World) -> Option<&Engine> {
    world.fog.engine.as_ref()
}

impl FogState {
    /// The world start for the FogOfWar rule (`0x551F90` / `0x551FD0`). A
    /// restart (the rule changed during the battle) begins a new engine; the
    /// supply zones are placed again by the next supply tick.
    fn prepare(&mut self, world: &World) {
        let map = &world.map;
        self.width = map.width as i32;
        self.height = map.height as i32;
        self.tile = if map.cell_size > 0.0 { map.cell_size } else { 4.0 };
        let mode = self.mode.unwrap_or_else(|| default_mode(world));
        self.mode = Some(mode);
        let restart = self.ticks_started();
        let engine = match self.engine.take() {
            Some(e) if e.fits(world) && !restart => Some(e),
            _ => Engine::new(world),
        };
        self.engine = engine;
        if let Some(e) = self.engine.as_mut() {
            let (explored, visible) = match mode {
                FogMode::None => (true, true),
                FogMode::Explored => (true, false),
                FogMode::Full => (false, false),
            };
            // 0x54F400 explores the start's static map objects (flag bit 20,
            // or bit 11 without bit 23): the neutral ones (cities, villages,
            // scenery). A side's base is not among them: the original shows
            // the other bases only once seen (compared with the running
            // original under EXPLORED).
            let explored_actor = |a: ActorId| {
                let x = &world.actors[a as usize];
                let neutral = x.kingdom.is_none_or(|k| world.kingdoms.get(k as usize).is_none_or(|k| k.independent));
                neutral && e.def_of(world, crate::zones::Key::Actor(a)).keeps_memory
            };
            let flags: Vec<bool> = (0..world.actors.len() as ActorId).map(explored_actor).collect();
            e.start_fog(world, explored, visible, |a| flags.get(a as usize).copied().unwrap_or(false));
            self.seen_tile_changes = e.parts.stats.tile_changes;
        }
        self.revision = self.revision.wrapping_add(1);
        self.ready = true;
    }

    fn ticks_started(&self) -> bool {
        self.stats.ticks > 0
    }
}

/// One fog tick (after the other systems of the battle tick): tile changes,
/// detection component ticks, the supply tiles of moved zones.
pub fn tick(world: &mut World) {
    let started = std::time::Instant::now();
    let mut st = std::mem::take(&mut world.fog);
    if st.engine.as_ref().is_some_and(|e| !e.fits(world)) {
        st.ready = false;
        st.engine = None;
    }
    if st.width != world.map.width as i32 || st.height != world.map.height as i32 {
        st.ready = false;
    }
    if !st.ready {
        st.prepare(world);
    }
    let enabled = st.enabled();
    if let Some(e) = st.engine.as_mut() {
        e.observe(world);
        if enabled {
            e.detection_tick(world);
            forget_gone(e, world);
        }
        e.refresh_supply_tiles();
        let changes = e.parts.fog_changes;
        st.stats.tile_ops = changes.wrapping_sub(st.seen_changes) as u32;
        if changes != st.seen_changes {
            st.seen_changes = changes;
            st.revision = st.revision.wrapping_add(1);
        }
        st.stats.zone_moves = e.parts.stats.tile_changes.wrapping_sub(st.seen_tile_changes) as u32;
        st.seen_tile_changes = e.parts.stats.tile_changes;
    }
    let us = started.elapsed().as_secs_f32() * 1.0e6;
    st.stats.last_us = us;
    st.stats.max_us = st.stats.max_us.max(us);
    st.stats.ticks += 1;
    world.fog = st;
    crate::supply::publish_zone_tiles(world);
}

/// Ghosts of actors destroyed while remembered go once their spot is seen
/// (visible to a kingdom sharing the rememberer's vision).
fn forget_gone(e: &mut Engine, world: &World) {
    let g = e.parts.geometry;
    let tile = f32::from_bits(g.tile_size_bits);
    for k in 0..e.parts.memory.len() {
        if e.parts.memory[k].is_empty() {
            continue;
        }
        let shared = e.parts.kingdoms.get(k).map_or(0, |x| x.shared_mask);
        let gone: Vec<ActorId> = e.parts.memory[k]
            .iter()
            .filter(|(id, m)| {
                let (x, y) = ((m.pos.x / tile).floor() as i32, (m.pos.y / tile).floor() as i32);
                world.actors.get(**id as usize).is_none_or(|a| a.removed)
                    && x >= 0
                    && y >= 0
                    && x < g.width
                    && y < g.height
                    && e.tile_fog((y * g.width + x) as usize) & shared & VISIBLE_ALL != 0
            })
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            e.parts.memory[k].remove(&id);
        }
    }
}

// ---------------------------------------------------------------------------
// Queries.

/// The fog is on (FogOfWar not NONE).
pub fn enabled(world: &World) -> bool {
    world.fog.enabled()
}

/// The viewer's engine and shared-vision mask (`+0x228`); None = sees
/// everything (fog off, no viewer or a non-major viewer, `0x54EB70`).
fn mask(world: &World, viewer: Option<KingdomId>) -> Option<(&Engine, u32, KingdomId)> {
    if !world.fog.enabled() || !world.fog.ready {
        return None;
    }
    let e = world.fog.engine.as_ref()?;
    let v = viewer?;
    let k = e.kingdom(v)?;
    (k.nonmajor == 0).then_some((e, k.shared_mask, v))
}

/// `0x4EA1D0`: the actor is visible to the viewer or a kingdom sharing its
/// vision (an organization member answers for itself).
pub fn actor_visible(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> bool {
    let Some((e, m, _)) = mask(world, viewer) else { return true };
    e.actor_word(actor) & m & VISIBLE_ALL != 0
}

/// `0x4EA0D0`: the actor has been seen by the viewer's side.
pub fn actor_seen(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> bool {
    let Some((e, m, _)) = mask(world, viewer) else { return true };
    e.actor_word(actor) & m & EXPLORED_ALL != 0
}

/// `0x4D3B80` with `deep`: an organization is visible while its own word or
/// a member is.
pub fn org_visible(world: &World, org: OrgId, viewer: Option<KingdomId>) -> bool {
    let Some((e, m, _)) = mask(world, viewer) else { return true };
    e.org_visible(world, org, m)
}

/// The viewer's ghost record of a static actor it no longer sees.
pub fn memory(world: &World, actor: ActorId, viewer: Option<KingdomId>) -> Option<&Memory> {
    let (e, _, v) = mask(world, viewer)?;
    e.parts.memory.get(v as usize)?.get(&actor)
}

/// Every ghost record of the viewer.
pub fn memories(world: &World, viewer: Option<KingdomId>) -> Vec<(ActorId, &Memory)> {
    let Some((e, _, v)) = mask(world, viewer) else { return Vec::new() };
    e.parts.memory.get(v as usize).map(|m| m.iter().map(|(k, v)| (*k, v)).collect()).unwrap_or_default()
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
    let Some((e, m, _)) = mask(world, viewer) else { return Sighting::Visible };
    let Some(a) = world.actors.get(actor as usize) else { return Sighting::Hidden };
    let Some(owner) = a.kingdom else { return Sighting::Visible };
    if let Some(k) = e.kingdom(owner)
        && k.nonmajor == 0
        && m & 1u32.wrapping_shl(k.index) != 0
    {
        return Sighting::Visible;
    }
    if e.actor_word(actor) & m & VISIBLE_ALL != 0 {
        return Sighting::Visible;
    }
    match memory(world, actor, viewer) {
        Some(r) => Sighting::Remembered(r),
        None => Sighting::Hidden,
    }
}

/// The viewer's view of tile `(x, y)` (shared vision).
pub fn tile_view(world: &World, viewer: Option<KingdomId>, x: i32, y: i32) -> TileView {
    let Some((e, m, _)) = mask(world, viewer) else { return TileView::Visible };
    let st = &world.fog;
    if x < 0 || y < 0 || x >= st.width || y >= st.height {
        return TileView::Unexplored;
    }
    view(e.tile_fog((y * st.width + x) as usize), m)
}

fn view(w: u32, m: u32) -> TileView {
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
    let Some((e, m, _)) = mask(world, viewer) else {
        out.resize(n, 0);
        return;
    };
    out.extend((0..n).map(|t| match view(e.tile_fog(t), m) {
        TileView::Visible => 0,
        TileView::Explored => gray,
        TileView::Unexplored => 255,
    }));
}
