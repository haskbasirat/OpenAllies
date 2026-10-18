use super::*;
use aa_body::F32;
use serde_json::{Value, json};
struct Probe {
    specs: Vec<Value>,
    enabled: Vec<bool>,
    writes: Vec<Value>,
    notified: bool,
}
impl RankWorld for Probe {
    fn ability_local_definition(&self, h: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self.specs[h as usize]["local"]
            .as_bool()
            .unwrap()
            .then_some(h))
    }
    fn rank_limits(&self, h: u32) -> aa_combat::Result<[i32; 2]> {
        let s = &self.specs[h as usize];
        Ok([
            s["minimum"].as_i64().unwrap() as i32,
            s["maximum"].as_i64().unwrap() as i32,
        ])
    }
    fn ability_enabled(&self, h: u32) -> aa_combat::Result<bool> {
        Ok(self.enabled[h as usize])
    }
    fn set_ability_enabled(&mut self, h: u32, v: bool) -> aa_combat::Result<()> {
        self.enabled[h as usize] = v;
        self.writes.push(json!([h, "enabled", u32::from(v)]));
        Ok(())
    }
    fn notify(&mut self, owner: u32) -> aa_combat::Result<()> {
        assert_eq!(owner, 91);
        self.notified = true;
        Err(combat("4D1350 required boundary"))
    }
}
fn probe(specs: Vec<Value>) -> Probe {
    Probe {
        enabled: specs
            .iter()
            .map(|s| s["enabled"].as_bool().unwrap())
            .collect(),
        specs,
        writes: vec![],
        notified: false,
    }
}
fn state(count: usize) -> performer::Performer {
    performer::Performer {
        owner: 91,
        stage: 4,
        abilities: (0..count as u32).collect(),
        mana: F32::ZERO,
        target_id: 0,
        target_position: [F32::ZERO; 2],
        active_ability: None,
        active_index: -1,
        started_at: F32::ZERO,
        animation_length: F32::ZERO,
        event_kind: 0,
        animation: None,
        turret_index: -1,
    }
}
#[test]
fn actual_init_native_enabled_prefix_and_required_tail_match_all_rows() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-performer-init/native-performer-init.json"
    ))
    .unwrap();
    let mut checked = 0;
    let mut changed = 0;
    for row in fixture["cases"].as_array().unwrap() {
        if row["input"]["kind"] == "after" {
            continue;
        }
        let specs = row["input"]["abilities"].as_array().unwrap().clone();
        let state = state(specs.len());
        let mut prefix = probe(specs.clone());
        let need = performer::gate_by_rank_prefix(&state, None, &mut prefix).unwrap();
        assert!(!prefix.notified);
        assert_eq!(need, row["output"]["outcome"] == "required");
        assert_eq!(
            json!(
                prefix
                    .enabled
                    .iter()
                    .map(|b| u32::from(*b))
                    .collect::<Vec<_>>()
            ),
            row["output"]["enabled"]
        );
        assert_eq!(json!(prefix.writes), row["output"]["writes"]);
        let mut full = probe(specs);
        let result = performer::gate_by_rank(&state, None, &mut full);
        assert_eq!(result.is_err(), need);
        assert_eq!(full.notified, need);
        assert_eq!(full.enabled, prefix.enabled);
        assert_eq!(full.writes, prefix.writes);
        checked += 1;
        changed += usize::from(need);
    }
    assert_eq!(checked, 141);
    assert!(changed > 0);
}
#[test]
#[ignore = "requires AA_GAME_ROOT actual tutorial02 sources and verified native source receipt"]
fn actual_tutorial4097_performer_init_keeps_same_held_component_and_ability_allocations() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::{RestoreMode, RestoredRecord};
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256},
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
    let bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
    )
    .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources.clone()),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
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
    catalog.settings = Some(settings);
    catalog.native_load_mode = Some(1);
    catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    *catalog.ability_arena.borrow_mut() = attempt.abilities;
    let id = aa_actors::ActorId::from_raw(4097);
    let mut held = std::mem::replace(
        attempt.registry.value_mut(id).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(entity) = &mut held else {
        panic!("actual tutorial4097 Actor")
    };
    assert!(entity.failure.is_none(), "{:?}", entity.failure);
    let mut record = Record {
        owner: id,
        shell: &mut entity.shell,
        definition: &entity.definition,
        registry: &mut attempt.registry,
    };
    let source_pc = validate(&catalog, &record).unwrap();
    let p = performer(&record).unwrap();
    let p_addr = std::ptr::from_ref(p) as usize;
    let shell_addr = std::ptr::from_ref(&*record.shell) as usize;
    let handles = p.state.abilities.clone();
    let inspect = || {
        let arena = catalog.ability_arena.borrow();
        handles
            .iter()
            .map(|h| {
                let a = arena.get(*h).unwrap();
                (
                    *h,
                    std::ptr::from_ref(a) as usize,
                    a.local_definition,
                    a.enabled,
                )
            })
            .collect::<Vec<_>>()
    };
    let initial = inspect();
    let inputs=initial.iter().map(|(h,_,local,b)|json!({"handle":h,"local":local,"enabled":b,"limits":local.map(|l|rank_limits(&catalog,l,source_pc).unwrap())})).collect::<Vec<_>>();
    let mut rows = vec![];
    for live_pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let before = inspect();
        let result = init_existing(&catalog, &attempt.inventory, &mut record, live_pc);
        let after = inspect();
        assert_eq!(performer(&record).unwrap() as *const _ as usize, p_addr);
        assert_eq!(std::ptr::from_ref(&*record.shell) as usize, shell_addr);
        assert_eq!(
            before.iter().map(|x| (x.0, x.1, x.2)).collect::<Vec<_>>(),
            after.iter().map(|x| (x.0, x.1, x.2)).collect::<Vec<_>>()
        );
        let mut expected = before.iter().map(|x| x.3).collect::<Vec<_>>();
        for (i, (_, _, local, _)) in before.iter().enumerate() {
            if let Some(local) = local {
                if let Some(v) =
                    performer::rank_allows(rank_limits(&catalog, *local, source_pc).unwrap(), 0)
                {
                    expected[i] = v
                }
            }
        }
        assert_eq!(after.iter().map(|x| x.3).collect::<Vec<_>>(), expected);
        assert!(result.as_ref().is_ok_and(|v| *v), "{result:?}");
        after_init(&catalog, &mut record).unwrap();
        rows.push(json!({"live_precision":live_pc,"input":inputs,"enabled_after":expected,"result":result.map_err(|e|e.to_string()),"same_allocations":true}));
        {
            let mut arena = catalog.ability_arena.borrow_mut();
            for (h, _, _, b) in &initial {
                arena.get_mut(*h).unwrap().enabled = *b;
            }
        }
    }
    let saved_owner = record.owner;
    record.owner = aa_actors::ActorId::from_raw(0xffff0001);
    assert!(init_existing(&catalog, &attempt.inventory, &mut record, source_pc).is_err());
    record.owner = saved_owner;
    assert_eq!(inspect(), initial);
    let definition = record.definition.proof.id.clone();
    drop(record);
    *attempt.registry.value_mut(id).unwrap() = held;
    let out = json!({"scope":"Actual diagnostic tutorial02 GTID4097 same held Performer/Ability allocations and completed source Bank. NULL rank input is the actual57E000 literal; all3 livePC sourcePC remains fixed. This focused leaf test is not full ActorInit/world/battle readiness.","map_sha256":sha256(&bytes),"owner":id.raw(),"definition":definition,"source_precision":source_pc,"ability_count":handles.len(),"rows":rows,"wrong_owner_refused_before_mutation":true});
    if let Some(path) = std::env::var_os("AA_PERFORMER_INIT_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &out,
        )
        .unwrap();
    } else {
        let expected: Value =
            serde_json::from_slice(
                &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                    "../../research/skirmish-20261017-performer-init/actual-tutorial4097.json",
                ))
                .unwrap(),
            )
            .unwrap();
        assert_eq!(out, expected);
    }
}
