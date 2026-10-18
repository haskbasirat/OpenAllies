use super::Catalog;
use crate::restore_lifecycle::{ScopedManagedLoad, TaskKind};
use aa_rules::{actors::ActorCatalog, sources::{Origin, ResolvedFile, sha256}, tgi::Limits};

fn token() -> crate::restore_lifecycle::InactiveLoad {
    let bytes = b"[Vars]\n{ string GoalPriorityCSV = null\n}\n".to_vec();
    ScopedManagedLoad::from_uvars(&ResolvedFile {
        logical_path: "UVars.tgi".into(), origin: Origin::V14Loose,
        sha256: sha256(&bytes), bytes,
    }, 0).unwrap().begin(TaskKind::Normal).unwrap()
}

#[test]
fn source_scope_transfer_preserves_bank_cache_and_identity_counters() {
    let actors=ActorCatalog::from_files(Vec::new(),Limits::default());
    let mut source=Catalog::new(&actors);
    let token=source.definition_bank.borrow_mut().intern("scope-test","existing").unwrap();
    source.component_masks.borrow_mut().insert(("scope-test".into(),53),Err("same failed producer".into()));
    source.mind_identities.set((41,92));source.command_identity.set(17);
    let mut actual=match source.into_runtime_scope(){Ok(value)=>value,Err((error,_))=>panic!("{error}")};
    assert!(std::ptr::eq(actual.actors,&actors));
    assert_eq!(actual.definition_bank.borrow().token("scope-test","existing"),Some(token));
    assert_eq!(actual.definition_bank.borrow_mut().intern("scope-test","next").unwrap(),token+1);
    assert_eq!(actual.component_masks.borrow()[&("scope-test".into(),53)],Err("same failed producer".into()));
    assert_eq!(actual.mind_identities.get(),(41,92));assert_eq!(actual.command_identity.get(),17);
    //A live clock cannot be silently discarded to manufacture a fresh scope.
    let clock=aa_simulation::NativeClock {tick:7,begin:aa_simulation::FloatBits(0),
        end:aa_simulation::FloatBits(1),step:aa_simulation::FloatBits(1)};
    actual.load_clock=Some(&clock);
    let (error,mut retained)=match actual.into_runtime_scope(){Err(value)=>value,Ok(_)=>panic!("active scope accepted")};
    assert!(error.contains("runtime load"));
    assert_eq!(retained.load_clock.unwrap().tick,7);
    retained.load_clock=None;
    assert!(retained.into_runtime_scope().is_ok());
}

#[test]
fn managed_observer_rejects_changed_mode_and_source_owner() {
    // Constructor fixture uses the same typed empty CSV declaration as the
    // original/v14 constructor oracle. This does not activate a live world.
    let actors = ActorCatalog::from_files(Vec::new(), Limits::default());
    let other = actors.clone();
    let load = token();
    let mut catalog = Catalog::new(&actors);
    assert!(catalog.saved_observer_present().unwrap_err().contains("not bound"));
    catalog.bind_inactive_load(&load).unwrap();
    assert!(catalog.saved_observer_present().unwrap());
    catalog.notify_saved_observer(0xdeadbeef).unwrap();
    assert!(catalog.bind_inactive_load(&load).is_err());
    catalog.native_load_mode = Some(2);
    assert!(catalog.saved_observer_present().unwrap_err().contains("modes disagree"));
    assert!(catalog.notify_saved_observer(0).unwrap_err().contains("modes disagree"));
    catalog.native_load_mode = None;
    assert!(catalog.saved_observer_present().is_err());
    catalog.native_load_mode = Some(1);
    assert!(catalog.saved_observer_present().unwrap());
    catalog.actors = &other;
    assert!(catalog.saved_observer_present().unwrap_err().contains("source owner changed"));
    assert!(catalog.notify_saved_observer(0).is_err());
    assert!(catalog.layout_data(aa_numeric::Precision::Double53).unwrap_err()
        .contains("source owner changed"));
    assert!(catalog.require_untranslated_actor_identity("same_id").unwrap_err()
        .contains("source owner changed"));
}

#[test]
fn source_transfer_crosses_os_thread_without_runtime_loans_or_rebuilding_bank() {
    let actors=std::sync::Arc::new(ActorCatalog::from_files(Vec::new(),Limits::default()));
    let catalog=Catalog::new(&actors);
    let token=catalog.definition_bank.borrow_mut().intern("handoff","first").unwrap();
    catalog.component_masks.borrow_mut().insert(("handoff".into(),53),Err("retained producer failure".into()));
    catalog.mind_identities.set((27,41));catalog.command_identity.set(83);
    let cache_buffer=catalog.component_masks.borrow()[&("handoff".into(),53)].as_ref().unwrap_err().as_ptr() as usize;
    let transfer=match catalog.into_source_transfer(std::sync::Arc::clone(&actors)) {
        Ok(v)=>v,Err((error,_))=>panic!("{error}"),
    };
    let returned=std::thread::spawn(move|| {
        let (actors,data)=transfer.into_parts();
        let foreign=ActorCatalog::from_files(Vec::new(),Limits::default());
        let (error,data)=match data.bind(&foreign) {Err(v)=>v,Ok(_)=>panic!("foreign source accepted")};
        assert!(error.contains("SAME"));
        let catalog=match data.bind(&actors) {Ok(v)=>v,Err((error,_))=>panic!("{error}")};
        assert_eq!(catalog.definition_bank.borrow().token("handoff","first"),Some(token));
        assert_eq!(catalog.definition_bank.borrow_mut().intern("handoff","second").unwrap(),token+1);
        assert_eq!(catalog.mind_identities.get(),(27,41));assert_eq!(catalog.command_identity.get(),83);
        assert_eq!(catalog.component_masks.borrow()[&("handoff".into(),53)].as_ref().unwrap_err().as_ptr() as usize,cache_buffer);
        match catalog.into_source_transfer(std::sync::Arc::clone(&actors)) {Ok(v)=>v,Err((error,_))=>panic!("{error}")}
    }).join().unwrap();
    let (actual,data)=returned.into_parts();assert!(std::sync::Arc::ptr_eq(&actual,&actors));
    let catalog=match data.bind(&actual) {Ok(v)=>v,Err((error,_))=>panic!("{error}")};
    assert_eq!(catalog.definition_bank.borrow().token("handoff","second"),Some(token+1));
}
