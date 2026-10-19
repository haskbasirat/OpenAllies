//! NiTimeController subclasses, animation keys and key data.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::scene::{object, R};

/// NiTimeController::LoadBinary 89F070.
pub(crate) fn time_controller(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("next", Value::Link(s.link_id()?));
    let read = s.u16()?;
    let flags_at = f.0.len();
    f.put("flags", Value::U16(read));
    f.put("frequency", Value::F32(s.f32()?));
    f.put("phase", Value::F32(s.f32()?));
    f.put("start", Value::F32(s.f32()?));
    f.put("stop", Value::F32(s.f32()?));
    f.put("target", Value::Link(s.link_id()?));
    let mut flags = read;
    if s.version == 0x0a00_0101 {
        let b = s.u8()?;
        f.put("flag_byte", Value::U8(b));
        if b != 0 {
            flags |= 0x10;
        } else {
            flags &= !0x10;
        }
    } else if s.below(0x0a00_0101) {
        flags = ((flags & 0xfff0) << 1) | (flags & 0xf);
    }
    if s.below(0x0a00_0102) {
        s.saved_controller_flags = flags;
        flags &= 0x1f;
    }
    if s.below(0x0a00_0103) {
        flags |= 0x20;
    }
    if flags != read {
        f.0[flags_at].value = Value::U16(flags);
        f.0[flags_at].read = Some(Value::U16(read));
    }
    Ok(())
}

/// The NiKeyframeController shape 8BFD00 (NiVisController, NiAlphaController
/// 8C2690): the controller and its data link.
pub(crate) fn data_controller(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    f.put("data", Value::Link(s.link_id()?));
    Ok(())
}

/// The flags word of the colour/look-at/path controllers: below 10.0.1.2 it
/// is the full time-controller flags (B44624) shifted down by 5 and masked.
fn sub_flags(s: &mut Stream, f: &mut Fields, mask: u16) -> R {
    if s.below(0x0a00_0102) {
        let word = s.saved_controller_flags;
        f.put_converted("controller_flags", Value::U16((word >> 5) & mask), Value::U16(word));
    } else {
        f.put("controller_flags", Value::U16(s.u16()?));
    }
    Ok(())
}

/// NiLookAtController / NiMaterialColorController::LoadBinary 8BDB40.
pub(crate) fn look_at_or_material_color(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    sub_flags(s, f, 0xff)?;
    f.put("link", Value::Link(s.link_id()?));
    Ok(())
}

/// NiLightColorController::LoadBinary 8BE200.
pub(crate) fn light_color(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    sub_flags(s, f, 0x3f)?;
    f.put("data", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPathController::LoadBinary 8BBC00.
pub(crate) fn path(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    sub_flags(s, f, 0xffff)?;
    f.put("bank_direction", Value::U32(s.u32()?));
    f.put("max_bank_angle", Value::F32(s.f32()?));
    f.put("smoothing", Value::F32(s.f32()?));
    f.put("follow_axis", Value::U16(s.u16()?));
    f.put("path_data", Value::Link(s.link_id()?));
    f.put("percent_data", Value::Link(s.link_id()?));
    Ok(())
}

/// NiFlipController::LoadBinary 87C980.
pub(crate) fn flip(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    f.put("texture_slot", Value::U32(s.u32()?));
    f.put("start_time", Value::F32(s.f32()?));
    f.put("delta", Value::F32(s.f32()?));
    f.put("sources", Value::Links(s.link_ids()?));
    Ok(())
}

/// NiTextureTransformController::LoadBinary 8B8980.
pub(crate) fn texture_transform(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    let read = s.u8()?;
    f.put_static("shader_map", read);
    f.put("texture_slot", Value::U32(s.u32()?));
    f.put("operation", Value::U32(s.u32()?));
    f.put("data", Value::Link(s.link_id()?));
    Ok(())
}

// ---------------------------------------------------------------------------
// Animation keys: the function tables at B45DC0 (load), filled by the C++
// static initializers 860AA0..865C10, indexed by class base + key type.

#[derive(Clone, Copy)]
enum KeyLoad {
    /// 4C9FA0: returns NULL, reads nothing.
    Null,
    /// A zero table entry (a call to address 0).
    Missing,
    FloatLinear,
    FloatQuadratic,
    FloatTbc,
    PosLinear,
    PosQuadratic,
    PosTbc,
    RotLinear,
    RotQuadratic,
    RotTbc,
    RotEuler,
    ColorLinear,
}

/// B45DC0[0..18] after start-up.
const KEY_LOADERS: [KeyLoad; 18] = [
    KeyLoad::Null,           // 4C9FA0
    KeyLoad::FloatLinear,    // 863A50
    KeyLoad::FloatQuadratic, // 860E30
    KeyLoad::FloatTbc,       // 865B10
    KeyLoad::Missing,
    KeyLoad::Null,           // 4C9FA0
    KeyLoad::PosLinear,      // 863DF0
    KeyLoad::PosQuadratic,   // 861BC0
    KeyLoad::PosTbc,         // 864E70
    KeyLoad::Missing,
    KeyLoad::Null,           // 4C9FA0
    KeyLoad::RotLinear,      // 864160
    KeyLoad::RotQuadratic,   // 862AB0
    KeyLoad::RotTbc,         // 8655B0
    KeyLoad::RotEuler,       // 8634C0
    KeyLoad::Null,           // 4C9FA0
    KeyLoad::ColorLinear,    // 863740
    KeyLoad::Missing,
];
pub(crate) const FLOAT_KEYS: u32 = 0;
pub(crate) const POS_KEYS: u32 = 5;
pub(crate) const ROT_KEYS: u32 = 10;
pub(crate) const COLOR_KEYS: u32 = 15;

fn words(s: &mut Stream, n: usize) -> Result<Vec<u32>, NifError> {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(s.f32()?);
    }
    Ok(v)
}

/// `B45DC0[base + type](stream, count)`. Each key is a record of the words
/// read for it, in read order (NiAnimationKey 8B7C80 reads the time first).
pub(crate) fn keys(s: &mut Stream, base: u32, key_type: u32, count: u32) -> Result<Value, NifError> {
    let index = base.checked_add(key_type).ok_or(NifError::OutOfRange("key type"))? as usize;
    let load = *KEY_LOADERS.get(index).ok_or(NifError::OutOfRange("key type"))?;
    // Per key: (words) for the float/pos/colour/linear rotation kinds.
    let per_key = match load {
        KeyLoad::Null => return Ok(Value::List(Vec::new())),
        KeyLoad::Missing => return Err(NifError::OutOfRange("key type with a NULL loader")),
        KeyLoad::FloatLinear => 2,     // 860B20: time, value
        KeyLoad::FloatQuadratic => 4,  // + forward, backward
        KeyLoad::FloatTbc => 5,        // + tension, continuity, bias
        KeyLoad::PosLinear => 4,       // 860FA0: time, NiPoint3
        KeyLoad::PosQuadratic => 10,   // + two NiPoint3 tangents
        KeyLoad::PosTbc => 7,          // + t, c, b
        KeyLoad::RotLinear | KeyLoad::RotQuadratic => 5, // 8620E0: time, NiQuaternion (871530)
        KeyLoad::RotTbc => 8,          // + t, c, b
        KeyLoad::ColorLinear => 5,     // 8620E0: time, four words
        KeyLoad::RotEuler => 0,
    };
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if let KeyLoad::RotEuler = load {
            // 8633D0: an order word, then per axis a key count and, when
            // positive, a float key type and the float keys (B45DC0[type]).
            let mut k = Fields::default();
            k.put("order", Value::U32(s.u32()?));
            let mut axes = Vec::with_capacity(3);
            for _ in 0..3 {
                let mut a = Fields::default();
                let n = s.u32()?;
                a.put("count", Value::U32(n));
                if n > 0 {
                    let t = s.u32()?;
                    a.put("type", Value::U32(t));
                    a.put("keys", keys(s, FLOAT_KEYS, t, n)?);
                }
                axes.push(Value::Record(a.0));
            }
            k.put("axes", Value::List(axes));
            out.push(Value::Record(k.0));
        } else {
            out.push(Value::F32s(words(s, per_key)?));
        }
    }
    Ok(Value::List(out))
}

/// The shape shared by the key data loaders: count, then (when positive)
/// the key type and the keys.
fn key_group(s: &mut Stream, f: &mut Fields, prefix: &'static [&'static str; 3], base: u32) -> R {
    let n = s.u32()?;
    f.put(prefix[0], Value::U32(n));
    if n > 0 {
        let t = s.u32()?;
        f.put(prefix[1], Value::U32(t));
        f.put(prefix[2], keys(s, base, t, n)?);
    }
    Ok(())
}

/// NiKeyframeData::LoadBinary 8BF9D0: rotation, translation, scale keys.
pub(crate) fn keyframe_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    key_group(s, f, &["rotation_count", "rotation_type", "rotations"], ROT_KEYS)?;
    key_group(s, f, &["translation_count", "translation_type", "translations"], POS_KEYS)?;
    key_group(s, f, &["scale_count", "scale_type", "scales"], FLOAT_KEYS)
}

/// NiFloatData::LoadBinary 8C0E30.
pub(crate) fn float_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    key_group(s, f, &["key_count", "key_type", "keys"], FLOAT_KEYS)
}

/// NiPosData::LoadBinary 8BB180.
pub(crate) fn pos_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    key_group(s, f, &["key_count", "key_type", "keys"], POS_KEYS)
}

/// NiColorData::LoadBinary 8C1130.
pub(crate) fn color_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    key_group(s, f, &["key_count", "key_type", "keys"], COLOR_KEYS)
}

/// NiBoolKey (8EA520): time and a byte stored as `!= 0`.
pub(crate) fn bool_keys(s: &mut Stream, n: u32) -> Result<Value, NifError> {
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let mut k = Fields::default();
        k.put("time", Value::F32(s.f32()?));
        let read = s.u8()?;
        k.put_static("value", read);
        out.push(Value::Record(k.0));
    }
    Ok(Value::List(out))
}

/// NiVisData::LoadBinary 8B9A30.
pub(crate) fn vis_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    let n = s.u32()?;
    f.put("key_count", Value::U32(n));
    if n > 0 {
        f.put("keys", bool_keys(s, n)?);
    }
    Ok(())
}

/// NiControllerSequence::LoadBinary 883650 (no version branch): NiObject,
/// the name (+0x0C), the text-key name (+0xA8), the text-key link, the
/// block count (SetSize of the +0x14 name array) and per block the target
/// name (stored in the +0x14 array) and the controller link. LinkObject
/// 883730 pops the text keys (+0x10), then SetAt(i) each controller in the
/// +0x2C array sized by the block count.
pub(crate) fn controller_sequence(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("name", Value::Str(s.cstring()?));
    f.put("text_keys_name", Value::Str(s.cstring()?));
    f.put("text_keys", Value::Link(s.link_id()?));
    let count = s.u32()?;
    f.put("block_count", Value::U32(count));
    let mut blocks = Vec::new();
    for _ in 0..count {
        let mut b = Fields::default();
        b.put("target_name", Value::Str(s.cstring()?));
        b.put("controller", Value::Link(s.link_id()?));
        blocks.push(Value::Record(b.0));
    }
    f.put("blocks", Value::List(blocks));
    Ok(())
}
