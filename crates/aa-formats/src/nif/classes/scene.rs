//! NiObject, NiObjectNET, NiAVObject, NiNode and node subclasses, lights
//! and cameras.
use super::super::stream::Stream;
use super::super::{Field, Fields, NifError, Value};

pub(crate) type R = Result<(), NifError>;

/// NiObject::LoadBinary 86A8E0: versions >= 5.0.0.6 read the object's group
/// ID (GroupFromID 880450 indexes the group table without a bounds check).
pub(crate) fn object(s: &mut Stream, f: &mut Fields) -> R {
    if s.at_least(0x0500_0006) {
        let id = s.u32()?;
        if id > s.groups {
            return Err(NifError::OutOfRange("group ID"));
        }
        f.put("group", Value::U32(id));
    }
    Ok(())
}

/// NiObjectNET::LoadBinary 86B090: name (LoadCString 880EE0); below 5.0.0.11
/// one extra-data link, otherwise the extra-data link list; the controller.
pub(crate) fn object_net(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("name", Value::Str(s.cstring()?));
    if s.below(0x0500_000b) {
        f.put("extra_data", Value::Link(s.link_id()?));
    } else {
        f.put("extra_data_list", Value::Links(s.link_ids()?));
    }
    f.put("controller", Value::Link(s.link_id()?));
    Ok(())
}

/// NiAVObject::LoadBinary 86F150.
pub(crate) fn av_object(s: &mut Stream, f: &mut Fields) -> R {
    object_net(s, f)?;
    let read = s.u16()?;
    let mut flags = read;
    if s.below(0x0401_000b) {
        flags = ((flags & 0xfff8) << 1) | (flags & 7);
    }
    if s.below(0x0401_000c) {
        flags = (((flags & 0xfff7) | 7) << 4) | (flags & 0xf);
    }
    if s.below(0x0500_0001) {
        flags = ((flags & 0xff00) << 1) | (flags & 0xff);
    }
    let flags_at = f.0.len();
    f.put("flags", Value::U16(flags));
    f.put("translation", Value::F32s(s.point3()?.to_vec()));
    f.put("rotation", Value::F32s(s.matrix3()?.to_vec()));
    f.put("scale", Value::F32(s.f32()?));
    if s.below(0x0500_0013) {
        // 86F216: the collision type is bits 1..3; the flags are rebuilt from
        // (bit 0 xor bit 3) xor (flags >> 3).
        let low = flags as u8;
        flags = (((low >> 3) ^ low) as u16 & 1) ^ (flags >> 3);
        f.put("velocity", Value::F32s(s.point3()?.to_vec()));
        f.put("properties", Value::Links(s.link_ids()?));
        let has_bound = if s.below(0x0401_0000) { s.u32()? != 0 } else { s.u8()? != 0 };
        f.put("has_bounding_volume", Value::U8(has_bound as u8));
        // 86F299..86F347: an NiCollisionData would be created by name through
        // the loader map (880420) for a bound, a nonzero velocity or a
        // collision type other than 2, and asked to load the old bound. AA.exe
        // registers no "NiCollisionData" loader, so 880420 returns NULL and
        // nothing more is read. (The comparison of the last error text B44068
        // with "NiCollisionSwitch" can only change that collision type.)
    } else {
        f.put("properties", Value::Links(s.link_ids()?));
        f.put("collision", Value::Link(s.link_id()?));
    }
    if s.below(0x0a00_0102) {
        // B43E0C keeps the full flags for subclasses; NiAVObject keeps 6 bits.
        s.saved_av_flags = flags;
        flags &= 0x3f;
    }
    f.0[flags_at] = Field { name: "flags", value: Value::U16(flags), read: (flags != read).then_some(Value::U16(read)) };
    Ok(())
}

/// NiNode::LoadBinary 866130: children and effects link lists.
pub(crate) fn node(s: &mut Stream, f: &mut Fields) -> R {
    av_object(s, f)?;
    f.put("children", Value::Links(s.link_ids()?));
    f.put("effects", Value::Links(s.link_ids()?));
    Ok(())
}

/// NiBillboardNode::LoadBinary 8A3710: below 10.0.1.2 the mode comes from the
/// full NiAVObject flags (B43E0C); otherwise a u16 is read.
pub(crate) fn billboard_node(s: &mut Stream, f: &mut Fields) -> R {
    node(s, f)?;
    if s.below(0x0a00_0102) {
        let mut mode = s.saved_av_flags;
        if s.below(0x0402_0003) {
            mode = ((mode & 0xe000) << 1) | (mode & 0x1fff);
        }
        mode = (mode >> 8) & 0x7ff;
        f.put("billboard_mode", Value::U16(mode));
    } else {
        f.put("billboard_mode", Value::U16(s.u16()?));
    }
    Ok(())
}

/// NiDynamicEffect::LoadBinary 89C8B0: below 4.1.0.0 a signed count of
/// discarded words; from 10.0.1.14 the affected-node link list.
pub(crate) fn dynamic_effect(s: &mut Stream, f: &mut Fields) -> R {
    av_object(s, f)?;
    if s.below(0x0401_0000) {
        let count = s.u32()? as i32;
        let mut words = Vec::new();
        for _ in 0..count.max(0) {
            words.push(s.u32()?);
        }
        f.put("old_affected", Value::U32s(words));
    }
    if s.at_least(0x0a00_010e) {
        f.put("affected_nodes", Value::Links(s.link_ids()?));
    }
    Ok(())
}

/// NiLight::LoadBinary 89B5E0 (NiAmbientLight 90FF80 is this function).
pub(crate) fn light(s: &mut Stream, f: &mut Fields) -> R {
    dynamic_effect(s, f)?;
    f.put("dimmer", Value::F32(s.f32()?));
    f.put("ambient", Value::F32s(s.point3()?.to_vec()));
    f.put("diffuse", Value::F32s(s.point3()?.to_vec()));
    f.put("specular", Value::F32s(s.point3()?.to_vec()));
    Ok(())
}

/// NiPointLight::LoadBinary 89B2A0: constant, linear, quadratic attenuation.
pub(crate) fn point_light(s: &mut Stream, f: &mut Fields) -> R {
    light(s, f)?;
    f.put("constant", Value::F32(s.f32()?));
    f.put("linear", Value::F32(s.f32()?));
    f.put("quadratic", Value::F32(s.f32()?));
    Ok(())
}

/// NiCamera::LoadBinary 8746F0.
pub(crate) fn camera(s: &mut Stream, f: &mut Fields) -> R {
    av_object(s, f)?;
    if s.below(0x0a00_0102) {
        f.put("camera_flags", Value::U16((s.saved_av_flags >> 6) & 0x1ff));
    } else {
        f.put("camera_flags", Value::U16(s.u16()?));
    }
    let mut frustum = Vec::with_capacity(6);
    for _ in 0..6 {
        frustum.push(s.f32()?);
    }
    f.put("frustum", Value::F32s(frustum));
    if s.below(0x0a00_0105) {
        f.put("orthographic", Value::U8(0));
    } else {
        f.put("orthographic", Value::U8((s.u8()? != 0) as u8));
    }
    let mut port = Vec::with_capacity(4);
    for _ in 0..4 {
        port.push(s.f32()?);
    }
    f.put("viewport", Value::F32s(port));
    f.put("lod_adjust", Value::F32(s.f32()?));
    f.put("scene", Value::Link(s.link_id()?));
    f.put("screen_polygons", Value::Links(s.link_ids()?));
    if s.at_least(0x0401_0004) {
        f.put("screen_textures", Value::Links(s.link_ids()?));
        if s.at_least(0x0a00_0107) && s.below(0x0a00_010f) {
            f.put("screen_textures_old", Value::Links(s.link_ids()?));
        }
    }
    Ok(())
}
