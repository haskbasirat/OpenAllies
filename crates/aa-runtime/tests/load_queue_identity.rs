//! Portable scheduler address substitution using an actual restored allocation.
//! The saved-ID hole below is an explicit transport probe, not a native battle.
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore_components::UpdatableTarget;
use aa_numeric::Precision;
use aa_runtime::{restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},
    scheduler_owners::Target,localization::Localization,settings::Settings};
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use aa_world::queues::{Callbacks,Delivery,Queues};
use std::sync::Arc;

struct Received(Vec<Delivery>);
impl Callbacks for Received {
    fn deliver(&mut self,_:&mut Queues,event:Delivery)->aa_world::queues::Result<()> {
        self.0.push(event);Ok(())
    }
}

#[test]
#[ignore="requires AA_GAME_ROOT original/v14 assets"]
fn scheduled_instance_absent_from_saved_table_keeps_identity_across_staging() {
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();
    let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();
    let resources=Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join(
        "reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap()).unwrap();
    let mut attempt=Attempt::load_with_source_setup(&map,&actors,settings,text,Precision::Double53,u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources)},|catalog| {
            let receipt=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
            aa_runtime::combat_source_completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;
            aa_runtime::combat_source_completion::compute(catalog)
        }).unwrap();
    assert!(attempt.report.events.iter().all(|e|e.error.is_none()));
    let id=aa_actors::ActorId::from_raw(781);
    let saved=UpdatableTarget::Component{owner:id.raw(),slot:ComponentSlot::Properties.index() as u8};
    // Remove only this saved-ID address. Init-created events can exist for a
    // real instance whose native saved ID is-1; all tokens must still move.
    assert!(attempt.updatables.remove(&saved));
    let owner=attempt.scheduler_owners.restoring(saved).unwrap();
    let data=0x7251;
    let event=attempt.queues.add_after(3,owner,data,0).unwrap();
    assert_eq!(attempt.queues.owner_head(owner),Some(event));
    let mut staged=attempt.into_staged().unwrap();
    let object=staged.actors.key(id).unwrap();
    assert_eq!(staged.owners.target(owner).unwrap(),Target::Component{object,slot:6});
    assert_eq!(staged.queues.owner_head(owner),Some(event));
    assert!(staged.owners.get(Target::Restoring(saved)).is_err());
    assert_eq!(staged.actors.component(id,ComponentSlot::Properties).unwrap().unwrap().slot(),ComponentSlot::Properties);
    staged.queues.clock.end_bits=1.0f32.to_bits();
    let mut received=Received(Vec::new());
    staged.queues.update(3,&mut received).unwrap();
    assert_eq!(received.0.iter().map(|e|(e.owner,e.data)).collect::<Vec<_>>(),vec![(owner,data)]);
    assert!(!staged.queues.has_owner(owner));
    assert!(!staged.report.runtime_publication_ready);
}
