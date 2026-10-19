//!Aspect TextureRename list (46A8A0 Aspect+3C/+40), the46A180 Aspect
//!texture-source chain and469D10 CStringT::Replace over counted UTF-16.
//!Proof: research/mapload-20261018/aspect-texture-rename (actual469D10 and
//!49C670->46A510->469A70->46A8A0->46A180 runs, original and v14, all PC).
//!Nothing here acquires a Texture: the returned name is the exact5A79F0
//!input; the caller owns that request.
use super::super::path;
type Result<T> = std::result::Result<T, String>;

///One46A8A0 node (operator new 0xC): +0 substring_from, +4 substring_to,
///+8 next. Both strings keep their counted CString content; 46A180 hands
///their data pointers to469D10, which reads them as C strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextureRename {
    pub(crate) from: Vec<u16>,
    pub(crate) to: Vec<u16>,
}

///Aspect+3C in46A180 traversal order. 46A8A0 PREPENDS each TextureRename
///child (+40 is set only by the first node), so the head is the LAST source
///child and renames apply in reverse source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextureRenames {
    traversal: Vec<TextureRename>,
}

impl TextureRenames {
    ///46A8A0: children in source order (index0..count) become the list.
    pub(crate) fn from_source_order(children: Vec<TextureRename>) -> Self {
        let mut traversal = children;
        traversal.reverse();
        Self { traversal }
    }
    ///Head-first (+3C, +8 links), exactly the46A180 application order.
    #[cfg(test)]
    pub(crate) fn traversal(&self) -> &[TextureRename] {
        &self.traversal
    }
    ///46A180(source, flag). `None` is the native early null: an empty source
    ///CString returns0 before any copy, rename, format or5A79F0 request.
    ///Otherwise the shared copy receives every469D10 Replace in traversal
    ///order; flag1 (`prefix` = 4108C0 Aspect+0 + Aspect+4, with NO '/'
    ///special case) formats it through405D60, flag0 assigns it unchanged;
    ///4763C0 then makes it relative to `folder`, the CURRENT 0xB36530 source
    ///context. The result is the exact name passed to5A79F0 (even if empty).
    pub(crate) fn texture_name_46a180(
        &self,
        source: &str,
        prefix: Option<&str>,
        folder: &str,
    ) -> Result<Option<String>> {
        if source.is_empty() {
            return Ok(None);
        }
        let mut copy: Vec<u16> = source.encode_utf16().collect();
        for rename in &self.traversal {
            replace_469d10(&mut copy, &rename.from, &rename.to)?;
        }
        let copy = String::from_utf16(&copy)
            .map_err(|_| "46A180 renamed source is not portable UTF-16 text".to_owned())?;
        let named = match prefix {
            Some(prefix) => {
                //405D60 reads its format and the %s argument as C strings.
                if copy.contains('\0') || prefix.contains('\0') {
                    return Err(
                        "46A180 405D60 embedded-NUL format/argument is outside the admitted domain"
                            .into(),
                    );
                }
                path::expand(&copy, prefix)?
            }
            None => copy,
        };
        path::relative(folder, &named).map(Some)
    }
}

fn c_len(s: &[u16]) -> usize {
    s.iter().position(|&u| u == 0).unwrap_or(s.len())
}
///96C26D wcslen inside the469D10 buffer. Storage that no store of this
///operation (or the preserved original content) wrote is not ground truth.
fn scan_len(buffer: &[Option<u16>], at: usize) -> Result<usize> {
    let mut i = at;
    loop {
        match buffer.get(i) {
            Some(Some(0)) => return Ok(i - at),
            Some(Some(_)) => i += 1,
            _ => return Err("469D10 wide scan reaches storage no native store wrote".into()),
        }
    }
}
///96C74E wcsstr on the C string at `at`; `old` is nonempty and NUL-free.
fn find(buffer: &[Option<u16>], at: usize, old: &[u16]) -> Result<Option<usize>> {
    let n = scan_len(buffer, at)?;
    if n < old.len() {
        return Ok(None);
    }
    Ok((at..=at + n - old.len()).find(|&k| {
        buffer[k..k + old.len()]
            .iter()
            .zip(old)
            .all(|(a, b)| *a == Some(*b))
    }))
}

///469D10 CStringT::Replace(pszOld, pszNew). `value` is the counted CString
///content (embedded NULs allowed); `old`/`new` end at their first NUL, and
///an empty `old` (or NULL) returns0 with the content unchanged. The count
///pass and the replace pass both walk NUL-separated segments up to the
///ORIGINAL length; the published length is the count-pass prediction. A
///negative memmove balance (native access violation) or a published unit no
///native store wrote is an explicit error, and `value` is then unchanged.
pub(crate) fn replace_469d10(value: &mut Vec<u16>, old: &[u16], new: &[u16]) -> Result<i32> {
    let old = &old[..c_len(old)];
    let new = &new[..c_len(new)];
    let n_src = old.len();
    if n_src == 0 {
        return Ok(0);
    }
    let domain = || "469D10 length outside the native signed int domain".to_owned();
    let old_len = value.len();
    let old_len_i = i32::try_from(old_len).map_err(|_| domain())?;
    let n_src_i = i32::try_from(n_src).map_err(|_| domain())?;
    let n_rep_i = i32::try_from(new.len()).map_err(|_| domain())?;
    let mut buffer: Vec<Option<u16>> = value.iter().copied().map(Some).collect();
    buffer.push(Some(0));
    let mut count: i32 = 0;
    let mut i = 0;
    if i < old_len {
        loop {
            while let Some(t) = find(&buffer, i, old)? {
                i = t + n_src;
                count = count.checked_add(1).ok_or_else(domain)?;
            }
            i += scan_len(&buffer, i)? + 1;
            if i >= old_len {
                break;
            }
        }
    }
    if count <= 0 {
        return Ok(count);
    }
    let new_len_i = (n_rep_i - n_src_i)
        .checked_mul(count)
        .and_then(|v| v.checked_add(old_len_i))
        .ok_or_else(domain)?;
    let new_len = usize::try_from(new_len_i).map_err(|_| domain())?;
    //401330 PrepareWrite2(max): Fork/Reallocate preserve [0, old_len]; any
    //other storage holds no value this operation can rely on.
    buffer.resize(new_len.max(old_len) + 1, None);
    let mut current = old_len;
    let mut i = 0;
    while i < old_len {
        while let Some(t) = find(&buffer, i, old)? {
            let balance = current as isize - t as isize - n_src as isize;
            if balance < 0 {
                return Err("469D10 negative memmove balance faults natively".into());
            }
            let balance = balance as usize;
            if t + new.len() + balance >= buffer.len() {
                return Err("469D10 store beyond PrepareWrite2 storage".into());
            }
            buffer.copy_within(t + n_src..t + n_src + balance, t + new.len());
            for (k, &u) in new.iter().enumerate() {
                buffer[t + k] = Some(u);
            }
            buffer[t + new.len() + balance] = Some(0);
            current = current + new.len() - n_src;
            i = t + new.len();
        }
        i += scan_len(&buffer, i)? + 1;
    }
    let published = buffer[..new_len]
        .iter()
        .copied()
        .collect::<Option<Vec<u16>>>()
        .ok_or("469D10 publishes length over storage no native store wrote")?;
    *value = published;
    Ok(count)
}

#[cfg(test)]
#[path = "data_aspect_rename_tests.rs"]
mod tests;
