//! Exact native map inputs; this is not a completed world or a playable match.
use aa_mapworld::ownership::OwnerTarget;
use aa_mapworld::{MapArchive, WorldInventory};
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Instant};

pub fn run(path: &Path, policy: MemoryPolicy) -> Result<Value> {
    let start = Instant::now();
    let file_bytes = std::fs::metadata(path)?.len();
    preflight(
        MemoryEstimate {
            resident_cpu_bytes: file_bytes,
            ..Default::default()
        },
        policy,
    )?;
    let archive = MapArchive::from_bytes(std::fs::read(path)?)?;
    let decoded_bytes = archive.directory().iter().try_fold(0_u64, |sum, row| {
        sum.checked_add(row.decoded_bytes)
            .context("Map decoded-byte total overflow")
    })?;
    //Includes decoded chunks, preserved subtype tails and typed field overhead.
    //This is an observed-hardware preflight estimate, not a fixed RAM ceiling.
    let assessment = preflight(
        MemoryEstimate {
            resident_cpu_bytes: count_bytes(decoded_bytes, 8)?,
            ..Default::default()
        },
        policy,
    )?;
    let inventory = WorldInventory::decode(&archive, assessment.effective_cpu_budget_bytes / 8)?;
    let mut classes = BTreeMap::new();
    let mut owner_handles = BTreeMap::new();
    for placement in &inventory.dynamic_things.placements {
        *classes.entry(placement.class_code).or_insert(0_usize) += 1;
        if let Some(actor) = &placement.actor {
            *owner_handles.entry(actor.owner_handle).or_insert(0_usize) += 1;
        }
    }
    let header = inventory.header.as_ref().map(|h| json!({
        "scope":"Header setup metadata; saved runtime identities are reported separately below",
        "version":h.version,"source":h.source,"world_extents_bits":h.creator.world_extents_bits,
        "world_extents":h.creator.world_extents_bits.map(f32::from_bits),
        "creator_seed":h.creator.rng_seed,"biome":String::from_utf16_lossy(&h.creator.biome),
        "weather":String::from_utf16_lossy(&h.creator.weather),
        "teams":h.creator.parameters.teams.iter().map(|t|String::from_utf16_lossy(&t.id)).collect::<Vec<_>>(),
        "kingdoms":h.creator.parameters.kingdoms.iter().map(|k|json!({"id":String::from_utf16_lossy(&k.id),
            "nation":String::from_utf16_lossy(&k.nation),"faction":String::from_utf16_lossy(&k.faction),
            "color":String::from_utf16_lossy(&k.color),"resource_entries":k.resources.len()})).collect::<Vec<_>>()
    }));
    let teams = inventory.teams.as_ref().map(|teams| {
        let records: Vec<_> = teams
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, value)| {
                let record = value.record.as_ref()?;
                Some(json!({
                    "source_slot":slot,"source_span":record.source_span,
                    "serialized_id":record.serialized_id,
                    "definition_id_utf16":record.definition_id_utf16,
                    "name":String::from_utf16_lossy(&record.thing.name_utf16),
                    "version":record.version,"slot_index":record.slot_index,
                    "display_key":String::from_utf16_lossy(&record.display_key_utf16)
                }))
            })
            .collect();
        json!({"source":teams.source,"slot_count":teams.slots.len(),
            "present_count":records.len(),"records":records})
    });
    let kingdoms = inventory.kingdoms.as_ref().map(|kingdoms| {
        let records: Vec<_> = kingdoms.slots.iter().enumerate().filter_map(|(slot, value)| {
            let record = value.record.as_ref()?;
            Some(json!({
                "source_slot":slot,"source_span":record.source_span,
                "serialized_id":record.serialized_id,
                "definition_id_utf16":record.definition_id_utf16,
                "name":String::from_utf16_lossy(&record.thing.name_utf16),
                "version":record.payload.version,
                "word_188":record.payload.word_188,"word_18c":record.payload.word_18c,
                "word_190":record.payload.word_190,"team_handle":record.payload.team_handle,
                "display_key":String::from_utf16_lossy(&record.payload.display_key_utf16),
                "nation_reference":record.payload.nation_reference_utf16.as_ref().map(|s|String::from_utf16_lossy(s)),
                "color_reference":String::from_utf16_lossy(&record.payload.color_reference_utf16),
                "resource_entries":record.payload.resources_1c4.values.len()
            }))
        }).collect();
        json!({"source":kingdoms.source,"world_version":kingdoms.world_version,
            "major_count":kingdoms.major_count,"slot_count":kingdoms.slots.len(),
            "present_count":records.len(),"records":records})
    });
    let unresolved_owners: Vec<_> = inventory
        .ownership
        .iter()
        .flat_map(|owners| &owners.references)
        .filter(|reference| {
            reference.owner_handle != 0 && !matches!(reference.target, OwnerTarget::Kingdom { .. })
        })
        .collect();
    let ownership = inventory.ownership.as_ref().map(|owners| {
        let mut targets = BTreeMap::new();
        for reference in &owners.references {
            targets.entry(reference.owner_handle).or_insert(&reference.target);
        }
        let null_count = owners.references.iter().filter(|r| r.owner_handle == 0).count();
        json!({
            "scope":"Saved owner-handle references; runtime binding additionally requires final Thing flag bit23 and restored Kingdom objects",
            "kingdoms_source":owners.kingdoms_source,"reference_count":owners.references.len(),
            "null_count":null_count,"non_null_count":owners.references.len()-null_count,
            "resolved_non_null_count":owners.references.len()-null_count-unresolved_owners.len(),
            "unresolved_non_null_count":unresolved_owners.len(),
            "by_saved_handle":targets.into_iter().map(|(handle,target)|json!({
                "owner_handle":handle,"actor_records":owner_handles.get(&handle).copied().unwrap_or(0),
                "target":target
            })).collect::<Vec<_>>(),
            "unresolved_records":unresolved_owners
        })
    });
    let passed = inventory.header.is_some()
        && inventory.header_error.is_none()
        && inventory.teams.is_some()
        && inventory.teams_error.is_none()
        && inventory.kingdoms.is_some()
        && inventory.kingdoms_error.is_none()
        && inventory.ownership.is_some()
        && inventory.generation_mismatches.is_empty()
        && inventory.team_generation_mismatches.is_empty()
        && inventory.kingdom_generation_mismatches.is_empty()
        && unresolved_owners.is_empty()
        && inventory.dynamic_things.unsupported.is_empty();
    let mut height_layers = Vec::new();
    if let Some(header) = &inventory.header {
        for name in ["HEIGHT_MAP", "MOUNTAIN_HEIGHT_MAP"] {
            let chunk = archive.decode_chunk(name, assessment.effective_cpu_budget_bytes / 8)?;
            let resolution = u32::from_le_bytes(
                chunk
                    .bytes
                    .get(16..20)
                    .context("Height layer has a truncated resolution field")?
                    .try_into()?,
            );
            let layout = aa_heightfield::binding::GridLayout::from_world_extents(
                header.creator.world_extents_bits,
                resolution,
            )?;
            let loaded = aa_heightfield::binding::LoadedHeightfield::decode(layout, &chunk.bytes)?;
            height_layers.push(json!({"name":name,"source":chunk.payload_source,"layout":layout,
                "samples":loaded.state.field.height_bits.len(),"sample_flags":loaded.state.sample_flags.len(),
                "minimum_bits":loaded.state.minimum_bits,"maximum_bits":loaded.state.maximum_bits,
                "default_bits":loaded.state.default_bits,"preserved_tail_bytes":loaded.trailing_bytes.len(),
                "scope":"Stored resolution and exact dimensions validated; live initialization must also match the resolved HeightmapResolutionBits SVar"}));
        }
    }
    Ok(json!({
        "passed":passed,
        "scope":"Original binary map inputs and base record validation; not a completed live world, save decoder or playable match",
        "map_source":inventory.map_source,"file":path,"seconds":start.elapsed().as_secs_f64(),
        "chunks":archive.directory().len(),"decoded_bytes":decoded_bytes,"header":header,"header_error":inventory.header_error,
        "serialized_teams":teams,"teams_error":inventory.teams_error,
        "team_generation_mismatches":inventory.team_generation_mismatches,
        "serialized_kingdoms":kingdoms,"kingdoms_error":inventory.kingdoms_error,
        "kingdom_generation_mismatches":inventory.kingdom_generation_mismatches,
        "ownership":ownership,
        "world_clock_and_rng":inventory.world,"id_manager":{"next_slot":inventory.id_manager.next_slot,
            "highest_slot":inventory.id_manager.highest_slot,"serialized_generations":inventory.id_manager.serialized_generations.len()},
        "placements":inventory.dynamic_things.placements.len(),"classes":classes,
        "serialized_owner_handles":owner_handles,"generation_mismatches":inventory.generation_mismatches,
        "unsupported_records":inventory.dynamic_things.unsupported.iter().map(|p|json!({"ordinal":p.record_ordinal,"reason":p.reason})).collect::<Vec<_>>(),
        "pending_sections":inventory.opaque_initialization.iter().map(|s|json!({"name":s.name,"source":s.source,"reason":s.reason})).collect::<Vec<_>>(),
        "absent_sections":inventory.absent_sections,"height_layers":height_layers
    }))
}
