use aa_mapworld::{
    Chunk, ChunkDirectory, MapArchive, SourceIdentity, archive::Encoding,
    restriction_groups::LocalizedStorage,
};
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    restriction_groups::{ListKind, LoadedRestrictionGroups},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

fn evidence() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-restriction-groups")
}
fn chunk(bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"restriction test map"),
        payload_source: SourceIdentity::of(&bytes),
        directory: ChunkDirectory {
            name: "RESTRICTION_GROUPS".into(),
            directory_ordinal: 0,
            packed_offset: 0,
            packed_bytes: bytes.len() as u64,
            source_timestamp: 0,
            decoded_bytes: bytes.len() as u64,
        },
        encoding: Encoding::Raw,
        bytes,
    }
}
fn source() -> ActorCatalog {
    let text = "[Thing]\n{ string IDS = Tank\n string category = ACTOR\n}\n[Thing]\n{ string IDS = Tree\n string category = OBJECT\n}\n[Property]\n{ string IDS = Tech\n bool is_technology = true\n}\n[Thing]\n{ string IDS = Old\n string category = RANDOM_ACTOR\n}\n[Property]\n{ string IDS = OldTech\n bool is_technology = true\n}\n[Translate]\n{ string node = Thing\n string ids_old = old\n string ids_new = Tank\n}\n[Translate]\n{ string node = Property\n string ids_old = oldtech\n string ids_new = Tech\n}\n[Translate]\n{ string node = Thing\n string ids_old = alias\n string ids_new = old\n}\n[Translate]\n{ string node = Property\n string ids_old = aliastech\n string ids_new = oldtech\n}\n";
    let bytes = text.as_bytes().to_vec();
    load_files(
        vec![ResolvedFile {
            logical_path: "restrictions-test.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }],
        Limits::default(),
        &mut NativeLoadReport::default(),
    )
}
fn trace(owner: &LoadedRestrictionGroups) -> Value {
    Value::Array(
        owner
            .lookups()
            .iter()
            .map(|entry| {
                json!([
                    if matches!(
                        entry.list,
                        ListKind::AllowedActors | ListKind::RestrictedActors
                    ) {
                        "actor"
                    } else {
                        "property"
                    },
                    String::from_utf16(&entry.authored)
                        .unwrap()
                        .split('\0')
                        .next()
                        .unwrap(),
                    entry.resolved.as_ref().map(|s| &s.id)
                ])
            })
            .collect(),
    )
}
fn compare(owner: &LoadedRestrictionGroups, expected: &Value) {
    assert_eq!(
        owner.input().payload_source().bytes,
        expected["bytes"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(owner.input().word_0()),
        expected["word"].as_u64().unwrap()
    );
    assert_eq!(
        owner.input().profiles().len(),
        expected["count"].as_u64().unwrap() as usize
    );
    assert_eq!(
        owner.lookups().len(),
        expected["lookups"].as_u64().unwrap() as usize
    );
    let bytes = serde_json::to_vec(&trace(owner)).unwrap();
    assert_eq!(
        SourceIdentity::of(&bytes).sha256,
        expected["trace_sha256"].as_str().unwrap()
    );
    let logs: Vec<u32> = owner
        .lookups()
        .iter()
        .filter(|e| e.duplicate_actor_skipped)
        .map(|e| {
            if e.list == ListKind::AllowedActors {
                0x188
            } else {
                0x1c4
            }
        })
        .collect();
    assert_eq!(json!(logs), expected["duplicate_log_lines"]);
    let mut seen = BTreeSet::new();
    for p in expected["profiles"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        if !seen.insert(name.to_ascii_lowercase()) {
            continue;
        }
        let got = owner.profiles().find(name).unwrap();
        for (list, index) in [(&got.actors, 2), (&got.technologies, 3)] {
            let mut keys: Vec<_> = list
                .iter()
                .filter_map(|(k, v)| v.then_some(k.clone()))
                .collect();
            keys.sort();
            let mut want: Vec<_> = p["maps"][index]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
            want.sort();
            assert_eq!(keys, want, "{name} restricted map {index}");
        }
    }
}
#[test]
fn native_current_profiles_aliases_unknown_duplicates_and_rehash() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(evidence().join("native-restriction-groups.json")).unwrap(),
    )
    .unwrap();
    let actors = source();
    assert!(actors.diagnostics.is_empty(), "{:?}", actors.diagnostics);
    let catalog = Catalog::new(&actors);
    for row in fixture["synthetic"].as_array().unwrap() {
        let bytes = row["payload_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect();
        let owner = LoadedRestrictionGroups::load(
            &chunk(bytes),
            &catalog,
            LocalizedStorage::KeyOnly,
            Precision::Double53,
        )
        .unwrap();
        compare(&owner, &row["expected"]);
        owner.require_catalog(&catalog).unwrap();
    }
    let row = &fixture["synthetic"][0];
    let bytes: Vec<_> = row["payload_hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    let payload = chunk(bytes);
    let owner = LoadedRestrictionGroups::load(
        &payload,
        &catalog,
        LocalizedStorage::KeyOnly,
        Precision::Double53,
    )
    .unwrap();
    {
        let _held_bank = catalog.definition_bank.borrow_mut();
        assert!(owner.require_catalog(&catalog).is_err());
        assert!(
            LoadedRestrictionGroups::load(
                &payload,
                &catalog,
                LocalizedStorage::KeyOnly,
                Precision::Double53
            )
            .is_err()
        );
    }
    assert!(
        owner
            .require_catalog(&Catalog::new(&actors))
            .unwrap_err()
            .contains("another Property source arena")
    );
    let mut changed = actors.clone();
    changed.actors.reverse();
    assert!(
        LoadedRestrictionGroups::load(
            &payload,
            &Catalog::new(&changed),
            LocalizedStorage::KeyOnly,
            Precision::Double53
        )
        .unwrap_err()
        .contains("Thing Find transport")
    );
    changed.actors.clear();
    assert!(
        LoadedRestrictionGroups::load(
            &payload,
            &Catalog::new(&changed),
            LocalizedStorage::KeyOnly,
            Precision::Double53
        )
        .unwrap_err()
        .contains("complete private source roots")
    );
    let mut legacy = vec![0u8; 12];
    legacy[8] = 1;
    legacy.extend_from_slice(&[0; 4]); // profile envelope0
    legacy.extend_from_slice(&[0; 2]); // name
    legacy.extend_from_slice(&[0; 4]); // word
    legacy.extend_from_slice(&[0; 2]); // display
    legacy.extend_from_slice(&[0; 2]); // discarded old label
    legacy.extend_from_slice(&[0; 8]); // empty allowed actor/property lists
    assert!(
        LoadedRestrictionGroups::load(
            &chunk(legacy),
            &catalog,
            LocalizedStorage::KeyOnly,
            Precision::Double53
        )
        .unwrap_err()
        .contains("complement enumeration")
    );
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets; optional AA_CAPTURE_RESTRICTIONS writes own source lookup receipt"]
fn actual_all_eighty_maps_current_canonical_sources() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let settings =
        aa_runtime::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text =
        aa_runtime::localization::Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let resources =
        std::sync::Arc::new(aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.definition_text = Some(text.clone());
    catalog.replace_current_text(text);
    catalog.bind_resource_catalog(resources).unwrap();
    let maps = root.join("reconstruction/assets/original/Data/Maps");
    fn walk(at: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                walk(&p, root, out)
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rmp")) {
                out.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let capture = std::env::var_os("AA_CAPTURE_RESTRICTIONS").is_some();
    let oracle = if capture {
        None
    } else {
        Some(
            serde_json::from_slice::<Value>(
                &std::fs::read(evidence().join("native-restriction-groups.json")).unwrap(),
            )
            .unwrap(),
        )
    };
    let mut names = Vec::new();
    walk(&maps, &maps, &mut names);
    names.sort();
    assert_eq!(names.len(), 80);
    let mut rows = Vec::new();
    for name in names {
        let archive = MapArchive::from_bytes(std::fs::read(maps.join(&name)).unwrap()).unwrap();
        let row = archive
            .directory()
            .iter()
            .find(|r| r.name == "RESTRICTION_GROUPS")
            .unwrap();
        let payload = archive
            .decode_chunk("RESTRICTION_GROUPS", row.decoded_bytes)
            .unwrap();
        let owner = LoadedRestrictionGroups::load(
            &payload,
            &catalog,
            LocalizedStorage::KeyOnly,
            Precision::Double53,
        )
        .unwrap();
        owner.require_catalog(&catalog).unwrap();
        if let Some(oracle) = &oracle {
            let found = oracle["maps"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["map"].as_str() == Some(&name))
                .unwrap();
            assert_eq!(
                owner.input().payload_source().sha256,
                found["payload_sha256"].as_str().unwrap()
            );
            compare(&owner, &found["expected"]);
        }
        let mut lookups: BTreeMap<&str, BTreeMap<String, Option<String>>> =
            BTreeMap::from([("actor", BTreeMap::new()), ("property", BTreeMap::new())]);
        for entry in owner.lookups() {
            let kind = if matches!(
                entry.list,
                ListKind::AllowedActors | ListKind::RestrictedActors
            ) {
                "actor"
            } else {
                "property"
            };
            let ids = String::from_utf16(&entry.authored)
                .unwrap()
                .to_ascii_lowercase();
            lookups
                .get_mut(kind)
                .unwrap()
                .insert(ids, entry.resolved.as_ref().map(|s| s.id.clone()));
        }
        rows.push(json!({"map":name,"payload_sha256":owner.input().payload_source().sha256,"lookups":lookups}));
    }
    if capture {
        let receipt = json!({"schema":1,"scope":"Actual current typed source lookup inputs from immutable native-order ActorCatalog; unknowns are null, canonical IDs are not translated again.","source_files":actors.sources,"maps":rows});
        std::fs::write(
            evidence().join("source-lookups.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    } else {
        // Explicitly diagnostic Bound source owner for ownership guards only.
        // It cannot supply completed means or authorize runtime startup.
        let fp = aa_runtime::combat_value_source_context::SourceFpContext::supplied_diagnostic(
            0x27f,
            0,
            0x1f80,
            true,
            "restriction source ownership regression only",
        )
        .unwrap();
        let csv = aa_runtime::combat_value_source_context::bind_csv_from_sources(
            &mut sources,
            aa_runtime::combat_value_source_context::CsvScope::FreshNoProfile {
                label: "restriction guard regression; no actual profile read",
            },
        )
        .unwrap();
        aa_runtime::combat_source_completion::install(
            &catalog,
            &mut sources,
            &fp,
            &csv,
            aa_runtime::combat_source_completion::Globals {
                assertion_guard: 0,
                ranged_lookup_guard: 0,
                csv_lookup_guard: 0,
                debug_stream_is_closed: true,
            },
        )
        .unwrap();
        assert!(
            catalog
                .definition_bank
                .borrow()
                .combat_source()
                .unwrap()
                .completed_means()
                .is_err()
        );
        let map =
            MapArchive::from_bytes(std::fs::read(maps.join("Campaign/tutorial02.RMP")).unwrap())
                .unwrap();
        let row = map
            .directory()
            .iter()
            .find(|r| r.name == "RESTRICTION_GROUPS")
            .unwrap();
        let payload = map
            .decode_chunk("RESTRICTION_GROUPS", row.decoded_bytes)
            .unwrap();
        let owner = LoadedRestrictionGroups::load(
            &payload,
            &catalog,
            LocalizedStorage::KeyOnly,
            Precision::Double53,
        )
        .unwrap();
        owner.require_catalog(&catalog).unwrap();
        assert!(
            LoadedRestrictionGroups::load(
                &payload,
                &catalog,
                LocalizedStorage::KeyOnly,
                Precision::Single24
            )
            .unwrap_err()
            .contains("source PC/identities")
        );
        let mut foreign = actors.clone();
        foreign.sources[0].sha256 = "foreign source identity for guard regression".into();
        let other = Catalog::new(&foreign);
        // Move the one actual Bank; do not create a parallel source state.
        *other.definition_bank.borrow_mut() =
            std::mem::take(&mut *catalog.definition_bank.borrow_mut());
        assert!(
            owner
                .require_catalog(&other)
                .unwrap_err()
                .contains("source PC/identities")
        );
        assert!(
            LoadedRestrictionGroups::load(
                &payload,
                &other,
                LocalizedStorage::KeyOnly,
                Precision::Double53
            )
            .unwrap_err()
            .contains("source PC/identities")
        );
    }
}
