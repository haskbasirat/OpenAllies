//! The original's CRT `qsort` (Visual Studio 2003, `_qsort` 96CE20 with
//! `shortsort` 96CDB0). It is not stable: the order of equal elements is a
//! property of this exact algorithm, so it is reproduced step by step
//! (element pointers become signed indices so out-of-range intermediate
//! positions behave like the native pointer comparisons).
//! Used by the shop loader (962290 sorts every shop list by cost with 961790).
//! Proven by `research/phase8-metagame/native-qsort.json`.
use std::cmp::Ordering;

const CUTOFF: isize = 8;

/// Sort `v` exactly as `_qsort(v, n, w, cmp)` would, where `cmp(a, b)` returns
/// the native comparator's sign. The explicit stack is growable (native: 30 entries,
/// which is sufficient for any 32-bit element count).
pub fn msvc_qsort<T, F>(v: &mut [T], mut cmp: F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    if v.len() < 2 {
        return;
    }
    let mut c = |v: &[T], a: isize, b: isize| cmp(&v[a as usize], &v[b as usize]);
    let mut stack: Vec<(isize, isize)> = Vec::new();
    let mut lo: isize = 0;
    let mut hi: isize = v.len() as isize - 1;
    loop {
        let size = hi - lo + 1;
        if size <= CUTOFF {
            // shortsort: repeatedly move the first maximum to the end.
            let mut h = hi;
            while h > lo {
                let mut max = lo;
                let mut p = lo + 1;
                while p <= h {
                    if c(v, p, max) == Ordering::Greater {
                        max = p;
                    }
                    p += 1;
                }
                v.swap(max as usize, h as usize);
                h -= 1;
            }
        } else {
            let mut mid = lo + size / 2;
            if c(v, lo, mid) == Ordering::Greater {
                v.swap(lo as usize, mid as usize);
            }
            if c(v, lo, hi) == Ordering::Greater {
                v.swap(lo as usize, hi as usize);
            }
            if c(v, mid, hi) == Ordering::Greater {
                v.swap(mid as usize, hi as usize);
            }
            let mut loguy = lo;
            let mut higuy = hi;
            loop {
                if mid > loguy {
                    loop {
                        loguy += 1;
                        if !(loguy < mid && c(v, loguy, mid) != Ordering::Greater) {
                            break;
                        }
                    }
                }
                if mid <= loguy {
                    loop {
                        loguy += 1;
                        if !(loguy <= hi && c(v, loguy, mid) != Ordering::Greater) {
                            break;
                        }
                    }
                }
                loop {
                    higuy -= 1;
                    if !(higuy > mid && c(v, higuy, mid) == Ordering::Greater) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                v.swap(loguy as usize, higuy as usize);
                if mid == higuy {
                    mid = loguy;
                }
            }
            higuy += 1;
            if mid < higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > mid && c(v, higuy, mid) == Ordering::Equal) {
                        break;
                    }
                }
            }
            if mid >= higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > lo && c(v, higuy, mid) == Ordering::Equal) {
                        break;
                    }
                }
            }
            if higuy - lo >= hi - loguy {
                if lo < higuy {
                    stack.push((lo, higuy));
                }
                if loguy < hi {
                    lo = loguy;
                    continue;
                }
            } else {
                if loguy < hi {
                    stack.push((loguy, hi));
                }
                if lo < higuy {
                    hi = higuy;
                    continue;
                }
            }
        }
        match stack.pop() {
            Some((l, h)) => {
                lo = l;
                hi = h;
            }
            None => return,
        }
    }
}
