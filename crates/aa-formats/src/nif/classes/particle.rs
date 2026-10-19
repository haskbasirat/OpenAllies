//! Old-style particles (NiParticles data, NiParticleSystemController and its
//! modifiers) and the NiPSys particle system classes.
use super::super::stream::Stream;
use super::super::{Fields, NifError, Value};
use super::controller::{bool_keys, keys, time_controller, FLOAT_KEYS};
use super::geometry::{geometry, geometry_data};
use super::scene::{object, R};

/// A presence flag: a u32 compared with 0 below `version`, otherwise a byte.
fn presence(s: &mut Stream, version: u32) -> Result<bool, NifError> {
    if s.below(version) { Ok(s.u32()? != 0) } else { Ok(s.u8()? != 0) }
}

/// NiParticlesData::LoadBinary 8C3370. `rotating` is the byte +0x3C that the
/// NiRotatingParticlesData create function 8C31C0 sets (8C3170 clears it).
fn particles_data_with(s: &mut Stream, f: &mut Fields, rotating: bool) -> R {
    geometry_data(s, f)?;
    let n = f.u16("vertex_count").ok_or(NifError::Unsupported("8C339F vertex count"))? as usize;
    if s.below(0x0401_0007) {
        f.put("old_active_word", Value::U16(s.u16()?));
    }
    if s.below(0x0a00_010d) {
        // One radius word fills the whole radius array.
        f.put("radius_fill", Value::F32(s.f32()?));
    } else {
        let has = s.u8()?;
        f.put("has_radii", Value::U8(has));
        if has != 0 {
            f.put("radii", Value::F32s(s.u32s(n)?));
        }
    }
    f.put("active_count", Value::U16(s.u16()?));
    let has = presence(s, 0x0401_0007)?;
    f.put("has_sizes", Value::U8(has as u8));
    if has {
        f.put("sizes", Value::F32s(s.u32s(n)?));
    }
    if rotating {
        let has = presence(s, 0x0401_0000)?;
        f.put("has_rotations", Value::U8(has as u8));
        if has {
            f.put("rotations", Value::F32s(quaternions(s, n)?));
        }
    } else if s.at_least(0x0500_0008) {
        let has = s.u8()?;
        f.put("has_rotations", Value::U8(has));
        if has != 0 {
            f.put("rotations", Value::F32s(quaternions(s, n)?));
        }
    }
    if s.below(0x0a00_0110) {
        // 8C35F4: (consistency & 0xFFF) | 0x8000; the create function set the
        // same from the constructor's 0 when the word was not read.
        let read = f.u16("consistency_flags");
        let native = (read.unwrap_or(0) & 0xfff) | 0x8000;
        match read {
            Some(word) => {
                let field = f.get_mut("consistency_flags").expect("read above");
                field.value = Value::U16(native);
                field.read = Some(Value::U16(word));
            }
            None => f.put("consistency_flags_native", Value::U16(native)),
        }
    }
    Ok(())
}

/// NiQuaternion::LoadBinary 871530 per particle: four separate reads.
fn quaternions(s: &mut Stream, n: usize) -> Result<Vec<u32>, NifError> {
    let mut v = Vec::with_capacity(n * 4);
    for _ in 0..n * 4 {
        v.push(s.f32()?);
    }
    Ok(v)
}

/// NiParticlesData / NiAutoNormalParticlesData (create 8C3170).
pub(crate) fn particles_data(s: &mut Stream, f: &mut Fields) -> R {
    particles_data_with(s, f, false)
}

/// NiRotatingParticlesData (create 8C31C0).
pub(crate) fn rotating_particles_data(s: &mut Stream, f: &mut Fields) -> R {
    particles_data_with(s, f, true)
}

/// NiParticleMeshesData::LoadBinary 90DB60 (its create function 90DF30 sets
/// the rotation byte +0x3C).
pub(crate) fn particle_meshes_data(s: &mut Stream, f: &mut Fields) -> R {
    particles_data_with(s, f, true)?;
    f.put("container", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysData::LoadBinary 8E6300: per particle 91C320 (velocity, rotation
/// axis, age, life span, last update, generation, code).
pub(crate) fn psys_data(s: &mut Stream, f: &mut Fields) -> R {
    particles_data(s, f)?;
    let n = f.u16("vertex_count").ok_or(NifError::Unsupported("8E6310 vertex count"))?;
    let mut info = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let mut p = Fields::default();
        p.put("velocity", Value::F32s(s.point3()?.to_vec()));
        p.put("rotation_axis", Value::F32s(s.point3()?.to_vec()));
        p.put("age", Value::F32(s.f32()?));
        p.put("life_span", Value::F32(s.f32()?));
        p.put("last_update", Value::F32(s.f32()?));
        p.put("generation", Value::U16(s.u16()?));
        p.put("code", Value::U16(s.u16()?));
        info.push(Value::Record(p.0));
    }
    f.put("particle_info", Value::List(info));
    f.put("max_added", Value::U16(s.u16()?));
    f.put("added_base", Value::U16(s.u16()?));
    Ok(())
}

/// NiParticleSystem::LoadBinary 89E230: NiParticles, world-space byte,
/// modifier links.
pub(crate) fn particle_system(s: &mut Stream, f: &mut Fields) -> R {
    geometry(s, f)?;
    let read = s.u8()?;
    f.put_static("world_space", read);
    f.put("modifiers", Value::Links(s.link_ids()?));
    Ok(())
}

/// NiParticleSystemController::LoadBinary 8CDB90.
pub(crate) fn particle_system_controller(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    let mut w = Vec::new();
    for _ in 0..6 {
        w.push(s.f32()?);
    }
    w.extend(s.point3()?);
    w.extend(quaternions(s, 1)?);
    for _ in 0..3 {
        w.push(s.f32()?);
    }
    f.put("emitter_words", Value::F32s(w));
    let read = s.u8()?;
    f.put_static("spawn_on_death", read);
    let mut w = Vec::new();
    for _ in 0..3 {
        w.push(s.f32()?);
    }
    f.put("birth_words", Value::F32s(w));
    let a = s.u8()?;
    let b = s.u8()?;
    f.put_static("use_birth_rate", a);
    f.put_static("spawn_on_emit", b);
    let mut w = Vec::new();
    for _ in 0..3 {
        w.push(s.f32()?);
    }
    f.put("emitter_box", Value::F32s(w));
    f.put("emitter", Value::Link(s.link_id()?));
    f.put("spawn_generations", Value::U16(s.u16()?));
    f.put("spawn_percentage", Value::F32(s.f32()?));
    f.put("spawn_multiplier", Value::U16(s.u16()?));
    f.put("spawn_speed_chaos", Value::F32(s.f32()?));
    f.put("spawn_direction_chaos", Value::F32(s.f32()?));
    let count = s.u16()?;
    f.put("particle_count", Value::U16(count));
    f.put("valid_count", Value::U16(s.u16()?));
    let mut list = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut p = Fields::default();
        p.put("velocity", Value::F32s(s.point3()?.to_vec()));
        p.put("rotation_axis", Value::F32s(s.point3()?.to_vec()));
        p.put("age", Value::F32(s.f32()?));
        p.put("life_span", Value::F32(s.f32()?));
        p.put("last_update", Value::F32(s.f32()?));
        p.put("generation", Value::U16(s.u16()?));
        p.put("code", Value::U16(s.u16()?));
        list.push(Value::Record(p.0));
    }
    f.put("particles", Value::List(list));
    f.put("modifier", Value::Link(s.link_id()?));
    f.put("collider", Value::Link(s.link_id()?));
    f.put("particle_data", Value::Link(s.link_id()?));
    if s.at_least(0x0303_000f) {
        let read = s.u8()?;
        f.put_static("static_target_bound", read);
    }
    Ok(())
}

/// NiParticleModifier::LoadBinary 91B3A0: next modifier and controller.
fn particle_modifier(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("next", Value::Link(s.link_id()?));
    f.put("controller", Value::Link(s.link_id()?));
    Ok(())
}

/// NiParticleRotation::LoadBinary 8E0240.
pub(crate) fn particle_rotation(s: &mut Stream, f: &mut Fields) -> R {
    particle_modifier(s, f)?;
    let read = s.u8()?;
    f.put_static("random_initial_axis", read);
    f.put("initial_axis", Value::F32s(s.point3()?.to_vec()));
    f.put("rotation_speed", Value::F32(s.f32()?));
    Ok(())
}

/// NiParticleMeshModifier::LoadBinary 8E0330.
pub(crate) fn particle_mesh_modifier(s: &mut Stream, f: &mut Fields) -> R {
    particle_modifier(s, f)?;
    f.put("meshes", Value::Links(s.link_ids()?));
    Ok(())
}

/// NiParticleGrowFade::LoadBinary 8E0A80.
pub(crate) fn particle_grow_fade(s: &mut Stream, f: &mut Fields) -> R {
    particle_modifier(s, f)?;
    f.put("grow", Value::F32(s.f32()?));
    f.put("fade", Value::F32(s.f32()?));
    Ok(())
}

/// NiParticleColorModifier::LoadBinary 8E0B00.
pub(crate) fn particle_color_modifier(s: &mut Stream, f: &mut Fields) -> R {
    particle_modifier(s, f)?;
    f.put("color_data", Value::Link(s.link_id()?));
    Ok(())
}

/// NiGravity::LoadBinary 8E1870.
pub(crate) fn gravity(s: &mut Stream, f: &mut Fields) -> R {
    particle_modifier(s, f)?;
    f.put("decay", Value::F32(s.f32()?));
    f.put("force", Value::F32(s.f32()?));
    f.put("field_type", Value::U32(s.u32()?));
    f.put("position", Value::F32s(s.point3()?.to_vec()));
    f.put("direction", Value::F32s(s.point3()?.to_vec()));
    Ok(())
}

/// NiPSysModifier::LoadBinary 910670: name, order, target, active.
fn psys_modifier(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("modifier_name", Value::Str(s.cstring()?));
    f.put("order", Value::U32(s.u32()?));
    f.put("target", Value::Link(s.link_id()?));
    let read = s.u8()?;
    f.put_static("active", read);
    Ok(())
}

/// NiPSysPositionModifier 8E39C0 is NiPSysModifier's loader.
pub(crate) fn psys_position(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)
}

/// NiPSysColliderManager / NiPSysColorModifier::LoadBinary 8CB880.
pub(crate) fn psys_modifier_link(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("link", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysBoundUpdateModifier::LoadBinary 8E7180.
pub(crate) fn psys_bound_update(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("update_skip", Value::U16(s.u16()?));
    Ok(())
}

/// NiPSysAgeDeathModifier::LoadBinary 8E74B0.
pub(crate) fn psys_age_death(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    let read = s.u8()?;
    f.put_static("spawn_on_death", read);
    f.put("spawn_modifier", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysSpawnModifier::LoadBinary 8E2F50.
pub(crate) fn psys_spawn(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("generations", Value::U16(s.u16()?));
    f.put("percentage", Value::F32(s.f32()?));
    f.put("min_spawned", Value::U16(s.u16()?));
    f.put("max_spawned", Value::U16(s.u16()?));
    f.put("speed_chaos", Value::F32(s.f32()?));
    f.put("direction_chaos", Value::F32(s.f32()?));
    f.put("life_span", Value::F32(s.f32()?));
    f.put("life_span_variation", Value::F32(s.f32()?));
    Ok(())
}

/// NiPSysRotationModifier::LoadBinary 8E35B0.
pub(crate) fn psys_rotation(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("initial_speed", Value::F32(s.f32()?));
    let read = s.u8()?;
    f.put_static("random_initial_axis", read);
    f.put("initial_axis", Value::F32s(s.point3()?.to_vec()));
    Ok(())
}

/// NiPSysGrowFadeModifier::LoadBinary 8E4460.
pub(crate) fn psys_grow_fade(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("grow_time", Value::F32(s.f32()?));
    f.put("grow_generation", Value::U16(s.u16()?));
    f.put("fade_time", Value::F32(s.f32()?));
    f.put("fade_generation", Value::U16(s.u16()?));
    Ok(())
}

/// NiPSysDragModifier::LoadBinary 8E54E0.
pub(crate) fn psys_drag(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("parent", Value::Link(s.link_id()?));
    f.put("axis", Value::F32s(s.point3()?.to_vec()));
    f.put("percentage", Value::F32(s.f32()?));
    f.put("range", Value::F32(s.f32()?));
    f.put("range_falloff", Value::F32(s.f32()?));
    Ok(())
}

/// NiPSysGravityModifier::LoadBinary 8CB4C0; turbulence from 10.0.1.14.
pub(crate) fn psys_gravity(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    f.put("gravity_object", Value::Link(s.link_id()?));
    f.put("axis", Value::F32s(s.point3()?.to_vec()));
    f.put("decay", Value::F32(s.f32()?));
    f.put("strength", Value::F32(s.f32()?));
    f.put("force_type", Value::U32(s.u32()?));
    if s.at_least(0x0a00_010e) {
        f.put("turbulence", Value::F32(s.f32()?));
        f.put("turbulence_scale", Value::F32(s.f32()?));
    }
    Ok(())
}

/// NiPSysEmitter::LoadBinary 8E9C00.
fn psys_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier(s, f)?;
    let mut w = Vec::new();
    for _ in 0..6 {
        w.push(s.f32()?);
    }
    w.extend(quaternions(s, 1)?);
    for _ in 0..3 {
        w.push(s.f32()?);
    }
    f.put("emitter_words", Value::F32s(w));
    Ok(())
}

/// NiPSysVolumeEmitter::LoadBinary 8C7310.
fn psys_volume_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_emitter(s, f)?;
    f.put("emitter_object", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysBoxEmitter::LoadBinary 8E6E80.
pub(crate) fn psys_box_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_volume_emitter(s, f)?;
    f.put("width", Value::F32(s.f32()?));
    f.put("height", Value::F32(s.f32()?));
    f.put("depth", Value::F32(s.f32()?));
    Ok(())
}

/// NiPSysCylinderEmitter::LoadBinary 8E66E0.
pub(crate) fn psys_cylinder_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_volume_emitter(s, f)?;
    f.put("radius", Value::F32(s.f32()?));
    f.put("height", Value::F32(s.f32()?));
    Ok(())
}

/// NiPSysSphereEmitter::LoadBinary 8E24D0.
pub(crate) fn psys_sphere_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_volume_emitter(s, f)?;
    f.put("radius", Value::F32(s.f32()?));
    Ok(())
}

/// NiPSysMeshEmitter::LoadBinary 8C92E0.
pub(crate) fn psys_mesh_emitter(s: &mut Stream, f: &mut Fields) -> R {
    psys_emitter(s, f)?;
    f.put("meshes", Value::Links(s.link_ids()?));
    f.put("initial_velocity_type", Value::U32(s.u32()?));
    f.put("emission_type", Value::U32(s.u32()?));
    f.put("emission_axis", Value::F32s(s.point3()?.to_vec()));
    Ok(())
}

/// NiPSysCollider::LoadBinary 910BB0.
fn psys_collider(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    f.put("bounce", Value::F32(s.f32()?));
    let a = s.u8()?;
    f.put_static("spawn_on_collide", a);
    let b = s.u8()?;
    f.put_static("die_on_collide", b);
    f.put("spawn_modifier", Value::Link(s.link_id()?));
    f.put("parent", Value::Link(s.link_id()?));
    f.put("next_collider", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysPlanarCollider::LoadBinary 8C8660.
pub(crate) fn psys_planar_collider(s: &mut Stream, f: &mut Fields) -> R {
    psys_collider(s, f)?;
    f.put("collider_object", Value::Link(s.link_id()?));
    f.put("width", Value::F32(s.f32()?));
    f.put("height", Value::F32(s.f32()?));
    f.put("x_axis", Value::F32s(s.point3()?.to_vec()));
    f.put("y_axis", Value::F32s(s.point3()?.to_vec()));
    Ok(())
}

/// NiPSysSphericalCollider::LoadBinary 8C7BB0; radius from 10.0.1.19.
pub(crate) fn psys_spherical_collider(s: &mut Stream, f: &mut Fields) -> R {
    psys_collider(s, f)?;
    f.put("collider_object", Value::Link(s.link_id()?));
    if s.at_least(0x0a00_0113) {
        f.put("radius", Value::F32(s.f32()?));
    }
    Ok(())
}

/// NiPSysModifierCtlr::LoadBinary 8E94C0: controller and modifier name.
fn psys_modifier_ctlr(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)?;
    f.put("modifier_name", Value::Str(s.cstring()?));
    Ok(())
}

/// NiPSysEmitterCtlr 8B8B60 (also the initial-radius, speed and
/// modifier-active controllers, 8E3B20): modifier controller and data link.
pub(crate) fn psys_emitter_ctlr(s: &mut Stream, f: &mut Fields) -> R {
    psys_modifier_ctlr(s, f)?;
    f.put("data", Value::Link(s.link_id()?));
    Ok(())
}

/// NiPSysUpdateCtlr 8E1A30 is NiTimeController's loader.
pub(crate) fn psys_update_ctlr(s: &mut Stream, f: &mut Fields) -> R {
    time_controller(s, f)
}

/// NiPSysEmitterCtlrData::LoadBinary 8E4EA0: float keys (B45DC0) and
/// visibility keys (8EA520).
pub(crate) fn psys_emitter_ctlr_data(s: &mut Stream, f: &mut Fields) -> R {
    object(s, f)?;
    let n = s.u32()?;
    f.put("float_key_count", Value::U32(n));
    if n > 0 {
        let t = s.u32()?;
        f.put("float_key_type", Value::U32(t));
        f.put("float_keys", keys(s, FLOAT_KEYS, t, n)?);
    }
    let n = s.u32()?;
    f.put("visibility_key_count", Value::U32(n));
    if n > 0 {
        f.put("visibility_keys", bool_keys(s, n)?);
    }
    Ok(())
}
