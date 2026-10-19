//! KKC_AudioData's scalar fields as 46C3A0 LoadData stores them, from the
//! actual source-registered Audio definition (registry mode 0):
//! +18 category (46BD40 registry SOUND/MUSIC/SPEECH, -1 unknown), +1C
//! priority (LOWEST..CRITICAL, -1 unknown), +20/+21/+22 control/world/who
//! flags (46C1C0: tokens split on ", ", each known name ORs 1 << ordinal into
//! the byte; an unknown name warns and ORs nothing), +24 volume, +28/+2C
//! range, +30 simultaneous_limit, +58 fade_time_s, +60 delay_ms, +68
//! pitch_adjust, +70 volume_adjust, +78 ldf_length (GetFixed) and the three
//! file lists as the depot-relative paths 46BB60 resolves (4763C0 against
//! the definition's folder). The Subtitle block (46B850, +7C) is reported
//! only as present/absent: a subtitled Audio is an explicit unsupported case.
use crate::catalog::Catalog;
use aa_numeric::Precision;
use aa_rules::native::TypedValue;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq)]
pub struct Fields {
    pub ids: String,
    pub category: i32,
    pub priority: i32,
    pub flags: [u8; 3],
    pub volume: u32,
    pub range: [u32; 2],
    pub simultaneous_limit: i32,
    pub fade_time_s: [u32; 2],
    pub delay_ms: [u32; 2],
    pub pitch_adjust: [u32; 2],
    pub volume_adjust: u32,
    pub ldf_length: u32,
    pub ignore_rate: bool,
    /// leadin, files, fallout: depot-relative paths in source order.
    pub lists: [Vec<String>; 3],
    /// The definition's source folder (for diagnostics).
    pub folder: String,
}

const CONTROL: [&str; 7] = ["INTERRUPT", "SEQUENTIAL", "ALL", "VOICE", "WORLD", "CAUSEFADE", "TESTSOUND"];
const WORLD: [&str; 2] = ["GLOBAL", "FOGGED"];
const WHO: [&str; 6] = ["LOCAL", "PLAYER", "ALLIES", "NEUTRALS", "ENEMIES", "EVERYONE"];
const PRIORITY: [&str; 5] = ["LOWEST", "LOW", "MEDIUM", "HIGH", "CRITICAL"];
const CATEGORY: [&str; 3] = ["SOUND", "MUSIC", "SPEECH"];

fn ordinal(names: &[&str], value: &str) -> i32 {
    names.iter().position(|n| n.eq_ignore_ascii_case(value)).map_or(-1, |i| i as i32)
}
/// 46C1C0 over one flags string.
pub fn flags_46c1c0(names: &[&str], value: &str) -> u8 {
    let mut byte = 0u8;
    for token in value.split([',', ' ']).filter(|t| !t.is_empty()) {
        let index = ordinal(names, token);
        //'\x01' << (index & 31) stored to a byte: an unknown (-1) adds nothing.
        byte |= (1u32.wrapping_shl(index as u32 & 31) & 0xff) as u8;
    }
    byte
}

fn field(node: &aa_rules::actors::ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc).map_err(|e| e.to_string())?.ok_or_else(|| format!("Audio field {name} absent"))
}
fn text(node: &aa_rules::actors::ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String> {
    match field(node, name, pc)? {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(format!("Audio {name} is not a string")),
    }
}
fn scalar(node: &aa_rules::actors::ResolvedNode<'_>, name: &str, pc: Precision) -> Result<u32> {
    match field(node, name, pc)? {
        TypedValue::Scalar(s) => Ok(s.bits),
        _ => Err(format!("Audio {name} is not a scalar")),
    }
}
fn pair(node: &aa_rules::actors::ResolvedNode<'_>, name: &str, pc: Precision) -> Result<[u32; 2]> {
    match field(node, name, pc)? {
        TypedValue::Point { bits, .. } if bits.len() >= 2 => Ok([bits[0], bits[1]]),
        _ => Err(format!("Audio {name} is not a point2")),
    }
}
fn is_null(value: &str) -> bool {
    value.is_empty() || value.eq_ignore_ascii_case("NULL")
}

/// The source-registered Audio definition named `ids` (one-hop translation
/// is the caller's: native 416370 translates before 4915F0).
pub fn read(catalog: &Catalog<'_>, ids: &str, pc: Precision) -> Result<Fields> {
    let registry = catalog.actors.native_data_registry("Audio").ok_or("actual Audio registry missing")?;
    let entry = registry.entries.iter().find(|v| v.ids.eq_ignore_ascii_case(ids)).ok_or_else(|| format!("Audio '{ids}' is not registered"))?;
    let node = catalog.actors.native_data_node(entry).map_err(|e| e.to_string())?;
    let path = entry.source.path.replace('\\', "/");
    let folder = path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
    let mut lists: [Vec<String>; 3] = Default::default();
    for (i, name) in ["leadin", "files", "fallout"].into_iter().enumerate() {
        let value = text(&node, name, pc)?;
        if is_null(&value) {
            continue;
        }
        for token in value.split([',', ' ']).filter(|t| !t.is_empty()) {
            lists[i].push(crate::data_pre_request::path::relative(&folder, token)?);
        }
    }
    let flag = |name: &str, names: &[&str]| -> Result<u8> {
        let value = text(&node, name, pc)?;
        Ok(if is_null(&value) { 0 } else { flags_46c1c0(names, &value) })
    };
    let ignore_rate = match field(&node, "ignore_rate", pc)? {
        TypedValue::Boolean { value, .. } => value,
        _ => return Err("Audio ignore_rate is not a bool".into()),
    };
    let limit = match field(&node, "simultaneous_limit", pc)? {
        TypedValue::Integer { value, .. } => value,
        _ => return Err("Audio simultaneous_limit is not an int".into()),
    };
    Ok(Fields {
        ids: entry.ids.clone(),
        category: ordinal(&CATEGORY, &text(&node, "category", pc)?),
        priority: ordinal(&PRIORITY, &text(&node, "priority", pc)?),
        flags: [flag("control_flags", &CONTROL)?, flag("world_flags", &WORLD)?, flag("who_flags", &WHO)?],
        volume: scalar(&node, "volume", pc)?,
        range: pair(&node, "range", pc)?,
        simultaneous_limit: limit,
        fade_time_s: pair(&node, "fade_time_s", pc)?,
        delay_ms: pair(&node, "delay_ms", pc)?,
        pitch_adjust: pair(&node, "pitch_adjust", pc)?,
        volume_adjust: scalar(&node, "volume_adjust", pc)?,
        ldf_length: scalar(&node, "ldf_length", pc).unwrap_or(0),
        ignore_rate,
        lists,
        folder,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flags_follow_the_46c1c0_registries() {
        assert_eq!(flags_46c1c0(&CONTROL, "INTERRUPT, CAUSEFADE"), 0x21);
        assert_eq!(flags_46c1c0(&CONTROL, "WORLD,VOICE"), 0x18);
        assert_eq!(flags_46c1c0(&WHO, "EVERYONE"), 0x20);
        assert_eq!(flags_46c1c0(&WORLD, "FOGGED bogus"), 2);
    }
}
