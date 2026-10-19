//! The native camera-frustum terrain visibility pass that drives TerrainBlock
//! residency (AA.exe, image base 0x400000). VWorld+0x94 (ctor 0x430290) is
//! updated once per VWorld update 0x43EC60, at 0x43ED0A, right before the
//! TerrainMap update 0x725340 at 0x43ED15:
//!
//! * 0x4312F0(vis): 0x42EED0(vis, *[B40048]) copies the camera eye +90,
//!   direction +114 and the six frustum planes +19C..+1F8; while
//!   TerrainMap+BC is set, it casts the four corner rays +2EC/+2D4/+2E0/+2F8
//!   through 0x71D840 into VWorld+144..+170 (0x43B2F0). It then intersects them
//!   (0x42ED60, behind accepted) with the up-normal planes (0x869C90) at
//!   the base heightfield [GWorld+34]+48 (vis+58/+64/+70/+7C) and +4C
//!   (vis+28/+34/+40/+4C). A parallel ray leaves the old point.
//! * 0x430E50: the six-point footprint (max 2EC, min 2EC, 2D4, 2E0, 2F8,
//!   max 2F8) in block units (`fstp(1 / fstp(fild +A0))`), its floor/ceil
//!   bounding box widened by one block, then the 0x430710 scanline
//!   rasterizer with 0x42F330 edge crossings. 0x430080 requests state 2,
//!   0x4300D0 requests state 1, and 0x42F1F0 tests the block's 0x71AFD0
//!   sphere against the six planes (0x42ED10).
//!   0x42FE80/0x71A230 store the request (+BC) and push the block at the
//!   front of the active list (vis+4/+8, +140).
//! * 0x42FF50 (TerrainMap+BC set): for each active block in list order,
//!   0x42FAD0 (objects, not ported), then 0x720080: +B4 = +B8, +B8 = +BC on a
//!   change, queued by 0x42F980 (state 2 appended, others pushed in front);
//!   +BC = 0. State-0 blocks leave the list (0x42FED0). Then 0x42FE10, in
//!   queue order: state 2 -> Request 0x721180 (count +4 += 1) + Load 0x7212D0
//!   (0x720AD0 when +44 is null) + 0x71CCB0; state 0/1 after 2 -> 0x71CD00 +
//!   Release 0x7211F0 (count -= 1; 0x721030 when it reaches zero). Finally
//!   0x7208D0 for every active block (water rebuild at 0x720A93).
//! * 0x42F670 (the VGrid view-cell traversal) at the end of 0x4312F0.
//!
//! Split with the view-cell owner: this module owns 0x42EED0, 0x430E50,
//! 0x430710 and its callbacks, and the block half of 0x42FF50 (0x720080,
//! 0x42FED0, 0x42FE10 load/release, 0x7208D0 water). The 0x42FF50
//! dispatcher is here; it calls [`ViewCellHooks::sweep_42fad0`] for each
//! active block in list order, right before that block's 0x720080, and
//! 0x4312F0 ends with [`ViewCellHooks::traverse_42f670`] (always, also
//! while TerrainMap+BC is clear).
//!
//! The terrain block resources ("Dynamic/Terrain x%d y%d", 0x723D30 ->
//! 0x5ACEC0) have no counterpart in the Rust ResourceManager (Resources has
//! no dynamic Kind), so the 0x5AA3F0/0x5AA550 request count +4 is kept
//! here. 0x42FE10 is its only requester found.
use crate::live_world_frame::World;
use crate::player_order_input::terrain_map::{Heights, TerrainMap, Tiles};
use crate::player_order_input::terrain_ray::{intersect_71d840, plane_42ed60, Plane};
use crate::view_heightfield;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
use std::collections::VecDeque;

type Result<T> = std::result::Result<T, String>;

const ONE: u32 = 0x3f80_0000;
/// 42EED0 corner-ray order (camera +2EC, +2D4, +2E0, +2F8) as indices into
/// [`CameraWords::rays`] (+2D4, +2E0, +2EC, +2F8).
const RAY_ORDER: [usize; 4] = [2, 0, 1, 3];

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| format!("terrain visibility binary32 {word:#010x}: {e}"))
}
fn st(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| format!("terrain visibility binary32 store: {e}"))
}
fn n<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
fn less(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == Ordering::Less
}
fn greater(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == Ordering::Greater
}

/// The current camera's words that 0x4312F0 reads (`*[B40048]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraWords {
    /// +0x90.
    pub eye: [u32; 3],
    /// +0x114.
    pub direction: [u32; 3],
    /// +0x19C..+0x1F8: six NiPlanes (normal, constant).
    pub planes: [[u32; 4]; 6],
    /// +0x2D4, +0x2E0, +0x2EC, +0x2F8 corner ray directions.
    pub rays: [[u32; 3]; 4],
}
impl CameraWords {
    pub fn read(camera: &crate::native_camera::State) -> Result<Self> {
        let w3 = |o: u32| -> Result<[u32; 3]> { Ok([camera.word(o)?, camera.word(o + 4)?, camera.word(o + 8)?]) };
        let mut planes = [[0; 4]; 6];
        for (i, plane) in planes.iter_mut().enumerate() {
            let o = 0x19c + 16 * i as u32;
            *plane = [camera.word(o)?, camera.word(o + 4)?, camera.word(o + 8)?, camera.word(o + 12)?];
        }
        Ok(Self {
            eye: w3(0x90)?,
            direction: w3(0x114)?,
            planes,
            rays: [w3(0x2d4)?, w3(0x2e0)?, w3(0x2ec)?, w3(0x2f8)?],
        })
    }
    /// The directory's current (`[B40048]`) normalized camera.
    pub fn current<K, R>(directory: &crate::native_camera::directory::Directory<K, R>) -> Result<Self> {
        let handle = directory.current_handle()?;
        Self::read(directory.camera(&handle)?.normalized_state()?)
    }
}

/// The view-cell half of the same per-frame family (VWorld+94 VGrid
/// +0..+24 and the object lists +100..+10C), owned by the view module.
pub trait ViewCellHooks {
    /// 0x42FAD0(vis, block) from 0x42FF50, per active block in list order,
    /// before 0x720080: block (+AC, +B0) at TerrainMap+A0 world units, with
    /// its pre-apply +B8 (the previous pass's state).
    fn sweep_42fad0(&mut self, index: [i32; 2], block_size_a0: i32, state_b8: u32) -> Result<()>;
    /// 0x42F670(vis): the tail jump of 0x4312F0.
    fn traverse_42f670(&mut self) -> Result<()>;
}

/// Visibility fields of one TerrainBlock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockVisibility {
    /// +0xBC request of the current pass (0x71A230), cleared by 0x720080.
    pub request_bc: u32,
    /// +0xB8 applied state and +0xB4 previous state.
    pub state_b8: u32,
    pub previous_b4: u32,
    /// +0x140: in the active list.
    pub active_140: bool,
    /// Resource +4 request count (0x5AA3F0 / 0x5AA550).
    pub requests_4: i32,
}

/// One VWorld+0x94 owner, paired with one TerrainMap from its construction.
pub struct TerrainVisibility {
    map: std::sync::Weak<()>,
    eye: Option<[u32; 3]>,
    direction: Option<[u32; 3]>,
    planes: Option<[[u32; 4]; 6]>,
    /// vis+28/+34/+40/+4C (base max plane) and vis+58/+64/+70/+7C (base
    /// min plane), rays in 42EED0 order; `None` is never written.
    max_hits: [Option<[u32; 3]>; 4],
    min_hits: [Option<[u32; 3]>; 4],
    /// vis+4 head .. vis+8 tail.
    active: Vec<usize>,
    cells: Vec<BlockVisibility>,
    /// VWorld+144..+170: the four 0x71D840 corner hits (unwritten words are
    /// `None`); an error where the Rust 0x71D840 refuses the native domain.
    footprint: Option<std::result::Result<[[Option<u32>; 3]; 4], String>>,
}

/// One rasterization request of 0x430710's callbacks.
struct Raster<'a> {
    vis: &'a mut TerrainVisibility,
    map: &'a TerrainMap,
    blocks: [i32; 2],
    planes: [[u32; 4]; 6],
    p: Precision,
}
impl Raster<'_> {
    fn cell(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.blocks[0] && y < self.blocks[1]).then(|| (self.blocks[0] * y + x) as usize)
    }
    /// 0x430080 (state 2) / 0x4300D0 (state 1): 0x42FE80 for an in-grid cell.
    fn mark(&mut self, x: i32, y: i32, state: u32) {
        if let Some(cell) = self.cell(x, y) {
            self.vis.request_42fe80(cell, state);
        }
    }
    /// 0x42F1F0: in the grid and not outside any frustum plane (0x42ED10).
    fn test(&self, x: i32, y: i32) -> Result<bool> {
        let Some(cell) = self.cell(x, y) else { return Ok(false) };
        let sphere = self.map.sphere_e8(cell);
        for plane in &self.planes {
            if outside_42ed10(sphere, *plane, self.p)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// 0x42ED10: `((nz*cz + ny*cy) + nx*cx) - k <= -r` (ordered) is outside (2).
fn outside_42ed10(sphere: [u32; 4], plane: [u32; 4], p: Precision) -> Result<bool> {
    let [nx, ny, nz, k] = [b(plane[0])?, b(plane[1])?, b(plane[2])?, b(plane[3])?];
    let [cx, cy, cz, r] = [b(sphere[0])?, b(sphere[1])?, b(sphere[2])?, b(sphere[3])?];
    let d = n(n(n(n(nz.mul(&cz, p))?.add(&n(ny.mul(&cy, p))?, p))?.add(&n(nx.mul(&cx, p))?, p))?.sub(&k, p))?;
    Ok(!greater(&d, &r.negated()))
}

/// 0x869C90(normal, point): normal copied, constant
/// `fstp((nz*pz + ny*py) + nx*px)`.
fn plane_869c90(normal: [u32; 3], point: [u32; 3], p: Precision) -> Result<Plane> {
    let [nx, ny, nz] = [b(normal[0])?, b(normal[1])?, b(normal[2])?];
    let [px, py, pz] = [b(point[0])?, b(point[1])?, b(point[2])?];
    let k = n(n(n(nz.mul(&pz, p))?.add(&n(ny.mul(&py, p))?, p))?.add(&n(nx.mul(&px, p))?, p))?;
    Ok(Plane { normal, constant: st(&k)? })
}

/// 0x42F330(a, b, c, d, out, t): the a-b segment parameter against the
/// c-d segment, both within [0, 1]; out = a + s (b - a).
fn segment_42f330(a: [u32; 2], bp: [u32; 2], c: [u32; 2], d: [u32; 2], p: Precision) -> Result<Option<([u32; 2], u32)>> {
    let e = n(b(d[1])?.sub(&b(c[1])?, p))?;
    let dxf = b(st(&n(b(bp[0])?.sub(&b(a[0])?, p))?)?)?;
    let dy = n(b(bp[1])?.sub(&b(a[1])?, p))?;
    let cxf = b(st(&n(b(d[0])?.sub(&b(c[0])?, p))?)?)?;
    let den = st(&n(n(dxf.mul(&e, p))?.sub(&n(cxf.mul(&dy, p))?, p))?)?;
    let den = b(den)?;
    if den.is_zero() {
        return Ok(None);
    }
    let ay = n(b(a[1])?.sub(&b(c[1])?, p))?;
    let ax = n(b(a[0])?.sub(&b(c[0])?, p))?;
    let inv = n(b(ONE)?.div(&den, p))?;
    let s = b(st(&n(n(n(ay.mul(&cxf, p))?.sub(&n(ax.mul(&e, p))?, p))?.mul(&inv, p))?)?)?;
    let u = b(st(&n(n(n(ay.mul(&dxf, p))?.sub(&n(ax.mul(&dy, p))?, p))?.mul(&inv, p))?)?)?;
    let (zero, one) = (Binary::from_i32(0), b(ONE)?);
    if less(&s, &zero) || greater(&s, &one) || less(&u, &zero) || greater(&u, &one) {
        return Ok(None);
    }
    let x = st(&n(n(dxf.mul(&s, p))?.add(&b(a[0])?, p))?)?;
    let y = st(&n(n(n(b(bp[1])?.sub(&b(a[1])?, p))?.mul(&s, p))?.add(&b(a[1])?, p))?)?;
    Ok(Some(([x, y], st(&s)?)))
}

/// CRT floor 0x96D5E0 / ceil 0x96D4C0 of the promoted binary32, then _ftol.
fn round_ftol(word: u32, up: bool) -> Result<i32> {
    let v = f64::from(f32::from_bits(word));
    if !v.is_finite() {
        return Err(format!("430E50 footprint bound {word:#010x}: native nonfinite floor/ceil/_ftol"));
    }
    let r = if up { v.ceil() } else { v.floor() };
    if r < f64::from(i32::MIN) || r > f64::from(i32::MAX) {
        return Err(format!("430E50 footprint bound {word:#010x}: _ftol integer indefinite"));
    }
    Ok(r as i32)
}

impl TerrainVisibility {
    /// 0x430290 for the TerrainMap just built by 0x723E00: no 0x42FE10 pass
    /// has run, so every block's +44 is still null (reported absent).
    pub fn for_map(map: &mut TerrainMap) -> Result<Self> {
        map.report_all_absent_after_723e00()?;
        Ok(Self {
            map: std::sync::Arc::downgrade(map.identity()),
            eye: None,
            direction: None,
            planes: None,
            max_hits: [None; 4],
            min_hits: [None; 4],
            active: Vec::new(),
            cells: vec![BlockVisibility::default(); map.block_count()],
            footprint: None,
        })
    }
    pub fn block(&self, cell: usize) -> Option<&BlockVisibility> {
        self.cells.get(cell)
    }
    /// The active list, head (vis+4) first.
    pub fn active(&self) -> &[usize] {
        &self.active
    }
    pub fn footprint(&self) -> Option<&std::result::Result<[[Option<u32>; 3]; 4], String>> {
        self.footprint.as_ref()
    }
    /// vis+28/+34/+40/+4C then vis+58/+64/+70/+7C.
    pub fn hits(&self) -> ([Option<[u32; 3]>; 4], [Option<[u32; 3]>; 4]) {
        (self.max_hits, self.min_hits)
    }

    /// 0x4312F0 over the live owners: the SAME World (base heightfield
    /// +48/+4C limits, GMap tiles for 0x7208D0), the SAME VWorld+BC
    /// heightfield the map was built from, and the current camera words.
    /// Call [`TerrainMap::update_725340`] afterwards (0x43ED15).
    pub fn update_4312f0(
        &mut self,
        map: &mut TerrainMap,
        world: &World<'_>,
        height: &view_heightfield::State,
        camera: &CameraWords,
        views: &mut impl ViewCellHooks,
        pc: Precision,
    ) -> Result<()> {
        let heights = map.bound(world, height)?;
        let parts = world.presentation_parts()?;
        let base = &parts.terrain.base.state;
        let limits = [base.minimum_bits, base.maximum_bits];
        let tiles = map.world_tiles(world)?;
        self.pass_4312f0(map, camera, limits, heights, &tiles, views, pc)
    }

    pub(crate) fn pass_4312f0(
        &mut self,
        map: &mut TerrainMap,
        camera: &CameraWords,
        base_limits: [u32; 2],
        heights: Heights<'_>,
        tiles: &impl Tiles,
        views: &mut impl ViewCellHooks,
        p: Precision,
    ) -> Result<()> {
        if !self.map.upgrade().is_some_and(|id| std::sync::Arc::ptr_eq(&id, map.identity())) {
            return Err("VWorld+94 visibility belongs to another TerrainMap allocation".into());
        }
        // 0x42EED0.
        self.eye = Some(camera.eye);
        self.direction = Some(camera.direction);
        self.planes = Some(camera.planes);
        if map.loaded_byte() != 0 {
            self.footprint = Some(footprint_42eed0(map, camera, p));
        }
        let up = map.up_words();
        for (max, h) in [(false, base_limits[0]), (true, base_limits[1])] {
            let plane = plane_869c90(up, [0, 0, h], p)?;
            for (slot, &ray) in RAY_ORDER.iter().enumerate() {
                if let Some(hit) = plane_42ed60(&plane, camera.eye, camera.rays[ray], true, p)? {
                    if max {
                        self.max_hits[slot] = Some(hit);
                    } else {
                        self.min_hits[slot] = Some(hit);
                    }
                }
            }
        }
        self.footprint_430e50(map, p)?;
        if map.loaded_byte() != 0 {
            self.apply_42ff50(map, heights, tiles, views, p)?;
        }
        views.traverse_42f670()
    }

    /// 0x42FE80(block, state) with 0x71A230.
    fn request_42fe80(&mut self, cell: usize, state: u32) {
        let c = &mut self.cells[cell];
        if state == 2 || state == 1 {
            c.request_bc = state;
        }
        if !c.active_140 {
            self.active.insert(0, cell);
            c.active_140 = true;
        }
    }

    /// 0x430E50: footprint polygon, bounding box, 0x430710.
    fn footprint_430e50(&mut self, map: &TerrainMap, p: Precision) -> Result<()> {
        let unwritten = |name: &str| format!("430E50 reads VWorld+94 {name}, never written by 42ED60 (parallel corner ray)");
        let source = [
            self.max_hits[0].ok_or_else(|| unwritten("+28"))?,
            self.min_hits[0].ok_or_else(|| unwritten("+58"))?,
            self.min_hits[1].ok_or_else(|| unwritten("+64"))?,
            self.min_hits[2].ok_or_else(|| unwritten("+70"))?,
            self.min_hits[3].ok_or_else(|| unwritten("+7C"))?,
            self.max_hits[3].ok_or_else(|| unwritten("+4C"))?,
        ];
        // 0x430E7C..0x430EB1: inv = fstp(1 / fstp(fild +A0)); x, y use it.
        let bs = b(st(&Binary::from_i32(map.block_size_a0()))?)?;
        let inv = b(st(&n(b(ONE)?.div(&bs, p))?)?)?;
        let mut points = [[0u32; 2]; 6];
        for (point, hit) in points.iter_mut().zip(source) {
            *point = [st(&n(inv.mul(&b(hit[0])?, p))?)?, st(&n(inv.mul(&b(hit[1])?, p))?)?];
        }
        // 0x430FF0..0x431237: min/max chains (ties take the later operand).
        let lo = |axis: usize| -> Result<u32> {
            let v: Vec<Binary> = points.iter().map(|q| b(q[axis])).collect::<Result<_>>()?;
            let pick = |a: usize, c: usize| if less(&v[a], &v[c]) { a } else { c };
            let m34 = pick(3, 4);
            let a = if less(&v[m34], &v[5]) { m34 } else { 5 };
            let m01 = pick(0, 1);
            let bb = if less(&v[m01], &v[2]) { m01 } else { 2 };
            Ok(points[if less(&v[bb], &v[a]) { bb } else { a }][axis])
        };
        let hi = |axis: usize| -> Result<u32> {
            let v: Vec<Binary> = points.iter().map(|q| b(q[axis])).collect::<Result<_>>()?;
            let pick = |a: usize, c: usize| if greater(&v[a], &v[c]) { a } else { c };
            let m34 = pick(3, 4);
            let a = if greater(&v[m34], &v[5]) { m34 } else { 5 };
            let m01 = pick(0, 1);
            let bb = if greater(&v[m01], &v[2]) { m01 } else { 2 };
            Ok(points[if greater(&v[bb], &v[a]) { bb } else { a }][axis])
        };
        let x0 = round_ftol(lo(0)?, false)?.wrapping_sub(1);
        let y0 = round_ftol(lo(1)?, false)?.wrapping_sub(1);
        let x1 = round_ftol(hi(0)?, true)?.wrapping_add(1);
        let y1 = round_ftol(hi(1)?, true)?.wrapping_add(1);
        let planes = self.planes.ok_or("430E50 before 42EED0 planes")?;
        let blocks = map.blocks_xy();
        let mut raster = Raster { vis: self, map, blocks, planes, p };
        rasterize_430710(&points, [x0, y0, x1, y1], &mut raster)
    }

    /// 0x42FF50 (after 0x430E50, TerrainMap+BC set).
    fn apply_42ff50(
        &mut self,
        map: &mut TerrainMap,
        heights: Heights<'_>,
        tiles: &impl Tiles,
        views: &mut impl ViewCellHooks,
        p: Precision,
    ) -> Result<()> {
        let mut queue = VecDeque::new();
        let mut i = 0;
        while i < self.active.len() {
            let cell = self.active[i];
            let [column, row] = map.block_index(cell);
            views.sweep_42fad0([column, row], map.block_size_a0(), self.cells[cell].state_b8)?;
            self.apply_720080(map, cell, &mut queue)?;
            if self.cells[cell].state_b8 == 0 {
                self.active.remove(i); // 0x42FED0
                self.cells[cell].active_140 = false;
            } else {
                i += 1;
            }
        }
        // 0x42FE10.
        for cell in queue {
            let state = self.cells[cell].state_b8 as i32;
            if state < 0 {
                continue;
            }
            if state <= 1 {
                if self.cells[cell].previous_b4 == 2 {
                    // 0x71CD00 (scene/zone detach), then Release 0x7211F0.
                    let c = &mut self.cells[cell];
                    c.requests_4 = c.requests_4.wrapping_sub(1);
                    if c.requests_4 <= 0 && map.resident(cell) {
                        map.release_721030(cell)?;
                    }
                }
            } else if state == 2 {
                // Request 0x721180 -> 0x5AA3F0(1, 4); Load 0x7212D0; 0x71CCB0.
                let c = &mut self.cells[cell];
                c.requests_4 = c.requests_4.wrapping_add(1);
                if !map.resident(cell) {
                    map.load_720ad0(heights, cell, p)?;
                }
            }
        }
        // 0x7208D0 for every active block: water at 0x720A93.
        for &cell in &self.active {
            if self.cells[cell].state_b8 == 2 && map.water_dirty_14f(cell) {
                map.water_7208d0(cell, tiles, p)?;
            }
        }
        Ok(())
    }

    /// 0x720080 and its 0x42F980 queueing.
    fn apply_720080(&mut self, map: &TerrainMap, cell: usize, queue: &mut VecDeque<usize>) -> Result<()> {
        let c = &mut self.cells[cell];
        if c.request_bc != c.state_b8 {
            c.previous_b4 = c.state_b8;
            c.state_b8 = c.request_bc;
            if c.state_b8 == 2 {
                queue.push_back(cell);
            } else {
                queue.push_front(cell);
            }
        }
        c.request_bc = 0;
        if c.state_b8 != 0 {
            let (refresh, minmax) = map.refresh_flags(cell);
            if refresh && map.resident(cell) {
                return Err("720080 +14E vertex refresh 71CFC0 is not ported (set only by 71CC00/73D7F0)".into());
            }
            if minmax {
                return Err("720080 +14D 71B8F0 recompute (set only by 71CC00/73D7F0) is not ported".into());
            }
        }
        Ok(())
    }
}

/// 0x42EED0's four 0x71D840 corner casts (margin 0, no secondary point).
fn footprint_42eed0(map: &TerrainMap, camera: &CameraWords, p: Precision) -> std::result::Result<[[Option<u32>; 3]; 4], String> {
    let mut out = [[None; 3]; 4];
    for (slot, &ray) in RAY_ORDER.iter().enumerate() {
        out[slot] = intersect_71d840(map, camera.eye, camera.rays[ray], None, 0, p)?.point;
    }
    Ok(out)
}

/// 0x430710(poly, 430080, vis, 0, 0, 42F1F0, vis, 430080, vis, 4300D0, vis):
/// scanline span marking (row stride |x1-x0|, a native quirk), then the
/// neighbour growth passes.
fn rasterize_430710(points: &[[u32; 2]], bounds: [i32; 4], r: &mut Raster<'_>) -> Result<()> {
    let [x0, y0, x1, y1] = bounds;
    let w = x1.wrapping_sub(x0).wrapping_abs();
    let h = y1.wrapping_sub(y0).wrapping_abs();
    let size = h
        .checked_add(1)
        .and_then(|h| w.checked_add(1).and_then(|w| h.checked_mul(w)))
        .filter(|s| *s >= 0)
        .ok_or("430710 grid size overflows the native 32-bit allocation")?;
    let mut grid = Vec::new();
    grid.try_reserve_exact(size as usize).map_err(|_| "430710 grid allocation failed")?;
    grid.resize(size as usize, 0u32);
    let at = |y: i32, x: i32| -> Result<usize> {
        let i = y.wrapping_sub(y0).wrapping_mul(w).wrapping_add(x.wrapping_sub(x0));
        usize::try_from(i).ok().filter(|i| *i < size as usize).ok_or_else(|| "430710 grid index outside the allocation".into())
    };
    let count = points.len() as i32;
    let p = r.p;
    let (mut xmin_word, mut xmax_word) = (0u32, 0u32); // b+80 / b+88
    let mut y = y0;
    while y <= y1 {
        let a = [st(&Binary::from_i32(x0))?, st(&Binary::from_i32(y))?];
        let bp = [st(&Binary::from_i32(x1))?, a[1]];
        let (mut tmin, mut tmax) = (b(0x4000_0000)?, b(0xbf80_0000)?);
        let mut hits = 0;
        for i in 0..count {
            let k = (i + 1) % count;
            let (c, d) = (points[i as usize], points[k as usize]);
            if let Some((out, t)) = segment_42f330(a, bp, c, d, p)? {
                let t = b(t)?;
                let mut first = false;
                if less(&t, &tmin) {
                    tmin = t.clone();
                    xmin_word = out[0];
                    hits += 1;
                    first = true;
                }
                if greater(&t, &tmax) {
                    tmax = t;
                    xmax_word = out[0];
                    if !first {
                        hits += 1;
                    }
                }
            }
        }
        if hits >= 2 {
            let lo = round_ftol(xmin_word, false)?;
            let hi = round_ftol(xmax_word, true)?;
            let mut x = x0;
            while x <= x1 {
                if x >= lo && x <= hi {
                    r.mark(x, y, 2);
                    let ci = at(y, x)?;
                    grid[ci] = 2;
                    // Upward growth.
                    if y > y0 && grid[at(y - 1, x)?] == 0 {
                        if r.test(x, y)? {
                            r.mark(x, y, 2);
                            let mut cy = y;
                            while cy > y0 {
                                cy -= 1;
                                if r.test(x, cy)? {
                                    r.mark(x, cy, 2);
                                    grid[at(cy, x)?] = 2;
                                }
                            }
                        } else {
                            r.mark(x, y, 1);
                            grid[ci] = 0;
                        }
                    }
                    // Leftward growth.
                    if x > x0 && grid[ci] == 2 && grid[ci - 1] == 0 {
                        if r.test(x, y)? {
                            let mut cx = x;
                            while cx > x0 {
                                cx -= 1;
                                if r.test(cx, y)? {
                                    r.mark(cx, y, 2);
                                    grid[at(y, cx)?] = 2;
                                }
                            }
                        } else {
                            r.mark(x, y, 1);
                            grid[ci] = 0;
                        }
                    }
                }
                x += 1;
            }
        }
        y += 1;
    }
    // 0x430BDD: downward and rightward growth, bottom-right first.
    let mut y = y1;
    while y >= y0 {
        let mut x = x1;
        while x >= x0 {
            let ci = at(y, x)?;
            if y < y1 && grid[ci] == 2 && grid[at(y + 1, x)?] == 0 {
                if r.test(x, y)? {
                    let mut cy = y;
                    while cy < y1 {
                        cy += 1;
                        if r.test(x, cy)? {
                            r.mark(x, cy, 2);
                            grid[at(cy, x)?] = 2;
                        }
                    }
                } else {
                    r.mark(x, y, 1);
                    grid[ci] = 0;
                }
            }
            if x < x1 && grid[ci] == 2 && grid[ci + 1] == 0 {
                if r.test(x, y)? {
                    let mut cx = x;
                    while cx < x1 {
                        cx += 1;
                        if r.test(cx, y)? {
                            r.mark(cx, y, 2);
                            grid[at(y, cx)?] = 2;
                        }
                    }
                } else {
                    r.mark(x, y, 1);
                    grid[ci] = 0;
                }
            }
            x -= 1;
        }
        y -= 1;
    }
    Ok(())
}

#[cfg(test)]
#[path = "terrain_visibility_tests.rs"]
mod tests;
