use super::*;
#[test]
fn constructor_class_and_flare_owners_match_native_region() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-owner-add/native-owner-add.json"
    ))
    .unwrap();
    for row in f["constructors"].as_array().unwrap() {
        let k = SpecialShell::kingdom(ActorId::from_raw(781), 0).unwrap();
        let lists = k.actor_lists.as_ref().unwrap();
        assert_eq!(
            lists.by_class.len(),
            row["output"]["class_lists"].as_u64().unwrap() as usize
        );
        assert!(lists.by_class.iter().all(Vec::is_empty));
        assert!(lists.flares.is_empty());
        assert!(lists.city_ownership.is_empty());
        let projection = k.scalar_projection();
        for offset in (0x26c..0x2c4).step_by(4) {
            assert!(!k.words.contains_key(&offset));
            assert_eq!(projection[&offset], 0);
        }
        for offset in [0x2f8, 0x2fc, 0x30c, 0x310] {
            assert!(!k.words.contains_key(&offset));
            assert_eq!(projection[&offset], 0);
        }
        for offset in [0x2c4, 0x2d0, 0x2dc, 0x2e8] {
            let v = &k.vectors[&offset];
            assert!(v.values.is_empty());
            assert_eq!(
                v.capacity_hint,
                row["output"]["vectors"][format!("{offset:#x}")]["capacity"]
                    .as_u64()
                    .unwrap() as u32
            );
        }
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_member_class_head_and_flag_prefix_matches_native_required_economy_stop() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use std::sync::Arc;
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
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-owner-add/native-actual-owner.json");
    let reference: Option<serde_json::Value> = if fixture_path.exists() {
        Some(serde_json::from_slice(&std::fs::read(&fixture_path).unwrap()).unwrap())
    } else {
        assert!(
            std::env::var_os("AA_OWNER_ADD_CAPTURE").is_some(),
            "native actual owner fixture required"
        );
        None
    };
    let mut captures = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources)),
            },
            |catalog| {
                let proof =
                    VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
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
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(attempt.settings.clone());
        catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
        //This isolated InspectPartial fixture has no constructed World. Bind
        //one canonical source cache for its held-record adapter; production
        //PostCreate still borrows the actual live World's disjoint field.
        let source_pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
        let source_fields=catalog.actors.native_source_requirements(source_pc).unwrap();
        let id = attempt
            .registry
            .iter()
            .find_map(|entry| {
                let RestoredRecord::Actor(a) = &entry.value else {
                    return None;
                };
                let actor = a.shell.actor.as_ref()?;
                let owner = ActorId::from_raw(*actor.words.get(&0xe8)?);
                (a.definition.native_class == 3
                    && actor.components.present(ComponentSlot::Economy)
                    && matches!(
                        attempt.registry.get(owner).map(|e| &e.value),
                        Some(RestoredRecord::Kingdom(_))
                    ))
                .then_some(entry.id)
            })
            .expect("actual saved Actor with current owner and Economy");
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(a) = &mut held else {
            unreachable!()
        };
        let actor = a.shell.actor.as_ref().unwrap();
        let flags = a.shell.thing.flags;
        let owner = ActorId::from_raw(actor.words[&0xe8]);
        let shell_address = &a.shell as *const _ as usize;
        let mut input = serde_json::json!({"actor_id":id.raw(),"flags":flags,"source_flags":a.definition.flags,"states":u64::from(actor.components.state.low)|(u64::from(actor.components.state.high)<<32),"kind":a.definition.proof.category.registry_index,"economy":true,"structure":actor.components.present(ComponentSlot::Structure),"settlement":actor.components.present(ComponentSlot::Settlement),"hq":actor.components.present(ComponentSlot::Hq),"organization":actor.components.present(ComponentSlot::Organization),"supply":actor.components.present(ComponentSlot::Supply),"properties":actor.components.present(ComponentSlot::Properties),"mobile":actor.components.present(ComponentSlot::MobileDeployment)});
        let mut record = Record {
            owner: id,
            shell: &mut a.shell,
            definition: &a.definition,
            registry: &mut attempt.registry,
        };
        // Explicit corrupted-owner metadata probe. The live constructor
        // containers stay untouched when immutable source binding is refused.
        let owner_flags = {
            let Some(RestoredRecord::Kingdom(k)) = record.registry.value_mut(owner) else {
                unreachable!()
            };
            let r = k.runtime.as_mut().unwrap();
            let flags = r.definition.flags;
            r.definition.flags ^= 1;
            flags
        };
        let rejected = add_actor_bound(
            &catalog,
            &attempt.inventory,
            &source_fields,
            &mut record,
            owner,
            pc,
            EconomyBinding::RequiredStop,
        );
        {
            let Some(RestoredRecord::Kingdom(k)) = record.registry.value_mut(owner) else {
                unreachable!()
            };
            let r = k.runtime.as_mut().unwrap();
            r.definition.flags = owner_flags;
            assert!(
                r.derived
                    .as_ref()
                    .unwrap()
                    .actor_lists
                    .as_ref()
                    .unwrap()
                    .by_class
                    .iter()
                    .all(Vec::is_empty)
            );
        }
        assert!(
            rejected
                .unwrap_err()
                .contains("canonical definition/source differs")
        );
        assert_eq!(record.shell.thing.flags, flags);
        let before = {
            let w = World {
                catalog: &catalog,
                inventory: &attempt.inventory,
                source_fields: &source_fields,
                record: &mut record,
                owner,
                pc,
                economy: EconomyBinding::Native,
            };
            let k = w.kingdom().unwrap();
            input["kingdom_supply"] = serde_json::json!(
                k.supply_sources
                    .iter()
                    .map(|id| {
                        assert!(
                            w.record.registry.get(*id).is_some(),
                            "actual supply source generation"
                        );
                        id.raw()
                    })
                    .collect::<Vec<_>>()
            );
            assert_eq!(w.capital().unwrap(), None);
            assert!(
                k.actor_lists
                    .as_ref()
                    .unwrap()
                    .by_class
                    .iter()
                    .all(Vec::is_empty)
            );
            [0x2c4, 0x2d0, 0x2dc, 0x2e8].map(|offset| {
                let v = &k.vectors[&offset];
                assert!(v.values.is_empty());
                v.capacity_hint
            })
        };
        let failure = add_actor_bound(
            &catalog,
            &attempt.inventory,
            &source_fields,
            &mut record,
            owner,
            pc,
            EconomyBinding::RequiredStop,
        )
        .unwrap_err();
        assert!(failure.contains("5704C0/570020"), "{failure}");
        assert_eq!(record.shell as *const _ as usize, shell_address);
        assert_eq!(record.shell.thing.flags, flags | native::FLAG_REGISTERED);
        let w = World {
            catalog: &catalog,
            inventory: &attempt.inventory,
            source_fields: &source_fields,
            record: &mut record,
            owner,
            pc,
            economy: EconomyBinding::Native,
        };
        let k = w.kingdom().unwrap();
        let class3 = k.actor_lists.as_ref().unwrap().by_class[3]
            .iter()
            .map(|i| i.raw())
            .collect::<Vec<_>>();
        assert_eq!(class3, vec![id.raw()]);
        let vectors:serde_json::Map<String,serde_json::Value>=[0x2c4,0x2d0,0x2dc,0x2e8].into_iter().zip(before).map(|(offset,capacity)|{let v=&k.vectors[&offset];assert!(v.values.is_empty());assert_eq!(v.capacity_hint,capacity);(format!("{offset:#x}"),serde_json::json!({"values":v.values,"count":v.values.len(),"capacity":v.capacity_hint}))}).collect();
        let output = serde_json::json!({"result":"required_service","service":"economy","class3":class3,"flags":w.record.shell.thing.flags,"counted":k.words[&0x2f4],"supply":k.supply_sources.iter().map(|v|v.raw()).collect::<Vec<_>>(),"capital":w.capital().unwrap(),"vectors":vectors});
        let capture = serde_json::json!({"precision":format!("{pc:?}"),"input":input,"output":output,"owner":owner.raw(),"definition":w.record.definition.proof.id,"same_held_allocation":true,"required_error":failure});
        if let Some(reference) = &reference {
            let row = reference["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["precision"] == format!("{pc:?}") && v["image"] == "original")
                .unwrap();
            assert_eq!(capture["input"], row["input"]);
            assert_eq!(capture["output"], row["output"]);
        }
        captures.push(capture);
    }
    if let Some(path) = std::env::var_os("AA_OWNER_ADD_CAPTURE") {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(f,&serde_json::json!({"scope":"actual source-completed held Actor/storedKingdom constructor prefix; immediateInit omitted; explicit reached Economy test boundary; body proved separately; no battle readiness claim","cases":captures})).unwrap();
    }
}
