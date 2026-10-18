use super::*;
use crate::{catalog::Catalog,combat_source_completion as completion,combat_source_receipt::VerifiedStartupReceipt,
 fx_graph::{self,CreationInputs},definition_fx::ListKind,settings::Settings,fx_manager};
use aa_numeric::Precision;
use aa_mapworld::restore::RestoreDefinitions;
use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
struct Inputs;
impl CreationInputs for Inputs{
 fn muted(&mut self)->Result<bool,String>{Ok(false)}
 fn creator_kingdom(&mut self)->Result<Option<aa_actors::ActorId>,String>{Ok(None)}
 fn current_position(&mut self)->Result<[u32;3],String>{Ok([0;3])}
 fn current_thing(&mut self)->Result<Option<aa_actors::ActorId>,String>{Ok(Some(aa_actors::ActorId::from_raw(2740)))}
 fn current_os_thread_id(&mut self)->Result<u32,String>{Ok(41)}
 fn application_os_thread_id(&mut self)->Result<u32,String>{Ok(42)}
 fn view_time(&mut self)->Result<u32,String>{panic!("explicit worker fixture uses actual world-time branch")}
 fn world_time(&mut self)->Result<u32,String>{Ok(0x3f800000)}
}
struct Enqueue;
impl fx_manager::Effects<Command> for Enqueue{
 fn deliver(&mut self,_:u64,_:Option<Command>)->Result<(),String>{panic!("manager starts unattached")}
 fn destroy(&mut self,_:Command)->Result<(),String>{panic!("fixture starts unmuted")}
}
struct Destination<'a>(&'a mut State);
impl fx_manager::AttachmentEffects<Command> for Destination<'_>{
 fn deliver(&mut self,view:u64,command:&mut Option<Command>)->Result<(),String>{assert_eq!(view,7);self.0.accept_43b870(command).map(|_|())}
 fn destroy(&mut self,_:&mut Command)->Result<(),String>{panic!("fixture starts unmuted")}
}
struct StopPrefix;
impl crate::view_init_prefix::Services for StopPrefix{
 type Grid=crate::view_grid_state::State<()>;
 fn begin_screen_7040e0(&mut self)->Result<(),String>{Err("explicit direct-child laboratory stop".into())}
 fn reset_user_746e30(&mut self)->Result<(),String>{panic!("after stop")}
 fn resource_context_5ae0a0(&mut self)->Result<(),String>{panic!("after stop")}
 fn construct_grid_430290(&mut self)->Result<Option<Self::Grid>,String>{panic!("after stop")}
 fn resetting_editor_variables_diagnostic(&mut self)->Result<(),String>{panic!("after stop")}
 fn reset_view_variables(&mut self)->Result<(),String>{panic!("after stop")}
}
#[test]
#[ignore="requires AA_GAME_ROOT source-bound command; direct attachment lab, not initialized VWorld"]
fn actual_source_commands_move_once_manager_to_same_vworld_and_null_retains_prefix(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let actors=ActorCatalog::load_v14(&mut sources).unwrap();
 let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);
 let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut sources).unwrap();
 completion::install(&catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals()).unwrap();completion::compute(&catalog).unwrap();
 let definition=catalog.resolve(&"barbed_wire".encode_utf16().collect::<Vec<_>>(),Precision::Double53).unwrap();
 let graph=std::sync::Arc::new(fx_graph::Graph::compile(&catalog,&definition,ListKind::Created).unwrap());assert!(!graph.roots().is_empty());
 let graph_address=std::sync::Arc::as_ptr(&graph);let mut queue=fx_manager::State::from_native_constructor();let mut enqueue=Enqueue;
 for _ in 0..3{let command=fx_graph::construct_created(graph.clone(),&mut Inputs).unwrap().unwrap();
  assert!(command.fxl_definition().is_none());assert!(command.fxl_parent_is_self());assert!(command.view_next().is_none());assert_eq!(command.word(0x68),Some(0));
  queue.submit(Some(command),0,&mut enqueue).unwrap();}
 queue.submit(None,0,&mut enqueue).unwrap();
 let mut world=crate::view_init_world::State::<(),()>::from_constructor_fields(());
 assert_eq!(world.fx_commands().native_count(),0);
 let mut revisions=crate::world_revision::Counter::fresh_application();let revision=revisions.construct();
 let mut foreign_app=crate::world_revision::Counter::fresh_application();let foreign=foreign_app.construct();
 assert!(world.fx_commands_mut_for(&revision).is_err());
 //Bind the real constructor identity and stop the first reached service.
 //This does not manufacture a completed prefix or initialized view.
 assert!(world.initialize_prefix(&revision,&mut StopPrefix).is_err());
 assert!(world.fx_commands_mut_for(&foreign).is_err());assert!(!world.prefix().initialized());
 let state=world.fx_commands_mut_for(&revision).unwrap();
 assert!(queue.attach_view(Some(7),&mut Destination(state)).unwrap_err().contains("429023"));
 assert_eq!(queue.attached_view(),Some(7));assert!(matches!(queue.detached_command(),Some(None)));assert_eq!(queue.pending().count(),0);
 assert_eq!(state.native_count(),3);assert_eq!(state.owned_count(),3);
 let mut key=state.head().unwrap();let mut seen=0;
 loop{let command=state.command(key).unwrap();assert_eq!(std::sync::Arc::as_ptr(command.graph()),graph_address);assert_eq!(command.source(),Some(aa_actors::ActorId::from_raw(2740)));
  assert_eq!(command.word(0x38),Some(0x3f800000));seen+=1;
  if let Some(next)=command.view_next(){assert_eq!(command.word(0x68),None);key=next;}else{assert_eq!(Some(key),state.tail());break}
 }
 assert_eq!(seen,3);assert_eq!(std::sync::Arc::strong_count(&graph),4);
}
