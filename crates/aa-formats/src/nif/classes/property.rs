//! NiProperty subclasses, including NiTexturingProperty and its maps.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::scene::{object_net, R};

/// NiProperty::LoadBinary 86A550: below 10.0.1.2 the flags word is read into
/// the process global B43DC8, which each subclass masks.
pub(crate) fn property(s: &mut Stream, f: &mut Fields) -> R {
    object_net(s, f)?;
    if s.below(0x0a00_0102) {
        let v = s.u16()?;
        s.saved_property_flags = v;
        f.put("property_flags_word", Value::U16(v));
    }
    Ok(())
}

/// The subclass flags: read for 10.0.1.2 and later, otherwise B43DC8 masked.
fn flags(s: &mut Stream, f: &mut Fields, mask: u16) -> R {
    if s.below(0x0a00_0102) {
        let word = s.saved_property_flags;
        f.put_converted("flags", Value::U16(word & mask), Value::U16(word));
    } else {
        f.put("flags", Value::U16(s.u16()?));
    }
    Ok(())
}

/// NiAlphaProperty::LoadBinary 87CFD0 (old flags keep bits 0..13).
pub(crate) fn alpha(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    flags(s, f, 0x3fff)?;
    f.put("threshold", Value::U8(s.u8()?));
    Ok(())
}

/// NiVertexColorProperty::LoadBinary 87B710 (old flags keep bits 0..2).
pub(crate) fn vertex_color(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    flags(s, f, 7)?;
    f.put("vertex_mode", Value::U32(s.u32()?));
    f.put("lighting_mode", Value::U32(s.u32()?));
    Ok(())
}

/// NiZBufferProperty::LoadBinary 88A320 (old flags keep bits 0..1); the test
/// function is read from 4.1.0.5.
pub(crate) fn zbuffer(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    flags(s, f, 3)?;
    if s.at_least(0x0401_0005) {
        f.put("function", Value::U32(s.u32()?));
    }
    Ok(())
}

/// NiDitherProperty / NiSpecularProperty::LoadBinary 88A0C0 (old flags keep bit 0).
pub(crate) fn single_flag(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    flags(s, f, 1)
}

/// NiStencilProperty::LoadBinary 8893E0: no version branch.
pub(crate) fn stencil(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    f.put("enabled", Value::U8(s.u8()?));
    for name in ["function", "reference", "mask", "fail_action", "z_fail_action", "pass_action", "draw_mode"] {
        f.put(name, Value::U32(s.u32()?));
    }
    Ok(())
}

/// NiMaterialProperty::LoadBinary 87D3C0: ambient, diffuse, specular,
/// emissive (NiColor, three reads each), glossiness, alpha.
pub(crate) fn material(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    for name in ["ambient", "diffuse", "specular", "emissive"] {
        f.put(name, Value::F32s(s.point3()?.to_vec()));
    }
    f.put("glossiness", Value::F32(s.f32()?));
    f.put("alpha", Value::F32(s.f32()?));
    Ok(())
}

/// NiTexturingProperty::Map::LoadBinary 86D720.
fn map(s: &mut Stream, m: &mut Fields) -> R {
    m.put("source", Value::Link(s.link_id()?));
    m.put("clamp", Value::U32(s.u32()?));
    m.put("filter", Value::U32(s.u32()?));
    m.put("uv_set", Value::U32(s.u32()?));
    m.put("ps2_l", Value::U16(s.u16()?));
    m.put("ps2_k", Value::U16(s.u16()?));
    if s.below(0x0401_0010) {
        m.put("old_word", Value::U16(s.u16()?));
    }
    if s.at_least(0x0a00_010a) {
        let has = s.u8()?;
        m.put("has_transform", Value::U8(has));
        if has == 1 {
            // NiTextureTransform::LoadBinary 8A6830: NiPoint2 (869CD0) at +0
            // and +0xC, two words at +8 and +0x44, NiPoint2 at +0x14.
            let mut t = Vec::with_capacity(8);
            t.push(s.f32()?);
            t.push(s.f32()?);
            t.push(s.f32()?);
            t.push(s.f32()?);
            t.push(s.f32()?);
            t.push(s.u32()?);
            t.push(s.f32()?);
            t.push(s.f32()?);
            m.put("transform", Value::F32s(t));
        }
    }
    Ok(())
}

/// NiTexturingProperty::BumpMap::LoadBinary 86D8B0: Map, luma scale/offset
/// and the 2x2 bump matrix.
fn bump_map(s: &mut Stream, m: &mut Fields) -> R {
    map(s, m)?;
    let mut v = Vec::with_capacity(6);
    for _ in 0..6 {
        v.push(s.f32()?);
    }
    m.put("bump", Value::F32s(v));
    Ok(())
}

/// NiTexturingProperty::ShaderMap::LoadBinary 86D9B0: Map and map ID.
fn shader_map(s: &mut Stream, m: &mut Fields) -> R {
    map(s, m)?;
    m.put("map_id", Value::U32(s.u32()?));
    Ok(())
}

/// Map presence: a u32 normalised with SETNE below 4.1.0.0, otherwise the
/// raw byte, which is also the pushed link count (86CFFC movzx).
fn has_map(s: &mut Stream) -> Result<u8, NifError> {
    if s.below(0x0401_0000) { Ok((s.u32()? != 0) as u8) } else { s.u8() }
}

/// NiTexturingProperty::LoadBinary 86CF20. `maps` mirrors the native map
/// array: every slot index the loop visits is SetAt (an empty `List` for an
/// absent map), so its length is the native array size. Each slot pushes its
/// presence as a link count; below 3.3.0.13 slots from 5 on move up by one
/// after an extra pushed count of 0.
pub(crate) fn texturing(s: &mut Stream, f: &mut Fields) -> R {
    property(s, f)?;
    f.put("apply_mode", Value::U32(s.u32()?));
    let count = s.u32()?;
    f.put("map_count", Value::U32(count));
    s.push_count(if s.below(0x0303_000d) { count.wrapping_add(1) } else { count });
    let mut slots: Vec<Value> = Vec::new();
    let set = |slots: &mut Vec<Value>, index: usize, value: Value| {
        if slots.len() <= index {
            slots.resize(index + 1, Value::List(Vec::new()));
        }
        slots[index] = value;
    };
    for i in 0..count as usize {
        let has = has_map(s)?;
        let mut index = i;
        if s.below(0x0303_000d) && i >= 5 {
            s.push_count(0);
            index = i + 1;
        }
        s.push_count(has as u32);
        let value = if has == 0 {
            Value::List(Vec::new())
        } else {
            let mut m = Fields::default();
            if !s.below(0x0303_000d) && i == 5 {
                bump_map(s, &mut m)?;
            } else {
                map(s, &mut m)?;
            }
            Value::Record(m.0)
        };
        set(&mut slots, index, value);
    }
    f.put("maps", Value::List(slots));
    if s.at_least(0x0500_0011) {
        let n = s.u32()?;
        f.put("shader_map_count", Value::U32(n));
        s.push_count(n);
        let mut shader = Vec::new();
        for _ in 0..n {
            let has = s.u8()?;
            s.push_count(has as u32);
            if has != 0 {
                let mut m = Fields::default();
                shader_map(s, &mut m)?;
                shader.push(Value::Record(m.0));
            } else {
                shader.push(Value::List(Vec::new()));
            }
        }
        f.put("shader_maps", Value::List(shader));
    }
    Ok(())
}
