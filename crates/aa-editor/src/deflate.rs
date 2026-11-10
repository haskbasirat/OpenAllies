//! The original's zlib deflater, ported: zlib 1.1.4 `deflate.c` (the lazy
//! matcher `deflate_slow` used by levels 4 to 9) and `trees.c` (Huffman trees
//! and block output), with the zlib wrapper (header and Adler-32 trailer).
//!
//! AA.exe links zlib 1.1.4 (its own "deflate 1.1.4" / "inflate 1.1.4"
//! strings); its chunk-file compressor object (8B72E0) calls
//! `deflateInit_(stream, 6, "1.1.4", 0x38)`, so map chunks are written at
//! level 6 with the defaults (32 KiB window, memLevel 8, default strategy).
//!
//! This file is an altered version of zlib 1.1.4: a Rust rewrite of the parts
//! the game uses. It is not the original software. zlib's notice:
//!
//! > Copyright (C) 1995-2002 Jean-loup Gailly and Mark Adler
//! >
//! > This software is provided 'as-is', without any express or implied
//! > warranty. In no event will the authors be held liable for any damages
//! > arising from the use of this software.
//! >
//! > Permission is granted to anyone to use this software for any purpose,
//! > including commercial applications, and to alter it and redistribute it
//! > freely, subject to the following restrictions:
//! >
//! > 1. The origin of this software must not be misrepresented; you must not
//! >    claim that you wrote the original software. If you use this software
//! >    in a product, an acknowledgment in the product documentation would be
//! >    appreciated but is not required.
//! > 2. Altered source versions must be plainly marked as such, and must not be
//! >    misrepresented as being the original software.
//! > 3. This notice may not be removed or altered from any source distribution.
//!
//! [`Deflater::write`] is `deflate(Z_NO_FLUSH)` on the bytes given (all of them
//! are consumed, as the original's output buffer is drained between calls),
//! and [`Deflater::finish`] is `deflate(Z_FINISH)`. The window starts zeroed;
//! zlib 1.1.4 leaves it as the allocator returned it. Window bytes past the
//! input are compared only within the last 258 bytes of a stream, where a
//! match is clipped to the bytes that remain, but they can still decide which
//! earlier match is taken there. With the zeroed window every shipped map
//! chunk comes out identical (tests/shipped_maps.rs).

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_MATCH + 1;
const TOO_FAR: usize = 4096;
const NIL: u16 = 0;

const MAX_WBITS: u32 = 15;
const DEF_MEM_LEVEL: u32 = 8;

const LENGTH_CODES: usize = 29;
const LITERALS: usize = 256;
const L_CODES: usize = LITERALS + 1 + LENGTH_CODES;
const D_CODES: usize = 30;
const BL_CODES: usize = 19;
const HEAP_SIZE: usize = 2 * L_CODES + 1;
const MAX_BITS: usize = 15;
const MAX_BL_BITS: u32 = 7;
const END_BLOCK: usize = 256;
const REP_3_6: usize = 16;
const REPZ_3_10: usize = 17;
const REPZ_11_138: usize = 18;
const STORED_BLOCK: u32 = 0;
const STATIC_TREES: u32 = 1;
const DYN_TREES: u32 = 2;
const BUF_SIZE: i32 = 16;
const SMALLEST: usize = 1;

const EXTRA_LBITS: [u8; LENGTH_CODES] =
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const EXTRA_DBITS: [u8; D_CODES] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const EXTRA_BLBITS: [u8; BL_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 7];
const BL_ORDER: [usize; BL_CODES] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// `configuration_table` of zlib 1.1.4: good_length, max_lazy, nice_length,
/// max_chain. Levels 0 to 3 (stored and `deflate_fast`) are not ported.
const CONFIGURATION: [(usize, usize, usize, usize); 10] = [
    (0, 0, 0, 0),
    (4, 4, 8, 4),
    (4, 5, 16, 8),
    (4, 6, 32, 32),
    (4, 4, 16, 16),
    (8, 16, 32, 32),
    (8, 16, 128, 128),
    (8, 32, 128, 256),
    (32, 128, 258, 1024),
    (32, 258, 258, 4096),
];

/// A level the port does not cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedLevel(pub u8);

impl std::fmt::Display for UnsupportedLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "zlib level {} is not ported (only the lazy matcher, levels 4 to 9)", self.0)
    }
}
impl std::error::Error for UnsupportedLevel {}

/// `ct_data`: `fc` is Freq or Code, `dl` is Dad or Len (C unions).
#[derive(Clone)]
struct Tree {
    fc: Vec<u16>,
    dl: Vec<u16>,
}
impl Tree {
    fn new(n: usize) -> Self {
        Tree { fc: vec![0; n], dl: vec![0; n] }
    }
}

/// `tr_static_init`: the fixed tables.
struct Static {
    ltree: Tree,
    dtree: Tree,
    dist_code: [u8; 512],
    length_code: [u8; 256],
    base_length: [u16; LENGTH_CODES],
    base_dist: [u16; D_CODES],
}

fn bi_reverse(mut code: u32, mut len: u32) -> u16 {
    let mut res = 0u32;
    loop {
        res |= code & 1;
        code >>= 1;
        res <<= 1;
        len -= 1;
        if len == 0 {
            break;
        }
    }
    (res >> 1) as u16
}

/// `gen_codes`.
fn gen_codes(tree: &mut Tree, max_code: usize, bl_count: &[u16; MAX_BITS + 1]) {
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut code: u16 = 0;
    for bits in 1..=MAX_BITS {
        code = code.wrapping_add(bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    for n in 0..=max_code {
        let len = tree.dl[n] as usize;
        if len == 0 {
            continue;
        }
        tree.fc[n] = bi_reverse(u32::from(next_code[len]), len as u32);
        next_code[len] = next_code[len].wrapping_add(1);
    }
}

impl Static {
    fn new() -> Self {
        let mut length_code = [0u8; 256];
        let mut base_length = [0u16; LENGTH_CODES];
        let mut length = 0usize;
        let mut code = 0usize;
        while code < LENGTH_CODES - 1 {
            base_length[code] = length as u16;
            for _ in 0..(1usize << EXTRA_LBITS[code]) {
                length_code[length] = code as u8;
                length += 1;
            }
            code += 1;
        }
        // match length 258 (index 255) is code 285 rather than 284
        length_code[length - 1] = code as u8;

        let mut dist_code = [0u8; 512];
        let mut base_dist = [0u16; D_CODES];
        let mut dist = 0usize;
        code = 0;
        while code < 16 {
            base_dist[code] = dist as u16;
            for _ in 0..(1usize << EXTRA_DBITS[code]) {
                dist_code[dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }
        dist >>= 7;
        while code < D_CODES {
            base_dist[code] = (dist << 7) as u16;
            for _ in 0..(1usize << (EXTRA_DBITS[code] - 7)) {
                dist_code[256 + dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }

        let mut bl_count = [0u16; MAX_BITS + 1];
        let mut ltree = Tree::new(L_CODES + 2);
        for n in 0..L_CODES + 2 {
            let len = match n {
                0..=143 => 8,
                144..=255 => 9,
                256..=279 => 7,
                _ => 8,
            };
            ltree.dl[n] = len;
            bl_count[len as usize] += 1;
        }
        gen_codes(&mut ltree, L_CODES + 1, &bl_count);
        let mut dtree = Tree::new(D_CODES);
        for n in 0..D_CODES {
            dtree.dl[n] = 5;
            dtree.fc[n] = bi_reverse(n as u32, 5);
        }
        Static { ltree, dtree, dist_code, length_code, base_length, base_dist }
    }

    fn d_code(&self, dist: usize) -> usize {
        if dist < 256 {
            self.dist_code[dist] as usize
        } else {
            self.dist_code[256 + (dist >> 7)] as usize
        }
    }
}

/// `static_tree_desc`.
#[derive(Clone, Copy)]
enum Desc {
    L,
    D,
    Bl,
}

/// The `trees.c` half of `deflate_state`.
struct Trees {
    st: Static,
    dyn_ltree: Tree,
    dyn_dtree: Tree,
    bl_tree: Tree,
    l_max_code: usize,
    d_max_code: usize,
    bl_max_code: usize,
    bl_count: [u16; MAX_BITS + 1],
    heap: [usize; HEAP_SIZE],
    heap_len: usize,
    heap_max: usize,
    depth: [u8; HEAP_SIZE],
    l_buf: Vec<u8>,
    d_buf: Vec<u16>,
    lit_bufsize: usize,
    last_lit: usize,
    opt_len: u64,
    static_len: u64,
    matches: u32,
    bi_buf: u16,
    bi_valid: i32,
    out: Vec<u8>,
}

impl Trees {
    fn new(lit_bufsize: usize) -> Self {
        let mut t = Trees {
            st: Static::new(),
            dyn_ltree: Tree::new(HEAP_SIZE),
            dyn_dtree: Tree::new(2 * D_CODES + 1),
            bl_tree: Tree::new(2 * BL_CODES + 1),
            l_max_code: 0,
            d_max_code: 0,
            bl_max_code: 0,
            bl_count: [0; MAX_BITS + 1],
            heap: [0; HEAP_SIZE],
            heap_len: 0,
            heap_max: 0,
            depth: [0; HEAP_SIZE],
            l_buf: vec![0; lit_bufsize],
            d_buf: vec![0; lit_bufsize],
            lit_bufsize,
            last_lit: 0,
            opt_len: 0,
            static_len: 0,
            matches: 0,
            bi_buf: 0,
            bi_valid: 0,
            out: Vec::new(),
        };
        t.init_block();
        t
    }

    fn init_block(&mut self) {
        for n in 0..L_CODES {
            self.dyn_ltree.fc[n] = 0;
        }
        for n in 0..D_CODES {
            self.dyn_dtree.fc[n] = 0;
        }
        for n in 0..BL_CODES {
            self.bl_tree.fc[n] = 0;
        }
        self.dyn_ltree.fc[END_BLOCK] = 1;
        self.opt_len = 0;
        self.static_len = 0;
        self.last_lit = 0;
        self.matches = 0;
    }

    fn put_byte(&mut self, b: u8) {
        self.out.push(b);
    }
    fn put_short(&mut self, w: u16) {
        self.out.push(w as u8);
        self.out.push((w >> 8) as u8);
    }

    fn send_bits(&mut self, value: u32, length: u32) {
        let len = length as i32;
        if self.bi_valid > BUF_SIZE - len {
            let val = value;
            self.bi_buf |= (val << self.bi_valid) as u16;
            let b = self.bi_buf;
            self.put_short(b);
            self.bi_buf = ((val as u16) >> (BUF_SIZE - self.bi_valid)) as u16;
            self.bi_valid += len - BUF_SIZE;
        } else {
            self.bi_buf |= (value << self.bi_valid) as u16;
            self.bi_valid += len;
        }
    }

    fn bi_windup(&mut self) {
        if self.bi_valid > 8 {
            let b = self.bi_buf;
            self.put_short(b);
        } else if self.bi_valid > 0 {
            let b = self.bi_buf as u8;
            self.put_byte(b);
        }
        self.bi_buf = 0;
        self.bi_valid = 0;
    }

    /// `_tr_tally`: true when the block is full.
    fn tally(&mut self, dist: usize, lc: usize) -> bool {
        self.d_buf[self.last_lit] = dist as u16;
        self.l_buf[self.last_lit] = lc as u8;
        self.last_lit += 1;
        if dist == 0 {
            self.dyn_ltree.fc[lc] = self.dyn_ltree.fc[lc].wrapping_add(1);
        } else {
            self.matches += 1;
            let dist = dist - 1;
            let l = self.st.length_code[lc] as usize + LITERALS + 1;
            self.dyn_ltree.fc[l] = self.dyn_ltree.fc[l].wrapping_add(1);
            let d = self.st.d_code(dist);
            self.dyn_dtree.fc[d] = self.dyn_dtree.fc[d].wrapping_add(1);
        }
        self.last_lit == self.lit_bufsize - 1
    }

    fn tree_mut(&mut self, desc: Desc) -> &mut Tree {
        match desc {
            Desc::L => &mut self.dyn_ltree,
            Desc::D => &mut self.dyn_dtree,
            Desc::Bl => &mut self.bl_tree,
        }
    }

    /// `smaller(tree, n, m, depth)`.
    fn smaller(tree: &Tree, depth: &[u8; HEAP_SIZE], n: usize, m: usize) -> bool {
        tree.fc[n] < tree.fc[m] || (tree.fc[n] == tree.fc[m] && depth[n] <= depth[m])
    }

    /// `pqdownheap`.
    fn pqdownheap(tree: &Tree, heap: &mut [usize; HEAP_SIZE], heap_len: usize, depth: &[u8; HEAP_SIZE], mut k: usize) {
        let v = heap[k];
        let mut j = k << 1;
        while j <= heap_len {
            if j < heap_len && Self::smaller(tree, depth, heap[j + 1], heap[j]) {
                j += 1;
            }
            if Self::smaller(tree, depth, v, heap[j]) {
                break;
            }
            heap[k] = heap[j];
            k = j;
            j <<= 1;
        }
        heap[k] = v;
    }

    /// `build_tree`.
    fn build_tree(&mut self, desc: Desc) {
        // static_l_desc, static_d_desc, static_bl_desc
        let (elems, extra, extra_base, max_length): (usize, &'static [u8], usize, u32) = match desc {
            Desc::L => (L_CODES, &EXTRA_LBITS, LITERALS + 1, MAX_BITS as u32),
            Desc::D => (D_CODES, &EXTRA_DBITS, 0, MAX_BITS as u32),
            Desc::Bl => (BL_CODES, &EXTRA_BLBITS, 0, MAX_BL_BITS),
        };
        let mut tree = std::mem::replace(self.tree_mut(desc), Tree::new(0));
        let stree: Option<&Tree> = match desc {
            Desc::L => Some(&self.st.ltree),
            Desc::D => Some(&self.st.dtree),
            Desc::Bl => None,
        };
        let mut max_code: isize = -1;
        self.heap_len = 0;
        self.heap_max = HEAP_SIZE;
        for n in 0..elems {
            if tree.fc[n] != 0 {
                self.heap_len += 1;
                self.heap[self.heap_len] = n;
                max_code = n as isize;
                self.depth[n] = 0;
            } else {
                tree.dl[n] = 0;
            }
        }
        while self.heap_len < 2 {
            let node = if max_code < 2 {
                max_code += 1;
                max_code as usize
            } else {
                0
            };
            self.heap_len += 1;
            self.heap[self.heap_len] = node;
            tree.fc[node] = 1;
            self.depth[node] = 0;
            self.opt_len = self.opt_len.wrapping_sub(1);
            if let Some(s) = stree {
                self.static_len = self.static_len.wrapping_sub(u64::from(s.dl[node]));
            }
        }
        let max_code = max_code as usize;
        match desc {
            Desc::L => self.l_max_code = max_code,
            Desc::D => self.d_max_code = max_code,
            Desc::Bl => self.bl_max_code = max_code,
        }
        let mut n = self.heap_len / 2;
        while n >= 1 {
            Self::pqdownheap(&tree, &mut self.heap, self.heap_len, &self.depth, n);
            n -= 1;
        }
        let mut node = elems;
        loop {
            // pqremove
            let n = self.heap[SMALLEST];
            self.heap[SMALLEST] = self.heap[self.heap_len];
            self.heap_len -= 1;
            Self::pqdownheap(&tree, &mut self.heap, self.heap_len, &self.depth, SMALLEST);
            let m = self.heap[SMALLEST];
            self.heap_max -= 1;
            self.heap[self.heap_max] = n;
            self.heap_max -= 1;
            self.heap[self.heap_max] = m;
            tree.fc[node] = tree.fc[n].wrapping_add(tree.fc[m]);
            self.depth[node] = self.depth[n].max(self.depth[m]).wrapping_add(1);
            tree.dl[n] = node as u16;
            tree.dl[m] = node as u16;
            self.heap[SMALLEST] = node;
            node += 1;
            Self::pqdownheap(&tree, &mut self.heap, self.heap_len, &self.depth, SMALLEST);
            if self.heap_len < 2 {
                break;
            }
        }
        self.heap_max -= 1;
        self.heap[self.heap_max] = self.heap[SMALLEST];

        // gen_bitlen
        for b in self.bl_count.iter_mut() {
            *b = 0;
        }
        let root = self.heap[self.heap_max];
        tree.dl[root] = 0;
        let mut overflow = 0i32;
        let mut h = self.heap_max + 1;
        while h < HEAP_SIZE {
            let n = self.heap[h];
            let mut bits = u32::from(tree.dl[tree.dl[n] as usize]) + 1;
            if bits > max_length {
                bits = max_length;
                overflow += 1;
            }
            tree.dl[n] = bits as u16;
            h += 1;
            if n > max_code {
                continue;
            }
            self.bl_count[bits as usize] += 1;
            let xbits = if n >= extra_base { u32::from(extra[n - extra_base]) } else { 0 };
            let f = u64::from(tree.fc[n]);
            self.opt_len = self.opt_len.wrapping_add(f * u64::from(bits + xbits));
            if let Some(s) = stree {
                self.static_len = self.static_len.wrapping_add(f * u64::from(u32::from(s.dl[n]) + xbits));
            }
        }
        if overflow != 0 {
            loop {
                let mut bits = max_length as usize - 1;
                while self.bl_count[bits] == 0 {
                    bits -= 1;
                }
                self.bl_count[bits] -= 1;
                self.bl_count[bits + 1] += 2;
                self.bl_count[max_length as usize] -= 1;
                overflow -= 2;
                if overflow <= 0 {
                    break;
                }
            }
            let mut bits = max_length as usize;
            while bits != 0 {
                let mut n = self.bl_count[bits];
                while n != 0 {
                    h -= 1;
                    let m = self.heap[h];
                    if m > max_code {
                        continue;
                    }
                    if tree.dl[m] as usize != bits {
                        let delta = (bits as i64 - i64::from(tree.dl[m])) * i64::from(tree.fc[m]);
                        self.opt_len = self.opt_len.wrapping_add(delta as u64);
                        tree.dl[m] = bits as u16;
                    }
                    n -= 1;
                }
                bits -= 1;
            }
        }
        gen_codes(&mut tree, max_code, &self.bl_count);
        *self.tree_mut(desc) = tree;
    }

    /// `scan_tree`.
    fn scan_tree(&mut self, desc: Desc, max_code: usize) {
        let mut tree = std::mem::replace(self.tree_mut(desc), Tree::new(0));
        let mut prevlen: i32 = -1;
        let mut nextlen = i32::from(tree.dl[0]);
        let mut count = 0i32;
        let mut max_count = 7;
        let mut min_count = 4;
        if nextlen == 0 {
            max_count = 138;
            min_count = 3;
        }
        tree.dl[max_code + 1] = 0xffff;
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = i32::from(tree.dl[n + 1]);
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                self.bl_tree.fc[curlen as usize] = self.bl_tree.fc[curlen as usize].wrapping_add(count as u16);
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.bl_tree.fc[curlen as usize] = self.bl_tree.fc[curlen as usize].wrapping_add(1);
                }
                self.bl_tree.fc[REP_3_6] = self.bl_tree.fc[REP_3_6].wrapping_add(1);
            } else if count <= 10 {
                self.bl_tree.fc[REPZ_3_10] = self.bl_tree.fc[REPZ_3_10].wrapping_add(1);
            } else {
                self.bl_tree.fc[REPZ_11_138] = self.bl_tree.fc[REPZ_11_138].wrapping_add(1);
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
        *self.tree_mut(desc) = tree;
    }

    fn send_code(&mut self, c: usize, desc_tree: Desc) {
        let (code, len) = match desc_tree {
            Desc::L => (self.dyn_ltree.fc[c], self.dyn_ltree.dl[c]),
            Desc::D => (self.dyn_dtree.fc[c], self.dyn_dtree.dl[c]),
            Desc::Bl => (self.bl_tree.fc[c], self.bl_tree.dl[c]),
        };
        self.send_bits(u32::from(code), u32::from(len));
    }

    /// `send_tree`.
    fn send_tree(&mut self, desc: Desc, max_code: usize) {
        let lens: Vec<u16> = match desc {
            Desc::L => self.dyn_ltree.dl[..max_code + 2].to_vec(),
            Desc::D => self.dyn_dtree.dl[..max_code + 2].to_vec(),
            Desc::Bl => unreachable!(),
        };
        let mut prevlen: i32 = -1;
        let mut nextlen = i32::from(lens[0]);
        let mut count = 0i32;
        let mut max_count = 7;
        let mut min_count = 4;
        if nextlen == 0 {
            max_count = 138;
            min_count = 3;
        }
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = i32::from(lens[n + 1]);
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                loop {
                    self.send_code(curlen as usize, Desc::Bl);
                    count -= 1;
                    if count == 0 {
                        break;
                    }
                }
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.send_code(curlen as usize, Desc::Bl);
                    count -= 1;
                }
                self.send_code(REP_3_6, Desc::Bl);
                self.send_bits((count - 3) as u32, 2);
            } else if count <= 10 {
                self.send_code(REPZ_3_10, Desc::Bl);
                self.send_bits((count - 3) as u32, 3);
            } else {
                self.send_code(REPZ_11_138, Desc::Bl);
                self.send_bits((count - 11) as u32, 7);
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
    }

    /// `build_bl_tree`.
    fn build_bl_tree(&mut self) -> usize {
        let (l, d) = (self.l_max_code, self.d_max_code);
        self.scan_tree(Desc::L, l);
        self.scan_tree(Desc::D, d);
        self.build_tree(Desc::Bl);
        let mut max_blindex = BL_CODES - 1;
        while max_blindex >= 3 {
            if self.bl_tree.dl[BL_ORDER[max_blindex]] != 0 {
                break;
            }
            max_blindex -= 1;
        }
        self.opt_len = self.opt_len.wrapping_add(3 * (max_blindex as u64 + 1) + 5 + 5 + 4);
        max_blindex
    }

    /// `send_all_trees`.
    fn send_all_trees(&mut self, lcodes: usize, dcodes: usize, blcodes: usize) {
        self.send_bits((lcodes - 257) as u32, 5);
        self.send_bits((dcodes - 1) as u32, 5);
        self.send_bits((blcodes - 4) as u32, 4);
        for rank in 0..blcodes {
            let len = self.bl_tree.dl[BL_ORDER[rank]];
            self.send_bits(u32::from(len), 3);
        }
        self.send_tree(Desc::L, lcodes - 1);
        self.send_tree(Desc::D, dcodes - 1);
    }

    /// `compress_block`, with the static or the dynamic trees.
    fn compress_block(&mut self, dynamic: bool) {
        let (ltree, dtree) = if dynamic {
            (self.dyn_ltree.clone(), self.dyn_dtree.clone())
        } else {
            (self.st.ltree.clone(), self.st.dtree.clone())
        };
        let mut lx = 0;
        if self.last_lit != 0 {
            loop {
                let mut dist = self.d_buf[lx] as usize;
                let mut lc = self.l_buf[lx] as usize;
                lx += 1;
                if dist == 0 {
                    self.send_bits(u32::from(ltree.fc[lc]), u32::from(ltree.dl[lc]));
                } else {
                    let mut code = self.st.length_code[lc] as usize;
                    let c = code + LITERALS + 1;
                    self.send_bits(u32::from(ltree.fc[c]), u32::from(ltree.dl[c]));
                    let mut extra = u32::from(EXTRA_LBITS[code]);
                    if extra != 0 {
                        lc -= self.st.base_length[code] as usize;
                        self.send_bits(lc as u32, extra);
                    }
                    dist -= 1;
                    code = self.st.d_code(dist);
                    self.send_bits(u32::from(dtree.fc[code]), u32::from(dtree.dl[code]));
                    extra = u32::from(EXTRA_DBITS[code]);
                    if extra != 0 {
                        dist -= self.st.base_dist[code] as usize;
                        self.send_bits(dist as u32, extra);
                    }
                }
                if lx >= self.last_lit {
                    break;
                }
            }
        }
        self.send_bits(u32::from(ltree.fc[END_BLOCK]), u32::from(ltree.dl[END_BLOCK]));
    }

    /// `_tr_stored_block` (`copy_block` with a header).
    fn stored_block(&mut self, buf: &[u8], eof: bool) {
        self.send_bits((STORED_BLOCK << 1) + u32::from(eof), 3);
        self.bi_windup();
        let len = buf.len() as u16;
        self.put_short(len);
        self.put_short(!len);
        self.out.extend_from_slice(buf);
    }

    /// `_tr_flush_block` at a level above 0.
    fn flush_block(&mut self, buf: Option<&[u8]>, stored_len: u64, eof: bool) {
        self.build_tree(Desc::L);
        self.build_tree(Desc::D);
        let max_blindex = self.build_bl_tree();
        let mut opt_lenb = self.opt_len.wrapping_add(3 + 7) >> 3;
        let static_lenb = self.static_len.wrapping_add(3 + 7) >> 3;
        if static_lenb <= opt_lenb {
            opt_lenb = static_lenb;
        }
        match buf {
            Some(buf) if stored_len + 4 <= opt_lenb => self.stored_block(buf, eof),
            _ if static_lenb == opt_lenb => {
                self.send_bits((STATIC_TREES << 1) + u32::from(eof), 3);
                self.compress_block(false);
            }
            _ => {
                self.send_bits((DYN_TREES << 1) + u32::from(eof), 3);
                let (l, d) = (self.l_max_code, self.d_max_code);
                self.send_all_trees(l + 1, d + 1, max_blindex + 1);
                self.compress_block(true);
            }
        }
        self.init_block();
        if eof {
            self.bi_windup();
        }
    }
}

/// zlib 1.1.4 `deflate_state` for the lazy matcher, writing a zlib stream.
pub struct Deflater {
    w_size: usize,
    w_mask: usize,
    window: Vec<u8>,
    window_size: usize,
    prev: Vec<u16>,
    head: Vec<u16>,
    ins_h: usize,
    hash_mask: usize,
    hash_shift: u32,
    block_start: i64,
    match_length: usize,
    prev_match: usize,
    match_available: bool,
    strstart: usize,
    match_start: usize,
    lookahead: usize,
    prev_length: usize,
    max_chain_length: usize,
    max_lazy_match: usize,
    good_match: usize,
    nice_match: usize,
    trees: Trees,
    adler_a: u32,
    adler_b: u32,
    total_in: u64,
}

impl Deflater {
    /// `deflateInit_(level)` with `deflateInit2` defaults, plus the zlib
    /// header the first `deflate` call writes.
    pub fn new(level: u8) -> Result<Self, UnsupportedLevel> {
        if !(4..=9).contains(&level) {
            return Err(UnsupportedLevel(level));
        }
        let w_bits = MAX_WBITS;
        let w_size = 1usize << w_bits;
        let hash_bits = DEF_MEM_LEVEL + 7;
        let hash_size = 1usize << hash_bits;
        let lit_bufsize = 1usize << (DEF_MEM_LEVEL + 6);
        let (good, lazy, nice, chain) = CONFIGURATION[level as usize];
        let mut d = Deflater {
            w_size,
            w_mask: w_size - 1,
            window: vec![0; 2 * w_size],
            window_size: 2 * w_size,
            prev: vec![0; w_size],
            head: vec![NIL; hash_size],
            ins_h: 0,
            hash_mask: hash_size - 1,
            hash_shift: (hash_bits + MIN_MATCH as u32 - 1) / MIN_MATCH as u32,
            block_start: 0,
            match_length: MIN_MATCH - 1,
            prev_match: 0,
            match_available: false,
            strstart: 0,
            match_start: 0,
            lookahead: 0,
            prev_length: MIN_MATCH - 1,
            max_chain_length: chain,
            max_lazy_match: lazy,
            good_match: good,
            nice_match: nice,
            trees: Trees::new(lit_bufsize),
            adler_a: 1,
            adler_b: 0,
            total_in: 0,
        };
        // deflate(): header (Z_DEFLATED + (w_bits-8)<<4) << 8, level flags
        // (level-1)>>1 capped at 3 (zlib 1.1.4's formula), check bits.
        let mut header: u32 = (8 + ((w_bits - 8) << 4)) << 8;
        let level_flags = (u32::from(level) - 1) >> 1;
        header |= level_flags.min(3) << 6;
        header += 31 - (header % 31);
        d.trees.out.push((header >> 8) as u8);
        d.trees.out.push(header as u8);
        Ok(d)
    }

    /// Bytes read so far (`total_in`).
    pub fn total_in(&self) -> u64 {
        self.total_in
    }

    /// Compressed bytes emitted so far.
    pub fn output(&self) -> &[u8] {
        &self.trees.out
    }

    /// `deflate(Z_NO_FLUSH)` on `input` (all of it is consumed).
    pub fn write(&mut self, input: &[u8]) {
        if !input.is_empty() || self.lookahead != 0 {
            let mut pos = 0;
            self.deflate_slow(input, &mut pos, false);
        }
    }

    /// `deflate(Z_FINISH)`: the last block and the Adler-32 trailer.
    pub fn finish(mut self) -> Vec<u8> {
        let mut pos = 0;
        self.deflate_slow(&[], &mut pos, true);
        let adler = (self.adler_b << 16) | self.adler_a;
        self.trees.out.extend_from_slice(&adler.to_be_bytes());
        self.trees.out
    }

    fn max_dist(&self) -> usize {
        self.w_size - MIN_LOOKAHEAD
    }

    fn update_hash(&self, h: usize, c: u8) -> usize {
        ((h << self.hash_shift) ^ c as usize) & self.hash_mask
    }

    /// `INSERT_STRING`: returns the previous head of the chain.
    fn insert_string(&mut self, s: usize) -> u16 {
        self.ins_h = self.update_hash(self.ins_h, self.window[s + MIN_MATCH - 1]);
        let head = self.head[self.ins_h];
        self.prev[s & self.w_mask] = head;
        self.head[self.ins_h] = s as u16;
        head
    }

    fn adler(&mut self, bytes: &[u8]) {
        const BASE: u32 = 65521;
        for chunk in bytes.chunks(5552) {
            for &b in chunk {
                self.adler_a += u32::from(b);
                self.adler_b += self.adler_a;
            }
            self.adler_a %= BASE;
            self.adler_b %= BASE;
        }
    }

    /// `fill_window` + `read_buf`.
    fn fill_window(&mut self, input: &[u8], pos: &mut usize) {
        loop {
            let mut more = self.window_size - self.lookahead - self.strstart;
            if self.strstart >= self.w_size + self.max_dist() {
                let w = self.w_size;
                self.window.copy_within(w..2 * w, 0);
                self.match_start = self.match_start.wrapping_sub(w);
                self.strstart -= w;
                self.block_start -= w as i64;
                for p in self.head.iter_mut() {
                    let m = *p as usize;
                    *p = if m >= w { (m - w) as u16 } else { NIL };
                }
                for p in self.prev.iter_mut() {
                    let m = *p as usize;
                    *p = if m >= w { (m - w) as u16 } else { NIL };
                }
                more += w;
            }
            if *pos == input.len() {
                return;
            }
            let n = more.min(input.len() - *pos);
            if n != 0 {
                let at = self.strstart + self.lookahead;
                let src = &input[*pos..*pos + n];
                self.window[at..at + n].copy_from_slice(src);
                self.adler(src);
                *pos += n;
                self.total_in += n as u64;
            }
            self.lookahead += n;
            if self.lookahead >= MIN_MATCH {
                self.ins_h = self.window[self.strstart] as usize;
                self.ins_h = self.update_hash(self.ins_h, self.window[self.strstart + 1]);
            }
            if !(self.lookahead < MIN_LOOKAHEAD && *pos != input.len()) {
                return;
            }
        }
    }

    /// `longest_match` (portable C version).
    fn longest_match(&mut self, mut cur_match: usize) -> usize {
        let mut chain_length = self.max_chain_length;
        let scan = self.strstart;
        let mut best_len = self.prev_length;
        let mut nice_match = self.nice_match;
        let limit = if self.strstart > self.max_dist() { self.strstart - self.max_dist() } else { 0 };
        let w = &self.window;
        let mut scan_end1 = w[(scan + best_len).wrapping_sub(1)];
        let mut scan_end = w[scan + best_len];
        if self.prev_length >= self.good_match {
            chain_length >>= 2;
        }
        if nice_match > self.lookahead {
            nice_match = self.lookahead;
        }
        loop {
            let m = cur_match;
            if w[m + best_len] == scan_end
                && w[(m + best_len).wrapping_sub(1)] == scan_end1
                && w[m] == w[scan]
                && w[m + 1] == w[scan + 1]
            {
                // scan[2] and match[2] are not compared: equal hash keys
                // with equal first two bytes imply an equal third byte.
                let mut len = 3;
                while len < MAX_MATCH && w[scan + len] == w[m + len] {
                    len += 1;
                }
                if len > best_len {
                    self.match_start = cur_match;
                    best_len = len;
                    if len >= nice_match {
                        break;
                    }
                    scan_end1 = w[scan + best_len - 1];
                    scan_end = w[scan + best_len];
                }
            }
            cur_match = self.prev[cur_match & self.w_mask] as usize;
            if cur_match <= limit {
                break;
            }
            chain_length -= 1;
            if chain_length == 0 {
                break;
            }
        }
        best_len.min(self.lookahead)
    }

    /// `FLUSH_BLOCK_ONLY`.
    fn flush_block(&mut self, eof: bool) {
        let stored_len = (self.strstart as i64 - self.block_start) as u64;
        let buf = if self.block_start >= 0 {
            let at = self.block_start as usize;
            Some(&self.window[at..at + stored_len as usize])
        } else {
            None
        };
        self.trees.flush_block(buf, stored_len, eof);
        self.block_start = self.strstart as i64;
    }

    /// `deflate_slow`.
    fn deflate_slow(&mut self, input: &[u8], pos: &mut usize, finish: bool) {
        // A local of zlib 1.1.4's deflate_slow: NIL on entry, and not reset
        // at a position too near the end to insert (its value is then only
        // used for a search whose result is clipped below MIN_MATCH).
        let mut hash_head: u16 = NIL;
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(input, pos);
                if self.lookahead < MIN_LOOKAHEAD && !finish {
                    return;
                }
                if self.lookahead == 0 {
                    break;
                }
            }
            if self.lookahead >= MIN_MATCH {
                hash_head = self.insert_string(self.strstart);
            }
            self.prev_length = self.match_length;
            self.prev_match = self.match_start;
            self.match_length = MIN_MATCH - 1;

            if hash_head != NIL
                && self.prev_length < self.max_lazy_match
                && self.strstart.wrapping_sub(hash_head as usize) <= self.max_dist()
            {
                self.match_length = self.longest_match(hash_head as usize);
                if self.match_length <= 5
                    && self.match_length == MIN_MATCH
                    && self.strstart.wrapping_sub(self.match_start) > TOO_FAR
                {
                    self.match_length = MIN_MATCH - 1;
                }
            }
            if self.prev_length >= MIN_MATCH && self.match_length <= self.prev_length {
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                let bflush = self.trees.tally(self.strstart - 1 - self.prev_match, self.prev_length - MIN_MATCH);
                self.lookahead -= self.prev_length - 1;
                self.prev_length -= 2;
                loop {
                    self.strstart += 1;
                    if self.strstart <= max_insert {
                        hash_head = self.insert_string(self.strstart);
                    }
                    self.prev_length -= 1;
                    if self.prev_length == 0 {
                        break;
                    }
                }
                self.match_available = false;
                self.match_length = MIN_MATCH - 1;
                self.strstart += 1;
                if bflush {
                    self.flush_block(false);
                }
            } else if self.match_available {
                let c = self.window[self.strstart - 1] as usize;
                if self.trees.tally(0, c) {
                    self.flush_block(false);
                }
                self.strstart += 1;
                self.lookahead -= 1;
            } else {
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            let c = self.window[self.strstart - 1] as usize;
            self.trees.tally(0, c);
            self.match_available = false;
        }
        self.flush_block(true);
    }
}

/// One `deflate(Z_NO_FLUSH)` with all of `data`, then `deflate(Z_FINISH)`.
pub fn compress(data: &[u8], level: u8) -> Result<Vec<u8>, UnsupportedLevel> {
    let mut d = Deflater::new(level)?;
    d.write(data);
    Ok(d.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn empty_and_short_streams() {
        assert_eq!(hex(&compress(b"", 6).unwrap()), "789c030000000001");
        assert_eq!(hex(&compress(b"hello", 6).unwrap()), "789ccb48cdc9c90700062c0215");
    }

    /// A regression check without game data (the proof is
    /// tests/shipped_maps.rs): 300,000 bytes of random, repetitive, text-like
    /// and small-alphabet runs, which take stored, dynamic and lazy-match
    /// paths and window slides. Expected: stock zlib's level-6 output for
    /// the same bytes (length and FNV-1a 64), which equals the original's on
    /// every shipped map chunk.
    #[test]
    fn mixed_input_regression() {
        const TEXT: &[u8] = b"to be or not to be, that is the question. ";
        let mut x: u32 = 2463534242;
        let data: Vec<u8> = (0..300_000usize)
            .map(|i| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                match (i / 8192) % 4 {
                    0 => x as u8,
                    1 => (i % 7) as u8 + b'a',
                    2 => TEXT[(x >> 24) as usize % TEXT.len()],
                    _ => (x >> 29) as u8,
                }
            })
            .collect();
        let z = compress(&data, 6).unwrap();
        let fnv = z.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
        assert_eq!((z.len(), fnv), (166_385, 0x0d4f_76a3_b975_69db));
        let mut pieces = Deflater::new(6).unwrap();
        for part in data.chunks(1000) {
            pieces.write(part);
        }
        assert_eq!(pieces.finish(), z);
    }

    #[test]
    fn levels_outside_the_lazy_matcher_are_refused() {
        assert!(Deflater::new(3).is_err());
        assert!(Deflater::new(10).is_err());
    }
}
