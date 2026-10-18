fn main(){
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
 use aa_runtime::{catalog::Catalog,settings::Settings,load_view_variables::NativeOverrides,combat_source_receipt::VerifiedStartupReceipt};
 let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
 catalog.settings=Some(Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 let aliases=aa_runtime::view_asset_aliases::State::from_fresh_sources(&catalog,&mut files,NativeOverrides::ControlledNone).unwrap();
 let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());let mut rows=vec![];
 for key in aa_runtime::view_init_assets::alias_keys().chain(["TextureEnvironmentMap"]){
  let alias=aliases.alias_410cb0(&catalog,&bank,key).unwrap();let name=String::from_utf16(alias.units()).unwrap();
  let mut row=serde_json::json!({"key":key,"query":name});
  if key.starts_with("VFX"){
   row["actual_find"]=match aa_runtime::view_vfx_source::find(&catalog,&bank,alias.units()){
    Ok(Some(effect))=>serde_json::json!({"id":effect.id(),"kind":effect.kind(&catalog,&bank).unwrap(),"sources":effect.sources()}),
    Ok(None)=>serde_json::Value::Null,Err(e)=>serde_json::json!({"error":e}),
   };
  }else{
   row["exact_source_matches"]=serde_json::json!(files.catalog().into_iter().filter(|f|f.logical_path.eq_ignore_ascii_case(&name)).collect::<Vec<_>>());
   row["basename_source_matches"]=serde_json::json!(files.catalog().into_iter().filter(|f|f.logical_path.rsplit('/').next().is_some_and(|p|p.eq_ignore_ascii_case(name.rsplit('/').next().unwrap()))).collect::<Vec<_>>());
  }
  rows.push(row);
 }
 println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"Actual completed-source Bank and current aliases from frozen runtime50 rlib; no native complete-loader claim","rows":rows})).unwrap());
}
