//! Actual assets, real Session, verified source completion and persistent
//! immediate lifecycle. This is deliberately a failing integration gate until
//! the native load can complete; it must not be mistaken for battle acceptance.
use aa_mapworld::restore::RestoreMode;
use aa_netcode::lobby::Session;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,
    sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{localization::Localization,restoration::{Attempt,LoadBindings},
    restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings};
use std::sync::Arc;
#[cfg(windows)]
#[path="../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
#[path="../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;

#[test]
#[ignore="requires AA_GAME_ROOT original/v14 assets"]
#[cfg(windows)]
fn actual_saved_map_completes_immediate_native_lifecycle() {
    use aa_runtime::execution_context::{ApplicationThread,ResourceWorker};
    let application=Arc::new(ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap());
    let worker=ResourceWorker::start(application,windows_reset_os::WindowsThreadIds,
        |identity| {
            if identity.parent_os_thread_id()==identity.child_os_thread_id() {
                return Err("resource worker did not create a separate OS thread".into());
            }
            Ok(())
        },|execution| Ok(run_actual_saved_map(execution))).unwrap();
    let staged=worker.join().unwrap();
    //Typed owned transfer of the SAME loaded allocation graph to its parent,
    //not a reconstructed map or a simulation/activation authorization.
    assert!(staged.load_world.as_ref().unwrap().live_spatial().is_ok());
    assert!(!staged.report.runtime_publication_ready);
}
#[cfg(windows)]
fn run_actual_saved_map(execution:&mut aa_runtime::execution_context::LoadExecution<'_>)->aa_runtime::restoration::Staged {
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {max_member_bytes:64*1024*1024}).unwrap();
    let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();
    let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();
    let resources=Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let name=std::env::var("AA_IMMEDIATE_MAP").unwrap_or_else(|_|"Campaign/tutorial02".into());
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join(format!(
        "reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
    let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    // This actual Session instance supplies only its observed startup E4/F8
    // fields read by this bounded test. Default's other fields do NOT claim
    // the full native Session constructor or local skirmish bootstrap.
    let session=Session::default();
    // Optional census preserves every partial prefix to identify independent
    // missing callbacks. It can NEVER satisfy this strict acceptance gate.
    let inspect=std::env::var_os("AA_IMMEDIATE_INSPECT_PARTIAL").is_some();
    let mode=if inspect {RestoreMode::InspectPartial} else {RestoreMode::StopOnError};
    let streams=aa_runtime::map_resource_streams::Owner::open(map);
    let attempt=Attempt::load_immediate_owned_runtime(streams,&actors,settings,text,Precision::Double53,
        u64::MAX,mode,LoadBindings { native_load_mode:Some(1),
            inactive_load:Some(inactive),resources:Some(resources)},&mut sources,&session,false,
            Some(execution),Some(runtime_fp::sample),None).unwrap();
    assert!(attempt.load_world.is_some());
    assert!(!attempt.resource_streams.as_ref().unwrap().is_closed());
    assert!(attempt.report.lifecycle.iter().all(|e|!e.omitted));
    let result=serde_json::json!({"map":name,"session_f8":session.field_f8,"session_e4":session.state,
        "diagnostic_partial_census":inspect,
        "scope":"actual immediate record lifecycle; no world-start/battle claim", "report":attempt.report});
    if let Some(path)=std::env::var_os("AA_IMMEDIATE_REPORT") {
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&result).unwrap();
    }
    assert!(!inspect,"partial census cannot satisfy immediate lifecycle acceptance");
    let summary=serde_json::json!({"map":name,
        "completed":attempt.report.lifecycle.iter().filter(|e|e.completed).count(),
        "first_event_error":attempt.report.events.iter().find(|e|e.error.is_some()),
        "first_lifecycle_error":attempt.report.lifecycle.iter().find(|e|!e.completed)});
    assert!(attempt.report.events.iter().all(|e|e.error.is_none()),"{}",summary);
    assert!(attempt.report.lifecycle.iter().all(|e|e.completed),"{}",summary);
    let mut staged=attempt.into_staged().unwrap();
    let post=staged.post_load(&actors,Precision::Double53);
    let phase=serde_json::json!({"scope":"actual PostLoad; not battle acceptance",
        "error":post.as_ref().err().map(ToString::to_string),
        "report":post.as_ref().ok(),"clock":staged.clock,"rng":staged.rng.state,
        "saved_queue_load":staged.saved_queue_load,
        "runtime_publication_ready":staged.report.runtime_publication_ready});
    if let Some(path)=std::env::var_os("AA_IMMEDIATE_REPORT") {
        let path=std::path::PathBuf::from(path).with_extension("post-load.json");
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&phase).unwrap();
    }
    post.unwrap();
    let world=staged.load_world.as_mut().unwrap();
    world.adopt_spatial(&mut staged.actors).unwrap();
    assert!(world.live_spatial().is_ok());
    // Successful ownership transfer alone cannot authorize simulation.
    assert!(!staged.report.runtime_publication_ready);
    staged
}
