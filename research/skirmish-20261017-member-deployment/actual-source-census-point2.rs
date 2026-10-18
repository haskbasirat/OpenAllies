//!Read-only actual source census at three explicit lab precisions.
//!No runtime Bank, startup precision or completed World is manufactured.
use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},native::TypedValue};
use aa_numeric::Precision;
fn main(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let catalog=ActorCatalog::load_v14(&mut files).unwrap();
 let roots=["Thing","Unit","Projectile","VFX"].into_iter().flat_map(|tag|catalog.resolved_definitions(tag).unwrap()).collect::<Vec<_>>();
 let mut report=vec![];
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  let backrefs=catalog.native_recruit_backrefs(pc).unwrap();let mut rows=vec![];let mut absent=0;
  for actor in &catalog.actors{
   if catalog.native_category(&actor.id).unwrap().native_kind!=2{continue}
   let node=roots.iter().find(|n|n.sources().first()==Some(&actor.source)).unwrap();
   let info=node.child("DenizenComponent").unwrap().and_then(|d|d.child("DeploymentInfo").unwrap());
   if let Some(info)=info{
    let mut words=vec![];let mut error=None;
    for name in ["creation_offset","deployment_offset"]{
     match info.field(name,pc){Ok(Some(TypedValue::Point{bits,..}))if bits.len()==2=>words.extend(bits),other=>{error=Some(format!("{name}: {other:?}"));break;}}
    }
    rows.push(serde_json::json!({"id":actor.id,"source":actor.source,"sources":info.sources(),"words":words,"error":error,"backrefs":backrefs.definition(&actor.id).unwrap().ids}));
   }else{absent+=1;}
  }
  report.push(serde_json::json!({"precision":pc,"absent":absent,"present":rows.len(),"rows":rows}));
 }
 let value=serde_json::json!({"scope":"actual canonical source census at explicit lab precision only; no runtime Bank/startup or World claim","source_identities":catalog.sources,"precisions":report});
 std::fs::write("research/skirmish-20261017-member-deployment/actual-source-census-point2.json",serde_json::to_vec_pretty(&value).unwrap()).unwrap();
 for pc in value["precisions"].as_array().unwrap(){println!("{} present={} absent={} typed_errors={}",pc["precision"],pc["present"],pc["absent"],pc["rows"].as_array().unwrap().iter().filter(|r|!r["error"].is_null()).count());for row in pc["rows"].as_array().unwrap(){if !row["error"].is_null(){println!("{} {}",row["id"],row["error"]);}}}
}

