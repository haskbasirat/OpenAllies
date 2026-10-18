//! Mutable49AE00 MapString assignment over the retained global text table.
//! The caller passes the actual pair's text slot; that write occurs before
//! registry lookup and survives a reached error. The original file provenance
//! remains the entry's origin after a runtime text mutation.
//! Native creation of a previously missing entry is still required explicitly.
use crate::localization::Localization;

pub fn set_existing_pair(
    table: &mut Localization,
    actual_key: &[u16],
    actual_pair_text: &mut Vec<u16>,
    supplied_text: &[u16],
) -> Result<(), String> {
    //49AE08..0C: CString value assignment retains embedded NULs and full length.
    actual_pair_text.clear();
    actual_pair_text.extend_from_slice(supplied_text);
    if actual_key.is_empty() {
        return Ok(());
    }
    if actual_key.iter().any(|&c| c == 0 || c > 127) {
        return Err("49AE00 MapString key needs actual native locale/NUL hash producer".into());
    }
    let key = String::from_utf16(actual_key)
        .map_err(|e| e.to_string())?
        .to_ascii_lowercase();
    let rows = table
        .declarations
        .get_mut(&key)
        .ok_or("49AE00/4BAA20 missing MapString requires its native entry creation producer")?;
    let [entry] = rows.as_mut_slice() else {
        return Err("49AE00 duplicate MapString registration ownership is unbound".into());
    };
    //49AE49..4D: after Find(create1) returns its existing allocation, overwrite
    //that SAME entry+10. No cloned localization table or copied actor exists.
    entry.plain_utf16 = Ok(supplied_text.to_vec());
    Ok(())
}

#[cfg(test)]
#[path = "live_map_strings_tests.rs"]
mod tests;
