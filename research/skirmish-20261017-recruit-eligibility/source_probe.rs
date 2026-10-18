use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources},native::{layout_data::NativeLayoutData,recruit_data}};
use aa_numeric::Precision;
fn main(){
 let game=std::path::Path::new(r"E:\Games\Axis and Alies");
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let source=ActorCatalog::load_v14(&mut files).unwrap();
 let pc=Precision::Double53;let layouts=NativeLayoutData::load(&source,pc).unwrap();
 let targets=recruit_data::actor_recruit_targets(&source,"ger_corps").unwrap();
 let mut ids=targets.ids.clone();let mut at=0;
 while at<ids.len(){if let Some(layout)=layouts.organization_component(&source,&ids[at],pc).unwrap(){for member in layout.parameters.into_iter().flatten(){if !ids.contains(&member){ids.push(member)}}}at+=1;}
 let mut definitions=vec![];let mut projected=vec![];
 for id in &ids{
  let economy=source.native_economy_inputs(id,pc).unwrap();let org=layouts.organization_component(&source,id,pc).unwrap();
  let index=org.as_ref().map(|_|projected.len());
  if let Some(org)=org{
   let layout=layouts.layouts.iter().find(|l|l.ordinal==org.layout).unwrap();assert_eq!(layout.ids,org.layout_ids);
   projected.push(serde_json::json!({"ids":org.layout_ids,"primary":layout.primary_slot,"members":org.parameters.iter().map(|m|m.as_ref().map(|m|ids.iter().position(|id|id==m).unwrap())).collect::<Vec<_>>(),"numbers":layout.entries.iter().map(|e|e.number).collect::<Vec<_>>()}));
  }
  definitions.push(serde_json::json!({"ids":id,"class":economy.recruitable_word,"mask":if index.is_some(){128}else{0},"layout":index,"strict":false,"met":true,"properties_met":true}));
 }
 let payload=serde_json::json!({"scope":"Actual source53 ger_corps recruit target and default layout metadata only. Other laboratory current/requirement inputs are explicit controls, not observed gameplay. mask is only the used Organization bit projection.","sources":source.sources,"definitions":definitions,"layouts":projected,"targets":targets.ids.iter().map(|id|ids.iter().position(|i|i==id).unwrap()).collect::<Vec<_>>()});
 let path=std::path::Path::new("research/skirmish-20261017-recruit-eligibility/actual-source-inputs.json");
 if std::env::args().any(|a|a=="--write"){std::fs::write(path,serde_json::to_string_pretty(&payload).unwrap()+"\n").unwrap()}else{assert_eq!(payload,serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()).unwrap());}
 println!("PASS actual{} targets/{} canonical definitions/{} layouts",targets.ids.len(),ids.len(),projected.len());
}
