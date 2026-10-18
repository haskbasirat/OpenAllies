use super::*;
use crate::{actors::ActorCatalog,sources::{Origin,ResolvedFile,sha256},tgi::Limits};
use serde_json::Value;
fn source_catalog(bytes:Vec<u8>)->ActorCatalog{ActorCatalog::from_files(vec![ResolvedFile{logical_path:"field-order-fixture.tgi".into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}],Limits::default())}
fn hex(text:&str)->Vec<u8>{(0..text.len()).step_by(2).map(|i|u8::from_str_radix(&text[i..i+2],16).unwrap()).collect()}
#[test]
fn native_fresh_svar_hash_growth_order_and_lazy_values(){
 let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-svar-field-order/native-field-order.json");
 let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
 for row in fixture["rows"].as_array().unwrap(){
  let catalog=source_catalog(hex(row["source_hex"].as_str().unwrap()));let nodes=catalog.resolved_definitions("SVars").unwrap();assert_eq!(nodes.len(),1);
  let fields=nodes[0].native_svar_fields().unwrap();let native=row["native"]["fields"].as_array().unwrap();
  assert_eq!(fields.iter().map(|f|f.name()).collect::<Vec<_>>(),native.iter().map(|r|r["name"].as_str().unwrap()).collect::<Vec<_>>());
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  for(f,n)in fields.iter().zip(native){
   assert_eq!(u64::from(f.source_kind().unwrap()),n["source_kind"]);assert_eq!(f.source().sha256,sha256(&hex(row["source_hex"].as_str().unwrap())));
   match f.value(pc).unwrap(){
    TypedValue::Integer{value,..}=>assert_eq!(u64::from(value as u32),n["value_word"]),
    TypedValue::Scalar(v)=>assert_eq!(u64::from(v.bits),n["value_word"]),
    TypedValue::Boolean{value,..}=>assert_eq!(u64::from(value),n["value_word"]),
    TypedValue::String{value,..}=>assert_eq!(value,"token"),
    TypedValue::Point{bits,..}=>assert_eq!(bits,[1f32.to_bits(),2f32.to_bits(),(-3f32).to_bits()]),
   }
  }
 }
}
#[test]
fn native_svar_tokens_do_not_eagerly_parse_later_values_or_erase_scope_guards(){
 let catalog=source_catalog(b"[SVars]\n{\n fixed First=1\n fixed Later=unknown_expression\n}\n".to_vec());
 let nodes=catalog.resolved_definitions("SVars").unwrap();let fields=nodes[0].native_svar_fields().unwrap();assert_eq!(fields.len(),2);
 let first=fields.iter().find(|f|f.name()=="First").unwrap();assert_eq!(first.value(Precision::Double53).unwrap().fixed_bits().unwrap(),1f32.to_bits());
 assert!(fields.iter().find(|f|f.name()=="Later").unwrap().value(Precision::Double53).is_err());
 for bytes in [b"[SVars]\n{\n bool Same=true\n bool same=false\n}\n".as_slice(),b"[Template Base]\n{\n int First=1\n}\n[SVars Template=Base]\n{\n int Second=2\n}\n".as_slice()]{
  let catalog=source_catalog(bytes.to_vec());let nodes=catalog.resolved_definitions("SVars").unwrap();assert!(nodes[0].native_svar_fields().is_err());
 }
 let catalog=source_catalog(b"[SVars]\n{\n Later=unknown_inferred_literal\n}\n".to_vec());let nodes=catalog.resolved_definitions("SVars").unwrap();let fields=nodes[0].native_svar_fields().unwrap();assert!(fields[0].source_kind().is_err());
}
#[test]
#[ignore="requires AA_GAME_ROOT original archive and v14 source overlay"]
fn actual_two_svar_sources_all_rows_keep_native_order_type_and_provenance(){
 use crate::sources::{SourceLimits,V14Sources};
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let catalog=ActorCatalog::from_files(vec![sources.read("Game/SVars.tgi").unwrap(),sources.read("Game/SVars_AA.tgi").unwrap()],Limits::default());
 let nodes=catalog.resolved_definitions("SVars").unwrap();assert_eq!(nodes.len(),2);let mut counts=vec![];let mut inferred=vec![];
 for node in nodes{let fields=node.native_svar_fields().unwrap();counts.push(fields.len());
  for field in fields{assert!(field.source().path.starts_with("Game/"));let kind=field.source_kind().unwrap();let value=field.value(Precision::Double53).unwrap();
   if field.declared_type().is_none(){inferred.push((field.name().to_owned(),kind));}drop(value);
  }
 }
 assert_eq!(inferred.len(),3);assert!(inferred.contains(&("DenizenResupplyRateUnderSiege".to_owned(),3)));assert!(inferred.contains(&("SupplyProvideUnderSiege".to_owned(),5)));assert!(inferred.contains(&("DisableSAIWhenGameLost".to_owned(),5)));
 println!("actual two SVars source row counts {counts:?}; inferred {inferred:?}");
}
