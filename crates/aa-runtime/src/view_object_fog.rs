//! Object fog-of-war leaves of AA.exe (image base 0x400000): the VObject
//! visibility virtuals vt+50 0x45BF20 and vt+54 0x45BD40 (both the Thing
//! and the Thing-less 0x4282A0/0x42A530/0x54F3E0 tile path), 0x45BBD0
//! PerformanceDisableDoodads, the GThing remembered leaf vt+60 0x4EAB20 with
//! 0x4EA350 / 0x4EA430, and the remembered aspect / data choices 0x4EAE20 /
//! 0x4EACC0 that the per-frame VObject update 0x459E10 applies (its 0x45C0F0
//! shows the view while vt+50(0) or vt+54(0) is true).
//!
//! Each kernel performs the native reads in native order through a trait;
//! nothing is defaulted. Proof: research/phase8-fog (cases view, thing_view,
//! doodads, remembered, potential, seen, apparent), replayed by
//! view_terrain_fog_tests.rs.
use super::Result;

/// GMap (`[B36534]+30`) words of 0x4282A0 / 0x42A530.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GMapWords {
    /// +4 world-to-tile shift (its low byte is the `sar` count).
    pub shift: u32,
    /// +C / +10 world extent (binary32).
    pub extent_bits: [u32; 2],
    /// +1C tiles per row.
    pub width: i32,
    /// Tile count (+1C * +20), for the index check of the Rust tile array.
    pub tiles: usize,
}

impl GMapWords {
    /// The SAME World GMap the live Detection tick writes (aa-zones keeps
    /// GMap +4 shift, +C/+10 extents and +1C/+20 tile counts).
    pub fn from_zones(zones: &aa_zones::tile::ZoneMap) -> Result<Self> {
        let g = zones.geometry;
        let tiles = usize::try_from(g.width).ok().zip(usize::try_from(g.height).ok()).and_then(|(w, h)| w.checked_mul(h));
        if tiles != Some(zones.tiles.len()) {
            return Err("GMap +1C * +20 differs from the zone tile array".into());
        }
        Ok(Self { shift: g.shift, extent_bits: g.extent_bits, width: g.width, tiles: zones.tiles.len() })
    }
}

/// The VObject words the two virtuals read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewWords {
    /// view+14 (GThing) is non-null.
    pub thing_14: bool,
    /// view+54 flags.
    pub flags_54: u32,
    /// view+38 / +3C position (binary32), read only by the Thing-less path
    /// (None: not lent by the caller, an error where 0x45BFA5 reads it).
    pub position_38: Option<[u32; 2]>,
}

/// Reads of 0x45BF20 / 0x45BD40.
pub trait ViewReads {
    type Kingdom: Copy + Eq;
    /// 0x45BBD0 (see [`doodads_45bbd0`]).
    fn doodads_45bbd0(&mut self) -> Result<bool>;
    /// 0x40CFA0.
    fn god_40cfa0(&mut self) -> Result<bool>;
    /// 0x5C0E10: B36550 non-null and its 0x76AC60 local player.
    fn local_player_5c0e10(&mut self) -> Result<bool>;
    /// 0x5C0E50 on the UI.
    fn kingdom_5c0e50(&mut self) -> Result<Option<Self::Kingdom>>;
    /// GThing vt+5C(k, a): 0x4EEF70 for every runtime class.
    fn thing_visible_5c(&mut self, k: Self::Kingdom, a: bool) -> Result<bool>;
    /// GThing vt+60(k, a): 0x4EAB20 (0x4CE890 jumps to it for GActors).
    fn thing_remembered_60(&mut self, k: Self::Kingdom, a: bool) -> Result<bool>;
    fn gmap(&mut self) -> Result<GMapWords>;
    /// `[[B39AB0]+108]` FogOfWar.
    fn fog_of_war(&mut self) -> Result<u32>;
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32>;
    fn kingdom_190(&mut self, k: Self::Kingdom) -> Result<u32>;
    fn kingdom_228(&mut self, k: Self::Kingdom) -> Result<u32>;
}

fn f32_bits(word: u32) -> Result<aa_numeric::Binary> {
    aa_numeric::Binary::from_f32_bits(word).map_err(|e| format!("view position binary32 {word:#010x}: {e}"))
}

/// 0x4282A0(x, y): `0 <= x < +C` and `0 <= y < +10` (ordered compares; a
/// NaN fails the first).
pub fn inside_4282a0(g: &GMapWords, xy: [u32; 2]) -> Result<bool> {
    for (v, extent) in xy.into_iter().zip(g.extent_bits) {
        // `fcomp 0.0; test ah,1` fails below or unordered; `fcomp +C;
        // test ah,5; jp` passes only an ordered below. Binary32 compares are
        // exact, so the IEEE comparisons below are the same predicate.
        let (v, e) = (f32::from_bits(v), f32::from_bits(extent));
        if !(v >= 0.0) || !(v < e) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 0x42A530(x, y): `(_ftol(x) >> s) + +1C * (_ftol(y) >> s)` (arithmetic
/// shifts by the low five bits of the +4 byte).
pub fn tile_42a530(g: &GMapWords, xy: [u32; 2]) -> Result<usize> {
    let shift = g.shift & 0x1f;
    let mut v = [0i32; 2];
    for (out, word) in v.iter_mut().zip(xy) {
        let t = f32_bits(word)?.truncate_i32().map_err(|e| format!("42A530 _ftol: {e}"))?;
        *out = t >> shift;
    }
    let index = g.width.wrapping_mul(v[1]).wrapping_add(v[0]);
    usize::try_from(index).ok().filter(|i| *i < g.tiles).ok_or_else(|| format!("42A530 tile {index} is outside the GMap tile array"))
}

/// 0x54F3E0(k) on a GMap tile: FogOfWar 0, a null k or k+190 answer true,
/// else `k+228 & tile+14 & 0xFFFF0000`.
fn tile_visible<R: ViewReads>(r: &mut R, tile: usize, k: R::Kingdom) -> Result<bool> {
    if r.fog_of_war()? == 0 {
        return Ok(true);
    }
    if r.kingdom_190(k)? != 0 {
        return Ok(true);
    }
    Ok(r.kingdom_228(k)? & r.tile_fog_14(tile)? & 0xffff_0000 != 0)
}

/// View vt+50 0x45BF20(a).
pub fn visible_45bf20<R: ViewReads>(r: &mut R, v: &ViewWords, a: bool) -> Result<bool> {
    if r.doodads_45bbd0()? {
        return Ok(false);
    }
    if r.god_40cfa0()? {
        return Ok(true);
    }
    if !r.local_player_5c0e10()? {
        return Ok(false);
    }
    if v.flags_54 & (1 << 26) != 0 || v.flags_54 & (1 << 20) != 0 {
        return Ok(true);
    }
    let Some(k) = r.kingdom_5c0e50()? else { return Ok(true) };
    if v.thing_14 {
        return r.thing_visible_5c(k, a);
    }
    let g = r.gmap()?;
    let xy = v.position_38.ok_or("45BFA5 reads view+38/+3C, which the caller did not lend")?;
    if !inside_4282a0(&g, xy)? {
        return Ok(false);
    }
    let tile = tile_42a530(&g, xy)?;
    tile_visible(r, tile, k)
}

/// View vt+54 0x45BD40(a).
pub fn remembered_45bd40<R: ViewReads>(r: &mut R, v: &ViewWords, a: bool) -> Result<bool> {
    if !r.local_player_5c0e10()? {
        return Ok(false);
    }
    if r.doodads_45bbd0()? {
        return Ok(false);
    }
    let k = r.kingdom_5c0e50()?;
    if !v.thing_14 {
        return Ok(false);
    }
    let Some(k) = k else { return Ok(false) };
    if v.flags_54 >> 20 & 1 != 0 {
        return Ok(false);
    }
    r.thing_remembered_60(k, a)
}

/// 0x45C0F0's show/hide decision after its +60 handle refresh: a dead view
/// (+54 bit 7) is hidden; otherwise vt+54(0) then vt+50(0), shown when
/// either is true.
pub fn shown_45c0f0<R: ViewReads>(r: &mut R, v: &ViewWords) -> Result<bool> {
    if v.flags_54 >> 7 & 1 != 0 {
        return Ok(false);
    }
    let remembered = remembered_45bd40(r, v, false)?;
    let visible = visible_45bf20(r, v, false)?;
    Ok(visible || remembered)
}

/// 0x45BBD0 (ESI = view+14 Thing, EDI = view+10 Data). `uvar` is the byte of
/// the lazily looked-up UVar record PerformanceDisableDoodads (a missing
/// record is a native NULL dereference: an error from the reader).
pub fn doodads_45bbd0(
    uvar: &mut dyn FnMut() -> Result<u8>,
    thing_flags_8: &mut dyn FnMut() -> Result<Option<u32>>,
    category_0c: &mut dyn FnMut() -> Result<u32>,
    data_doodad_40: &mut dyn FnMut() -> Result<bool>,
) -> Result<bool> {
    if uvar()? == 0 {
        return Ok(false);
    }
    if thing_flags_8()?.is_some_and(|f| f >> 10 & 1 != 0) {
        return Ok(false);
    }
    // 0x4BB3C0: Data+18 category +C is 1 or 0x26.
    if matches!(category_0c()?, 1 | 0x26) {
        return Ok(true);
    }
    data_doodad_40()
}

/// Reads of one GThing for 0x4EAB20 / 0x4EA350 / 0x4EA430 / 0x4EAE20.
pub trait ThingReads {
    type Kingdom: Copy;
    type Record: Copy;
    /// vt+5C(k, a) 0x4EEF70.
    fn visible_5c(&mut self, k: Option<Self::Kingdom>, a: bool) -> Result<bool>;
    /// vt+60(k, a) (0x4EAB20; see [`remembered_4eab20`]).
    fn remembered_60(&mut self, k: Option<Self::Kingdom>, a: bool) -> Result<bool>;
    /// vt+74(k, a) (0x4EA0D0 / 0x4D3C50).
    fn explored_74(&mut self, k: Option<Self::Kingdom>, a: bool) -> Result<bool>;
    /// Thing+8.
    fn flags_8(&mut self) -> Result<u32>;
    /// Data vt+38 / vt+40 nonzero.
    fn data_actor_38(&mut self) -> Result<bool>;
    fn data_doodad_40(&mut self) -> Result<bool>;
    /// Data+174 and Data+16D.
    fn data_mask_174(&mut self) -> Result<u32>;
    fn data_16d(&mut self) -> Result<u8>;
    /// vt+A8 and vt+8C (0x4EA3D0: Thing+58 > 0).
    fn vt_a8(&mut self) -> Result<bool>;
    fn vt_8c(&mut self) -> Result<bool>;
    fn kingdom_190(&mut self, k: Self::Kingdom) -> Result<u32>;
    fn kingdom_18c(&mut self, k: Self::Kingdom) -> Result<u32>;
    /// `[Thing+5C][index]` (None: NULL).
    fn memory_slot_5c(&mut self, index: u32) -> Result<Option<Self::Record>>;
    /// The direct call 0x4EA350 ([`potential_4ea350`]).
    fn call_4ea350(&mut self) -> Result<bool>
    where
        Self: Sized,
    {
        potential_4ea350(self)
    }
    /// The direct call 0x4EA430(k) ([`seen_4ea430`]).
    fn call_4ea430(&mut self, k: Option<Self::Kingdom>) -> Result<Option<Self::Record>>
    where
        Self: Sized,
    {
        seen_4ea430(self, k)
    }
}

/// 0x4EA350: may be drawn from memory although not visible now.
pub fn potential_4ea350<R: ThingReads>(r: &mut R) -> Result<bool> {
    if r.data_doodad_40()? {
        return Ok(false);
    }
    if r.data_actor_38()? && r.data_mask_174()? >> 26 & 1 != 0 {
        return Ok(false);
    }
    let f = r.flags_8()?;
    if f >> 29 & 1 == 0 || f >> 11 & 1 == 0 || f >> 23 & 1 != 0 || f >> 22 & 1 != 0 {
        return Ok(false);
    }
    if r.data_16d()? != 0 {
        return Ok(false);
    }
    Ok(!r.vt_a8()?)
}

/// 0x4EA430(k): `[Thing+5C][k+18C]` when k is non-null, k+190 == 0 and
/// vt+8C holds.
pub fn seen_4ea430<R: ThingReads>(r: &mut R, k: Option<R::Kingdom>) -> Result<Option<R::Record>> {
    let Some(k) = k else { return Ok(None) };
    if r.kingdom_190(k)? != 0 {
        return Ok(None);
    }
    if !r.vt_8c()? {
        return Ok(None);
    }
    let index = r.kingdom_18c(k)?;
    r.memory_slot_5c(index)
}

/// GThing vt+60 0x4EAB20(k, a).
pub fn remembered_4eab20<R: ThingReads>(r: &mut R, k: Option<R::Kingdom>, a: bool) -> Result<bool> {
    if r.visible_5c(k, a)? {
        return Ok(false);
    }
    if !r.explored_74(k, a)? {
        return Ok(false);
    }
    if r.call_4ea350()? {
        return Ok(true);
    }
    Ok(r.call_4ea430(k)?.is_some())
}

/// The three outcomes of 0x4EAE20 / 0x4EACC0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apparent<T> {
    /// 0x4EAAA0 (current aspect) / Thing+4 (current data).
    Current,
    /// The 0x4EA430(k) record's +18 aspect / +10 data.
    Remembered(T),
    /// B3CC04 / B3CBFC: zero-filled .bss, never stored (NULL).
    Default,
}

/// 0x4EAE20(k) and 0x4EACC0(k): vt+5C(k, 1), vt+60(k, 1); a dead (Thing+8
/// bit 7) visible Thing clears both; Data vt+38 with +174 bit 26 (flare)
/// keeps the current one.
pub fn apparent_4eae20<R: ThingReads>(r: &mut R, k: Option<R::Kingdom>) -> Result<Apparent<R::Record>> {
    let mut visible = r.visible_5c(k, true)?;
    let mut remembered = r.remembered_60(k, true)?;
    if visible && r.flags_8()? >> 7 & 1 != 0 {
        remembered = false;
        visible = false;
    }
    let flare = r.data_actor_38()? && r.data_mask_174()? >> 26 & 1 != 0;
    if visible {
        return Ok(Apparent::Current);
    }
    if remembered && r.call_4ea350()? {
        return Ok(Apparent::Current);
    }
    if flare {
        return Ok(Apparent::Current);
    }
    if remembered {
        // 0x4EAEA9 dereferences the record: NULL is a native fault.
        return r
            .call_4ea430(k)?
            .map(Apparent::Remembered)
            .ok_or_else(|| "4EAE20 reads +18 of a NULL 4EA430 record (native NULL dereference)".into());
    }
    Ok(Apparent::Default)
}
