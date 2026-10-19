//!VGrid view streaming: the VWorld+94 On (+100) and Near (+108) lists, the
//!VThing proximity +18/+1C, and the per-frame passes that drive them (AA.exe,
//!image base 0x400000). 4312F0 runs 42FF50 (TerrainMap+BC set), which calls
//!42FAD0 for each active terrain block before 720080, then tail-jumps to
//!42F670 every VWorld update.
//!
//!* 42FAD0(block): the VGrid cells under the block (+AC/+B0 times
//!  TerrainMap+A0, shifted by VGrid+24), each node's view tested as a sphere
//!  (42F4F0) against the VGrid+A0..+FC frustum planes: inside -> 45B2B0(2),
//!  leave Near (45B930), join On (45B8F0, +54 bit18); else, when the block's
//!  +B8 is non-zero -> 45B2B0(1), join Near (45B910, bit19).
//!* 42F670: B39BB8 counter rotation; Near walk then On walk, each view
//!  through 45B2E0 (vt+14 455290 -> 45B330 with the merged +1C, then +1C = 0);
//!  a Near view not at proximity 1 leaves; an On view at 2 runs vt+0, at 1
//!  moves to Near, otherwise leaves.
//!* 45B330 sets +18 with vt+D8 (45B3D0 -> vt+24 45B430: +58 count, the first
//!  request -> vt+D0 and Data vt+14), vt+DC (45B3E0 -> vt+28 45BB50: the last
//!  release -> +54 bit1 clear, vt+D4 and Data vt+18), vt+E0 / vt+E4.
//!
//!Proof: research/skirmish-20261019-view-streaming (original code under
//!Unicorn), replayed by view_streaming_tests.rs with the virtual call order.
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
pub type Result<T> = std::result::Result<T, String>;

///One terrain block as 42FAD0 reads it: +AC/+B0 block column/row, +B8 the
///state applied by the previous pass (720080 runs after 42FAD0), and
///[B36518]+A0 the TerrainMap block size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainBlockCell {
    pub column_ac: i32,
    pub row_b0: i32,
    pub state_b8: u32,
    pub size_a0: i32,
}
///The 4312F0 dispatcher's object side. 42FF50 calls `sweep_42fad0` for each
///active block at 0x42FFA8 (before 720080); 4312F0 tail-jumps to
///`traverse_42f670` at 0x431323, whether or not TerrainMap+BC ran 42FF50.
pub trait BlockObjects {
    ///`planes` are VWorld+94 +A0..+FC as 42EED0 copied them this pass.
    fn sweep_42fad0(&mut self, block: TerrainBlockCell, planes: &[[u32; 4]; 6]) -> Result<()>;
    fn traverse_42f670(&mut self) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    ///VGrid+100 (+54 bit18).
    On,
    ///VGrid+108 (+54 bit19).
    Near,
}
///VWorld+94 +100/+108 lists ({+0 tail, +4 head}) and VThing +4 next / +8
///prev / +C owning list, kept in head-first order.
pub struct Lists<V> {
    on: Vec<V>,
    near: Vec<V>,
}
impl<V> Default for Lists<V> {
    fn default() -> Self {
        Self { on: Vec::new(), near: Vec::new() }
    }
}
impl<V: Copy + Eq> Lists<V> {
    ///430290 stores both list heads and tails NULL.
    pub fn from_constructor() -> Self {
        Self::default()
    }
    ///VThing+C.
    pub fn list_of(&self, v: V) -> Option<List> {
        if self.on.contains(&v) {
            Some(List::On)
        } else if self.near.contains(&v) {
            Some(List::Near)
        } else {
            None
        }
    }
    ///Head (+4) first, the 42F670 walk order.
    pub fn head_first(&self, list: List) -> &[V] {
        match list {
            List::On => &self.on,
            List::Near => &self.near,
        }
    }
    ///Tail (+0) first through VThing+8, the 43D240 FindObjectAt order.
    pub fn tail_first(&self, list: List) -> Vec<V> {
        self.head_first(list).iter().rev().copied().collect()
    }
    fn vec(&mut self, list: List) -> &mut Vec<V> {
        match list {
            List::On => &mut self.on,
            List::Near => &mut self.near,
        }
    }
    ///45B0F0 (only reached with a non-NULL +C).
    fn unlink_45b0f0(&mut self, v: V) {
        for list in [List::On, List::Near] {
            let vec = self.vec(list);
            if let Some(i) = vec.iter().position(|x| *x == v) {
                vec.remove(i);
            }
        }
    }
    ///45B4C0: leave the current list, then append after the tail.
    fn append_45b4c0(&mut self, v: V, list: List) {
        if self.list_of(v).is_some() {
            self.unlink_45b0f0(v);
        }
        self.vec(list).push(v);
    }
    ///Remove a view whose allocation is being destroyed (45BCA0's 45B0F0).
    pub fn remove(&mut self, v: V) {
        self.unlink_45b0f0(v);
    }
}
///B39BB8 +24..+40: four (current, previous) counter pairs 42F670 rotates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub words: [u32; 8],
}
impl Stats {
    fn rotate(&mut self) {
        for pair in 0..4 {
            self.words[2 * pair + 1] = self.words[2 * pair];
            self.words[2 * pair] = 0;
        }
    }
    fn bump(&mut self, offset: u32) {
        let i = ((offset - 0x24) / 4) as usize;
        self.words[i] = self.words[i].wrapping_add(1);
    }
}

///The view vtable class (VObject A02798, VActor A02270, VOrg A01EF0): vt+B0
///is 6A71F0 for all three (445060 is never reached); vt+A8 is 6A71D0 only
///for VOrg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Object,
    Actor,
    Organization,
}
///One VThing's fields and virtuals at their native read/call positions.
pub trait Views {
    type V: Copy + Eq;
    ///VThing scalars +18, +1C, +38/+3C/+40, +54, +58, +74.
    fn word(&mut self, v: Self::V, offset: u32) -> Result<u32>;
    ///Stores to +18, +1C, +54, +58.
    fn store(&mut self, v: Self::V, offset: u32, value: u32) -> Result<()>;
    fn kind(&mut self, v: Self::V) -> Result<ViewKind>;
    ///view+9C, the attached VFX list, is non-NULL.
    fn attached_vfx_9c(&mut self, v: Self::V) -> Result<bool>;
    ///vt+3C / vt+40: vt+FC 5A4ED0 (view+7C Model) +74 and +78.
    fn model_74_78(&mut self, v: Self::V) -> Result<[u32; 2]>;
    ///43D050(xy, 0x222) -> VWorld+BC 431550.
    fn height_431550(&mut self, xy: [u32; 2]) -> Result<Binary>;
    ///VGrid+1C (columns) and +24 (coordinate shift).
    fn grid_word(&mut self, offset: u32) -> Result<u32>;
    ///VGrid+0 cell `index`: node views, head first.
    fn cell_views(&mut self, index: usize) -> Result<Vec<Self::V>>;
    fn update_0(&mut self, v: Self::V) -> Result<()>;
    fn model_request_d0(&mut self, v: Self::V, near: u32) -> Result<()>;
    fn model_release_d4(&mut self, v: Self::V) -> Result<()>;
    fn show_e0(&mut self, v: Self::V) -> Result<()>;
    fn hide_e4(&mut self, v: Self::V) -> Result<()>;
    ///view+10 Data vt+14(delta, mode) / vt+18(delta).
    fn data_request_14(&mut self, v: Self::V, delta: u32, mode: u32) -> Result<()>;
    fn data_release_18(&mut self, v: Self::V, delta: u32) -> Result<()>;
}

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| format!("view streaming binary32 {word:#010x}: {e}"))
}
fn st(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| format!("view streaming binary32 store: {e}"))
}
fn n<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
const HALF: u32 = 0x3f00_0000;
const MINUS_HALF: u32 = 0xbf00_0000;

///45B2B0(p): 2 always, 1 unless already 2.
fn request_45b2b0<W: Views>(w: &mut W, v: W::V, p: u32) -> Result<()> {
    if p == 2 {
        w.store(v, 0x1c, 2)
    } else if p == 1 && w.word(v, 0x1c)? != 2 {
        w.store(v, 0x1c, 1)
    } else {
        Ok(())
    }
}
fn flags<W: Views>(w: &mut W, v: W::V) -> Result<u32> {
    w.word(v, 0x54)
}
///45B930.
fn leave_45b930<W: Views>(lists: &mut Lists<W::V>, w: &mut W, v: W::V) -> Result<()> {
    let f = flags(w, v)?;
    w.store(v, 0x54, f & !0xc0000)?;
    if lists.list_of(v).is_some() {
        lists.unlink_45b0f0(v);
    }
    Ok(())
}
///45B8F0 / 45B910.
fn join<W: Views>(lists: &mut Lists<W::V>, w: &mut W, v: W::V, list: List) -> Result<()> {
    lists.append_45b4c0(v, list);
    let f = flags(w, v)?;
    w.store(v, 0x54, f | if list == List::On { 0x40000 } else { 0x80000 })
}
///vt+24 45B430(delta, near).
fn request_45b430<W: Views>(w: &mut W, v: W::V, delta: u32, near: u32) -> Result<()> {
    let count = w.word(v, 0x58)?.wrapping_add(delta);
    w.store(v, 0x58, count)?;
    if count == delta {
        w.model_request_d0(v, near)?;
        w.data_request_14(v, 1, near)?;
    }
    Ok(())
}
///vt+28 45BB50(delta).
fn release_45bb50<W: Views>(w: &mut W, v: W::V, delta: u32) -> Result<()> {
    let count = w.word(v, 0x58)?.wrapping_sub(delta);
    w.store(v, 0x58, count)?;
    if count == 0 {
        let f = flags(w, v)?;
        w.store(v, 0x54, f & !2)?;
        w.model_release_d4(v)?;
        w.data_release_18(v, 1)?;
    }
    Ok(())
}
///45B330(p): the proximity transition; false when unchanged.
pub(crate) fn proximity_45b330<W: Views>(w: &mut W, v: W::V, p: u32) -> Result<bool> {
    let old = w.word(v, 0x18)?;
    if p == old {
        return Ok(false);
    }
    match p {
        2 => {
            if old == 0 {
                request_45b430(w, v, 1, 1)?;
            }
            w.show_e0(v)?;
        }
        1 => {
            if old == 0 {
                request_45b430(w, v, 1, 0)?;
            } else {
                w.hide_e4(v)?;
            }
        }
        0 => {
            if old == 2 {
                w.hide_e4(v)?;
            }
            release_45bb50(w, v, 1)?;
        }
        _ => {}
    }
    w.store(v, 0x18, p)?;
    Ok(true)
}
///vt+14 455290(p, 0) (VActor/VOrg 451800 wraps it).
fn proximity_455290<W: Views>(w: &mut W, v: W::V, p: u32) -> Result<()> {
    if p == 0 && w.attached_vfx_9c(v)? {
        return Err("4552A7 42E8A0 over a non-NULL view+9C attached VFX list is not ported".into());
    }
    if !proximity_45b330(w, v, p)? && flags(w, v)? & 8 == 0 {
        return Ok(());
    }
    if p != 0 && w.attached_vfx_9c(v)? {
        return Err("4552DC 42E8A0 over a non-NULL view+9C attached VFX list is not ported".into());
    }
    let f = flags(w, v)?;
    w.store(v, 0x54, f & !8)
}
///45B2E0.
fn apply_45b2e0<W: Views>(w: &mut W, v: W::V) -> Result<()> {
    let p = w.word(v, 0x1c)?;
    proximity_455290(w, v, p)?;
    w.store(v, 0x1c, 0)
}
///42F4F0: inside unless one plane has `n.c - d <= -r` (unordered stays inside;
///finite binary32 inputs never produce it).
fn inside_42f4f0(planes: &[[u32; 4]; 6], sphere: [u32; 4], p: Precision) -> Result<bool> {
    let [x, y, z, r] = sphere.map(b);
    let (x, y, z, r) = (x?, y?, z?, r?);
    let minus = r.negated();
    for plane in planes {
        let [n0, n1, n2, c] = plane.map(b);
        let (n0, n1, n2, c) = (n0?, n1?, n2?, c?);
        let sum = n(n(n2.mul(&z, p))?.add(&n(n0.mul(&x, p))?, p))?;
        let dist = n(n(sum.add(&n(y.mul(&n1, p))?, p))?.sub(&c, p))?;
        if dist.compare(&minus) != Ordering::Greater {
            return Ok(false);
        }
    }
    Ok(true)
}
///42FAD0's sphere for one view: x, y, the half-height-centred z and radius.
fn sphere_42fad0<W: Views>(w: &mut W, v: W::V, p: Precision) -> Result<[u32; 4]> {
    let mut radius = w.word(v, 0x74)?;
    let [top, bottom] = w.model_74_78(v)?;
    let extent = n(b(top)?.sub(&b(bottom)?, p))?;
    if extent.compare(&b(radius)?) == Ordering::Greater {
        radius = st(&extent)?;
    }
    let x = w.word(v, 0x38)?;
    let y = w.word(v, 0x3c)?;
    let height = w.height_431550([x, y])?;
    let z40 = b(w.word(v, 0x40)?)?;
    let above = n(z40.sub(&height, p))?;
    let above32 = st(&above)?;
    let offset = if above.compare(&b(radius)?) == Ordering::Greater {
        let half = b(above32)?;
        radius = st(&n(half.mul(&b(HALF)?, p))?)?;
        n(half.mul(&b(MINUS_HALF)?, p))?
    } else {
        b(0)?
    };
    let z = st(&n(offset.add(&z40, p))?)?;
    Ok([x, y, z, radius])
}
///42FAD0(block).
pub fn sweep_42fad0<W: Views>(
    lists: &mut Lists<W::V>,
    w: &mut W,
    block: TerrainBlockCell,
    planes: &[[u32; 4]; 6],
    p: Precision,
) -> Result<()> {
    let size = block.size_a0;
    let col = block.column_ac.wrapping_mul(size);
    let row = block.row_b0.wrapping_mul(size);
    let shift = w.grid_word(0x24)? & 31;
    let (x0, x1) = (col >> shift, size.wrapping_add(col) >> shift);
    let (y0, y1) = (row >> shift, row.wrapping_add(size) >> shift);
    for y in y0..y1 {
        for x in x0..x1 {
            let columns = w.grid_word(0x1c)? as i32;
            let index = usize::try_from(columns.wrapping_mul(y).wrapping_add(x))
                .map_err(|_| "42FB48 VGrid cell index is outside the allocation".to_string())?;
            for v in w.cell_views(index)? {
                let sphere = sphere_42fad0(w, v, p)?;
                if inside_42f4f0(planes, sphere, p)? {
                    request_45b2b0(w, v, 2)?;
                    if flags(w, v)? & 0x80000 != 0 {
                        leave_45b930(lists, w, v)?;
                    }
                    if flags(w, v)? & 0xc0000 == 0 {
                        join(lists, w, v, List::On)?;
                    }
                } else if block.state_b8 != 0 {
                    request_45b2b0(w, v, 1)?;
                    if flags(w, v)? & 0xc0000 == 0 {
                        join(lists, w, v, List::Near)?;
                    }
                }
            }
        }
    }
    Ok(())
}
///42F670.
pub fn traverse_42f670<W: Views>(lists: &mut Lists<W::V>, stats: &mut Stats, w: &mut W) -> Result<()> {
    stats.rotate();
    for v in lists.head_first(List::Near).to_vec() {
        apply_45b2e0(w, v)?;
        if w.word(v, 0x18)? == 1 {
            stats.bump(0x2c);
            if w.kind(v)? == ViewKind::Organization {
                stats.bump(0x3c);
            }
        } else {
            leave_45b930(lists, w, v)?;
        }
    }
    for v in lists.head_first(List::On).to_vec() {
        apply_45b2e0(w, v)?;
        match w.word(v, 0x18)? {
            2 => w.update_0(v)?,
            1 => {
                leave_45b930(lists, w, v)?;
                join(lists, w, v, List::Near)?;
                stats.bump(0x2c);
                if w.kind(v)? == ViewKind::Organization {
                    stats.bump(0x3c);
                }
            }
            _ => leave_45b930(lists, w, v)?,
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "view_streaming_tests.rs"]
mod tests;
