"""Mechanically include current production helpers for bounded source replay.

No alternative algorithms: the Actor projection functions and selected-model
base-texture functions/test are copied verbatim. The surrounding real Catalog
test exercises source projection only; canonical Data lifetime remains covered
by the separate runtime test and is not claimed by this host.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
actor=ROOT/'crates/aa-runtime/src/data_actor_view_source.rs'
model=ROOT/'crates/aa-client/src/model_payload_backend.rs'
a=actor.read_text();m=model.read_text()
projection=a[a.index('#[derive(Debug)]'):a.index('impl State{')]
projection=projection.replace('pub struct ActorView{data:Data}\n','')
functions=m[m.index('fn ascii_equal('):m.index('impl Model for Selected{')]
property=m[m.index('#[derive(Debug,Deserialize,PartialEq,Eq)]struct Property'):m.index('#[derive(Debug,Deserialize)]struct Controller')]
extra=m[m.index('#[derive(Debug,PartialEq,Eq)]enum Extra'):m.index('#[derive(Debug)]struct Selected')]
test=m[m.index(' #[test]fn base_texture_selector_and_same_property_mutation_match_native'):m.index(' #[test]fn first_bone_query_uses_own_property')]
actor_test=r'''
#[test] fn actual_mover_fixed_projection_in_all_selected_source_branches(){
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
 catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
 let source_nodes=["Thing","Unit","Projectile","VFX"].into_iter().flat_map(|tag|actors.resolved_definitions(tag).unwrap()).collect::<Vec<_>>();
 let mut selected=[None,None,None];
 for actor in &actors.actors{
  if actors.native_category(&actor.id).unwrap().native_kind!=2{continue}
  let turrets=actors.native_turrets(&actor.id,pc).unwrap();
  let node=source_nodes.iter().find(|n|n.sources().first()==Some(&actor.source)).unwrap();
  let mover=node.child("MoverComponent").unwrap();
  let tires=mover.as_ref().map(|n|n.children("Tire").unwrap().len()).unwrap_or(0);
  for(slot,eligible)in [(0,turrets.is_empty()&&mover.is_none()),(1,!turrets.is_empty()),(2,tires>0)]{
   if selected[slot].is_none()&&eligible{selected[slot]=Some(catalog.actor_bindings(&actor.id,pc).unwrap());}
  }
  if selected.iter().all(Option::is_some){break}
 }
 assert!(selected.iter().all(Option::is_some));
 let mut rows=vec![];
 for (i,binding) in selected.into_iter().enumerate(){let binding=binding.unwrap();let fields=project(&catalog,&binding.definition,pc).unwrap();
  if i==0{assert_eq!((fields.turrets.len(),fields.tires.len(),fields.circumference,fields.tracks.len()),(0,0,0,0));}
  if i==1{assert!(!fields.turrets.is_empty());}
  if i==2{assert!(!fields.tires.is_empty());assert_ne!(fields.circumference,0);let root=actor_node(&catalog,&binding.definition).unwrap();let mover=root.child("MoverComponent").unwrap().unwrap();let value=mover.field("tire_circumference",pc).unwrap().unwrap();assert_eq!(value.fixed_bits().unwrap(),fields.circumference);}
  rows.push(serde_json::json!({"id":binding.definition.proof.id,"turrets":fields.turrets.len(),"tires":fields.tires.len(),"circumference":fields.circumference,"tracks":fields.tracks.len()}));
 }
 println!("{}",serde_json::json!({"scope":"exact production source projection, not canonical Data/World lifecycle","rows":rows}));
}
'''
host='''#![allow(dead_code)]
mod actor_projection {
use aa_runtime::{catalog::Catalog,definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode,native::{TypedValue,ScalarType},resources::Provenance,turrets::TurretDefinition};
type Result<T>=std::result::Result<T,String>;
'''+projection+actor_test+'}\nmod base_texture {\nuse serde::Deserialize;\nuse std::{collections::BTreeMap,sync::Arc};\nuse aa_runtime::resource_model_payload::Metadata;\ntype Result<T>=std::result::Result<T,String>;\n'+property+extra+functions+test+'}\n'
out=HERE/'payload_source_harness.rs';out.write_text(host)
pins={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [actor,model,out]}
(HERE/'payload-source-harness-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
print(json.dumps(pins))
