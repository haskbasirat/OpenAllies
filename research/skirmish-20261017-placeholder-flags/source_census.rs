//! Read-only diagnostic census; not a completed native source certificate.
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog, sources::{SourceLimits,V14Sources}};
use aa_runtime::{definitions::Bank,skirmish_start::{Definitions,substitution,Substitution}};
use serde_json::json;
fn main() {
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();
 let mut all=Vec::new();
 for pc in[Precision::Single24,Precision::Double53,Precision::Extended64] {
  let mut bank=Bank::default();let defs=Definitions::bind(&actors,&mut bank,pc).unwrap();
  let mut placeholders=Vec::new();
  for(&token,def)in &defs.things {
   if def.category!=31 {continue}
   let flags=actors.native_final_flags(&def.ids,pc).unwrap();
   let shape=actors.native_object_definition(&def.ids,pc).unwrap().blocking.words;
   let mut rows=Vec::new();
   for nation in &actors.native_data_registry("Nation").unwrap().entries {
    let entry=&defs.nations[&nation.ids.to_ascii_lowercase()];
    let target=match substitution(entry,token) {
     Substitution::Missing=>json!({"kind":"missing"}),
     Substitution::Present(None)=>json!({"kind":"null"}),
     Substitution::Present(Some(id))=>{
      let replacement=&defs.things[&id];
      let rflags=actors.native_final_flags(&replacement.ids,pc).unwrap();
      let rshape=actors.native_object_definition(&replacement.ids,pc).unwrap().blocking.words;
      // Actual standard ActorData A05540+3C is4C9FA0, which returns0.
      // Category/kind are not an invented virtual truth-value.
      assert_eq!(def.native_kind,2);
      let mask=0xa74c00u32;
      json!({"kind":"present","id":replacement.ids,"flags":rflags,"mask":mask,"unchanged":flags&mask==rflags&mask,"shape":rshape,"source":actors.find(&replacement.ids).unwrap().source})
     }
    };
    rows.push(json!({"nation":nation.ids,"source":nation.source,"target":target}));
   }
   placeholders.push(json!({"id":def.ids,"kind":def.native_kind,"flags":flags,"shape":shape,"rows":rows,"source":actors.find(&def.ids).unwrap().source}));
  }
  all.push(json!({"pc":format!("{pc:?}"),"definitions":defs.things.len(),"random_target_count":defs.random.len(),"placeholders":placeholders}));
 }
 let upgrade=actors.construction_upgrade_census().unwrap();
 let out=json!({"scope":"read-only actual source census, no final verifier certificate","sources":actors.sources,"precisions":all,"upgrade_census":upgrade});
 std::fs::write("research/skirmish-20261017-placeholder-flags/actual-source-census.json",serde_json::to_vec_pretty(&out).unwrap()).unwrap();
 println!("actual census captured");
}
