//! Command Object-definition references on the Thing DataManager [B39F58].
//!
//! * Registration 0x4B4E60 (reached from 4BCA00 at 4BCF66 for every
//!   Thing/Unit/Projectile/VFX root, the only Thing-manager registration):
//!   `def+0C = [mgr+28]`, then the def is appended to the vector `+24`
//!   (`+28` count); a repeated IDS (case-insensitive) throws. The index of a
//!   reference is therefore the registration slot, which is
//!   `ActorCatalog::actors` order of the native load (aa_rules native_load
//!   projects `catalog.actors` from the 4BCA00 registration owners in slot
//!   order).
//! * Reader 0x76A960: 0x411FA0 reads `40C7C0([mgr+28]+1)` bits;
//!   `0 <= v < [mgr+28]`, `[mgr+24][v] != NULL` and the definition's vt+38
//!   true store it, anything else stores NULL. vt+38 is 0x4F0500 (returns 1)
//!   for the two ActorData classes 4BCA00 constructs for category kinds 2
//!   (4624B0, vtable A05540) and 19 (469980, vtable A09B30), and 0x4C9FA0
//!   (returns 0) for the other 22 kinds.
//! * Writer 0x76A880: `def ? def+0C : [mgr+28]` with the same width.
//!
//! Proven by research/phase8-orders-input/recruit/native-object-reference.json.
use crate::catalog::Catalog;
use aa_rules::actors::{ActorDefinition, ThingRef};

/// 0x76A960 vt+38 (0x4F0500) is true exactly for these category+18 kinds.
pub const ACTOR_DATA_KINDS: [u32; 2] = [2, 19];

/// Pure 0x76A960 decision on a read value: `slot_kind(v)` is the category
/// kind of the non-NULL `[mgr+24][v]` (None for a NULL entry). It is asked
/// only for `v < count`, exactly where the native reads the entry.
pub fn reader_76a960<E>(value: u32, count: usize, slot_kind: impl FnOnce(u32) -> Result<Option<u32>, E>) -> Result<Option<u32>, E> {
    if (value as usize) >= count {
        return Ok(None);
    }
    Ok(match slot_kind(value)? {
        Some(kind) if ACTOR_DATA_KINDS.contains(&kind) => Some(value),
        _ => None,
    })
}

/// Pure 0x76A880 written field: (value, bit width) for `def+0C` (None =
/// NULL definition).
pub fn writer_76a880(count: usize, index_0c: Option<u32>) -> Result<(u32, u8), String> {
    let bits = aa_orders::registry_bits(count.checked_add(1).ok_or("76A880 count overflow")?).map_err(|e| e.to_string())?;
    let value = match index_0c {
        Some(v) => v,
        None => u32::try_from(count).map_err(|_| "76A880 count exceeds 32 bits")?,
    };
    Ok((value, bits))
}

/// The ActorCatalog must be the native-order registry (`load_v14`): only that
/// load creates the generic DataManager registries and projects `actors` in
/// 4BCA00 registration order. A `from_files` census is not the Thing manager.
fn native_registry<'c>(catalog: &'c Catalog<'_>) -> Result<&'c aa_rules::actors::ActorCatalog, String> {
    catalog.require_source_owner()?;
    if catalog.actors.native_data_registry("ActorCommand").is_none() {
        return Err("[B39F58]+28: the catalog is not the native-order Thing registry (load_v14)".into());
    }
    Ok(catalog.actors)
}

/// `[B39F58]+28`: the Thing DataManager definition count (each serializer's
/// null sentinel equals it).
pub fn object_registry_count_b39f58(catalog: &Catalog<'_>) -> Result<usize, String> {
    Ok(native_registry(catalog)?.actors.len())
}

/// `[B39F58]+24[index]`: the definition registered in that slot. The slot's
/// IDS must re-identify the same allocation (no duplicate registration).
pub fn registered_object_b39f58<'c>(catalog: &'c Catalog<'_>, index: u32) -> Result<Option<&'c ActorDefinition>, String> {
    let actors = native_registry(catalog)?;
    let Some(definition) = actors.actors.get(index as usize) else { return Ok(None) };
    match actors.registered(&definition.id) {
        Some(d) if std::ptr::eq(d, definition) => Ok(Some(definition)),
        _ => Err(format!("[B39F58]+24[{index}] does not re-identify its registered IDS {:?}", definition.id)),
    }
}

/// 0x76A960 on a decoded wire value: Some only for an ActorData definition.
pub fn resolve_object_76a960<'c>(catalog: &'c Catalog<'_>, value: u32) -> Result<Option<&'c ActorDefinition>, String> {
    let count = object_registry_count_b39f58(catalog)?;
    let mut found = None;
    let accepted = reader_76a960(value, count, |v| {
        let Some(definition) = registered_object_b39f58(catalog, v)? else { return Ok(None) };
        found = Some(definition);
        catalog
            .actors
            .native_category(ThingRef::registered(&definition.id))
            .map(|c| Some(c.native_kind))
            .map_err(|e| format!("76A960 vt+38 category of {:?}: {e}", definition.id))
    })?;
    Ok(accepted.and(found))
}

/// 0x76A880's written value for a registered IDS (None = NULL).
pub fn object_reference_76a880(catalog: &Catalog<'_>, definition: Option<&str>) -> Result<(u32, u8), String> {
    let actors = native_registry(catalog)?;
    let index_0c = match definition {
        None => None,
        Some(ids) => {
            let d = actors.registered(ids).ok_or_else(|| format!("76A880 unregistered Thing IDS {ids:?}"))?;
            let slot = actors.actors.iter().position(|a| std::ptr::eq(a, d)).ok_or("76A880 registered definition is outside +24")?;
            Some(u32::try_from(slot).map_err(|_| "76A880 Thing registry index exceeds 32 bits")?)
        }
    };
    writer_76a880(actors.actors.len(), index_0c)
}

/// The Bank token of a resolved command `+14` definition (the domain the
/// saved-command binder uses for the same ActorData pointer).
pub(crate) fn object_definition_token(banks: &dyn crate::player_order_world::Banks, definition: &ActorDefinition) -> Result<u32, String> {
    banks
        .borrow_bank_mut()
        .map_err(|e| e.to_string())?
        .intern("ActorDefinition", &definition.id)
}

#[cfg(test)]
#[path = "player_order_object_reference_tests.rs"]
mod tests;
