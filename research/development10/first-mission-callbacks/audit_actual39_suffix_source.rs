//! Read-only source inventory paired with the retained actual39 failure.
//! No Actor, Staged, World, queue or gameplay callback is constructed here.
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,native::layout_data,sources::{SourceLimits,V14Sources}};
fn main(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();
 let pc=Precision::Double53;
 let data=actors.native_layout_data(pc).unwrap();
 let actor=actors.resolved_actor("ger_corps").unwrap();
 let properties=layout_data::native_actor_properties(&actors,&data,"ger_corps",pc).unwrap();
 let mover=actor.child("MoverComponent").unwrap();
 let tracks=mover.as_ref().map(|m|m.children("TrackInfo").unwrap().len());
 let view=actor.child("View").unwrap().unwrap();
 let fx=["WhenCreated","WhenConstructed","WhenSelected"].into_iter().map(|name|{
  let roots=view.children(name).unwrap().iter().map(|node|serde_json::json!({
   "sources":node.sources(),"type":node.field("type",pc).unwrap(),
   "vfx_ids":node.field("vfx_ids",pc).unwrap(),
   "chain":node.children("Chain").unwrap().len(),
   "chain_immediate":node.children("ChainImmediate").unwrap().len(),
  })).collect::<Vec<_>>();
  serde_json::json!({"list":name,"count":roots.len(),"roots":roots})
 }).collect::<Vec<_>>();
 let fields=["needs_space_for_settlement_zone","required_object","name_list_ids","numbered"].into_iter().map(|name|{
  (name,format!("{:?}",actors.native_field("ger_corps",name,pc)))
 }).collect::<std::collections::BTreeMap<_,_>>();
 println!("{}",serde_json::json!({
  "scope":"Source-only inventory, paired with separately pinned retained actual39 ger_corps. No gameplay callback or future success.",
  "sources":actor.sources(),"source_precision":53,
  "initial_property_references":properties,
  "initial_property_count":properties.len(),
  "global_registration_slot0":data.properties.first(),
  "mover_component_present":mover.is_some(),"track_count":tracks,
  "fields":fields,"fx":fx,
 }));
}
