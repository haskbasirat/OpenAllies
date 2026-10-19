//! Terrain fog of war: the per-TerrainBlock fog pixel grids of AA.exe
//! (image base 0x400000), ported from the original machine code.
//!
//! Native owners and order (one VWorld update 0x43EC60):
//!
//! * 0x4312F0 (0x43ED0A) ends its 0x42FF50 block pass with 0x7208D0 for
//!   every active block. For a state-2 block (+B8 == 2) 0x7208D0 runs
//!   0x71B500 (debug-UVar change: +14B), 0x7203F0 for each of the block's
//!   (d0 + 2)^2 cells (+88, d0 = +D0 gtiles per block axis), then the fog
//!   half: when +14C, `b403c9`, +142 or +141 is set it advances the fade
//!   timer +108 by TerrainMap+13C and calls 0x71C290(1 - +108 / fade, rebuild).
//!   0x71C290 blends the n*n pixels (+124 NiPixelData, +12C = n) from +10C
//!   (previous) toward +118 (target); a rebuild resamples the targets with
//!   0x71BEF0, which darkens a cell's 0x71BD00 brightness (0 unexplored,
//!   `b285d4` explored only, 0xFF visible) toward darker neighbours (0x71BE70).
//! * 0x725340 (0x43ED15): `b403c9` = 0, TerrainMap+138/+13C (seconds and
//!   delta from 0x59ABA0), +140 local kingdom (0x5C0E50; a change sets
//!   `b403c9`), `b285d4` = _ftol(AVar64 TerrainFogGrayPercentage * 255), and
//!   the ViewShowFogDebug (UVar31) toggle of `b403c8` (sets `b403c9`).
//! * 0x42FE10 (inside 0x42FF50, before the 0x7208D0 loop): entering state 2
//!   is Request 0x721180 (0x720670 builds +88 when it is null), Load 0x7212D0
//!   (0x720AD0 when the mesh +44 is null: its tail 0x720C79.. sizes the fog
//!   grid +12C/+130/+134/+13C, creates +124 zeroed and refills +10C/+118 with
//!   zeros) and 0x71CCB0 (+141 = 1). Leaving state 2 releases the mesh
//!   (0x7211F0 -> 0x721030(0) frees +124/+128 when the request count is 0).
//!
//! Proof: research/phase8-fog (the original code under Unicorn, both pinned
//! images, PC24/PC53/PC64), replayed by view_terrain_fog_tests.rs.
//!
//! Not ported here (explicit errors where reached): 0x71CE70 (the debug
//! overlay rebuild after +14B), 0x71F3B0/0x71C470 (RenderDrawTerrainGrid),
//! 0x71FFA0 (a GMap tile variation change seen by 0x7203F0), the 0x54EF80
//! zone queries of the settlement/guard/control debug views, and the
//! 0x71BD00 world-sampling path (reached only when +B8 != 2 or +88 is null,
//! never from 0x7208D0). The 0x720670 terrain texture-use calls 0x71FEE0
//! and the 0x720AD0 mesh/texture-effect construction belong to the terrain
//! renderer, not to this state.
//!
//! Removed caps: none; the grids are growable vectors sized from the native
//! values (the 32-bit products are checked where native code would wrap).
use crate::player_order_input::terrain_map::TerrainMap;
use crate::terrain_visibility::TerrainVisibility;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, String>;

#[path = "view_object_fog.rs"]
pub mod object;
#[path = "view_terrain_fog_live.rs"]
pub mod live;

const ONE: u32 = 0x3f80_0000; // 0x9F32E0
const HALF: u32 = 0x3f00_0000; // 0x9FBDB8
const NEAR_A03078: u32 = 0x3c23_d70a; // 0.01
const FAR_A326BC: u32 = 0x3f66_6666; // 0.9
const DEBUG_A88ACC: u32 = 0xbf5f_dfe0; // -223/255 (ViewShowFogDebug remap)
const SCALE_9FD5D4: u32 = 0x437f_0000; // 255.0
const MILLI_9F4580: u32 = 0x3a83_126f; // 0.001
const TWO32_9F240C: u32 = 0x4f80_0000; // 2^32
/// Cell +C bit 5: unexplored by the local kingdom (0x7203F0).
pub const UNEXPLORED: u8 = 0x20;
/// Cell +C bit 4: explored but not visible.
pub const EXPLORED_ONLY: u8 = 0x10;

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| format!("terrain fog binary32 {word:#010x}: {e}"))
}
fn st(v: &Binary) -> Result<u32> {
    v.store_f32().map_err(|e| format!("terrain fog fstp m32: {e}"))
}
fn n<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| format!("terrain fog x87: {e}"))
}
/// 0x96CC4C `_ftol`: truncation toward zero of ST0. Results outside the
/// signed 32-bit domain (where the native 64-bit EDX:EAX would be used) and
/// non-finite values are outside the proved domain.
fn ftol(v: &Binary, site: &str) -> Result<i32> {
    v.truncate_i32().map_err(|e| format!("{site} _ftol 96CC4C: {e}"))
}
fn less(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == Ordering::Less
}

// ------------------------------------------------------------ globals --

/// Process-image globals of the fog code. B403C8/B403C9 are .bss (zero);
/// B285D4 is .data 0x80 in AA.exe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Globals {
    /// B403C8: the ViewShowFogDebug remap is active (read by 0x71C290).
    pub fog_debug_b403c8: u8,
    /// B403C9: every state-2 block re-blends this frame (set by 0x725340).
    pub reblend_b403c9: u8,
    /// B285D4: explored-only brightness (0x71BD00).
    pub explored_b285d4: u8,
}
impl Globals {
    pub fn from_fresh_process_image() -> Self {
        Self { fog_debug_b403c8: 0, reblend_b403c9: 0, explored_b285d4: 0x80 }
    }
}

/// The three AVars the fog code reads, registered by 0x401BD0:
/// index 0x3D TerrainFogPixelsPerMeter = int 2 (0x403464), 0x40
/// TerrainFogGrayPercentage = float 0.5 (0x403530), 0x41 TerrainFogFadeTime =
/// float 0.5 (0x403576). Read as `[[B39990]+F4]`, `+100` and `+104`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FogVariables {
    pub pixels_per_meter_61: i32,
    pub gray_percentage_64: u32,
    pub fade_time_65: u32,
}
pub const AVAR_NAMES: [&str; 3] = ["TerrainFogPixelsPerMeter", "TerrainFogGrayPercentage", "TerrainFogFadeTime"];
impl FogVariables {
    /// The 0x401BD0 constructor values.
    pub fn constructor() -> Self {
        Self { pixels_per_meter_61: 2, gray_percentage_64: HALF, fade_time_65: HALF }
    }
    /// The current values after the native AVars*.tgi load: the constructor
    /// values when no source file assigns them. An assignment would need the
    /// native AVar scalar parser for these names, which is not bound here.
    pub fn from_sources(
        catalog: &crate::catalog::Catalog<'_>,
        files: &mut aa_rules::sources::V14Sources,
        overrides: crate::load_view_variables::NativeOverrides,
    ) -> Result<Self> {
        use aa_rules::tgi::{Document, Item, Limits};
        catalog.require_source_owner()?;
        if !matches!(overrides, crate::load_view_variables::NativeOverrides::ControlledNone) {
            return Err("terrain fog AVars: the current profile/console AVar owner is unbound".into());
        }
        let names = files.catalog().into_iter().map(|e| e.logical_path).collect::<Vec<_>>();
        let ordered = aa_rules::native::index::native_enumerate("AVars*.tgi", &names);
        if !ordered.iter().any(|p| p.eq_ignore_ascii_case("AVars.tgi")) {
            return Err("terrain fog AVars: the root AVars source is absent".into());
        }
        for path in ordered {
            let file = files.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("terrain fog AVars: source hash differs".into());
            }
            let doc = Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc.header_name(&node.header).is_some_and(|v| v.eq_ignore_ascii_case("Vars")) {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(property) = item else { continue };
                    let Some(name) = doc.property_name(property) else { continue };
                    if AVAR_NAMES.iter().any(|n| name.eq_ignore_ascii_case(n)) {
                        return Err(format!("terrain fog AVar {name} is assigned in {path}; its native scalar parse is not bound"));
                    }
                }
            }
        }
        Ok(Self::constructor())
    }
}

// ------------------------------------------------------------ the map --

/// TerrainMap +138 (seconds), +13C (frame delta) and +140 (the local
/// kingdom of the previous 0x725340), all zero from 0x721330.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFog<K> {
    pub seconds_138: u32,
    pub delta_13c: u32,
    pub kingdom_140: Option<K>,
}
impl<K: Copy + Eq> MapFog<K> {
    pub fn from_721330() -> Self {
        Self { seconds_138: 0, delta_13c: 0, kingdom_140: None }
    }
    /// The fog parts of 0x725340, in native order. `milliseconds` is the
    /// 0x59ABA0 (B396E0) result, `kingdom` the 0x5C0E50 result, `gray` the
    /// AVar64 word and `fog_debug_31` the UVar31 ViewShowFogDebug byte. The
    /// RenderDrawRegionLines, +BF, +C0 (TerrainMap::update_725340), +101,
    /// 0x72EF70 and +14C branches between them read and write none of these.
    pub fn update_725340(&mut self, g: &mut Globals, milliseconds: u32, kingdom: Option<K>, gray: u32, fog_debug_31: u8, p: Precision) -> Result<()> {
        g.reblend_b403c9 = 0;
        // 0x72535D..0x72537D.
        let mut t = Binary::from_i32(milliseconds as i32);
        if (milliseconds as i32) < 0 {
            t = n(t.add(&b(TWO32_9F240C)?, p))?;
        }
        let t = n(t.mul(&b(MILLI_9F4580)?, p))?;
        let delta = n(t.sub(&b(self.seconds_138)?, p))?;
        self.delta_13c = st(&delta)?;
        self.seconds_138 = st(&t)?;
        if kingdom != self.kingdom_140 {
            self.kingdom_140 = kingdom;
            g.reblend_b403c9 = 1;
        }
        // 0x7253D8..0x7253F3: the byte store of the _ftol result.
        let gray = n(b(gray)?.mul(&b(SCALE_9FD5D4)?, p))?;
        g.explored_b285d4 = ftol(&gray, "725340 b285d4")? as u8;
        // 0x72546D..0x725489.
        if g.fog_debug_b403c8 != fog_debug_31 {
            g.fog_debug_b403c8 = u8::from(g.fog_debug_b403c8 == 0);
            g.reblend_b403c9 = 1;
        }
        Ok(())
    }
}

// ------------------------------------------------------------- blocks --

/// One +88 cell (16 bytes): +0 snapshot of tile+0 (the variation pointer),
/// +4 the GMap tile (here its linear index), +8 the owning TerrainBlock
/// (here its cell index), +C flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub snapshot_0: u64,
    pub tile_4: Option<usize>,
    pub block_8: Option<usize>,
    pub flags_c: u8,
}

/// The block's fog pixel grid: +124 NiPixelData (RGBA, n*n, rows = native
/// y), +10C previous and +118 target bytes, +12C n, +130 n - 1, +134
/// pixels per gtile, +13C sample step in gtiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub n_12c: i32,
    pub w130: i32,
    pub w134: i32,
    pub step_13c: u32,
    pub pixels: Vec<[u8; 4]>,
    pub prev_10c: Vec<u8>,
    pub target_118: Vec<u8>,
    /// Increments of +124+4C since 0x720AD0 created +124.
    pub revisions: u32,
}

/// 0x720C79..0x720CC3 (720AD0): `+134 = UVar0 * AVar61`, `+12C = d0 * +134`,
/// `+130 = +12C - 1`, `+13C = fstp(fild d0 / fidiv +130)`; then 0x875AC0
/// creates +124 (n x n, zero-filled at 0x720D2B) and 0x5ECBF0 fills +10C and
/// +118 with n*n zeros.
pub fn grid_720ad0(d0: i32, quad_size_uvar0: i32, pixels_per_meter_61: i32, p: Precision) -> Result<Grid> {
    let w134 = quad_size_uvar0.wrapping_mul(pixels_per_meter_61);
    let size = d0.wrapping_mul(w134);
    let w130 = size.wrapping_sub(1);
    if w130 == 0 {
        return Err("720AD0 fidiv +130 by zero (a 1x1 fog grid) is outside the proved domain".into());
    }
    let step = n(Binary::from_i32(d0).div(&Binary::from_i32(w130), p))?;
    let step_13c = st(&step)?;
    if size <= 0 {
        return Err(format!("875AC0 fog pixel data of {size}x{size} is outside the proved domain"));
    }
    let count = size.checked_mul(size).ok_or("720AD0 n*n fog grid wraps 32 bits")? as usize;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(count).map_err(|_| "fog pixel allocation failed")?;
    pixels.resize(count, [0; 4]);
    Ok(Grid { n_12c: size, w130, w134, step_13c, pixels, prev_10c: vec![0; count], target_118: vec![0; count], revisions: 0 })
}

/// GMap and TerrainMap words 0x720670 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellLayout {
    /// GMap +1C / +20 tile counts.
    pub tiles: [i32; 2],
    /// TerrainMap +84 blocks per row and +A8 / +B4 (gtiles per block).
    pub blocks_x: i32,
    pub quads_a8: i32,
    pub quads_per_gtile_b4: i32,
}

/// 0x720670 (from Request 0x721180 while +88 is null): (d0 + 2)^2 zeroed
/// cells, rows outer; an in-map cell gets its tile (+4), the tile's +0
/// (+0), and `[TerrainMap+B8][+84 * (gy / q) + gx / q]` (+8), q = +A8 / +B4;
/// every cell gets flag 0x20.
pub fn cells_720670(d0: i32, index: [i32; 2], layout: CellLayout, variation: &mut dyn FnMut(usize) -> Result<u64>) -> Result<Vec<Cell>> {
    let side = d0.wrapping_add(2);
    let count = side.wrapping_mul(side);
    if side <= 0 || count <= 0 || count.checked_mul(16).and_then(|c| c.checked_add(4)).is_none() {
        return Err(format!("720670 cell array of {side}^2 is outside the proved domain"));
    }
    let q = match layout.quads_a8.checked_div(layout.quads_per_gtile_b4) {
        Some(q) => q,
        None => return Err("720670 idiv TerrainMap+A8 / +B4 raises #DE".into()),
    };
    let mut cells = Vec::new();
    cells.try_reserve_exact(count as usize).map_err(|_| "720670 cell allocation failed")?;
    let gx0 = index[0].wrapping_mul(d0).wrapping_sub(1);
    let gy0 = index[1].wrapping_mul(d0).wrapping_sub(1);
    for yy in 0..side {
        let gy = gy0.wrapping_add(yy);
        for xx in 0..side {
            let gx = gx0.wrapping_add(xx);
            let mut cell = Cell { snapshot_0: 0, tile_4: None, block_8: None, flags_c: 0 };
            if gx >= 0 && gx < layout.tiles[0] && gy >= 0 && gy < layout.tiles[1] {
                let tile = layout.tiles[0].wrapping_mul(gy).wrapping_add(gx) as usize;
                cell.tile_4 = Some(tile);
                cell.snapshot_0 = variation(tile)?;
                let (by, bx) = match (gy.checked_div(q), gx.checked_div(q)) {
                    (Some(by), Some(bx)) => (by, bx),
                    _ => return Err("720670 idiv by q raises #DE".into()),
                };
                cell.block_8 = Some(layout.blocks_x.wrapping_mul(by).wrapping_add(bx) as usize);
            }
            cell.flags_c |= UNEXPLORED;
            cells.push(cell);
        }
    }
    Ok(cells)
}

/// UVar bytes the fog code reads (`[[B39940]+4*slot]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Uvar {
    /// 22 ViewShowBlocking (+58).
    ShowBlocking22,
    /// 25 ViewShowSettlementDebug (+64).
    SettlementDebug25,
    /// 27 ViewShowGuardDebug (+6C).
    GuardDebug27,
    /// 29 ViewShowControlDebug (+74).
    ControlDebug29,
    /// 31 ViewShowFogDebug (+7C).
    FogDebug31,
    /// 100 RenderDrawTerrainGrid (+190).
    TerrainGrid100,
    /// 102 RenderDrawAlphaTransitions (+198).
    AlphaTransitions102,
}

/// The owners 0x7208D0 / 0x7203F0 read, one method per native read. Each
/// call is made exactly where (and only when) the native code reads it.
pub trait Reads {
    type Kingdom: Copy + Eq;
    /// 0x40FCA0(B39690, 2, B43B40): Session+E4 == 3 or 0x40EAA0.
    fn debug_gate_40fca0(&mut self) -> Result<bool>;
    /// The byte of the 0x4B2E60 record "DebugObjectBlocking" (B40464).
    fn debug_object_blocking(&mut self) -> Result<u8>;
    /// The byte of the record "DebugWaterBlocking" (B40460).
    fn debug_water_blocking(&mut self) -> Result<u8>;
    fn uvar_byte(&mut self, slot: Uvar) -> Result<u8>;
    /// Tile +20 (blocking word of the debug views).
    fn tile_blocking_20(&mut self, tile: usize) -> Result<u32>;
    /// Tile +4 (zone set) is non-null.
    fn tile_zone_set_4(&mut self, tile: usize) -> Result<bool>;
    /// 0x54EF80(tile, bit, 0, 0) nonzero.
    fn zone_debug_54ef80(&mut self, tile: usize, bit: u8) -> Result<bool>;
    /// 0x40CFA0 (B36534 GWorld present and 0x4F0930 fog cheat).
    fn god_40cfa0(&mut self) -> Result<bool>;
    /// 0x5C0E50 on the UI (B3653C).
    fn kingdom_5c0e50(&mut self) -> Result<Option<Self::Kingdom>>;
    /// `[[B39AB0]+108]` FogOfWar SVar.
    fn fog_of_war(&mut self) -> Result<u32>;
    /// Tile +14 fog word.
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32>;
    fn kingdom_190(&mut self, k: Self::Kingdom) -> Result<u32>;
    fn kingdom_228(&mut self, k: Self::Kingdom) -> Result<u32>;
    /// Tile +0 (variation pointer identity).
    fn tile_variation_0(&mut self, tile: usize) -> Result<u64>;
    /// `[[B39990]+104]` AVar65 TerrainFogFadeTime word.
    fn fade_time_65(&mut self) -> Result<u32>;
    /// B36550 Session +E4 (None: NULL B36550), read by 0x40CEC0 / 0x40CEE0.
    fn application_state_e4(&mut self) -> Result<Option<u32>>;
}

/// 0x54F3E0(k) on tile: FogOfWar 0 answers true, else 0x54EB70.
fn visible_54f3e0<R: Reads>(r: &mut R, tile: usize, k: Option<R::Kingdom>) -> Result<bool> {
    if r.fog_of_war()? == 0 {
        return Ok(true);
    }
    let Some(k) = k else { return Ok(true) };
    if r.kingdom_190(k)? != 0 {
        return Ok(true);
    }
    Ok(r.kingdom_228(k)? & r.tile_fog_14(tile)? & 0xffff_0000 != 0)
}
/// 0x54ED10 -> 0x54EB40 (no FogOfWar test).
fn explored_54ed10<R: Reads>(r: &mut R, tile: usize, k: Option<R::Kingdom>) -> Result<bool> {
    let Some(k) = k else { return Ok(true) };
    if r.kingdom_190(k)? != 0 {
        return Ok(true);
    }
    Ok(r.kingdom_228(k)? & r.tile_fog_14(tile)? & 0xffff != 0)
}
/// 0x4B6F90(mask, on).
fn set_bits(flags: &mut u8, mask: u8, on: bool) {
    if on {
        *flags |= mask;
    } else {
        *flags &= !mask;
    }
}

/// The block fields 0x7208D0 and its callees read and write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockFog {
    pub index: [i32; 2],
    /// +88 (None: null).
    pub cells: Option<Vec<Cell>>,
    /// +124 and its arrays (None: +124 null).
    pub grid: Option<Grid>,
    /// +108 fade timer (binary32 word).
    pub fade_108: u32,
    pub loaded_141: u8,
    pub fading_142: u8,
    /// +143 terrain-grid lines built (0x71F3B0, never run here).
    pub terrain_grid_143: u8,
    /// +145..+14A cached debug UVars (written by 0x71CE70, never run here).
    pub debug_cache_145: [u8; 6],
    pub debug_14b: u8,
    pub fog_14c: u8,
}
impl BlockFog {
    /// The 0x723960 constructor values of these fields.
    pub fn from_723960(index: [i32; 2]) -> Self {
        Self { index, cells: None, grid: None, fade_108: 0, loaded_141: 0, fading_142: 0, terrain_grid_143: 0, debug_cache_145: [0; 6], debug_14b: 0, fog_14c: 0 }
    }

    /// 0x7203F0(cell, block) for cell `i`.
    pub fn cell_7203f0<R: Reads>(&mut self, i: usize, r: &mut R) -> Result<()> {
        let cells = self.cells.as_mut().ok_or("7203F0 reads +88 of a block without cells")?;
        let cell = cells.get_mut(i).ok_or("7203F0 cell index outside +88")?;
        if cell.block_8.is_none() {
            return Ok(());
        }
        let tile = cell.tile_4.ok_or("7203F0 cell +8 is set while its +4 tile is null")?;
        let old = cell.flags_c;
        let mut f = 0u8;
        if r.debug_gate_40fca0()? {
            if r.debug_object_blocking()? != 0 {
                set_bits(&mut f, 1, r.tile_blocking_20(tile)? >> 10 & 1 != 0);
            } else if r.debug_water_blocking()? != 0 {
                set_bits(&mut f, 1, r.tile_blocking_20(tile)? >> 9 & 1 != 0);
            } else if r.uvar_byte(Uvar::ShowBlocking22)? != 0 {
                set_bits(&mut f, 1, r.tile_blocking_20(tile)? >> 8 & 1 != 0);
            }
            if r.tile_zone_set_4(tile)? {
                if r.uvar_byte(Uvar::SettlementDebug25)? != 0 {
                    set_bits(&mut f, 2, r.zone_debug_54ef80(tile, 2)?);
                }
                if r.uvar_byte(Uvar::GuardDebug27)? != 0 {
                    set_bits(&mut f, 4, r.zone_debug_54ef80(tile, 5)?);
                }
                if r.uvar_byte(Uvar::ControlDebug29)? != 0 {
                    set_bits(&mut f, 8, r.zone_debug_54ef80(tile, 4)?);
                }
            }
        }
        if !r.god_40cfa0()? {
            let k = r.kingdom_5c0e50()?;
            let visible = visible_54f3e0(r, tile, k)?;
            let explored = explored_54ed10(r, tile, k)?;
            set_bits(&mut f, UNEXPLORED, k.is_some() && !visible && !explored);
            let explored_only = f & UNEXPLORED == 0 && k.is_some() && !visible;
            set_bits(&mut f, EXPLORED_ONLY, explored_only);
        }
        cell.flags_c = f;
        if (f ^ old) & 0x0f != 0 {
            self.debug_14b = 1;
        }
        if (f ^ old) & 0x30 != 0 {
            self.fog_14c = 1;
        }
        if cell.snapshot_0 != r.tile_variation_0(tile)? {
            return Err("7203F0 tile variation changed: 71FFA0 (terrain texture/water/neighbour refresh) is not ported".into());
        }
        Ok(())
    }

    /// 0x71B500: any of UVar22/102/25/27/29/31 differing (as a boolean)
    /// from its cache byte +146/+145/+147/+148/+149/+14A sets +14B.
    pub fn debug_71b500<R: Reads>(&mut self, r: &mut R) -> Result<()> {
        let c = self.debug_cache_145;
        for (slot, cache) in [
            (Uvar::ShowBlocking22, c[1]),
            (Uvar::AlphaTransitions102, c[0]),
            (Uvar::SettlementDebug25, c[2]),
            (Uvar::GuardDebug27, c[3]),
            (Uvar::ControlDebug29, c[4]),
            (Uvar::FogDebug31, c[5]),
        ] {
            if (r.uvar_byte(slot)? != 0) != (cache != 0) {
                self.debug_14b = 1;
                return Ok(());
            }
        }
        Ok(())
    }

    /// 0x7208D0 except its 0x720A93 water tail (TerrainMap::water_7208d0),
    /// which reads and writes none of these fields. `state_b8` is the
    /// block's applied visibility state.
    pub fn update_7208d0<R: Reads>(&mut self, state_b8: u32, d0: i32, g: &Globals, delta_13c: u32, r: &mut R, p: Precision) -> Result<()> {
        if state_b8 != 2 {
            return Ok(());
        }
        self.debug_71b500(r)?;
        let count = d0.wrapping_add(2).wrapping_mul(d0.wrapping_add(2));
        for i in 0..count.max(0) as usize {
            self.cell_7203f0(i, r)?;
        }
        if self.fog_14c != 0 || g.reblend_b403c9 != 0 || self.fading_142 != 0 || self.loaded_141 != 0 {
            let fade_word = r.fade_time_65()?;
            let mut rebuild = false;
            if self.fog_14c != 0 || self.loaded_141 != 0 {
                self.fog_14c = 0;
                rebuild = true;
                self.fade_108 = fade_word;
                self.fading_142 = 1;
            }
            let v = n(b(self.fade_108)?.sub(&b(delta_13c)?, p))?;
            self.fade_108 = st(&v)?;
            let clear = self.loaded_141 != 0
                || v.compare(&Binary::from_i32(0)) != Ordering::Greater
                || matches!(r.application_state_e4()?, Some(3))
                || matches!(r.application_state_e4()?, Some(5));
            if clear {
                self.fading_142 = 0;
                self.fade_108 = 0;
            }
            let fade = b(fade_word)?;
            if fade.is_zero() {
                return Err("7208D0 fdiv by a zero TerrainFogFadeTime is outside the proved domain".into());
            }
            let t = n(b(ONE)?.sub(&n(b(self.fade_108)?.div(&fade, p))?, p))?;
            self.blend_71c290(d0, st(&t)?, rebuild, g, p)?;
        }
        if self.debug_14b != 0 {
            self.debug_14b = 0;
            return Err("7208D0 +14B: the 71CE70 debug overlay rebuild is not ported".into());
        }
        // +C4 is created only by 0x71CE70 (never run): null, no 0x71CE70.
        let grid_lines = r.uvar_byte(Uvar::TerrainGrid100)?;
        if grid_lines != 0 && self.terrain_grid_143 == 0 {
            return Err("7208D0 RenderDrawTerrainGrid: 71F3B0 is not ported".into());
        }
        if grid_lines == 0 && self.terrain_grid_143 != 0 {
            return Err("7208D0 RenderDrawTerrainGrid: 71C470 is not ported".into());
        }
        self.loaded_141 = 0;
        Ok(())
    }

    /// 0x71C290(t, rebuild) on this block.
    pub fn blend_71c290(&mut self, d0: i32, t_bits: u32, rebuild: bool, g: &Globals, p: Precision) -> Result<()> {
        let cells = self.cells.as_deref();
        let grid = self.grid.as_mut().ok_or("71C290 reads +124 of a block without a fog grid")?;
        let size = grid.n_12c;
        let step = b(grid.step_13c)?;
        let t = b(t_bits)?;
        let view = CellView { d0, cells, b285d4: g.explored_b285d4 };
        for x in 0..size.max(0) {
            let fx = st(&n(Binary::from_i32(x).mul(&step, p))?)?;
            for y in 0..size {
                let fy = st(&n(Binary::from_i32(y).mul(&step, p))?)?;
                let i = size.wrapping_mul(y).wrapping_add(x) as usize;
                if rebuild {
                    grid.prev_10c[i] = grid.pixels[i][0];
                    grid.target_118[i] = sample_71bef0(&view, fx, fy, p)?;
                }
                let prev = i32::from(grid.prev_10c[i]);
                let diff = i32::from(grid.target_118[i]) - prev;
                let v = n(n(Binary::from_i32(diff).mul(&t, p))?.add(&Binary::from_i32(prev), p))?;
                let al = ftol(&v, "71C35B")? as u8;
                grid.pixels[i][3] = 0xff;
                let c = if g.fog_debug_b403c8 != 0 {
                    let v = n(Binary::from_i32(i32::from(al)).mul(&b(DEBUG_A88ACC)?, p))?;
                    0x20u8.wrapping_sub(ftol(&v, "71C384")? as u8)
                } else {
                    al
                };
                grid.pixels[i][0] = c;
                grid.pixels[i][1] = c;
                grid.pixels[i][2] = c;
            }
        }
        grid.revisions = grid.revisions.wrapping_add(1);
        Ok(())
    }
}

/// The block fields 0x71BD00 reads (+B8 is 2 inside 0x71C290).
pub struct CellView<'a> {
    pub d0: i32,
    pub cells: Option<&'a [Cell]>,
    pub b285d4: u8,
}
/// 0x71BD00(x, y): 0 outside the (d0+2)^2 cells, else from +88[(d0+2)y+x]:
/// 0 when 0x20, B285D4 when 0x10, else 0xFF.
pub fn cell_71bd00(v: &CellView<'_>, x: i32, y: i32) -> Result<u8> {
    let side = v.d0.wrapping_add(2);
    if x < 0 || x >= side || y < 0 || y >= side {
        return Ok(0);
    }
    let cells = v.cells.ok_or("71BD00 with a null +88: the 5543F0 world-sampling path is not ported (unreachable from 7208D0)")?;
    let cell = cells
        .get(side.wrapping_mul(y).wrapping_add(x) as usize)
        .ok_or("71BD00 cell outside +88")?;
    Ok(if cell.flags_c & UNEXPLORED != 0 {
        0
    } else if cell.flags_c & EXPLORED_ONLY != 0 {
        v.b285d4
    } else {
        0xff
    })
}

/// 0x71BE70(&res, _, neighbour, d2): darken toward a darker neighbour.
pub fn darken_71be70(res: &mut u8, neighbour: u8, d2_bits: u32, p: Precision) -> Result<()> {
    if neighbour == 0xff {
        return Ok(());
    }
    let d2 = b(d2_bits)?;
    if !less(&d2, &b(FAR_A326BC)?) {
        return Ok(());
    }
    let bl = *res;
    if bl <= neighbour {
        return Ok(());
    }
    if less(&d2, &b(NEAR_A03078)?) {
        *res = neighbour;
        return Ok(());
    }
    let root = n(d2.sqrt(p))?;
    let v = n(n(b(ONE)?.sub(&root, p))?.mul(&Binary::from_i32(255 - i32::from(neighbour)), p))?;
    let al = ftol(&v, "71BECB")? as u8;
    if al >= bl {
        *res = 0;
        return Ok(());
    }
    *res = (i32::from(bl) - i32::from(al)).max(i32::from(neighbour)) as u8;
    Ok(())
}

/// 0x71BEF0(fx, fy): one fog sample (gtile units within the block).
pub fn sample_71bef0(v: &CellView<'_>, fx_bits: u32, fy_bits: u32, p: Precision) -> Result<u8> {
    let fx = b(fx_bits)?;
    let fy = b(fy_bits)?;
    let ix = ftol(&fx, "71BEFC")?;
    let x1 = ix.wrapping_add(1);
    let iy = ftol(&fy, "71BF0C")?;
    let y1 = iy.wrapping_add(1);
    let mut res = cell_71bd00(v, x1, y1)?;
    if res == 0 {
        return Ok(0);
    }
    let one = b(ONE)?;
    let half = b(HALF)?;
    let fx1 = n(fx.add(&one, p))?;
    let fy1 = n(fy.add(&one, p))?;
    let x1f = Binary::from_i32(x1);
    let a = b(st(&n(fx1.sub(&x1f, p))?)?)?;
    let y1f = b(st(&Binary::from_i32(y1))?)?;
    let bb = b(st(&n(fy1.sub(&y1f, p))?)?)?;
    let c = b(st(&n(n(x1f.add(&one, p))?.sub(&fx1, p))?)?)?;
    let d = b(st(&n(n(y1f.add(&one, p))?.sub(&fy1, p))?)?)?;
    let e = b(st(&n(c.sub(&half, p))?)?)?;
    let f = b(st(&n(d.sub(&half, p))?)?)?;
    let b2 = b(st(&n(bb.mul(&bb, p))?)?)?;
    let a2 = b(st(&n(a.mul(&a, p))?)?)?;
    let near = |x: i32, y: i32, d2: &Binary, res: &mut u8| -> Result<()> { darken_71be70(res, cell_71bd00(v, x, y)?, st(d2)?, p) };
    // (ix, iy): A2 + B2.
    near(ix, iy, &n(a2.add(&b2, p))?, &mut res)?;
    // (ix, y1): rnd(F*F) + A2; F2 stored.
    let ff = n(f.mul(&f, p))?;
    let f2 = b(st(&ff)?)?;
    near(ix, y1, &n(ff.add(&a2, p))?, &mut res)?;
    // (ix, iy+2): D2 + A2.
    let iy2 = iy.wrapping_add(2);
    let d2 = b(st(&n(d.mul(&d, p))?)?)?;
    near(ix, iy2, &n(d2.add(&a2, p))?, &mut res)?;
    // (x1, iy): rnd(E*E) + B2; E2 stored.
    let ee = n(e.mul(&e, p))?;
    let e2 = b(st(&ee)?)?;
    near(x1, iy, &n(ee.add(&b2, p))?, &mut res)?;
    // (x1, iy+2): E2 + D2.
    near(x1, iy2, &n(e2.add(&d2, p))?, &mut res)?;
    // (ix+2, iy / y1 / iy+2): C2 + B2, C2 + F2, C2 + D2.
    let ix2 = ix.wrapping_add(2);
    let c2 = b(st(&n(c.mul(&c, p))?)?)?;
    near(ix2, iy, &n(c2.add(&b2, p))?, &mut res)?;
    near(ix2, y1, &n(c2.add(&f2, p))?, &mut res)?;
    near(ix2, iy2, &n(c2.add(&d2, p))?, &mut res)?;
    Ok(res)
}

// ---------------------------------------------------------- map owner --

/// The fog state of one TerrainMap (B36518), paired with its visibility
/// owner (VWorld+94). Blocks are in the map's row-major cell order.
pub struct TerrainFog<K> {
    map: std::sync::Weak<()>,
    blocks_xy: [i32; 2],
    d0: i32,
    block_size_a0: i32,
    layout: CellLayout,
    pub blocks: Vec<BlockFog>,
    pub clock: MapFog<K>,
    /// Per block: the +B8 state and mesh residency last observed after a
    /// 0x42FF50 pass (both 0/absent after 0x723E00).
    observed: Vec<(u32, bool)>,
}

impl<K: Copy + Eq> TerrainFog<K> {
    /// For the TerrainMap built by 0x723E00. `gtile_bits` is GMap+8 and
    /// `quad_size_ac` the TerrainMap+AC (UVar0 at 0x72422B) the map was
    /// built with; `tiles` the GMap +1C/+20 counts.
    pub fn for_map(map: &TerrainMap, gtile_bits: u32, quad_size_ac: i32, tiles: [i32; 2]) -> Result<Self> {
        let gtile = b(gtile_bits)?.truncate_i32().map_err(|e| format!("GMap+8 gtile: {e}"))?;
        let quads_per_gtile = gtile.checked_div(quad_size_ac).ok_or("723E00 +B4 idiv raises #DE")?;
        let a0 = map.block_size_a0();
        let quads = a0.checked_div(quad_size_ac).ok_or("TerrainMap +AC is zero")?;
        if quads.wrapping_mul(quad_size_ac) != a0 {
            return Err("TerrainMap +A0 is not +A8 * +AC for this quad size".into());
        }
        let d0 = quads.checked_div(quads_per_gtile).ok_or("723AB2 +D0 idiv raises #DE")?;
        let blocks_xy = map.blocks_xy();
        if d0 <= 0 || blocks_xy[0].checked_mul(d0) != Some(tiles[0]) || blocks_xy[1].checked_mul(d0) != Some(tiles[1]) {
            return Err(format!("TerrainMap blocks {blocks_xy:?} x d0 {d0} do not cover the GMap {tiles:?}"));
        }
        let count = map.block_count();
        Ok(Self {
            map: std::sync::Arc::downgrade(map.identity()),
            blocks_xy,
            d0,
            block_size_a0: a0,
            layout: CellLayout { tiles, blocks_x: blocks_xy[0], quads_a8: quads, quads_per_gtile_b4: quads_per_gtile },
            blocks: (0..count).map(|cell| BlockFog::from_723960(map.block_index(cell))).collect(),
            clock: MapFog::from_721330(),
            observed: vec![(0, false); count],
        })
    }
    pub fn d0(&self) -> i32 {
        self.d0
    }
    fn check_map(&self, map: &TerrainMap) -> Result<()> {
        if !self.map.upgrade().is_some_and(|id| std::sync::Arc::ptr_eq(&id, map.identity())) {
            return Err("terrain fog belongs to another TerrainMap allocation".into());
        }
        Ok(())
    }

    /// Request 0x721180's 0x720670 (when +88 is null).
    pub fn request_721180<R: Reads>(&mut self, cell: usize, r: &mut R) -> Result<()> {
        let layout = self.layout;
        let d0 = self.d0;
        let block = self.blocks.get_mut(cell).ok_or("block index outside the TerrainMap")?;
        if block.cells.is_none() {
            block.cells = Some(cells_720670(d0, block.index, layout, &mut |t| r.tile_variation_0(t))?);
        }
        Ok(())
    }
    /// 0x7212D0 -> 0x720AD0 (when the mesh +44 was null): the fog grid.
    pub fn load_720ad0(&mut self, cell: usize, quad_size_uvar0: i32, vars: &FogVariables, p: Precision) -> Result<()> {
        let d0 = self.d0;
        let block = self.blocks.get_mut(cell).ok_or("block index outside the TerrainMap")?;
        block.grid = Some(grid_720ad0(d0, quad_size_uvar0, vars.pixels_per_meter_61, p)?);
        Ok(())
    }
    /// 0x71CCB0's +141 = 1.
    pub fn loaded_71ccb0(&mut self, cell: usize) -> Result<()> {
        self.blocks.get_mut(cell).ok_or("block index outside the TerrainMap")?.loaded_141 = 1;
        Ok(())
    }
    /// 0x721030(0): +124 freed (and +128); +88, +10C and +118 remain.
    pub fn release_721030(&mut self, cell: usize) -> Result<()> {
        self.blocks.get_mut(cell).ok_or("block index outside the TerrainMap")?.grid = None;
        Ok(())
    }

    /// The fog parts of one 0x42FF50 pass, called right after
    /// [`TerrainVisibility::update_4312f0`] of EVERY VWorld update (before
    /// 0x725340). Each block's 0x42FE10 events are recovered from its +B8
    /// change and mesh residency since the previous pass (a block changes
    /// state at most once per pass); then 0x7208D0's fog half runs for the
    /// active list. Blocks share no fog state, so their relative order is
    /// not observable here.
    pub fn after_42ff50<R: Reads<Kingdom = K>>(
        &mut self,
        vis: &TerrainVisibility,
        map: &TerrainMap,
        globals: &Globals,
        vars: &FogVariables,
        quad_size_uvar0: &mut dyn FnMut() -> Result<i32>,
        r: &mut R,
        p: Precision,
    ) -> Result<()> {
        self.check_map(map)?;
        for cell in 0..self.blocks.len() {
            let state = vis.block(cell).ok_or("visibility owner has fewer blocks than the map")?.state_b8;
            let resident = map.resident(cell);
            let (old_state, old_resident) = self.observed[cell];
            if state == 2 && old_state != 2 {
                self.request_721180(cell, r)?;
                if !old_resident {
                    if !resident {
                        return Err("42FE10 state 2 left the block without its 720AD0 mesh".into());
                    }
                    self.load_720ad0(cell, quad_size_uvar0()?, vars, p)?;
                }
                self.loaded_71ccb0(cell)?;
            } else if old_state == 2 && state != 2 && old_resident && !resident {
                self.release_721030(cell)?;
            }
            self.observed[cell] = (state, resident);
        }
        let delta = self.clock.delta_13c;
        for &cell in vis.active() {
            let state = vis.block(cell).ok_or("active block outside the map")?.state_b8;
            self.blocks[cell].update_7208d0(state, self.d0, globals, delta, r, p)?;
        }
        Ok(())
    }

    /// The whole map's fog pixels for the renderer: block (c, r) occupies
    /// texels [c*n, c*n+n) x [r*n, r*n+n), rows = native y. Blocks without
    /// a +124 grid are not drawn by the original (they are outside the
    /// camera footprint) and read 0.
    pub fn compose(&self) -> Result<FogImage> {
        let mut size = None;
        for block in &self.blocks {
            if let Some(grid) = &block.grid {
                match size {
                    None => size = Some(grid.n_12c),
                    Some(s) if s == grid.n_12c => {}
                    Some(_) => return Err("terrain fog blocks have different +12C sizes (UVar0 changed between loads); one map texture cannot hold them".into()),
                }
            }
        }
        let Some(size) = size else {
            return Ok(FogImage { size: [0, 0], pixels_per_block: 0, block_size_a0: self.block_size_a0, blocks: self.blocks_xy, luminance: Vec::new(), revision: 0 });
        };
        let w = u32::try_from(self.blocks_xy[0].checked_mul(size).ok_or("fog texture width overflows")?).map_err(|_| "fog texture width")?;
        let h = u32::try_from(self.blocks_xy[1].checked_mul(size).ok_or("fog texture height overflows")?).map_err(|_| "fog texture height")?;
        let mut luminance = Vec::new();
        luminance.try_reserve_exact(w as usize * h as usize).map_err(|_| "fog texture allocation failed")?;
        luminance.resize(w as usize * h as usize, 0);
        let mut revision = 0u64;
        for (i, block) in self.blocks.iter().enumerate() {
            let Some(grid) = &block.grid else { continue };
            revision = revision.wrapping_add(u64::from(grid.revisions)).wrapping_mul(31).wrapping_add(i as u64 + 1);
            let [c, r] = block.index;
            for y in 0..size {
                let row = (r * size + y) as usize * w as usize + (c * size) as usize;
                for x in 0..size {
                    luminance[row + x as usize] = grid.pixels[(y * size + x) as usize][0];
                }
            }
        }
        Ok(FogImage { size: [w, h], pixels_per_block: size as u32, block_size_a0: self.block_size_a0, blocks: self.blocks_xy, luminance, revision })
    }
}

/// The map-wide fog pixels (R of each RGBA pixel; R = G = B).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FogImage {
    pub size: [u32; 2],
    pub pixels_per_block: u32,
    /// TerrainMap +A0 world units per block.
    pub block_size_a0: i32,
    pub blocks: [i32; 2],
    pub luminance: Vec<u8>,
    /// Changes whenever any block's +124+4C revision or presence changes.
    pub revision: u64,
}

#[cfg(test)]
#[path = "view_terrain_fog_tests.rs"]
mod tests;
