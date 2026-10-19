//! NiExtraData and its subclasses.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::scene::{object, R};

/// NiExtraData::LoadBinary 87B430. Below 5.0.0.11: the next-extra-data link
/// and a size word; only an object whose RTTI is exactly NiExtraData (B43F0C)
/// then reads a blob of that size. From 5.0.0.11: the name.
fn extra_data(s: &mut Stream, f: &mut Fields, exact_base: bool) -> R {
    object(s, f)?;
    if s.below(0x0500_000b) {
        f.put("next", Value::Link(s.link_id()?));
        let size = s.u32()?;
        f.put("size", Value::U32(size));
        if size > 0 && exact_base {
            // 87B483..87B4C7 reads the blob, wraps it in a new object
            // (877F90) and replaces this object in the stream (vtable +0x24).
            return Err(NifError::Unsupported("87B483 old NiExtraData blob replacement"));
        }
    } else {
        f.put("extra_name", Value::Str(s.cstring()?));
    }
    Ok(())
}

pub(crate) fn extra(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, true)
}

/// NiStringExtraData::LoadBinary 8C5590.
pub(crate) fn string(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    f.put("string", Value::Str(s.cstring()?));
    Ok(())
}

/// NiIntegerExtraData::LoadBinary 87B110.
pub(crate) fn integer(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    f.put("integer", Value::U32(s.u32()?));
    Ok(())
}

/// NiBooleanExtraData::LoadBinary 90F980: the byte is stored as `!= 0`.
pub(crate) fn boolean(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    let read = s.u8()?;
    let value = (read != 0) as u8;
    if value == read {
        f.put("boolean", Value::U8(value));
    } else {
        f.put_converted("boolean", Value::U8(value), Value::U8(read));
    }
    Ok(())
}

/// NiBinaryExtraData::LoadBinary 8C5920.
pub(crate) fn binary(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    let size = s.u32()?;
    f.put("binary_size", Value::U32(size));
    if size > 0 {
        f.put("binary", Value::Bytes(s.bytes(size as usize)?));
    }
    Ok(())
}

/// NiStringsExtraData::LoadBinary 887710.
pub(crate) fn strings(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    let count = s.u32()?;
    f.put("string_count", Value::U32(count));
    let mut list = Vec::new();
    for _ in 0..count {
        list.push(Value::Str(s.cstring()?));
    }
    f.put("strings", Value::List(list));
    Ok(())
}

/// NiTextKeyExtraData::LoadBinary 8BAAA0: per key 909150 (time, LoadCString).
pub(crate) fn text_keys(s: &mut Stream, f: &mut Fields) -> R {
    extra_data(s, f, false)?;
    let count = s.u32()?;
    f.put("key_count", Value::U32(count));
    let mut keys = Vec::new();
    for _ in 0..count {
        let mut k = Fields::default();
        k.put("time", Value::F32(s.f32()?));
        k.put("text", Value::Str(s.cstring()?));
        keys.push(Value::Record(k.0));
    }
    f.put("keys", Value::List(keys));
    Ok(())
}
