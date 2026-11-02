//! The generator's working world: the GMap tile grid (tile variations and
//! water areas), the two heightmaps (base HEIGHT_MAP at world+34, mountain/water
//! overlay MOUNTAIN_HEIGHT_MAP at world+38; 0x4F27E0 creates them with the SVars
//! Elevation limits, base default ElevationDefault, overlay default 0; the
//! setters are in `height.rs`) and the terrain change batch that enforces the
//! biome Transition table (0x559980 / 0x5599F0 / 0x559FB0 / 0x559A70).
use super::data::{BiomeDef, RmgData};
use std::collections::HashMap;

/// GMap+0x50: entry 0 is null, ids 1..=254 are records (ids are bytes in the
/// saved tile and vertex data, 0xFF marks a shared vertex).
const AREA_TABLE_LEN: usize = 255;
const SHARED_VERTEX: u8 = 0xFF;
/// 0xB322E0: (dx, dy) per direction; 0xAB7804 next, 0xAB7828 previous,
/// 0xAB784C next-but-one, 0xAB7870 previous-but-one, 0xAB7894 opposite.
const FLOOD_DELTA: [[i32; 2]; 9] = [[0, 0], [-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
const FLOOD_NEXT: [usize; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const FLOOD_PREV: [usize; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7];
const FLOOD_NEXT2: [usize; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2];
const FLOOD_PREV2: [usize; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6];
const FLOOD_OPPOSITE: [usize; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];
/// 8-neighbour order of the native tables (B322E0 / B3D610 / aa-heightfield).
pub(super) const NEIGHBORS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];

pub(super) struct World {
    pub width: f32,
    pub height: f32,
    pub tile: f32,
    pub tile_bits: u32,
    pub tw: usize,
    pub th: usize,
    pub var: Vec<u32>,
    pub hw: usize,
    pub hh: usize,
    pub spacing: f32,
    pub base: Vec<f32>,
    pub overlay: Vec<f32>,
    pub limits: [f32; 2],
    pub height_bits: u32,
    /// Tile +0x2C, the water area id (0 none).
    pub area: Vec<u8>,
    /// The flood's visit stamp (tile +0x28, GMap+0x48) and arrival direction
    /// (tile +0x2D).
    flood_visit: Vec<u32>,
    flood_from: Vec<u8>,
    flood_stamp: u32,
    /// GMap+0x50: tiles per water area id (record +4).
    area_count: Vec<i32>,
    /// Base map +0x44, the per-vertex water-area bytes.
    pub verts: Vec<u8>,
    /// The overlay's +0x44 bytes: nothing writes them.
    overlay_verts: Vec<u8>,
    /// The tiles whose terrain changed since [`Self::take_changed`] (0x54FDC0
    /// also deletes their doodads and refreshes their flag word).
    changed: Vec<usize>,
}

impl World {
    pub(super) fn new(width: f32, height: f32, data: &RmgData, initial: u32) -> Self {
        let tile_bits = data.svars.tile_bits.max(1);
        let height_bits = data.svars.height_bits.max(1);
        let ext = [width.ceil() as i32, height.ceil() as i32];
        let (tw, th) = ((ext[0] >> tile_bits) as usize, (ext[1] >> tile_bits) as usize);
        let (hw, hh) = (((ext[0] >> height_bits) + 1) as usize, ((ext[1] >> height_bits) + 1) as usize);
        let e = data.svars.elevation;
        Self {
            width,
            height,
            tile: (1u32 << tile_bits) as f32,
            tile_bits,
            tw,
            th,
            var: vec![initial; tw * th],
            hw,
            hh,
            spacing: (1u32 << height_bits) as f32,
            base: vec![e[2]; hw * hh],
            overlay: vec![0.0; hw * hh],
            limits: [e[0], e[1]],
            height_bits,
            area: vec![0; tw * th],
            flood_visit: vec![0; tw * th],
            flood_from: vec![0; tw * th],
            flood_stamp: 0,
            area_count: vec![0; AREA_TABLE_LEN],
            verts: vec![0; hw * hh],
            overlay_verts: vec![0; hw * hh],
            changed: Vec::new(),
        }
    }
    /// The parity probe's world dump (see the RmgProbe sampler): tile
    /// variation ids, water areas, both heightmaps as bits, then the things.
    pub(super) fn dump(&self, data: &RmgData, path: &std::path::Path, things: &[(String, [f32; 2])]) {
        use std::fmt::Write;
        let mut out = String::new();
        let line = |o: &mut String, v: Vec<String>| {
            o.push_str(&v.join(" "));
            o.push('\n');
        };
        line(&mut out, self.var.iter().map(|&v| data.variations[v as usize].ids.to_ascii_lowercase()).collect());
        line(&mut out, self.area.iter().map(|a| a.to_string()).collect());
        line(&mut out, self.base.iter().map(|h| format!("{:x}", h.to_bits())).collect());
        line(&mut out, self.overlay.iter().map(|h| format!("{:x}", h.to_bits())).collect());
        for (id, p) in things {
            let _ = writeln!(out, "{id} {:x} {:x}", p[0].to_bits(), p[1].to_bits());
        }
        let _ = std::fs::write(path, out);
    }
    /// One heightmap's +0x44 bytes.
    pub(super) fn vertex_areas(&self, map: super::height::Map) -> &[u8] {
        match map {
            super::height::Map::Base => &self.verts,
            super::height::Map::Overlay => &self.overlay_verts,
        }
    }
    pub(super) fn height_at(&self, map: super::height::Map, x: i32, y: i32) -> f32 {
        let at = y as usize * self.hw + x as usize;
        match map {
            super::height::Map::Base => self.base[at],
            super::height::Map::Overlay => self.overlay[at],
        }
    }
    /// 0x552C90: the first free water area id, 0 when none.
    pub(super) fn free_area(&self) -> u8 {
        (1..AREA_TABLE_LEN).find(|&i| self.area_count[i] == 0).map_or(0, |i| i as u8)
    }
    /// 0x550190 at a tile's centre: the nearest base sample.
    fn level_at_tile(&self, cell: [i32; 2]) -> f32 {
        let half = (1i32 << self.tile_bits) as f64 * 0.5;
        let bias = 1 << (self.height_bits - 1);
        let v = |c: i32, size: usize| ((((f64::from(c << self.tile_bits) + half) as f32) as i32 + bias) >> self.height_bits).clamp(0, size as i32 - 1);
        self.base[v(cell[1], self.hh) as usize * self.hw + v(cell[0], self.hw) as usize]
    }
    /// 0x539C40 (IsIndexBlocked): the fill enters only water, and a tile of
    /// another area only within 0.1 of the start level.
    fn flood_blocked(&self, data: &RmgData, cell: [i32; 2], area: u8, level: f32) -> bool {
        let t = cell[1] as usize * self.tw + cell[0] as usize;
        if data.terrains[data.variations[self.var[t] as usize].terrain].kind != 2 {
            return true;
        }
        let other = self.area[t];
        if other == 0 || other == area {
            return false;
        }
        let d = (f64::from(self.level_at_tile(cell)) - f64::from(level)).abs();
        d > f64::from(f32::from_bits(0x3DCC_CCCD))
    }
    /// 0x539C00: move the tile into the area (0x552150 then 0x552110).
    fn flood_paint(&mut self, cell: [i32; 2], area: u8) {
        let t = cell[1] as usize * self.tw + cell[0] as usize;
        let current = self.area[t];
        if current == area {
            return;
        }
        if current != 0 {
            self.leave_area(t);
        }
        self.area[t] = area;
        let scale = self.tile_bits as i32 - self.height_bits as i32;
        let (vx, vy) = (cell[0] << scale, cell[1] << scale);
        for [dx, dy] in [[0, 0], [1, 0], [1, 1], [0, 1]] {
            if let Some(v) = self.vertex(vx + dx, vy + dy) {
                let byte = &mut self.verts[v];
                if *byte != area {
                    *byte = if *byte == 0 { area } else { SHARED_VERTEX };
                }
            }
        }
        self.area_count[usize::from(area)] += 1;
    }
    /// 0x539E30 on the flood solver 0x4F4050, as aa_spatial::water_areas
    /// proves it (aa-play does not link that crate's height type): breadth
    /// first from `start`, which is reached and queued unconditionally and
    /// painted when popped; a popped tile visits the directions around its
    /// arrival (0x554ED0: all eight from the start, five after a diagonal,
    /// three after an orthogonal step, never straight back).
    pub(super) fn flood_area(&mut self, data: &RmgData, start: [i32; 2], area: u8) {
        self.flood_stamp += 1;
        let stamp = self.flood_stamp;
        let level = self.level_at_tile(start);
        let mut fifo = std::collections::VecDeque::new();
        let s = start[1] as usize * self.tw + start[0] as usize;
        self.flood_visit[s] = stamp;
        self.flood_from[s] = 0;
        fifo.push_back(s);
        while let Some(current) = fifo.pop_front() {
            let origin = self.tile_xy(current);
            if origin == start {
                self.flood_paint(origin, area);
            }
            let came = self.flood_from[current] as usize;
            let (first, end) = match came {
                0 => (1usize, 8usize),
                1 | 3 | 5 | 7 => (FLOOD_PREV2[came], FLOOD_NEXT2[came]),
                _ => (FLOOD_PREV[came], FLOOD_NEXT[came]),
            };
            let valid = |w: &Self, dir: usize| -> bool {
                if FLOOD_OPPOSITE[came] == dir {
                    return false;
                }
                let d = FLOOD_DELTA[dir];
                let n = [origin[0] + d[0], origin[1] + d[1]];
                if n[0] < 0 || n[1] < 0 || n[0] >= w.tw as i32 || n[1] >= w.th as i32 {
                    return false;
                }
                !w.flood_blocked(data, n, area, level)
            };
            // 0x4F2760: step until valid, the last direction or a full turn.
            let advance = |w: &Self, mut dir: usize, begin: usize| -> usize {
                while dir != end {
                    dir = FLOOD_NEXT[dir];
                    if dir == begin {
                        break;
                    }
                    if valid(w, dir) {
                        return dir;
                    }
                }
                0
            };
            let mut dir = first;
            if !valid(self, dir) {
                dir = advance(self, dir, dir);
            }
            while dir != 0 {
                let d = FLOOD_DELTA[dir];
                let n = [origin[0] + d[0], origin[1] + d[1]];
                let t = n[1] as usize * self.tw + n[0] as usize;
                if self.flood_visit[t] != stamp {
                    self.flood_visit[t] = stamp;
                    self.flood_from[t] = dir as u8;
                    if !self.flood_blocked(data, n, area, level) {
                        self.flood_paint(n, area);
                        fifo.push_back(t);
                    }
                }
                dir = advance(self, dir, dir);
            }
        }
    }
    /// 0x552150: tile `t` leaves its water area: +0x2C cleared, its four
    /// vertices rebuilt (0x550F20 -> 0x5507C0), the record count decremented.
    fn leave_area(&mut self, t: usize) {
        let area = self.area[t];
        self.area[t] = 0;
        let [tx, ty] = self.tile_xy(t);
        let scale = self.tile_bits as i32 - self.height_bits as i32;
        let (vx, vy) = (tx << scale, ty << scale);
        for [dx, dy] in [[0, 0], [1, 0], [1, 1], [0, 1]] {
            let (x, y) = (vx + dx, vy + dy);
            let Some(v) = self.vertex(x, y) else { continue };
            self.verts[v] = 0;
            let (cx, cy) = ((x << self.height_bits) >> self.tile_bits, (y << self.height_bits) >> self.tile_bits);
            let mut done = false;
            for ox in [-1, 0] {
                for oy in [-1, 0] {
                    if done {
                        continue;
                    }
                    let Some(n) = self.tile_at(cx + ox, cy + oy) else { continue };
                    let a = self.area[n];
                    let byte = &mut self.verts[v];
                    if a != 0 && *byte != a {
                        if *byte != 0 {
                            *byte = SHARED_VERTEX;
                            done = true;
                        } else {
                            *byte = a;
                        }
                    }
                }
            }
        }
        self.area_count[usize::from(area)] -= 1;
    }
    /// The tile half of 0x54FDC0: the variation, and when a mountain or water
    /// tile changes kind the overlay reset 0x54F040 run on the old variation
    /// (with its water area left when it was water).
    pub(super) fn set_variation(&mut self, data: &RmgData, t: usize, var: u32) {
        let old = self.var[t];
        if old == var {
            return;
        }
        let old_terrain = &data.terrains[data.variations[old as usize].terrain];
        let new_kind = data.terrains[data.variations[var as usize].terrain].kind;
        if old_terrain.kind != new_kind && matches!(old_terrain.kind, 1 | 2) {
            super::height::reset_tile_overlay(self, data, t, old_terrain.max_slope);
            if old_terrain.kind == 2 && self.area[t] != 0 {
                self.leave_area(t);
            }
        }
        if data.variations[old as usize].terrain != data.variations[var as usize].terrain {
            self.changed.push(t);
        }
        self.var[t] = var;
    }
    pub(super) fn take_changed(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.changed)
    }
    pub(super) fn tile_of(&self, p: [f32; 2]) -> Option<usize> {
        let (x, y) = ((p[0] as i32) >> self.tile_bits, (p[1] as i32) >> self.tile_bits);
        (p[0] >= 0.0 && p[1] >= 0.0 && (x as usize) < self.tw && (y as usize) < self.th).then(|| y as usize * self.tw + x as usize)
    }
    pub(super) fn tile_xy(&self, t: usize) -> [i32; 2] {
        [(t % self.tw) as i32, (t / self.tw) as i32]
    }
    pub(super) fn tile_at(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.tw && (y as usize) < self.th).then(|| y as usize * self.tw + x as usize)
    }
    /// Tile centre (0x54EDC0).
    pub(super) fn tile_center(&self, t: usize) -> [f32; 2] {
        let [x, y] = self.tile_xy(t);
        [x as f32 * self.tile + self.tile * 0.5, y as f32 * self.tile + self.tile * 0.5]
    }
    pub(super) fn vertex(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.hw && (y as usize) < self.hh).then(|| y as usize * self.hw + x as usize)
    }
    pub(super) fn vertex_xy(&self, v: usize) -> [f32; 2] {
        [(v % self.hw) as f32 * self.spacing, (v / self.hw) as f32 * self.spacing]
    }
}

/// 0xB3D610: the eight neighbour offsets of the batch, in its order.
const BATCH_NEIGHBORS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];

/// One record of a batch (0x5598B0, 0x14 bytes: tile, x, y, variation,
/// state 0 untouched / 1 queued / 2 being processed / 3 settled).
struct Rec {
    tile: usize,
    x: i32,
    y: i32,
    var: u32,
    state: u8,
}

/// One terrain change batch (KKC_GMap change set: constructor 0x559980,
/// paint 0x5599F0, commit 0x559FB0, per-tile transitions 0x559A70): painted
/// tiles plus the neighbours the biome transitions convert. The batch's
/// stamp (GMap +0x48, advanced by the constructor) ties a tile to its
/// record through tile +0x24/+0x28; here a map does. Records are kept in
/// creation order (the +0x14 list the commit applies); the work queue
/// (+0xC) is prepended to and walked from its head.
pub(super) struct Batch<'a> {
    biome: &'a BiomeDef,
    data: &'a RmgData,
    /// +0x1C: a painted variation is water (0x5596B0 allows water
    /// transitions only then).
    water: bool,
    /// +0xA: land stays land: only land terrain is painted, only on land
    /// (0x5599F0, 0x559760). Set by a thing's base terrain (0x4EB500's last
    /// argument, the game mode is not the editor's).
    protect: bool,
    /// +9: the commit's queue keeps the records whose variation the tile already
    /// has (0x5972F0 sets it when a batch did not settle and is run again).
    pub again: bool,
    recs: Vec<Rec>,
    index: HashMap<usize, usize>,
    queue: std::collections::VecDeque<usize>,
}

impl<'a> Batch<'a> {
    pub(super) fn new(data: &'a RmgData, biome: &'a BiomeDef) -> Self {
        Self { biome, data, water: false, protect: false, again: false, recs: Vec::new(), index: HashMap::new(), queue: std::collections::VecDeque::new() }
    }
    /// A batch that keeps land on land.
    pub(super) fn protecting(data: &'a RmgData, biome: &'a BiomeDef) -> Self {
        Self { protect: true, ..Self::new(data, biome) }
    }
    /// 0x559760: the cell is on the map and, protecting, its terrain lets land
    /// units through.
    fn may_paint(&self, world: &World, x: i32, y: i32) -> bool {
        let Some(t) = world.tile_at(x, y) else { return false };
        !self.protect || self.data.terrains[self.terrain(world.var[t])].land
    }
    fn terrain(&self, var: u32) -> usize {
        self.data.variations[var as usize].terrain
    }
    /// 0x5598B0: the record of cell (x, y), created from the tile when the
    /// batch has none; None outside the map.
    fn record(&mut self, world: &World, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= world.tw as i32 || y >= world.th as i32 {
            return None;
        }
        let tile = y as usize * world.tw + x as usize;
        if let Some(&r) = self.index.get(&tile) {
            return Some(r);
        }
        let r = self.recs.len();
        self.recs.push(Rec { tile, x, y, var: world.var[tile], state: 0 });
        self.index.insert(tile, r);
        Some(r)
    }
    /// 0x5599F0.
    pub(super) fn paint(&mut self, world: &World, t: usize, var: u32) {
        let [x, y] = world.tile_xy(t);
        if !(!self.protect || self.data.terrains[self.terrain(var)].land) || !self.may_paint(world, x, y) {
            return;
        }
        if self.data.terrains[self.terrain(var)].kind == 2 {
            self.water = true;
        }
        let Some(r) = self.record(world, x, y) else { return };
        if self.recs[r].state == 0 {
            self.queue.push_front(r);
        }
        self.recs[r].var = var;
        self.recs[r].state = 1;
    }
    /// 0x5596B0: the transition variation to put next to `a` when `b` is
    /// painted beside it, following the biome's chain (0x4B6D90 per step);
    /// a water one only when this batch paints water.
    fn chain(&self, a: u32, b: u32) -> Option<u32> {
        let tb = self.terrain(b);
        let step = |v: u32| self.biome.transitions.get(&(self.terrain(v), tb)).map(|&t| t as u32);
        let mut prev = a;
        let mut t = step(a);
        while let Some(v) = t {
            if v == prev || v == b {
                break;
            }
            prev = v;
            t = step(v);
        }
        if self.data.terrains[self.terrain(prev)].kind == 2 && !self.water {
            return None;
        }
        (prev != a).then_some(prev)
    }
    /// 0x5597D0: the longest run of equal candidates, the first one on ties,
    /// null candidates skipped. It calls qsort first, but its comparator
    /// 0x559740 compares the element addresses it is handed, not the
    /// elements, and the CRT's short sort (8 elements, up to the cutoff) then
    /// leaves the array in place: the runs are in direction order.
    fn most_common(trans: &[Option<u32>; 8]) -> Option<u32> {
        let (mut cur, mut run, mut best, mut best_v) = (trans[0], 1, 0, None);
        for &v in &trans[1..] {
            let Some(_) = v else { continue };
            if v == cur {
                run += 1;
            } else {
                if best < run && cur.is_some() {
                    best = run;
                    best_v = cur;
                }
                run = 1;
                cur = v;
            }
        }
        if run <= best || cur.is_none() { best_v } else { cur }
    }
    /// 0x559A70 for one record.
    fn process(&mut self, world: &World, r: usize) {
        let (x, y, pvar) = (self.recs[r].x, self.recs[r].y, self.recs[r].var);
        let mut mode = 1u8;
        let mut conflict = false;
        let mut target: Option<u32> = 'first: {
            let mut trans: [Option<u32>; 8] = [None; 8];
            for (i, d) in BATCH_NEIGHBORS.iter().enumerate() {
                let Some(n) = self.record(world, x + d[0], y + d[1]) else { continue };
                if self.recs[n].state != 0 {
                    if self.chain(self.recs[n].var, pvar).is_some() {
                        mode = 3;
                        conflict = true;
                        break 'first Some(pvar);
                    }
                    continue;
                }
                trans[i] = self.chain(self.recs[n].var, pvar);
            }
            Self::most_common(&trans)
        };
        if target.is_none() {
            mode = 0;
        }
        // A transition that is no land keeps the record's own variation.
        if self.protect && mode != 0 && target.is_some_and(|t| !self.data.terrains[self.terrain(t)].land) {
            target = Some(pvar);
            mode = 3;
        }
        if let Some(t) = target {
            let pterrain = self.terrain(pvar);
            let mut i = 0usize;
            while i < 8 {
                let d = BATCH_NEIGHBORS[i];
                if let Some(n) = self.record(world, x + d[0], y + d[1]) {
                    let cur = world.var[self.recs[n].tile];
                    if self.recs[n].state == 0 {
                        if mode == 3 {
                            self.recs[n].var = cur;
                        } else if self.terrain(cur) != pterrain && self.may_paint(world, x + d[0], y + d[1]) {
                            self.recs[n].var = t;
                        }
                    } else if mode != 0 && mode != 3 && self.recs[n].state != 3 && self.chain(self.recs[n].var, t).is_some() {
                        mode = 3;
                        conflict = true;
                        i = 0;
                        continue;
                    }
                }
                i += 1;
            }
            target = Some(t);
        }
        let _ = target;
        if conflict {
            self.recs[r].state = 1;
            self.queue.push_front(r);
        } else if mode != 0 {
            self.recs[r].state = 3;
            for d in BATCH_NEIGHBORS {
                let Some(n) = self.record(world, x + d[0], y + d[1]) else { continue };
                if self.recs[n].state == 0 {
                    self.recs[n].state = 1;
                    if self.recs[n].var != world.var[self.recs[n].tile] {
                        self.queue.push_front(n);
                    }
                }
            }
        }
    }
    /// The parity probe (`AA_RMG_CM` names a file): at every commit the hash
    /// of the batch's records and of all tile variations, as RmgProbe logs
    /// the original's (CM lines).
    fn probe(&self, world: &World) {
        static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
        let Some(path) = PATH.get_or_init(|| std::env::var_os("AA_RMG_CM").map(Into::into)) else { return };
        static INDEX: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);
        let index = INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let fnv = |s: &str| {
            let mut h = 14695981039346656037u64;
            for c in s.encode_utf16() {
                h ^= u64::from(c);
                h = h.wrapping_mul(1099511628211);
            }
            h
        };
        let hashes: Vec<u64> = self.data.variations.iter().map(|v| fnv(&v.ids.to_ascii_lowercase())).collect();
        let mut hv = 14695981039346656037u64;
        for &v in &world.var {
            hv ^= hashes[v as usize];
            hv = hv.wrapping_mul(1099511628211);
        }
        let mut hr = 14695981039346656037u64;
        let detail = std::env::var("AA_RMG_CM_DETAIL").is_ok_and(|d| d.parse() == Ok(index));
        let mut text = String::new();
        for r in &self.recs {
            hr ^= r.x as u32 as u64;
            hr = hr.wrapping_mul(1099511628211);
            hr ^= r.y as u32 as u64;
            hr = hr.wrapping_mul(1099511628211);
            hr ^= hashes[r.var as usize];
            hr = hr.wrapping_mul(1099511628211);
            if detail {
                text += &format!(" {},{},{}", r.x, r.y, self.data.variations[r.var as usize].ids.to_ascii_lowercase());
            }
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "CM {index} n {} recs {hr:016x} vars {hv:016x}", self.recs.len());
            if detail {
                let _ = writeln!(f, "CMD {index}{text}");
                for y in 0..world.th {
                    let row: Vec<String> = (0..world.tw).map(|x| self.data.variations[world.var[y * world.tw + x] as usize].ids.to_ascii_lowercase()).collect();
                    let _ = writeln!(f, "CMV {index} {y} {}", row.join(" "));
                }
            }
        }
    }
    /// 0x559FB0(0): up to 16 passes over the queue (records whose variation
    /// differs from the tile's), then every changed record is applied
    /// (0x54FDC0). Returns false when the queue was not empty at the end
    /// (the batch +8 "unsettled" byte).
    pub(super) fn commit(mut self, world: &mut World) -> bool {
        self.probe(world);
        for _ in 0..16 {
            if self.queue.is_empty() {
                break;
            }
            let again = self.again;
            let list: Vec<usize> = self.queue.drain(..).filter(|&r| again || self.recs[r].var != world.var[self.recs[r].tile]).collect();
            for &r in &list {
                self.recs[r].state = 2;
            }
            for r in list {
                self.process(world, r);
            }
        }
        let settled = self.queue.is_empty();
        for rec in &self.recs {
            world.set_variation(self.data, rec.tile, rec.var);
        }
        settled
    }
}
