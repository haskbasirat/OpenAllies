use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-load-detection-init/native-detection-activation.json"
    ))
    .unwrap()
}
fn precision(s: &str) -> Precision {
    match s {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
fn geometry(f: &Value) -> aa_zones::tile::MapGeometry {
    serde_json::from_value(f["geometry"].clone()).unwrap()
}
fn zone(z: &Zone) -> Value {
    json!({"radius_bits":z.radius_bits,"x":z.x as u32,"y":z.y as u32,"flags":z.flags,"state":z.state})
}

#[test]
fn actual_inputs_and_labelled_storage_probes_match54_native_activation_prefixes() {
    let f = fixture();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 54);
    for row in rows {
        let c = &row["input"];
        let n = &row["native"];
        let owner = ActorId::from_raw(c["owner"].as_u64().unwrap() as u32);
        let mut shell = aa_actors::loading::construct_shell_with_components::<(), RestoredComponent>(
            aa_actors::loading::LoadKind::Actor,
            (),
        );
        let actor = shell.actor.as_mut().unwrap();
        let mut ids = Identities::new();
        if c["existing"].as_bool().unwrap() {
            let key = ids.allocate(owner, 6).unwrap();
            let mut z = Zone::new(key, owner.raw(), 0x40);
            z.radius_bits = c["existing_radius"].as_u64().unwrap() as u32;
            z.x = c["existing_xy"][0].as_i64().unwrap() as i32;
            z.y = c["existing_xy"][1].as_i64().unwrap() as i32;
            actor.zones_148.zones_mut().slots = vec![None; 9];
            actor.zones_148.zones_mut().slots[6] = Some(z);
        }
        let mut map = ZoneMap::new(geometry(&f)).unwrap();
        let flags = c["flags"].as_u64().unwrap() as u32;
        let radius = c["radius"].as_u64().unwrap() as u32;
        let position: [u32; 3] = serde_json::from_value(c["position_bits"].clone()).unwrap();
        let pc = precision(row["precision"].as_str().unwrap());
        let enabled =
            flags & 1 == 0 && fpu::less(&fpu::load(0).unwrap(), &fpu::load(radius).unwrap());
        let result = zone_change(
            owner,
            actor,
            &flags,
            position,
            radius,
            enabled,
            &mut map,
            &mut ids,
            pc,
            &aa_actors::IdentityRegistry::new(),
            &mut RequiredEffects,
        );
        let actual = actor
            .zones_148
            .zones()
            .slots
            .get(6)
            .and_then(Option::as_ref)
            .map(zone)
            .unwrap_or(Value::Null);
        assert_eq!(actual, n["zone6"], "{row}");
        assert_eq!(json!(actor.zones_148.len()), n["vector_size"]);
        assert_eq!(
            json!(actor.zones_148.native_capacity()),
            n["vector_capacity"]
        );
        match n["outcome"].as_str().unwrap() {
            "complete" => assert!(result.is_ok(), "{row}: {result:?}"),
            "required0x515a20" => assert!(result.unwrap_err().to_string().contains("515A1A")),
            "required0x514ce0" => assert!(result.unwrap_err().to_string().contains("514CE0")),
            v => panic!("unexpectednativeoutcome{v}"),
        }
        if let Some(tile) = n["stop"]["tile"].as_array() {
            let clamped =
                aa_zones::zone::clamp_position(&map, [position[0], position[1]], pc).unwrap();
            let xy = clamped.map(|bits| {
                fpu::ftol(&fpu::load(bits).unwrap()).unwrap() >> (map.geometry.shift & 31)
            });
            assert_eq!(json!(xy), json!(tile));
        }
        assert!(
            map.tiles
                .iter()
                .all(|t| t.set.is_none() && t.fog == 0 && t.supply == 0)
        );
        let expected_count = usize::from(!n["zone6"].is_null());
        assert_eq!(ids.len(), expected_count);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_two_detection_owners_keep_same_allocations_and_error_prefix_at_all_live_pc() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
    )
    .unwrap();
    assert_eq!(
        sha256(&bytes),
        "525ed33f7627294e9a83bdbabb557db2844badf0267e4312da7807089750823d"
    );
    let map = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        &native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify(
                catalog,
                &mut sources,
                TargetFiles {
                    native_exe: &exe,
                    renderer_dll: &dll,
                    renderer_config: &config,
                    capture_receipt: &receipt,
                    file_manifest_receipt: &manifest,
                },
            )?;
            completion::install(
                catalog,
                &mut sources,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )?;
            completion::compute(catalog)
        },
    )
    .unwrap();
    assert_eq!(attempt.report.world_version, 27);
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(attempt.settings.clone());
    catalog.native_load_mode = attempt.native_load_mode;
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    if let Some(inactive) = attempt.inactive_load.as_ref() {
        catalog.bind_inactive_load(inactive).unwrap();
    }
    let terrain_source = crate::battle_terrain::Source::bind(
        &mut sources,
        &actors,
        &attempt.settings,
        Precision::Double53,
    )
    .unwrap();
    let terrain = crate::battle_terrain::BattleTerrain::load(&map, &terrain_source).unwrap();
    let spatial =
        crate::load_spatial::LoadSpatial::new(&terrain.bound, Precision::Double53).unwrap();
    let mut zones = crate::load_zone_services::before_first_record(
        &terrain.bound,
        spatial.index(),
        attempt.inventory.header.as_ref().unwrap(),
    )
    .unwrap();
    assert_eq!(zones.geometry, geometry(&fixture()));
    let mut identities = Identities::new();
    let mut outputs = vec![];
    for raw in [2696, 6477] {
        let owner = ActorId::from_raw(raw);
        let mut held = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(entity) = &mut held else {
            panic!("actualownerisnotActor")
        };
        let before_pointer = match entity
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Detection)
            .unwrap()
        {
            RestoredComponent::Detection(d) => d.as_ref() as *const _ as usize,
            _ => unreachable!(),
        };
        let expected = fixture()["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["label"] == format!("actual{raw}_fresh"))
            .unwrap()
            .clone();
        let before_flags = entity.shell.thing.flags;
        let before_position = entity.shell.thing.position_bits;
        let before_actor = entity.shell.actor.as_ref().unwrap().components.state;
        // This source-only stopped attempt precedes Object Init's spatial
        // bit26 store.56EC90/515970 read only deathbit0 on this branch.
        assert_eq!(before_flags&1,expected["input"]["flags"].as_u64().unwrap() as u32&1);
        assert_eq!(before_actor.low&0x20_0000,expected["input"]["state_low"].as_u64().unwrap() as u32&0x20_0000);
        assert_eq!(
            &before_position[..2],
            &serde_json::from_value::<[u32; 3]>(expected["input"]["position_bits"].clone())
                .unwrap()[..2]
        );
        let mut record = Record {
            owner,
            shell: &mut entity.shell,
            definition: &entity.definition,
            registry: &mut attempt.registry,
        };
        let mut vector_pointer = None;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            assert!(
                crate::load_detection_init_world::init_existing(
                    &catalog,
                    &mut record,
                    ComponentSlot::Detection,
                    &mut attempt.rng,
                    pc,
                    || Ok(attempt.report.world_version)
                )
                .unwrap()
            );
            let result = after_init(
                &catalog,
                &mut record,
                &mut zones,
                &mut identities,
                pc,
                &mut RequiredEffects,
            )
            .unwrap_err()
            .to_string();
            assert!(result.contains("515A1A"), "{result}");
            let actor = record.shell.actor.as_ref().unwrap();
            let RestoredComponent::Detection(d) =
                actor.components.get(ComponentSlot::Detection).unwrap()
            else {
                unreachable!()
            };
            assert_eq!(d.as_ref() as *const _ as usize, before_pointer);
            assert_eq!(
                d.radius.0,
                expected["input"]["radius"].as_u64().unwrap() as u32
            );
            let z = actor.zones_148.zones().slots[6].as_ref().unwrap();
            assert_eq!(zone(z), expected["native"]["zone6"]);
            assert_eq!(identities.allocation(z.key), Some((owner, 6)));
            let pointer = actor.zones_148.zones().slots.as_ptr() as usize;
            if let Some(before) = vector_pointer {
                assert_eq!(pointer, before);
            } else {
                vector_pointer = Some(pointer);
            }
            assert_eq!(record.shell.thing.flags, before_flags);
            assert_eq!(record.shell.thing.position_bits, before_position);
            assert_eq!(actor.components.state, before_actor);
            outputs.push(json!({"owner":raw,"precision":pc,"source_precision":Precision::Double53,"actual_before_flags":before_flags,"actual_before_state":before_actor,"actual_before_position_bits":before_position,"radius":d.radius.0,"zone":z,"required_error":result}));
        }
        *attempt.registry.value_mut(owner).unwrap() = held;
    }
    assert_eq!(identities.len(), 2);
    assert!(zones.tiles.iter().all(|t| t.set.is_none() && t.fog == 0));
    if let Some(path) = std::env::var_os("AA_DETECTION_ACTIVATION_CAPTURE") {
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&json!({"scope":"Actual source-owned actors before Init are semantically equal on every56EC90 reached guard/radius/XY input to the separately observed exact boundary; same component and zone allocation is retained across livePC calls. Required World+248 remains unbound; no full Init/solver/fog success.","rows":outputs})).unwrap();
    }
}
