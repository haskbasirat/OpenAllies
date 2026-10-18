#![allow(dead_code)]
mod actor_projection {
use aa_runtime::{catalog::Catalog,definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode,native::{TypedValue,ScalarType},resources::Provenance,turrets::TurretDefinition};
type Result<T>=std::result::Result<T,String>;
#[derive(Debug)]
pub(super) struct Fields{
    actor:bool,precision:Precision,source:Provenance,
    turrets:Vec<TurretDefinition>,tires:Vec<Vec<Provenance>>,tracks:Vec<Vec<Provenance>>,circumference:u32,
}
///Readonly identity capability for the actual canonical Data allocation.
///There is no raw count/value constructor or detached source-owner clone.
fn actor_node<'a>(catalog:&'a Catalog<'_>,d:&RuntimeDefinition)->Result<ResolvedNode<'a>>{
    let mut found=None;
    for tag in ["Thing","Unit","Projectile","VFX"]{
        for node in catalog.actors.resolved_definitions(tag).map_err(|e|e.to_string())?{
            if node.sources().first()==Some(&d.proof.source){
                if found.is_some(){return Err("Actor view canonical source position is ambiguous".into())}
                found=Some(node);
            }
        }
    }
    let node=found.ok_or("Actor view canonical source absent")?;
    if node.sources()!=d.proof.template_chain{return Err("Actor view source layers differ".into())}Ok(node)
}
fn project(catalog:&Catalog<'_>,d:&RuntimeDefinition,precision:Precision)->Result<Fields>{
    let actor=d.proof.category.native_kind==2;
    //4624B0 constructor stores1FC=0,244=0,23C=0. Only the reached
    //Performer/Mover source branches below replace those actual fields.
    let mut fields=Fields{actor,precision,source:d.proof.source.clone(),turrets:vec![],tires:vec![],tracks:vec![],circumference:0};
    if actor{
        fields.turrets=catalog.actors.native_turrets(&d.proof.id,precision).map_err(|e|e.to_string())?;
        let root=actor_node(catalog,d)?;
        if let Some(mover)=root.child("MoverComponent").map_err(|e|e.to_string())?{
            fields.circumference=match mover.field("tire_circumference",precision).map_err(|e|e.to_string())?{
                //465A6E calls5B5130 GetFixed, then465A73 spills binary32
                //toData23C. The source declaration is FIXED, not FLOAT.
                Some(TypedValue::Scalar(v))if v.field_type==ScalarType::Fixed=>v.bits,
                _=>return Err("ActorData23C requires actual typed Mover tire_circumference".into()),
            };
            aa_numeric::Binary::from_f32_bits(fields.circumference).map_err(|e|e.to_string())?;
            if f32::from_bits(fields.circumference)==0.0{return Err("ActorData loader rejects zero Mover tire_circumference".into())}
            for tire in mover.children("Tire").map_err(|e|e.to_string())?{fields.tires.push(tire.sources());}
            //464BD0's distinct TrackInfo25C/260 list. It must never be
            //confused with Tire240/244; this binds count/source identities,
            //not the positive42D7B0/42D830 VActor Track payload.
            for track in mover.children("TrackInfo").map_err(|e|e.to_string())?{fields.tracks.push(track.sources());}
        }
    }
    //The native consumers compare counts as signed32. Do not wrap an owned
    //Rust list length or silently truncate it to a fabricated field value.
    i32::try_from(fields.turrets.len()).map_err(|_|"ActorData1FC count exceeds native signed domain")?;
    i32::try_from(fields.tires.len()).map_err(|_|"ActorData244 count exceeds native signed domain")?;
    i32::try_from(fields.tracks.len()).map_err(|_|"ActorData260 count exceeds native signed domain")?;
    Ok(fields)
}

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
}
mod base_texture {
use serde::Deserialize;
use std::{collections::BTreeMap,sync::Arc};
use aa_runtime::resource_model_payload::Metadata;
type Result<T>=std::result::Result<T,String>;
#[derive(Debug,Deserialize,PartialEq,Eq)]struct Property{kind:String,block:u32,#[serde(default)]flags:u32,#[serde(default)]slots:Vec<Option<Descriptor>>,#[serde(default)]shader_textures:u32,#[serde(default)]extras:Vec<serde_json::Value>,#[serde(skip)]generated:BTreeMap<String,Extra>}
#[derive(Debug,Deserialize,PartialEq,Eq)]struct Descriptor{texture:Option<String>,source:Option<u32>,filter:u32}
#[derive(Debug,PartialEq,Eq)]enum Extra{Integer(u32),Boolean(u32),Text(Vec<u16>),Processed(Arc<Metadata>),TextureNames(Arc<Vec<Vec<u16>>>) }
fn ascii_equal(a:&[u16],b:&[u16])->bool{a.len()==b.len()&&a.iter().zip(b).all(|(a,b)|a==b||(*a<=127&&*b<=127&&(*a as u8).eq_ignore_ascii_case(&(*b as u8))))}
enum TextureOperand{Absent,Internal,External(Vec<u16>)}
fn apply_base_texture(property:&mut Property,texture:&TextureOperand,selected:Option<&[u16]>,particles:bool)->Result<()>{
 if particles{return Ok(())}
 if property.extras.iter().filter(|v|v["extra_name"]=="bt").count()>1{return Err("878170 duplicate named bt removal order requires actual alias owner".into())}
 property.extras.retain(|v|v["extra_name"]!="bt");property.generated.remove("bt");
 let qualifies=match texture{TextureOperand::Absent=>selected.is_none(),TextureOperand::Internal=>true,TextureOperand::External(name)=>selected.is_none_or(|s|ascii_equal(s,name))};
 if qualifies{property.generated.insert("bt".into(),Extra::Boolean(1));}Ok(())
}
//87A820 static-unskinned domain: every collected texture has ordinary uses
//and no NiParticles uses. The native filename, never selected disk spelling,
//determines the preferred basename; last matching *base fallback is retained.
fn select_base_texture(filename:&[u16],textures:&[Vec<u16>],used:&[bool])->Result<Option<Vec<u16>>>{
 if used.len()!=textures.len(){return Err("actual native texture-use vector differs".into())}
 if textures.is_empty(){return Ok(None)}if textures.len()==1&&used[0]{return Ok(Some(textures[0].clone()))}
 let filename=String::from_utf16(filename).map_err(|e|e.to_string())?;if !filename.is_ascii(){return Err("87A820 native wide-to-narrow filename conversion is required".into())}
 let name=filename.rsplit(['\\','/',':']).next().ok_or("actual Model filename is absent")?;let stem=name.rsplit_once('.').map_or(name,|v|v.0);let mut fallback=None;
 for (texture,used) in textures.iter().zip(used){if !used{continue}let name=String::from_utf16(texture).map_err(|e|e.to_string())?;if !name.is_ascii(){return Err("87A820 non-ASCII texture-name comparison is required".into())}
  if name.len()>=stem.len()&&name[..stem.len()].eq_ignore_ascii_case(stem)&&((name.len()==stem.len()+8&&name[stem.len()..stem.len()+5].eq_ignore_ascii_case("base."))||(name.len()==stem.len()+4&&name.as_bytes()[stem.len()]==b'.')){return Ok(Some(texture.clone()))}
  if let Some(dot)=name.rfind('.') {if dot>4&&name[dot-4..dot].eq_ignore_ascii_case("base"){fallback=Some(texture.clone())}}
 }Ok(fallback)
}
 #[test]fn base_texture_selector_and_same_property_mutation_match_native(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-model-resource-load/native-model-base-texture.json")).unwrap();
  let wide=|v:&serde_json::Value|v.as_str().unwrap().encode_utf16().collect::<Vec<_>>();
  let property=|initial:&serde_json::Value|Property{kind:"NiTexturingProperty".into(),block:17,flags:0,slots:vec![],shader_textures:0,extras:initial.as_array().unwrap().iter().map(|v|serde_json::json!({"extra_name":v[0],"kind":"NiBooleanExtraData","value":v[1]})).collect(),generated:BTreeMap::new()};
  let projection=|p:&Property|{let mut rows=p.extras.iter().map(|e|serde_json::json!({"name":e["extra_name"],"value":e["value"],"boolean_vtable":e["kind"]=="NiBooleanExtraData"})).collect::<Vec<_>>();rows.extend(p.generated.iter().map(|(name,v)|match v{Extra::Boolean(value)=>serde_json::json!({"name":name,"value":value,"boolean_vtable":true}),_=>panic!("unexpected base-texture extra")}));rows.sort_by(|a,b|a["name"].as_str().cmp(&b["name"].as_str()));serde_json::Value::Array(rows)};
  for row in fixture["rows"].as_array().unwrap(){match row["kind"].as_str().unwrap(){
   "choice"=>{let input=&row["input"];let names=input["names"].as_array().unwrap().iter().map(wide).collect::<Vec<_>>();let used=input["used"].as_array().unwrap().iter().map(|v|v.as_bool().unwrap()).collect::<Vec<_>>();let selected=select_base_texture(&wide(&input["file"]),&names,&used).unwrap();let expected=row["selected_index"].as_u64().map(|i|names[i as usize].clone());assert_eq!(selected,expected,"{input}");},
   "mutation"=>{let input=&row["input"];if !input["property"].as_bool().unwrap(){assert_eq!(row["node_before"],row["node_after"]);continue}let mut p=property(&input["initial"]);let texture=match &input["texture"]{serde_json::Value::Null=>TextureOperand::Internal,serde_json::Value::Bool(false)=>TextureOperand::Absent,v if v=="wrong-class"=>TextureOperand::Absent,v=>TextureOperand::External(wide(v))};let selected=input["selected"].as_str().map(|v|v.encode_utf16().collect::<Vec<_>>());apply_base_texture(&mut p,&texture,selected.as_deref(),input["kind"]=="particles").unwrap();assert_eq!(projection(&p),row["after"],"{input}");assert_eq!(row["node_before"],row["node_after"]);},
   "recursive"=>{let mut child=property(&serde_json::json!([]));let mut particles=property(&serde_json::json!([["bt",0]]));let selected="A.DDS".encode_utf16().collect::<Vec<_>>();let texture=TextureOperand::External("a.dds".encode_utf16().collect());apply_base_texture(&mut child,&texture,Some(&selected),false).unwrap();apply_base_texture(&mut particles,&texture,Some(&selected),true).unwrap();assert_eq!(projection(&child),row["child_property"]);assert_eq!(projection(&particles),row["particle_property"]);},
   "shared_property"=>{let mut p=property(&serde_json::json!([]));let selected="A.DDS".encode_utf16().collect::<Vec<_>>();let texture=TextureOperand::External("a.dds".encode_utf16().collect());for _ in 0..2{apply_base_texture(&mut p,&texture,Some(&selected),false).unwrap();}assert_eq!(projection(&p),row["left"]);assert_eq!(projection(&p),row["right"]);assert_eq!(p.generated.len(),1);},
   other=>panic!("unreplayed native base-texture case {other}")
  }}
 }
}
