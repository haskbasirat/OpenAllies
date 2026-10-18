use super::*;
use serde_json::{Value, json};

#[test]
fn native_refresh_gate_prefix_matches_all186_fixture_rows() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-actor-terrain-refresh/native-actor-terrain-refresh.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 186);
    let mut skipped = 0;
    for row in rows {
        let c = &row["input"];
        let entered = gates(c["flags"].as_u64().unwrap() as u32,
            c["state_low"].as_u64().unwrap() as u32,
            c["state_high"].as_u64().unwrap() as u32);
        let read_tile_or_element = row["output"]["events"].as_array().unwrap().iter()
            .any(|v| v == "0054ece0" || v == "00570560");
        assert_eq!(entered, read_tile_or_element, "{row}");
        if !entered {
            skipped += 1;
            assert_eq!(row["output"]["outcome"], "returned");
            assert_eq!(row["output"]["current_terrain"], c["old"]);
        }
    }
    assert_eq!(skipped, 21);
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_held_actor_uses_retained_terrain_and_preserves_f8_before_required_org() {
    use crate::{
        battle_terrain, combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_numeric::Binary;
    use aa_rules::{
        actors::ActorCatalog, resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256}, tgi::Limits,
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
    let map_bytes = std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP")).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(map_bytes.clone()).unwrap();
    let mut sources = V14Sources::open(game.join("Data.rwd"), &native,
        SourceLimits { max_member_bytes:64*1024*1024 }).unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive = ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    let mut attempt = Attempt::load_with_source_setup(&map, &actors, settings.clone(), text,
        Precision::Double53, u64::MAX, RestoreMode::InspectPartial,
        LoadBindings { native_load_mode:Some(1), inactive_load:Some(inactive), resources:Some(resources) },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify(catalog, &mut sources, TargetFiles {
                native_exe:&exe, renderer_dll:&dll, renderer_config:&config,
                capture_receipt:&receipt, file_manifest_receipt:&manifest })?;
            completion::install(catalog, &mut sources, proof.context(), proof.csv(), proof.globals())?;
            completion::compute(catalog)
        }).unwrap();
    let terrain_source = battle_terrain::Source::bind(&mut sources,&actors,&settings,Precision::Double53).unwrap();
    let terrain = BattleTerrain::load(&map,&terrain_source).unwrap();
    let catalog = Catalog::new(&actors);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let shift = terrain.bound.layout.coordinate_shift;
    // Diagnostic Load has not run Object Init. Seed only its cached cell from
    // the ACTUAL saved pose; this is an explicit test setup, not loader parity.
    let cell = |shell:&aa_actors::loading::LoadShell<aa_mapworld::bootstrap::DefinitionProof,RestoredComponent>| {
        [0,1].map(|i| Binary::from_f32_bits(shell.thing.position_bits[i]).unwrap().truncate_i32().unwrap() >> shift)
    };
    let eligible = |v:&aa_mapworld::restore::LoadedEntity| {
        let Some(a)=v.shell.actor.as_ref() else { return false };
        // This transition test explicitly controls the native gate bits and
        // retains the real allocation/components. It does not establish that
        // an unmodified saved actor reaches this positive branch.
        match a.components.get(ComponentSlot::Element) {
            None => true,
            Some(RestoredComponent::Element(e)) => !e.flag_1c || e.organization.is_none(),
            _ => false,
        }
    };
    let mut chosen = Vec::new();
    for need_org in [false,true] {
        let id = attempt.registry.iter().find_map(|entry| {
            let RestoredRecord::Actor(v)=&entry.value else { return None };
            if !eligible(v) { return None }
            let a=v.shell.actor.as_ref().unwrap();
            if a.components.get(ComponentSlot::Organization).is_some()!=need_org { return None }
            if !need_org && a.components.get(ComponentSlot::Properties).is_none() { return None }
            let [x,y]=cell(&v.shell);
            let definition=terrain.terrain_definition_at(x,y).ok()?;
            let registration=actors.native_data_registry("Terrain")?.entries.iter()
                .find(|e|e.ids.eq_ignore_ascii_case(&definition.id))?;
            let node=actors.native_data_node(registration).ok()?;
            (need_org || node.children("Property").ok()?.is_empty()).then_some(entry.id)
        }).unwrap_or_else(||panic!("actual actor/source terrain path required (organization={need_org})"));
        chosen.push((id,need_org));
    }
    let mut rows=vec![];
    for (id,need_org) in chosen {
        let mut held=std::mem::replace(attempt.registry.value_mut(id).unwrap(),RestoredRecord::Loading);
        let RestoredRecord::Actor(v)=&mut held else { unreachable!() };
        let [x,y]=cell(&v.shell);
        let pointer=v.shell.actor.as_ref().unwrap() as *const _ as usize;
        let original_f8=*v.shell.actor.as_ref().unwrap().words.get(&0xF8).unwrap();
        let original_flags=v.shell.thing.flags;
        let original_state=v.shell.actor.as_ref().unwrap().components.state;
        v.shell.thing.flags &= !((1<<29)|(1<<12));
        v.shell.actor.as_mut().unwrap().components.state.low &= !((1<<24)|(1<<25)|(1<<26));
        v.shell.actor.as_mut().unwrap().components.state.high &= !0x14;
        assert_eq!(original_f8,0,"native initialized saved F8");
        v.shell.object.as_mut().unwrap().word_44=x as u32;
        v.shell.object.as_mut().unwrap().word_48=y as u32;
        let mut record=Record { owner:id,shell:&mut v.shell,definition:&v.definition,registry:&mut attempt.registry };
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            record.shell.actor.as_mut().unwrap().words.insert(0xF8,original_f8);
            let result=refresh(&catalog,&mut record,&terrain,pc,&mut RequiredEffects);
            let f8=word(&record,0xF8).unwrap();
            assert_ne!(f8,0);
            let terrain_id=checked_terrain_id(&catalog,f8).unwrap();
            assert_eq!(terrain_id,terrain.terrain_definition_at(x,y).unwrap().id.to_ascii_lowercase());
            assert_eq!(record.shell.actor.as_ref().unwrap() as *const _ as usize,pointer);
            if need_org { assert!(result.unwrap_err().to_string().contains("579EB0")); }
            else {
                result.unwrap();
                assert!(properties(&catalog,f8,Precision::Double53).unwrap().is_empty());
                // Equality returns before any effect, retaining the same word.
                refresh(&catalog,&mut record,&terrain,pc,&mut RequiredEffects).unwrap();
            }
            rows.push(json!({"owner":id.raw(),"definition":record.definition.proof,"pc":pc,
                "cell":[x,y],"f8":f8,"terrain_id":terrain_id,
                "outcome":if need_org{"required_579EB0"}else{"complete_empty_terrain_properties"},"same_allocation":true}));
        }
        let before=word(&record,0xF8).unwrap();
        assert!(refresh(&Catalog::new(&actors),&mut record,&terrain,Precision::Double53,&mut RequiredEffects).is_err());
        assert_eq!(word(&record,0xF8).unwrap(),before);
        record.shell.thing.flags=original_flags;
        record.shell.actor.as_mut().unwrap().components.state=original_state;
        record.shell.actor.as_mut().unwrap().words.insert(0xF8,original_f8);
        *attempt.registry.value_mut(id).unwrap()=held;
    }
    let result=json!({"scope":"actual held Actors/source manager/BattleTerrain; explicit active-gate bits and cached-cell seed because diagnostic Load omits Object Init; no unmodified ActorInit or playable battle claim",
        "source_precision":Precision::Double53,"map_sha256":sha256(&map_bytes),"rows":rows});
    if let Some(path)=std::env::var_os("AA_TERRAIN_REFRESH_CAPTURE") {
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&result).unwrap();
    } else {
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261016-actor-terrain-refresh/actual-held-terrain.json");
        let expected:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(result,expected);
    }
}
