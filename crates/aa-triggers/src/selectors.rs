//! Native trigger register selectors (546AF0 argument resolution).
//!
//! 5422F0/54A5A0 copy every parameter into a reusable 0x24-byte register:
//! +0 word, +4/+8/+C strings, +10 position, +1C the element KIND (definition
//! +24: 0 event, 1 condition, 2 action) and +20 the item index. The selectors
//! below read those registers exactly as the original:
//!
//! * 54DAC0 Kingdom: named(18)/raw ID(19)/manager+24(23)/manager+28(24); a
//!   nonnull result is recorded in manager+28 for an event (54BA20) and in
//!   manager+24 for a condition (51C220). Actions record nothing.
//! * 54DC20 actor: raw ID(12)/manager+10(13)/current subject(14)/named(15); a
//!   nonnull result of a CONDITION writes its GTID (+14) to manager+10 (5906A0).
//! * Raw-ID paths (registry generation check) have no class filter.
//!
//! Registers are reused between elements in native code; a selector reading a
//! storage the parameter does not own (stale register contents) is rejected
//! here as an explicit error instead of reproducing a stale value.
use crate::{
    decode::{Element, Parameter, Value},
    manager::{Error, Manager, Result},
    program::{text, word},
};
use aa_navigation::startup::Object;

/// Element kind from the original schema initialization (definition+24).
pub fn kind(element: &Element) -> Result<u32> {
    crate::registrations::schema(element.opcode)
        .map(|(kind, _)| kind)
        .ok_or_else(|| Error::State(format!("unknown native element opcode {}", element.opcode)))
}

/// Registry, identity and name-table services shared by every selector.
pub trait Registry {
    /// 0xB364C0 generation/slot lookup: `(gtid >> 16) == generation[slot]`.
    fn resolve_id(&self, raw: u32) -> Result<Option<Object>>;
    /// Object+14 GTID of a resolved object.
    fn raw_id(&self, object: Object) -> Result<u32>;
    /// 4F8E90 shared name table, live ID lookup and actor class check.
    fn named_actor(&mut self, name: &[u16]) -> Result<Option<Object>>;
    /// 4F8F00 shared name table, live ID lookup and Kingdom class check.
    fn named_kingdom(&mut self, name: &[u16]) -> Result<Option<Object>>;
}

/// 54DAC0.
pub fn kingdom<R: Registry + ?Sized>(
    manager: &mut Manager,
    element_kind: u32,
    p: &Parameter,
    r: &mut R,
) -> Result<Option<Object>> {
    let result = match p.item_index {
        18 => r.named_kingdom(text(p)?)?,
        19 => r.resolve_id(word(p)?)?,
        23 => manager.condition_kingdom().map(Object),
        24 => manager.event_kingdom().map(Object),
        _ => return Ok(None),
    };
    if let Some(object) = result {
        match element_kind {
            0 => manager.set_event_kingdom(Some(object.0)),
            1 => manager.set_condition_kingdom(Some(object.0)),
            _ => {}
        }
    }
    Ok(result)
}

/// 54DC20.
pub fn actor<R: Registry + ?Sized>(
    manager: &mut Manager,
    element_kind: u32,
    p: &Parameter,
    r: &mut R,
) -> Result<Option<Object>> {
    let result = match p.item_index {
        12 => r.resolve_id(word(p)?)?,
        13 => r.resolve_id(manager.context_word())?,
        // 54C3D0: a null cursor reads GTID 0 through a local.
        14 => r.resolve_id(manager.current_subject().unwrap_or(0))?,
        15 => r.named_actor(text(p)?)?,
        _ => return Ok(None),
    };
    if let Some(object) = result
        && element_kind == 1
    {
        manager.set_context_word(r.raw_id(object)?);
    }
    Ok(result)
}

/// 54DA20: only mi_boolean(32) yields its word != 0.
pub fn boolean(p: &Parameter) -> Result<bool> {
    Ok(p.item_index == 32 && word(p)? != 0)
}

/// 54DA40: only mi_compare(6) supplies the comparison kind; otherwise 2 (==).
pub fn comparison(p: &Parameter) -> Result<u32> {
    if p.item_index == 6 { word(p) } else { Ok(2) }
}

/// 54DE00: copy of register+4 (string storage).
pub fn string(p: &Parameter) -> Result<Vec<u16>> {
    match &p.value {
        Value::Text(t) => Ok(t.clone()),
        _ => Err(Error::State(
            "string selector on a non-string register would read stale native contents".into(),
        )),
    }
}

/// 54DA60: mi_location(38) register+10 position, otherwise null.
pub fn location(p: &Parameter) -> Result<Option<[u32; 3]>> {
    if p.item_index != 38 {
        return Ok(None);
    }
    match p.value {
        Value::Position(v) => Ok(Some(v)),
        _ => Err(Error::State("location register without position storage".into())),
    }
}

/// MSVC `_ftol2` on a binary32 value: truncation toward zero; NaN, infinity and
/// values outside signed 32-bit produce the x87 integer indefinite 0x80000000.
pub fn ftol(bits: u32) -> i32 {
    let exponent = ((bits >> 23) & 0xff) as i32;
    if exponent == 0xff {
        return i32::MIN;
    }
    if exponent < 127 {
        return 0;
    }
    let shift = exponent - 150;
    let mantissa = u64::from((bits & 0x7f_ffff) | 0x80_0000);
    let magnitude = if shift >= 0 {
        if shift > 40 {
            return i32::MIN;
        }
        mantissa << shift
    } else {
        mantissa >> (-shift)
    };
    let negative = bits >> 31 != 0;
    if negative {
        if magnitude > 0x8000_0000 {
            i32::MIN
        } else {
            (0u64.wrapping_sub(magnitude)) as u32 as i32
        }
    } else if magnitude > 0x7fff_ffff {
        i32::MIN
    } else {
        magnitude as i32
    }
}

/// 54DB80: actor GTID register without context writes. Raw IDs (12) are
/// returned unvalidated; other items resolve and read +14, or yield 0.
pub fn actor_id<R: Registry + ?Sized>(manager: &Manager, p: &Parameter, r: &mut R) -> Result<u32> {
    let object = match p.item_index {
        12 => return word(p),
        13 => r.resolve_id(manager.context_word())?,
        14 => r.resolve_id(manager.current_subject().unwrap_or(0))?,
        15 => r.named_actor(text(p)?)?,
        _ => return Ok(0),
    };
    object.map(|o| r.raw_id(o)).transpose().map(|id| id.unwrap_or(0))
}

/// Typed-object checks performed by the original register conversions.
pub trait Checks: Registry {
    /// Virtual +BC (Area) on a resolved object.
    fn is_area(&mut self, object: Object) -> Result<bool>;
    /// Virtual +C0 (camera) on a resolved object.
    fn is_camera(&mut self, object: Object) -> Result<bool>;
}

/// 54DA80: raw registry lookup of the register word (no item check) followed by
/// virtual +BC.
pub fn area<R: Checks + ?Sized>(p: &Parameter, r: &mut R) -> Result<Option<Object>> {
    let raw = match p.value {
        Value::Word(w) => w,
        _ => return Err(Error::State("area register without word storage".into())),
    };
    match r.resolve_id(raw)? {
        Some(o) if r.is_area(o)? => Ok(Some(o)),
        _ => Ok(None),
    }
}
