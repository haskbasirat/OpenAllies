//! Extracts the saved Organization (slot 7) and Transport (slot 24) payloads
//! of every original map with the restoration decoders for the components
//! that precede them (aa-mapworld SavedComponent::decode_prefix, native
//! 4D919C stage order). Prints JSON: transport payload bytes (component start
//! to the end of the actor tail) and an Organization prefix summary.
use aa_mapworld::{Inventory, MapArchive, actor_payload::ActorTail, saved_components::SavedComponent};
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
/// `--masks <map>...`: the default/saved component masks (the first two
/// words of every actor tail) of each actor record, for the post-load
/// inventory (which component PostLoad virtuals a map reaches).
fn masks(paths: &[String]) {
    let mut rows = Vec::new();
    for path in paths {
        let map = MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
        let inv = Inventory::decode(&map, 1 << 31).unwrap();
        let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy().into_owned();
        for p in &inv.placements {
            let Some(actor) = &p.actor else { continue };
            let t = &p.opaque_tail;
            if t.len() < 8 {
                continue;
            }
            let default = u32::from_le_bytes(t[0..4].try_into().unwrap());
            let saved = u32::from_le_bytes(t[4..8].try_into().unwrap());
            rows.push(serde_json::json!({"map": name, "ordinal": p.record_ordinal, "class": p.class_code,
                "definition": String::from_utf16_lossy(&p.thing_id_utf16), "actor_version": actor.version,
                "default_mask": default, "saved_mask": saved}));
        }
    }
    println!("{}", serde_json::json!({"actors": rows}));
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--masks") {
        return masks(&args[1..]);
    }
    let mut transports = Vec::new();
    let mut organizations = Vec::new();
    let mut denizens = Vec::new();
    for path in std::env::args().skip(1) {
        let map = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let Ok(inv) = Inventory::decode(&map, 1 << 31) else { continue };
        let name = std::path::Path::new(&path).file_name().unwrap().to_string_lossy().into_owned();
        for p in &inv.placements {
            let Some(actor) = &p.actor else { continue };
            let tail = &p.opaque_tail;
            let mut found = None;
            let mut denizen: Option<String> = None;
            let _ = ActorTail::decode_prefix_with_components(tail, actor.version, 0, |slot, offset, bytes| {
                if slot == 10 {
                    // Record the Denizen payload, then keep decoding with the
                    // restoration codec (it reads every occupant pair).
                    let (_, n) = SavedComponent::decode_prefix(slot, bytes)?;
                    denizen = Some(hex(&bytes[..n]));
                    return Ok((n, None));
                }
                if slot == 7 || slot == 24 {
                    found = Some((slot, offset));
                    return Err(aa_mapworld::Error::Format("stop".into()));
                }
                let (_, n) = SavedComponent::decode_prefix(slot, bytes)?;
                Ok((n, None))
            });
            if let Some(payload) = denizen.take() {
                denizens.push(serde_json::json!({"map": name, "ordinal": p.record_ordinal,
                    "definition": String::from_utf16_lossy(&p.thing_id_utf16), "payload": payload}));
            }
            let Some((slot, offset)) = found else { continue };
            let row = serde_json::json!({"map": name, "ordinal": p.record_ordinal,
                "definition": String::from_utf16_lossy(&p.thing_id_utf16), "actor_version": actor.version});
            let rest = &tail[offset..];
            if slot == 24 {
                let mut row = row;
                row["payload"] = hex(rest).into();
                transports.push(row);
            } else {
                let mut row = row;
                row["prefix"] = hex(&rest[..rest.len().min(512)]).into();
                organizations.push(row);
            }
        }
    }
    // Producer-side Denizen inventories (aa-rules, definitions agent) for every
    // definition with a dumped Denizen payload, and each slot target's
    // ActorData+284 (the 45D730 null-modifier value used by 56C320).
    let mut inventories = serde_json::Map::new();
    if let Ok(root) = std::env::var("AA_GAME_ROOT") {
        let root = std::path::PathBuf::from(root);
        let mut sources = aa_rules::sources::V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
        )
        .unwrap();
        let (catalog, _) = aa_rules::actors::native_load::load(&mut sources).unwrap();
        let pc = aa_numeric::Precision::Double53;
        let mut ids: Vec<String> = denizens.iter().map(|d| d["definition"].as_str().unwrap().to_owned()).collect();
        ids.sort();
        ids.dedup();
        for id in ids {
            let value = match catalog.native_denizen_inventory(&id, pc) {
                Ok(Some(inv)) => {
                    let slots: Vec<serde_json::Value> = inv.slots.iter().map(|s| {
                        let health = catalog.native_ground_definition(&s.target, pc).ok()
                            .and_then(|g| g.attributes.body.as_ref().map(|b| b.health.bits));
                        serde_json::json!({"target": s.target, "worker": s.worker,
                            "militia_or_air_defensive": s.militia_or_air_defensive, "health_284_bits": health})
                    }).collect();
                    serde_json::json!({"slots": slots})
                }
                Ok(None) => serde_json::json!({"slots": null}),
                Err(e) => serde_json::json!({"error": e.to_string()}),
            };
            inventories.insert(id, value);
        }
    }
    println!("{}", serde_json::json!({"transports": transports, "organizations": organizations, "denizens": denizens, "denizen_inventories": inventories}));
}
