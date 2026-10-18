use aa_actors::{ActorId, components::ComponentSlot};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::json;
use std::sync::Arc;
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn tutorial02_actual_hq_load_advances_the_original_record() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive_load = ScopedManagedLoad::from_sources(&mut sources, 375)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempt=Attempt::load_with_source_setup(&map,&actors,settings,text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::StopOnError,LoadBindings{native_load_mode:None,inactive_load:Some(inactive_load),resources:Some(resources)},|catalog| {
  let reference=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
  aa_runtime::combat_source_completion::install(catalog,&mut sources,reference.context(),reference.csv(),reference.globals())?;
  aa_runtime::combat_source_completion::compute(catalog)
 }).unwrap();
    let entry = attempt.registry.get(ActorId::from_raw(4099)).unwrap();
    let entity = entry.value.loaded().unwrap();
    let actor = entity.shell.actor.as_ref().unwrap();
    let hq = match actor.components.get(ComponentSlot::Hq) {
        Some(RestoredComponent::Hq(v)) => v,
        other => panic!(
            "HQ not constructed: {other:?}; first {:?}",
            attempt.report.events.iter().find(|e| e.error.is_some())
        ),
    };
    let payload = entity.saved_components.iter().find(|p| p.slot == 30);
    let owner = ActorId::from_raw(*actor.words.get(&0xe8).unwrap());
    let aa_mapworld::restore::RestoredRecord::Kingdom(kingdom) =
        &attempt.registry.get(owner).unwrap().value
    else {
        panic!("actual HQ Kingdom owner")
    };
    let nation = kingdom
        .runtime
        .as_ref()
        .unwrap()
        .derived
        .as_ref()
        .unwrap()
        .references
        .get(&0x234)
        .unwrap()
        .as_ref()
        .unwrap();
    let badge_count = actors
        .native_nation_badges(&nation.id, Precision::Double53)
        .unwrap()
        .count()
        .unwrap();
    let report = json!({"definition":entity.definition.proof.id,"actor":4099,"owner_e8":actor.words.get(&0xe8),"source_slots_530":entity.definition.ground.hq_slots_530,"nation":nation.id,"nation_badge_count":badge_count,"updatable_table_bound":attempt.updatables.len(),"badge":hq.badge_index,"slots":hq.slots.iter().map(|s|s.map(|v|[v.kind,v.word_04,v.word_08])).collect::<Vec<_>>(),"pending":hq.pending.iter().map(|v|[v.kind,v.word_04,v.word_08]).collect::<Vec<_>>(),"payload":payload.map(|p|json!({"slot":p.slot,"source_span":p.source_span,"value":p.payload})),"first_failure":attempt.report.events.iter().find(|e|e.error.is_some()),"publication_ready":attempt.report.runtime_publication_ready,"scope":"Real original tutorial02 record load in managed diagnostic task with verified source CV; immediate world publication not certified"});
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Some(out) = std::env::var_os("AA_HQ_REPORT") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    }
    assert!(payload.is_some(), "HQ Load did not complete");
    assert!(
        attempt.report.events.iter().all(|e| e.error.is_none()),
        "original tutorial02 record pass advanced through HQ but found a later boundary"
    );
}
