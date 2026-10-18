//! Port of the statically linked Visual Studio 2003 CRT `qsort` (`96CE20`)
//! and its `shortsort` (`96CDB0`), as used by the SAI goal selection (`7A93B0`).
//!
//! The comparison sequence, swaps and partition order are reproduced exactly,
//! so the result is identical for comparators that are not strict weak
//! orders (equal or unordered goal priorities). The native 30-entry partition
//! stack becomes a growable vector (no artificial cap; the native algorithm
//! always defers the larger partition, so depth is logarithmic anyway).
use crate::Result;

/// `96CDB0`: selection sort of `v[lo..=hi]` moving the maximum to the end.
fn shortsort<T>(
    v: &mut [T],
    lo: isize,
    mut hi: isize,
    cmp: &mut impl FnMut(&T, &T) -> Result<i32>,
) -> Result<()> {
    while hi > lo {
        let mut max = lo;
        let mut p = lo + 1;
        while p <= hi {
            if cmp(&v[p as usize], &v[max as usize])? > 0 {
                max = p;
            }
            p += 1;
        }
        if max != hi {
            v.swap(max as usize, hi as usize);
        }
        hi -= 1;
    }
    Ok(())
}

/// Complete `96CE20` over whole elements (the native byte-wise swap of
/// `width` bytes is an element swap).
pub fn qsort<T>(v: &mut [T], mut cmp: impl FnMut(&T, &T) -> Result<i32>) -> Result<()> {
    if v.len() < 2 {
        return Ok(());
    }
    let c = &mut cmp;
    let mut stack: Vec<(isize, isize)> = Vec::new();
    let (mut lo, mut hi) = (0_isize, v.len() as isize - 1);
    loop {
        let size = hi - lo + 1;
        if size <= 8 {
            shortsort(v, lo, hi, c)?;
        } else {
            let mut mid = lo + (size >> 1);
            if c(&v[lo as usize], &v[mid as usize])? > 0 {
                v.swap(lo as usize, mid as usize);
            }
            if c(&v[lo as usize], &v[hi as usize])? > 0 {
                v.swap(lo as usize, hi as usize);
            }
            if c(&v[mid as usize], &v[hi as usize])? > 0 {
                v.swap(mid as usize, hi as usize);
            }
            let mut loweq = lo;
            let mut higheq = hi;
            loop {
                // 96CF50: two forms of the upward scan.
                let mut second = mid <= loweq;
                if !second {
                    loop {
                        loweq += 1;
                        if loweq >= mid {
                            second = true;
                            break;
                        }
                        if c(&v[loweq as usize], &v[mid as usize])? > 0 {
                            break;
                        }
                    }
                }
                if second {
                    loop {
                        loweq += 1;
                        if loweq > hi || c(&v[loweq as usize], &v[mid as usize])? > 0 {
                            break;
                        }
                    }
                }
                // 96CF90: downward scan.
                loop {
                    higheq -= 1;
                    if higheq <= mid || c(&v[higheq as usize], &v[mid as usize])? <= 0 {
                        break;
                    }
                }
                if higheq < loweq {
                    break;
                }
                if loweq != higheq {
                    v.swap(loweq as usize, higheq as usize);
                }
                if mid == higheq {
                    mid = loweq;
                }
            }
            // 96CFEC: skip elements equal to the partition element.
            higheq += 1;
            let mut done = false;
            if mid < higheq {
                loop {
                    higheq -= 1;
                    if higheq <= mid {
                        break;
                    }
                    if c(&v[higheq as usize], &v[mid as usize])? != 0 {
                        done = mid < higheq;
                        break;
                    }
                }
            }
            if !done {
                loop {
                    higheq -= 1;
                    if higheq <= lo || c(&v[higheq as usize], &v[mid as usize])? != 0 {
                        break;
                    }
                }
            }
            if higheq - lo >= hi - loweq {
                if lo < higheq {
                    stack.push((lo, higheq));
                }
                if loweq < hi {
                    lo = loweq;
                    continue;
                }
            } else {
                if loweq < hi {
                    stack.push((loweq, hi));
                }
                if lo < higheq {
                    hi = higheq;
                    continue;
                }
            }
        }
        match stack.pop() {
            Some((l, h)) => {
                lo = l;
                hi = h;
            }
            None => return Ok(()),
        }
    }
}
