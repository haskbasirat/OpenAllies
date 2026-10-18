//!879A90 static tag/context branches. The caller supplies actual named-extra
//!lookups and retains all mutations on its existing Node/Property allocations.
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]pub(super) struct Context{pub shadow:bool,pub shadow_two:bool,pub replacement_owner:Option<usize>}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub(super) enum VertexColor{Keep,KeepDiagnostic,RemoveDiagnostic}
pub(super) fn enter(mut inherited:Context,node:usize,geometry:bool,sg:bool,sg_two:bool,replacement:bool)->(Context,bool){
 if sg{inherited.shadow=true;inherited.shadow_two=false}if sg_two{inherited.shadow=true;inherited.shadow_two=true}
 let mut add=false;if !inherited.shadow{if replacement{inherited.replacement_owner=Some(node)}else if geometry&&inherited.replacement_owner.is_some(){add=true}}
 (inherited,add)
}
pub(super) fn vertex_color(mode:u32,lighting:u32,has_parent:bool)->VertexColor{
 if mode!=0{VertexColor::KeepDiagnostic}else if lighting==1&&has_parent{VertexColor::RemoveDiagnostic}else{VertexColor::Keep}
}
pub(super) fn replacement_flags(old:u16,sets:u8,additional:u16)->u16{(old&0xfc0)|(sets as u16&0x3f)|additional}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn native_static_tag_inheritance_and_vertex_color_branches_match(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-static-metadata.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];let tag=input["tag"].as_str().unwrap();let inherited=input["inherited"].as_bool().unwrap();let(root,_)=enter(Context::default(),0,false,inherited&&tag=="Sg",inherited&&tag=="Sg2",inherited&&tag=="RepColor");let(child,added)=enter(root,1,true,!inherited&&tag=="Sg",!inherited&&tag=="Sg2",!inherited&&tag=="RepColor");
   assert_eq!(child.shadow,row["output"]["metadata"][2]==0);assert_eq!(added||(!inherited&&tag=="RepColor"),row["output"]["geometry_rep"].as_bool().unwrap());
   let vc=&input["vc"];if let Some(vc)=vc.as_array(){let action=vertex_color(vc[0].as_u64().unwrap()as u32,vc[1].as_u64().unwrap()as u32,true);assert_eq!(action!=VertexColor::RemoveDiagnostic,row["output"]["vertex_color_present"].as_bool().unwrap());let line=match action{VertexColor::Keep=>None,VertexColor::KeepDiagnostic=>Some(0x410),VertexColor::RemoveDiagnostic=>Some(0x41d)};assert_eq!(line,row["output"]["events"].as_array().unwrap().iter().find_map(|event|event["diagnostic_line"].as_u64()));}
  }
 }
 #[test]fn native_geometry_replacement_flag_bits_match_all_operands(){let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-model-resource-load/native-geometry-replace.json")).unwrap();for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];let old=input["flags"].as_u64().unwrap()as u32;let actual=(old&0xffff0000)|replacement_flags(old as u16,input["uv_sets"].as_u64().unwrap()as u8,input["additional_flags"].as_u64().unwrap()as u16)as u32;assert_eq!(actual,row["output"]["flags_word"].as_u64().unwrap()as u32);}}
}
