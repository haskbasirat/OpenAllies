//! Diagnostic: dump the raw saved bytes of a given component slot for chosen
//! records of an original map (to drive native oracle cases). Ignored; prints.
use aa_mapworld::{
    MapArchive, WorldInventory, actor_payload::ActorTail, saved_components::SavedComponent,
};

fn dump(map: &str, ids: &[u32], wanted: u8) {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let map = MapArchive::from_bytes(
        std::fs::read(game.join(format!("reconstruction/assets/original/Data/Maps/{map}")))
            .unwrap(),
    )
    .unwrap();
    let inventory = WorldInventory::decode(&map, u64::MAX).unwrap();
    for p in &inventory.dynamic_things.placements {
        if !ids.contains(&p.serialized_id) {
            continue;
        }
        let actor = p.actor.as_ref().unwrap();
        let mut found = None;
        let _ = ActorTail::decode_prefix_with_stages(
            &p.opaque_tail,
            actor.version,
            0,
            |slot, saved, _, offset, bytes| {
                if !saved {
                    return Ok((0, None));
                }
                if slot == wanted {
                    found = Some((offset, bytes.to_vec()));
                    return Err(aa_mapworld::Error::Format("probe stop".into()));
                }
                let (payload, consumed) = SavedComponent::decode_prefix(slot, bytes)?;
                let properties = if let SavedComponent::Properties(v) = &payload {
                    Some(v.clone())
                } else {
                    None
                };
                Ok((consumed, properties))
            },
        );
        let id = String::from_utf16_lossy(&p.thing_id_utf16);
        match found {
            Some((offset, bytes)) => {
                let hex: String = bytes
                    .iter()
                    .take(1200)
                    .map(|b| format!("{b:02x}"))
                    .collect();
                println!(
                    "{id} {} slot{wanted} offset {offset} remaining {} :: {hex}",
                    p.serialized_id,
                    bytes.len()
                );
            }
            None => println!("{id} {}: slot {wanted} not reached", p.serialized_id),
        }
    }
}

#[test]
#[ignore = "diagnostic; requires AA_GAME_ROOT"]
fn dump_saved_component_bytes() {
    let map = std::env::var("PROBE_MAP").unwrap_or("Campaign/tutorial01.RMP".into());
    let slot: u8 = std::env::var("PROBE_SLOT")
        .unwrap_or("4".into())
        .parse()
        .unwrap();
    let ids: Vec<u32> = std::env::var("PROBE_IDS")
        .unwrap_or("1220".into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    dump(&map, &ids, slot);
}

/// Corpus mode: every saved Mind (slot 4) payload reachable through already
/// decodable components, with the definition's component mask (Mind kind).
#[test]
#[ignore = "diagnostic; requires AA_GAME_ROOT; writes PROBE_OUT"]
fn dump_saved_mind_corpus() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = aa_runtime::catalog::Catalog::new(&actors);
    let root = game.join("reconstruction/assets/original/Data/Maps");
    let mut paths = Vec::new();
    fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for e in entries {
            if e.is_dir() {
                walk(&e, out)
            } else if e.extension().is_some_and(|x| x.eq_ignore_ascii_case("rmp")) {
                out.push(e)
            }
        }
    }
    walk(&root, &mut paths);
    let mut lines = Vec::new();
    for path in &paths {
        let map = MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
        let Ok(inventory) = WorldInventory::decode(&map, u64::MAX) else {
            continue;
        };
        for p in &inventory.dynamic_things.placements {
            let Some(actor) = &p.actor else { continue };
            let id = String::from_utf16_lossy(&p.thing_id_utf16);
            let Ok(binding) = catalog.actor_bindings(&id, aa_numeric::Precision::Double53) else {
                continue;
            };
            let mask = binding.definition.component_mask.unwrap_or(0);
            let mut found = None;
            let _ = ActorTail::decode_prefix_with_stages(
                &p.opaque_tail,
                actor.version,
                0,
                |slot, saved, _, _, bytes| {
                    if !saved {
                        return Ok((0, None));
                    }
                    if slot == 4 {
                        found = Some(bytes.to_vec());
                        return Err(aa_mapworld::Error::Format("probe stop".into()));
                    }
                    let (payload, consumed) = SavedComponent::decode_prefix(slot, bytes)?;
                    let properties = if let SavedComponent::Properties(v) = &payload {
                        Some(v.clone())
                    } else {
                        None
                    };
                    Ok((consumed, properties))
                },
            );
            if let Some(bytes) = found {
                let decoded = aa_runtime::catalog::mind_kind(mask)
                    .and_then(|kind| aa_mind::saved_load::decode(kind, &bytes).ok());
                let has_commands = decoded.as_ref().is_some_and(|(saved, _)| {
                    saved.base().is_some_and(|base| {
                        !base.commands.is_empty() || base.current_command.is_some()
                    })
                });
                if std::env::var_os("PROBE_COMMANDS_ONLY").is_some() && !has_commands {
                    continue;
                }
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                let mind_hex = decoded.as_ref().map(|(_, used)| {
                    bytes[..*used]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                });
                lines.push(serde_json::json!({
                    "map": path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/"),
                    "saved_id": p.serialized_id, "definition": id, "component_mask": mask, "tail_hex": hex,
                    "mind_payload_hex": mind_hex, "has_commands": has_commands,
                }).to_string());
            }
        }
    }
    let out = std::env::var("PROBE_OUT").unwrap();
    std::fs::write(out, lines.join("\n")).unwrap();
    println!("mind payloads: {}", lines.len());
}

/// The Organization saved Load owns embedded member records. This path visits
/// the real restore adapter so those members are included; top-level placement
/// scans alone cannot reach their saved Minds. InspectPartial is diagnostic,
/// and records remain explicitly failed if their command owner is unavailable.
#[test]
#[ignore = "diagnostic; requires AA_GAME_ROOT; writes PROBE_OUT"]
fn dump_restored_mind_commands() {
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::sources::{SourceLimits, V14Sources};
    use aa_rules::tgi::Limits;
    use aa_runtime::{localization::Localization, restoration::Attempt, settings::Settings};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut sources).unwrap();
    let root = game.join("reconstruction/assets/original/Data/Maps");
    let mut paths = Vec::new();
    fn walk(dir: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, paths);
            } else if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("rmp"))
            {
                paths.push(path);
            }
        }
    }
    walk(&root, &mut paths);
    paths.sort();
    let mut lines = Vec::new();
    for path in paths {
        let map = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let attempt = Attempt::load_with_mode(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            aa_numeric::Precision::Double53,
            u64::MAX,
            RestoreMode::InspectPartial,
        )
        .unwrap();
        for entry in attempt.registry.iter() {
            let Some(entity) = entry.value.loaded() else {
                continue;
            };
            for component in &entity.saved_components {
                let SavedComponent::Mind(payload) = &component.payload else {
                    continue;
                };
                let kind =
                    aa_runtime::catalog::mind_kind(entity.definition.component_mask.unwrap())
                        .unwrap();
                let (saved, used) = aa_mind::saved_load::decode(kind, &payload.bytes).unwrap();
                assert_eq!(used, payload.bytes.len());
                let Some(base) = saved.base() else { continue };
                if base.commands.is_empty() && base.current_command.is_none() {
                    continue;
                }
                let hex: String = payload.bytes.iter().map(|b| format!("{b:02x}")).collect();
                lines.push(serde_json::json!({
                    "map":path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\',"/"),
                    "saved_id":entity.source.serialized_id,"definition":entity.definition.proof.id,
                    "source_offset":component.source_span.offset,"source_length":component.source_span.length,
                    "mind_payload_hex":hex,"has_commands":true,
                }).to_string());
            }
        }
    }
    std::fs::write(std::env::var_os("PROBE_OUT").unwrap(), lines.join("\n")).unwrap();
    println!("restored command-bearing Mind payloads: {}", lines.len());
}
