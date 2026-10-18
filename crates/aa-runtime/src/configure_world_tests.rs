use super::*;
#[test]
fn actual_native_after_kingdoms_gate_reads_saved_header_lazily() {
 let f:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-configure-guard.json")).unwrap();
 assert_eq!(f["cases"].as_array().unwrap().len(),126);
 for r in f["cases"].as_array().unwrap() {
  let input=&r["input"];let mut reads=0;
  let yes=required(input["load_mode"].as_u64().unwrap()as u32,|| {reads+=1;Ok(input["header_mode"].as_u64().unwrap()as u32)}).unwrap();
  assert_eq!(yes,r["output"]["required_50E8B0"].as_bool().unwrap(),"{r}");
  assert_eq!(reads,usize::from(input["load_mode"]==1));
 }
 for mode in [2,4] {assert!(!required(mode,||Err("unbound header".into())).unwrap())}
 assert!(required(1,||Err("unbound header".into())).is_err());
 assert!(required(1,||Ok(7)).is_err());
 assert!(State::from_world_constructor(false).require_boundary().is_err());
 assert!(State::from_world_constructor(true).require_boundary().is_err());
}
#[test]
fn rng_getter_logs_both_same_owners_before_draw_and_keeps_first_log_on_error() {
 use aa_orders::net::sync::Synchronizer;
 use std::cell::RefCell;
 //A leaf ownership fixture, deliberately not a source/game readiness owner.
 let actors=aa_rules::actors::ActorCatalog::from_files(Vec::new(),aa_rules::tgi::Limits::default());
 let mut a=Synchronizer::new(16);a.reset(0,-1);
 let mut s=Synchronizer::new(16);s.reset(0,-1);
 let a=RefCell::new(a);let s=RefCell::new(s);
 let mut gate_a=||Ok(true);let mut gate_s=||Ok(true);
 let mut catalog=Catalog::new(&actors);
 catalog.bind_configuration_sync(crate::live_clocked::SyncBinding::Shared{owner:&a,gate:&mut gate_a}).unwrap();
 let mut rng=NativeRng::new(0xffffffff);
 assert!(random_word(&catalog,&mut rng).unwrap_err().contains("SyncS"));
 assert_eq!(rng.state,0xffffffff);assert_eq!(a.borrow().entry(1).unwrap().value,0xffffffff);
 catalog.bind_configuration_sync_s(crate::live_clocked::SyncBinding::Shared{owner:&s,gate:&mut gate_s}).unwrap();
 let held=s.borrow_mut();
 assert!(random_word(&catalog,&mut rng).is_err());
 assert_eq!(rng.state,0xffffffff);assert!(a.borrow().entry(2).is_some());drop(held);
 let mut expected=NativeRng::new(0xffffffff);let word=expected.next_u32();
 assert_eq!(random_word(&catalog,&mut rng).unwrap(),word);assert_eq!(rng.state,expected.state);
 assert_eq!(a.borrow().entry(3).unwrap().label,"(int)m_rng.GetSeed()");
 assert_eq!(s.borrow().entry(1).unwrap().value,0xffffffff);
}

#[test]
fn native_queued_kingdom_order_and_required_prefix_precede_final_iterator() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-configure-removal-order.json")).unwrap();
 for row in fixture["cases"].as_array().unwrap() {
  let c=&row["input"];let ids=c["ids"].as_array().unwrap().iter().map(|v|ActorId::from_raw(v.as_u64().unwrap()as u32)).collect::<Vec<_>>();
  let mut attempted=Vec::new();let mut completed=0;
  let result=drain_pending(&ids,|id|{
   attempted.push(id.raw());
   if c["fail_call"].as_u64()==Some(attempted.len()as u64){return Err("required4F53D0".into())}
   completed+=1;Ok(())
  });
  let o=&row["output"];
  assert_eq!(serde_json::json!(attempted),o["attempted"]);
  assert_eq!(completed,o["completed"].as_u64().unwrap());
  assert_eq!(result.is_err(),o["failed"].as_bool().unwrap());
  assert_eq!(result.is_ok(),o["final_iterator_reached"].as_bool().unwrap());
 }
 assert_eq!(fixture["cases"].as_array().unwrap().len(),45);
}
