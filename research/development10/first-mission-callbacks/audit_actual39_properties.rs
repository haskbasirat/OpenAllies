//! Additional read-only property names; not a fresh Actor or Staged owner.
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,native::layout_data::{self,PropertyRef},sources::{SourceLimits,V14Sources}};
fn main(){
 let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();
 let data=actors.native_layout_data(Precision::Double53).unwrap();
 let refs=layout_data::native_actor_properties(&actors,&data,"ger_corps",Precision::Double53).unwrap();
 let values=refs.iter().map(|reference|match reference {
  PropertyRef::Registered(index)=>serde_json::json!({"reference":reference,"source":data.properties.get(*index as usize).unwrap()}),
  PropertyRef::Embedded(_)=>panic!("actual source unexpectedly embedded"),
 }).collect::<Vec<_>>();
 println!("{}",serde_json::json!({"scope":"Read-only source initialProperty list in exact native head-first order; duplicates retained. No publication or gameplay execution.","initial_properties":values}));
}
