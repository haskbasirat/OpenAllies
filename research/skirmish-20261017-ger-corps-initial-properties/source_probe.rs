use aa_runtime::{catalog::Catalog,combat_source_receipt::VerifiedStartupReceipt,load_property_instances::State};
use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits,native::{TypedValue,layout_data::{self,PropertyRef}}};
fn main(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
 catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
 let binding=catalog.actor_bindings("ger_corps",pc).unwrap();let mut bank=std::mem::take(&mut*catalog.definition_bank.borrow_mut());
 let data=catalog.layout_data(pc).unwrap();let mut state=State::from_world_constructor();state.bind_in_bank(&catalog,&mut bank).unwrap();
 let roots=actors.resolved_definitions("Property").unwrap();let mut rows=Vec::new();
 for reference in layout_data::native_actor_properties(&actors,&data,"ger_corps",pc).unwrap(){
  let PropertyRef::Registered(index)=reference else{panic!("embedded initial")};let p=&data.properties[index as usize];
  let source=roots.iter().find(|n|n.sources()==p.sources).unwrap();let definition=state.definition_for_reference(reference).unwrap();let facts=state.facts(definition).unwrap();
  let fx=["WhenAdded","WhenRemoved"].into_iter().map(|name|{let nodes=source.children(name).unwrap();serde_json::json!({"name":name,"nodes":nodes.iter().map(|n|{let kind=match n.field("type",pc).unwrap().unwrap(){TypedValue::String{value,..}=>value,_=>panic!("type")};serde_json::json!({"type":kind,"sources":n.sources()})}).collect::<Vec<_>>()})}).collect::<Vec<_>>();
  rows.push(serde_json::json!({"reference":reference,"canonical_id":facts.id(),"definition_token":definition.0,"source":p.sources,"compiled_definition":state.definitions().get(&definition).unwrap(),"required":facts.required().iter().map(|v|bank.entry(v.0).unwrap().key.name.clone()).collect::<Vec<_>>(),"performer_cv_bias":facts.performer_cv_bias().0,"pass_down_members":facts.pass_down_members(),"pass_down_recruit":facts.pass_down_recruit(),"pass_down_denizen":facts.pass_down_denizen(),"fx":fx}));
 }
 println!("{}",serde_json::json!({"scope":"Actual same-Bank source definition/initial-list and compiled property metadata only; no current Actor, World, insertion, FX or mission execution","source_precision":pc,"component_mask":binding.definition.component_mask,"native_class":binding.definition.native_class,"catalog_bank_empty_after_move":catalog.definition_bank.borrow().combat_source().is_err(),"rows":rows}));
}
