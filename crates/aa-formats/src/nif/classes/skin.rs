//! NiSkinInstance, NiSkinData, NiSkinPartition.
use super::super::stream::Stream;
use super::super::{Fields, Value};
use super::scene::{object, R};

/// NiSkinInstance::LoadBinary 8C3810: data, root parent, bone links.
pub(crate) fn skin_instance(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("data", Value::Link(s.link_id()?));
    f.put("root_parent", Value::Link(s.link_id()?));
    f.put("bones", Value::Links(s.link_ids()?));
    Ok(())
}

/// NiTransform::LoadBinary 8752E0: rotation (9 reads), translation (3), scale.
fn transform(s: &mut Stream) -> Result<Vec<u32>, super::super::NifError> {
    let mut v = s.matrix3()?.to_vec();
    v.extend(s.point3()?);
    v.push(s.f32()?);
    Ok(v)
}

/// NiSkinData::LoadBinary 9096F0.
pub(crate) fn skin_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("root_transform", Value::F32s(transform(s)?));
    let bones = s.u32()?;
    f.put("bone_count", Value::U32(bones));
    if s.at_least(0x0400_0001) {
        f.put("partition", Value::Link(s.link_id()?));
    }
    let has_weights = if s.at_least(0x0401_000d) {
        let v = s.u8()?;
        f.put("has_vertex_weights", Value::U8(v));
        v != 0
    } else {
        true
    };
    let mut list = Vec::with_capacity(bones as usize);
    for _ in 0..bones {
        let mut b = Fields::default();
        b.put("transform", Value::F32s(transform(s)?));
        b.put("center", Value::F32s(s.point3()?.to_vec()));
        b.put("radius", Value::F32(s.f32()?));
        let n = s.u16()?;
        b.put("vertex_count", Value::U16(n));
        if has_weights {
            let mut index = Vec::with_capacity(n as usize);
            let mut weight = Vec::with_capacity(n as usize);
            for _ in 0..n {
                index.push(s.u16()?);
                weight.push(s.f32()?);
            }
            b.put("vertex_indices", Value::U16s(index));
            b.put("vertex_weights", Value::F32s(weight));
        }
        list.push(Value::Record(b.0));
    }
    f.put("bone_list", Value::List(list));
    if s.version == 0x0401_0009 {
        f.put("old_byte", Value::U8(s.u8()?));
    }
    Ok(())
}

/// NiSkinPartition::Partition::LoadBinary 87E650.
fn partition(s: &mut Stream) -> Result<Value, super::super::NifError> {
    let mut p = Fields::default();
    let vertices = s.u16()?;
    let triangles = s.u16()?;
    let bones = s.u16()?;
    let strips = s.u16()?;
    let per_vertex = s.u16()?;
    p.put("vertex_count", Value::U16(vertices));
    p.put("triangle_count", Value::U16(triangles));
    p.put("bone_count", Value::U16(bones));
    p.put("strip_count", Value::U16(strips));
    p.put("bones_per_vertex", Value::U16(per_vertex));
    p.put("bones", Value::U16s(s.u16s(bones as usize)?));
    let recent = s.at_least(0x0a00_0112);
    let flag = |s: &mut Stream, p: &mut Fields, name: &'static str| -> Result<bool, super::super::NifError> {
        if recent {
            let v = s.u8()?;
            p.put(name, Value::U8(v));
            Ok(v != 0)
        } else {
            Ok(true)
        }
    };
    if flag(s, &mut p, "has_vertex_map")? {
        p.put("vertex_map", Value::U16s(s.u16s(vertices as usize)?));
    }
    let weights = vertices as usize * per_vertex as usize;
    if flag(s, &mut p, "has_weights")? {
        p.put("weights", Value::F32s(s.u32s(weights)?));
    }
    let points = if strips == 0 {
        triangles as usize * 3
    } else {
        let lengths = s.u16s(strips as usize)?;
        let sum = lengths.iter().map(|&l| l as usize).sum();
        p.put("strip_lengths", Value::U16s(lengths));
        sum
    };
    if flag(s, &mut p, "has_faces")? {
        p.put("faces", Value::U16s(s.u16s(points)?));
    }
    // 87E80E: the bone-index flag byte is read for every version.
    let has = s.u8()?;
    p.put("has_bone_indices", Value::U8(has));
    if has != 0 {
        p.put("bone_indices", Value::Bytes(s.bytes(weights)?));
    }
    Ok(Value::Record(p.0))
}

/// NiSkinPartition::LoadBinary 87ECC0.
pub(crate) fn skin_partition(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    let n = s.u32()?;
    f.put("partition_count", Value::U32(n));
    let mut list = Vec::with_capacity(n as usize);
    for _ in 0..n {
        list.push(partition(s)?);
    }
    f.put("partitions", Value::List(list));
    Ok(())
}
