//! NiGeometry and its data classes.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::scene::{av_object, object, R};

/// NiGeometry::LoadBinary 8689E0 (NiTriShape, NiTriStrips, NiParticles and
/// NiParticleMeshes use it unchanged: 87FC50 -> 867F40 -> 8689E0).
pub(crate) fn geometry(s: &mut Stream, f: &mut Fields) -> R {
    av_object(s, f)?;
    f.put("data", Value::Link(s.link_id()?));
    f.put("skin_instance", Value::Link(s.link_id()?));
    if s.at_least(0x0500_0015) {
        let has_shader = s.u8()?;
        f.put("has_shader", Value::U8(has_shader));
        if has_shader != 0 {
            // 8A6950 creates the shader reference, 8A6A20 loads it: name
            // (LoadCString) and implementation index.
            let mut shader = Fields::default();
            shader.put("name", Value::Str(s.cstring()?));
            shader.put("implementation", Value::U32(s.u32()?));
            f.put("shader", Value::Record(shader.0));
        }
    }
    Ok(())
}

/// A presence flag: a u32 compared with 0 below 4.1.0.0, otherwise a byte.
fn presence(s: &mut Stream) -> Result<(u32, bool), NifError> {
    if s.below(0x0401_0000) {
        let v = s.u32()?;
        Ok((v, v != 0))
    } else {
        let v = s.u8()?;
        Ok((v as u32, v != 0))
    }
}

/// NiGeometryData::LoadBinary 8AC0B0.
pub(crate) fn geometry_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    let count = s.u16()?;
    f.put("vertex_count", Value::U16(count));
    let n = count as usize;
    if s.at_least(0x0a00_0110) {
        f.put("keep_flags", Value::U8(s.u8()?));
        f.put("compress_flags", Value::U8(s.u8()?));
    }
    let (word, has) = presence(s)?;
    f.put("has_vertices", Value::U32(word));
    if has {
        f.put("vertices", Value::F32s(s.u32s(n * 3)?));
    }
    // +0x30: UV-set count (bits 0..5) and extra-vector flags.
    let mut data_flags: u16 = 0;
    let mut normal_count = n;
    if s.at_least(0x0a00_0002) {
        data_flags = s.u16()?;
        f.put("data_flags", Value::U16(data_flags));
        if (data_flags >> 8) as u8 & 0xf0 != 0 {
            normal_count = n * 3;
        }
    }
    let (word, has) = presence(s)?;
    f.put("has_normals", Value::U32(word));
    if has {
        f.put("normals", Value::F32s(s.u32s(normal_count * 3)?));
    }
    // NiBound::LoadBinary 8C5470: centre (three reads) and radius.
    f.put("center", Value::F32s(s.point3()?.to_vec()));
    f.put("radius", Value::F32(s.f32()?));
    let (word, has) = presence(s)?;
    f.put("has_colors", Value::U32(word));
    if has {
        f.put("colors", Value::F32s(s.u32s(n * 4)?));
    }
    if s.below(0x0500_000a) {
        let sets = s.u16()?;
        data_flags ^= (data_flags as u8 ^ sets as u8) as u16 & 0x3f;
        f.put("data_flags", Value::U16(data_flags));
        f.put("uv_set_word", Value::U16(sets));
    } else if s.below(0x0a00_0002) {
        data_flags = s.u16()?;
        f.put("data_flags", Value::U16(data_flags));
    }
    if s.below(0x0401_0000) {
        f.put("has_uv", Value::U32(s.u32()?));
    }
    let sets = (data_flags & 0x3f) as usize;
    if sets > 0 {
        f.put("uv_sets", Value::F32s(s.u32s(sets * n * 2)?));
    }
    if s.at_least(0x0500_000a) {
        f.put("consistency_flags", Value::U16(s.u16()?));
    }
    Ok(())
}

/// NiTriBasedGeomData::LoadBinary 87FA50: geometry data and triangle count.
pub(crate) fn tri_based_geom_data(s: &mut Stream, f: &mut Fields) -> R {
    geometry_data(s, f)?;
    f.put("triangle_count", Value::U16(s.u16()?));
    Ok(())
}

/// NiTriShapeData::LoadBinary 8D0800.
pub(crate) fn tri_shape_data(s: &mut Stream, f: &mut Fields) -> R {
    tri_based_geom_data(s, f)?;
    let length = s.u32()?;
    f.put("triangle_points", Value::U32(length));
    let has = if s.at_least(0x0a00_0111) {
        let v = s.u8()?;
        f.put("has_triangles", Value::U8(v));
        v != 0
    } else {
        true
    };
    if has && length > 0 {
        f.put("triangles", Value::U16s(s.u16s(length as usize)?));
    }
    let groups = s.u16()?;
    f.put("match_group_count", Value::U16(groups));
    let mut list = Vec::new();
    for _ in 0..groups {
        let n = s.u16()?;
        list.push(if n != 0 { Value::U16s(s.u16s(n as usize)?) } else { Value::U16s(Vec::new()) });
    }
    if groups != 0 {
        f.put("match_groups", Value::List(list));
    }
    Ok(())
}

/// NiTriStripsData::LoadBinary 8D1160. The point count is the native
/// 16-bit `strips * 2 + triangles`, not the sum of the strip lengths.
pub(crate) fn tri_strips_data(s: &mut Stream, f: &mut Fields) -> R {
    tri_based_geom_data(s, f)?;
    let strips = s.u16()?;
    f.put("strip_count", Value::U16(strips));
    if strips == 0 {
        return Ok(());
    }
    f.put("strip_lengths", Value::U16s(s.u16s(strips as usize)?));
    let has = if s.at_least(0x0a00_0111) {
        let v = s.u8()?;
        f.put("has_points", Value::U8(v));
        v != 0
    } else {
        true
    };
    if has {
        let triangles = f.u16("triangle_count").ok_or(NifError::Unsupported("8D11F6 triangle count"))?;
        let points = strips.wrapping_shl(1).wrapping_add(triangles);
        f.put("points", Value::U16s(s.u16s(points as usize)?));
    }
    Ok(())
}
