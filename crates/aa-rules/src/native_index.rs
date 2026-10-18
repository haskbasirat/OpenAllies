//! Original data index expansion: which TGI files AA.exe loads, in what order.
//!
//! Port of the original loader chain, not a directory walk:
//! - 48DB60 (GameData init, 4901DC..49024C): enumerates root resources matching
//!   `index*.LST` (85BD30 with full-path, non-recursive flags) and calls
//!   48D970 LoadIndex for each, in the sorted order.
//! - 5AB590 KKC_IndexFileResource::LoadIndex: for each stored line (5AB3C0
//!   keeps nonempty lines after right-trimming CR/LF), skips lines whose
//!   first unit is `;`, resolves the line against the index's directory
//!   (858850), and either recurses into a `...LST` entry (5AB350) or appends
//!   the file; lines containing `?` or `*` are enumerated through 85BD30.
//! - 85BD30/85B980/5AF720/5AEF60: the resource manager enumeration. A
//!   resource matches when the whole-path wildcard (859830) matches AND the
//!   per-`/` segment rule of 5AEF60 holds (no recursion into deeper
//!   directories). Matches are sorted by the original qsort comparator
//!   (859A10, ordinal code-unit order).
//!
//! Removed engine cap (recorded in research/phase8-definitions/findings.json):
//! 5AB3C0 reads index lines through an 0x800-unit line buffer; this port has
//! no line-length limit. Original index files never approach it.

/// Original 859830 wildcard matcher (exact control flow, including quirks):
/// `*` records the next pattern unit and skips the path until that unit; once
/// any `*` was seen the result is a match whenever either side is exhausted.
/// Comparison is CRT `toupper` on each UTF-16 unit (ASCII-only folding).
pub fn native_wildcard_match(pattern: &[u16], path: &[u16]) -> bool {
    fn upper(c: u16) -> u16 {
        if (u16::from(b'a')..=u16::from(b'z')).contains(&c) {
            c - 0x20
        } else {
            c
        }
    }
    let lp = pattern.len();
    let ls = path.len();
    let (mut i, mut j) = (0usize, 0usize);
    let mut skipping = false;
    let mut seen_star = false;
    let mut target = 0u16;
    if lp > 0 {
        loop {
            if ls <= j {
                break;
            }
            if skipping {
                if path[j] == target {
                    skipping = false;
                } else {
                    j += 1;
                }
            } else {
                let c = pattern[i];
                if c != u16::from(b'*') {
                    if upper(path[j]) != upper(c) {
                        return false;
                    }
                    i += 1;
                    j += 1;
                } else {
                    target = pattern.get(i + 1).copied().unwrap_or(0);
                    i += 1;
                    skipping = true;
                    seen_star = true;
                }
            }
            if i >= lp {
                break;
            }
        }
        if seen_star {
            return true;
        }
    }
    lp == ls
}

fn find_slash(s: &[u16]) -> Option<usize> {
    s.iter().position(|&c| c == u16::from(b'/'))
}

/// Original 5AEF60 per-resource test with state+20C == 0 (index enumeration):
/// whole-path wildcard first, then directory segments compared pairwise.
pub fn native_resource_matches(pattern: &[u16], path: &[u16]) -> bool {
    if !native_wildcard_match(pattern, path) {
        return false;
    }
    let mut pat = pattern;
    let mut rest = path;
    loop {
        let ps = find_slash(pat);
        let rs = find_slash(rest);
        let Some(ps) = ps else {
            return rs.is_none() && native_wildcard_match(pat, rest);
        };
        let Some(rs) = rs else {
            return false;
        };
        if !native_wildcard_match(&pat[..ps], &rest[..rs]) {
            return false;
        }
        let pat_tail = pat.len() - ps - 1;
        let rest_tail = rest.len() - rs - 1;
        // 5AF3E2..5AF3FA: an empty remainder on either side jumps to the
        // successful copy-out (LAB_005af592 -> LAB_005af621).
        if pat_tail < 1 || rest_tail < 1 {
            return true;
        }
        pat = &pat[ps + 1..];
        rest = &rest[rs + 1..];
    }
}

/// Original 85BD30 collection + 859A10 comparator: ordinal UTF-16 order.
pub fn native_enumerate(pattern: &str, resources: &[String]) -> Vec<String> {
    let p: Vec<u16> = pattern.encode_utf16().collect();
    let mut out: Vec<(Vec<u16>, String)> = resources
        .iter()
        .filter_map(|r| {
            let units: Vec<u16> = r.encode_utf16().collect();
            native_resource_matches(&p, &units).then(|| (units, r.clone()))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.into_iter().map(|(_, r)| r).collect()
}

/// 8562B0 case-insensitive substring search for `LST`; 5AB350/5AB6xx treat a
/// line as an index only when the FIRST occurrence ends the string.
pub fn native_is_index_name(path: &str) -> bool {
    let units: Vec<u16> = path.encode_utf16().map(|c| c.to_ascii_uppercase_u16()).collect();
    let needle: Vec<u16> = "LST".encode_utf16().collect();
    match units.windows(needle.len()).position(|w| w == needle.as_slice()) {
        Some(at) => at == units.len() - needle.len(),
        None => false,
    }
}

trait AsciiUpper16 {
    fn to_ascii_uppercase_u16(self) -> u16;
}
impl AsciiUpper16 for u16 {
    fn to_ascii_uppercase_u16(self) -> u16 {
        // _towupper in the C locale folds only ASCII letters.
        if (u16::from(b'a')..=u16::from(b'z')).contains(&self) {
            self - 0x20
        } else {
            self
        }
    }
}

/// 858850: a leading `/` is removed; otherwise a line is prefixed with the
/// directory of the index file that contains it (text before its last `/`).
pub fn native_resolve_line(index_path: &str, line: &str) -> String {
    if let Some(stripped) = line.strip_prefix('/') {
        return stripped.trim_start_matches('/').to_owned();
    }
    match index_path.rfind('/') {
        Some(at) => format!("{}/{}", &index_path[..at], line),
        None => line.to_owned(),
    }
}

/// 5AB3C0: nonempty lines after the CR/LF right trim (425310 with "\r\n").
/// Bytes are widened one-to-one, as the original line reader does.
pub fn native_index_lines(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in bytes.split(|&b| b == b'\n') {
        let mut end = raw.len();
        while end > 0 && matches!(raw[end - 1], b'\r' | b'\n') {
            end -= 1;
        }
        if end == 0 {
            continue;
        }
        out.push(raw[..end].iter().map(|&b| char::from(b)).collect());
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct IndexedFile {
    /// Resource path as registered (actual spelling of the matched resource).
    pub path: String,
    /// The index file and 1-based stored-line ordinal that produced it.
    pub index: String,
    pub line: usize,
    /// The wildcard line, when the file came from an enumeration.
    pub pattern: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct IndexExpansion {
    /// Index files in LoadIndex order (root `index*.LST` and nested `.lst`).
    pub index_files: Vec<String>,
    /// Data files in the order 48D970 hands them to the TGI loader.
    pub files: Vec<IndexedFile>,
}

fn find_resource<'a>(resources: &'a [String], path: &str) -> Option<&'a String> {
    resources.iter().find(|r| r.eq_ignore_ascii_case(path))
}

/// Complete 48DB60 index chain over the registered resource paths. `read`
/// returns the bytes of an index resource by its registered path.
pub fn native_expand_index(
    resources: &[String],
    read: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<IndexExpansion, String> {
    let mut out = IndexExpansion::default();
    for index in native_enumerate("index*.LST", resources) {
        load_index(&index, resources, read, &mut out, &mut Vec::new())?;
    }
    Ok(out)
}

fn load_index(
    index: &str,
    resources: &[String],
    read: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    out: &mut IndexExpansion,
    open: &mut Vec<String>,
) -> Result<(), String> {
    // The original recurses without a guard (a cyclic index would exhaust
    // its stack); a cycle is reported as an explicit error instead.
    if open.iter().any(|i| i.eq_ignore_ascii_case(index)) {
        return Err(format!("cyclic index file reference {index}"));
    }
    let registered = find_resource(resources, index)
        .ok_or_else(|| format!("Unable to find index file '{index}'"))?
        .clone();
    out.index_files.push(registered.clone());
    open.push(registered.clone());
    let bytes = read(&registered)?;
    for (n, line) in native_index_lines(&bytes).into_iter().enumerate() {
        if line.starts_with(';') {
            continue;
        }
        let resolved = native_resolve_line(index, &line);
        if resolved.contains(['?', '*']) {
            for found in native_enumerate(&resolved, resources) {
                if native_is_index_name(&found) {
                    load_index(&found, resources, read, out, open)?;
                } else {
                    out.files.push(IndexedFile {
                        path: found,
                        index: registered.clone(),
                        line: n + 1,
                        pattern: Some(resolved.clone()),
                    });
                }
            }
        } else if native_is_index_name(&resolved) {
            load_index(&resolved, resources, read, out, open)?;
        } else {
            let path = find_resource(resources, &resolved)
                .cloned()
                .unwrap_or(resolved.clone());
            out.files.push(IndexedFile {
                path,
                index: registered.clone(),
                line: n + 1,
                pattern: None,
            });
        }
    }
    open.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn m(p: &str, s: &str) -> bool {
        let p: Vec<u16> = p.encode_utf16().collect();
        let s: Vec<u16> = s.encode_utf16().collect();
        native_resource_matches(&p, &s)
    }
    #[test]
    fn segment_rule_does_not_recurse() {
        assert!(m("Units/USA/*.tgi", "Units/USA/usa_battleship.tgi"));
        assert!(!m("Units/*.tgi", "Units/USA/usa_battleship.tgi"));
        assert!(!m(
            "Buildings/Villages/EasternEurope/*.tgi",
            "Buildings/Villages/EasternEurope/EasternEuropeCityCenter02/eastern_europe_city_center02.tgi"
        ));
        assert!(m("index*.LST", "index_aa.lst"));
        assert!(native_is_index_name("templates_AA.lst"));
        assert!(!native_is_index_name("Lists.tgi"));
    }
    #[test]
    fn index_lines_nest_in_place_and_keep_duplicates() {
        let resources: Vec<String> = [
            "index_aa.lst",
            "sub/inner.lst",
            "b.tgi",
            "a.tgi",
            "B.tgi",
            "sub/x.tgi",
            "sub/deeper/y.tgi",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let mut read = |path: &str| -> Result<Vec<u8>, String> {
            Ok(match path {
                "index_aa.lst" => b"; comment\r\n\r\n*.tgi\r\nsub/inner.lst\r\na.tgi\r\n".to_vec(),
                "sub/inner.lst" => b"x.tgi\r\n/a.tgi\r\n*.tgi".to_vec(),
                _ => return Err(path.into()),
            })
        };
        let out = native_expand_index(&resources, &mut read).unwrap();
        let files: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
        // Ordinal sort: "B.tgi" < "a.tgi" < "b.tgi"; inner.lst lines are
        // relative to "sub/", a leading '/' is root-relative; no dedup.
        assert_eq!(
            files,
            ["B.tgi", "a.tgi", "b.tgi", "sub/x.tgi", "a.tgi", "sub/x.tgi", "a.tgi"]
        );
        assert_eq!(out.index_files, ["index_aa.lst", "sub/inner.lst"]);
    }
}
